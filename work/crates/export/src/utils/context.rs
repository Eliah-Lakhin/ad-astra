////////////////////////////////////////////////////////////////////////////////
// This file is part of "Ad Astra", an embeddable scripting programming       //
// language platform.                                                         //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/ad-astra/blob/master/EULA.md               //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::{
    any::TypeId,
    cell::RefCell,
    env::var,
    hash::{Hash, Hasher},
};

use ahash::{AHashMap, AHasher};
use convert_case::{Case, Casing};
use proc_macro2::{Delimiter, Ident, Spacing, Span, TokenStream, TokenTree};
use quote::{quote_spanned, ToTokens};
use syn::{spanned::Spanned, Item, LitStr, Type};

use crate::utils::{seed_hash_map, seed_hasher, Facade};

pub type OriginRef = Ident;
pub type IdRef = Ident;
pub type SectionName = Ident;
pub type FunctionName = Ident;
pub type TypeName = Ident;
pub type StaticName = Ident;

#[derive(Clone, Copy)]
pub struct Context;

impl ToTokens for Context {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.borrow(|inner| inner.to_tokens(tokens));
    }
}

impl Context {
    #[inline(always)]
    pub fn init(self, item: &Item) {
        INNER.with_borrow_mut(|inner| {
            if inner.is_some() {
                panic!("Internal error. Inner context already initialized.")
            }

            *inner = Some(ContextInner::new(item));
        })
    }

    #[inline(always)]
    pub fn release(self) {
        INNER.with_borrow_mut(|inner| {
            if inner.is_none() {
                panic!("Internal error. Inner context already released.")
            }

            *inner = None;
        })
    }

    #[inline(always)]
    pub fn name(self) -> String {
        self.borrow(move |inner| inner.name.clone())
    }

    #[inline(always)]
    pub fn span(self) -> Span {
        self.borrow(move |inner| inner.span())
    }

    #[inline(always)]
    pub fn primary_origin(self) -> OriginRef {
        self.borrow(move |inner| {
            inner
                .origins
                .get(0)
                .expect("Internal error. Missing primary origin.")
                .clone()
        })
    }

    #[inline(always)]
    pub fn make_origin(self, name: &str, span: Span) -> OriginRef {
        self.borrow(move |inner| {
            let reference = ContextInner::format_ident::<OriginTag>(
                &mut inner.hasher,
                "origin",
                Case::UpperSnake,
                name,
                span,
            );

            inner.origins.push(reference.clone());

            reference
        })
    }

    #[inline(always)]
    pub fn make_unique_identifier(self, name: &str, span: Span) -> IdRef {
        self.borrow(move |inner| {
            let reference = ContextInner::format_ident::<IdentifierTag>(
                &mut inner.hasher,
                "id",
                Case::UpperSnake,
                name,
                span,
            );

            inner
                .unique_identifiers
                .push((LitStr::new(name, span), reference.clone()));

            reference
        })
    }

    #[inline(always)]
    pub fn make_shared_identifier(self, name: &str, span: Span) -> IdRef {
        self.borrow(move |inner| {
            let string = LitStr::new(name, span);

            if let Some(reference) = inner.shared_identifiers.get(&string) {
                return reference.clone();
            }

            let reference = ContextInner::format_ident::<IdentifierTag>(
                &mut inner.hasher,
                "id",
                Case::UpperSnake,
                name,
                span,
            );

            let _ = inner.shared_identifiers.insert(string, reference.clone());

            reference
        })
    }

    #[inline(always)]
    pub fn make_section_name(self, name: &str, span: Span) -> SectionName {
        self.borrow(move |inner| {
            ContextInner::format_ident::<SectionTag>(
                &mut inner.hasher,
                "__adastra_export",
                Case::UpperSnake,
                name,
                span,
            )
        })
    }

    #[inline(always)]
    pub fn make_function_name(self, name: &str, span: Span) -> FunctionName {
        self.borrow(move |inner| {
            ContextInner::format_ident::<FunctionTag>(
                &mut inner.hasher,
                "fn",
                Case::Snake,
                name,
                span,
            )
        })
    }

    #[inline(always)]
    pub fn make_type_name(self, name: &str, span: Span) -> TypeName {
        self.borrow(move |inner| {
            ContextInner::format_ident::<TypeTag>(
                &mut inner.hasher,
                "ty",
                Case::UpperCamel,
                name,
                span,
            )
        })
    }

    #[inline(always)]
    pub fn make_static_name(self, name: &str, span: Span) -> StaticName {
        self.borrow(move |inner| {
            ContextInner::format_ident::<StaticTag>(
                &mut inner.hasher,
                "st",
                Case::UpperSnake,
                name,
                span,
            )
        })
    }

    #[inline(always)]
    fn borrow<R>(&self, f: impl FnOnce(&mut ContextInner) -> R) -> R {
        INNER.with_borrow_mut(|inner| {
            let Some(inner) = inner else {
                panic!("Internal error. Inner context is not initialized.");
            };

            f(inner)
        })
    }
}

thread_local! {
    static INNER: RefCell<Option<ContextInner>> = RefCell::new(None)
}

struct ContextInner {
    name: String,
    hasher: AHasher,
    origins: Vec<Ident>,
    unique_identifiers: Vec<(LitStr, Ident)>,
    shared_identifiers: AHashMap<LitStr, Ident>,
}

impl ToTokens for ContextInner {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for ident in &self.origins {
            let span = ident.span();

            let core = span.face_core();
            let module_path = span.face_module_path();
            let line = span.face_line();
            let column = span.face_column();
            let panic = span.face_panic();
            let env = span.face_env();
            let option = span.face_option();

            quote_spanned!(span=> static #ident: #core::runtime::RustOrigin = {
                #[cold]
                #[track_caller]
                fn blame_fn(message: &str) {
                    #panic("{}", message);
                }

                #core::runtime::RustOrigin {
                    package: #option::<(&'static str, &'static str)>::Some(
                        (#env("CARGO_PKG_NAME"), #env("CARGO_PKG_VERSION")),
                    ),
                    code: #option::<#core::runtime::RustCode>::Some(
                        #core::runtime::RustCode {
                            module: #module_path(),
                            line: #line(),
                            column: #column(),
                            blame_fn,
                        },
                    ),
                }
            };)
            .to_tokens(tokens)
        }

        let identifiers = self
            .unique_identifiers
            .iter()
            .map(|(string, ident)| (string, ident))
            .chain(self.shared_identifiers.iter());

        for (string, ident) in identifiers {
            let span = ident.span();

            let core = span.face_core();
            let module_path = span.face_module_path();
            let line = span.face_line();
            let column = span.face_column();
            let panic = span.face_panic();
            let env = span.face_env();
            let option = span.face_option();

            quote_spanned!(span=> static #ident: #core::runtime::RustIdent = {
                #[cold]
                #[track_caller]
                fn blame_fn(message: &str) {
                    #panic("{}", message);
                }

                static ORIGIN: #core::runtime::RustOrigin = #core::runtime::RustOrigin {
                    package: #option::<(&'static str, &'static str)>::Some(
                        (#env("CARGO_PKG_NAME"), #env("CARGO_PKG_VERSION")),
                    ),
                    code: #option::<#core::runtime::RustCode>::Some(
                        #core::runtime::RustCode {
                            module: #module_path(),
                            line: #line(),
                            column: #column(),
                            blame_fn,
                        },
                    ),
                };

                #core::runtime::RustIdent {
                    origin: &ORIGIN,
                    string: #string,
                }
            };)
            .to_tokens(tokens)
        }
    }
}

impl ContextInner {
    fn new(item: &Item) -> Self {
        let mut hasher = seed_hasher();

        Self::feed_tag::<CargoNameTag>(&mut hasher);
        if let Ok(value) = var("CARGO_PKG_NAME") {
            Self::feed_string(&mut hasher, &value)
        }

        Self::feed_tag::<CargoVersionTag>(&mut hasher);
        if let Ok(value) = var("CARGO_PKG_VERSION") {
            Self::feed_string(&mut hasher, &value)
        }

        Self::feed_tag::<ItemTag>(&mut hasher);
        Self::feed_stream(&mut hasher, item.to_token_stream());

        let span;
        let name;

        match item {
            Item::Const(item) => {
                span = item.ident.span();
                name = format!("const {}", item.ident);
            }

            Item::Fn(item) => {
                span = item.sig.ident.span();
                name = format!("fn {}", item.sig.ident);
            }

            Item::Impl(item) => {
                span = item.self_ty.span();

                let this = match &item.self_ty.as_ref() {
                    Type::Path(ty) => match ty.path.get_ident() {
                        Some(ident) => Some(ident.to_string()),
                        None => None,
                    },
                    _ => None,
                };

                let this = this.unwrap_or_else(|| String::from("type"));

                name = match &item.trait_ {
                    Some((_, path, _)) => match path.get_ident() {
                        Some(ident) => {
                            format!("{ident} for {}", this)
                        }
                        None => format!("trait for {}", this),
                    },

                    None => this,
                };
            }

            Item::Static(item) => {
                span = item.ident.span();
                name = format!("static {}", item.ident);
            }

            Item::Struct(item) => {
                span = item.ident.span();
                name = format!("struct {}", item.ident);
            }

            Item::Trait(item) => {
                span = item.ident.span();
                name = format!("trait {}", item.ident);
            }

            Item::Type(item) => {
                span = item.ident.span();
                name = format!("type {}", item.ident);
            }

            _ => {
                span = Span::call_site();
                name = String::new();
            }
        };

        let primary_origin = Self::format_ident::<OriginTag>(
            &mut hasher,
            "origin",
            Case::UpperSnake,
            name.as_ref(),
            span,
        );

        Self {
            name,
            hasher,
            origins: vec![primary_origin],
            unique_identifiers: Vec::new(),
            shared_identifiers: seed_hash_map(),
        }
    }

    fn format_ident<T: 'static>(
        hasher: &mut impl Hasher,
        suffix: &'static str,
        case: Case,
        ident: &str,
        span: Span,
    ) -> Ident {
        Self::feed_tag::<T>(hasher);
        Self::feed_string(hasher, ident);

        let mut hash = format!("{:x}", hasher.finish());

        hash.truncate(16);

        let mut string = String::new();

        string.push_str(suffix);
        string.push('_');
        string.push_str(&hash);

        if let Case::UpperCamel | Case::UpperKebab | Case::UpperFlat | Case::UpperSnake = case {
            string = string.to_ascii_uppercase();
        }

        let mut first = true;
        for char in ident.to_case(case).chars() {
            if char.is_ascii_alphanumeric() || char == '_' {
                if first {
                    string.push('_');
                    first = false;
                }

                string.push(char);
                continue;
            }
        }

        Ident::new(&string, span)
    }

    #[inline(always)]
    fn feed_tag<T: 'static>(hasher: &mut impl Hasher) {
        TypeId::of::<T>().hash(hasher);
    }

    #[inline]
    fn feed_string(hasher: &mut impl Hasher, string: &str) {
        string.hash(hasher);
    }

    #[inline]
    fn feed_char(hasher: &mut impl Hasher, ch: &char) {
        ch.hash(hasher);
    }

    #[inline]
    #[allow(unused)]
    fn feed_int(hasher: &mut impl Hasher, int: usize) {
        int.hash(hasher);
    }

    #[inline]
    fn feed_stream(hasher: &mut impl Hasher, stream: TokenStream) {
        for tree in stream {
            Self::feed_token_tree(hasher, tree);
        }
    }

    fn feed_token_tree(hasher: &mut impl Hasher, tree: TokenTree) {
        match tree {
            TokenTree::Group(variant) => {
                match variant.delimiter() {
                    Delimiter::Parenthesis => Self::feed_tag::<ParenthesisTag>(hasher),
                    Delimiter::Brace => Self::feed_tag::<BraceTag>(hasher),
                    Delimiter::Bracket => Self::feed_tag::<BracketTag>(hasher),
                    Delimiter::None => Self::feed_tag::<GroupTag>(hasher),
                }

                Self::feed_stream(hasher, variant.stream());
            }

            TokenTree::Ident(variant) => {
                Self::feed_tag::<IdentTag>(hasher);
                Self::feed_string(hasher, variant.to_string().as_str());
            }

            TokenTree::Punct(variant) => {
                match variant.spacing() {
                    Spacing::Alone => Self::feed_tag::<AloneTag>(hasher),
                    Spacing::Joint => Self::feed_tag::<JointTag>(hasher),
                }

                Self::feed_char(hasher, &variant.as_char());
            }

            TokenTree::Literal(variant) => {
                Self::feed_tag::<LiteralTag>(hasher);
                Self::feed_string(hasher, variant.to_string().as_str());
            }
        }
    }
}

struct CargoNameTag;
struct CargoVersionTag;
struct ParenthesisTag;
struct BraceTag;
struct BracketTag;
struct GroupTag;
struct IdentTag;
struct AloneTag;
struct JointTag;
struct LiteralTag;
struct OriginTag;
struct IdentifierTag;
struct ItemTag;
struct SectionTag;
struct FunctionTag;
struct TypeTag;
struct StaticTag;
