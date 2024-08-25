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
    borrow::Cow,
    fmt::{Debug, Display, Formatter},
    hash::Hash,
};

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote_spanned, ToTokens};
use syn::{spanned::Spanned, Expr, LitStr, Type};

use crate::utils::{
    context::{Context, SectionName},
    ty::is_str_type,
    Facade,
    IdRef,
    ManifestMeta,
    OriginRef,
    Shallow,
    TypeUtils,
};

pub struct Group {
    section: SectionName,
    packages: Vec<TokenStream>,
    type_metas: Vec<TokenStream>,
    prototypes: Vec<TokenStream>,
    custom: Vec<TokenStream>,
}

impl Default for Group {
    fn default() -> Self {
        let span = Context.span();
        let name = Context.name();

        let section = Context.make_section_name(name.as_str(), span);

        Self {
            section,
            packages: Vec::new(),
            type_metas: Vec::new(),
            prototypes: Vec::new(),
            custom: Vec::new(),
        }
    }
}

impl ToTokens for Group {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = Context.span();
        let origin = Context.primary_origin();

        let intrinsics = span.face_intrinsics();
        let option = span.face_option();
        let vec_macro = span.face_vec_macro();

        let context = Context;
        let section = &self.section;

        let packages = &self.packages;
        let type_metas = &self.type_metas;
        let prototypes = &self.prototypes;
        let custom = &self.custom;

        if packages.is_empty()
            && type_metas.is_empty()
            && prototypes.is_empty()
            && custom.is_empty()
        {
            return;
        }

        quote_spanned! (span=>
            #[no_mangle]
            extern "C" fn #section() {
                #[used]
                #[cfg_attr(
                    any(
                        target_os = "none",
                        target_os = "linux",
                        target_os = "android",
                        target_os = "fuchsia",
                        target_os = "psp",
                        target_os = "freebsd",
                    ),
                    link_section = "adastrexpr",
                )]
                #[cfg_attr(
                    any(
                        target_os = "macos",
                        target_os = "ios",
                        target_os = "tvos",
                    ),
                    link_section = "__DATA,__adastrexpr,regular,no_dead_strip",
                )]
                #[cfg_attr(
                    any(target_os = "illumos"),
                    link_section = "set_adastrexpr",
                )]
                #[cfg_attr(
                    any(target_os = "windows"),
                    link_section = ".adastrexpr$b",
                )]
                static __LINKED: extern "C" fn() = #section;

                let #option::Some(entry) = #intrinsics::ExportEntry::get(#section)
                else {
                    return;
                };

                let group = {
                    #context

                    #(
                    #custom
                    )*

                    #intrinsics::DeclarationGroup {
                        origin: &#origin,

                        packages: #vec_macro[#(
                            #packages,
                        )*],

                        type_metas: #vec_macro[#(
                            #type_metas,
                        )*],

                        prototypes: #vec_macro[#(
                            #prototypes,
                        )*],
                    }
                };

                #intrinsics::ExportEntry::export(entry, group);
            }
        )
        .to_tokens(tokens)
    }
}

impl Group {
    #[inline(always)]
    pub fn package(&mut self, declaration: Package<'_>) -> &mut Self {
        if Shallow.enabled() {
            return self;
        }

        self.packages.push(declaration.into_token_stream());

        self
    }

    #[inline(always)]
    pub fn type_meta(&mut self, declaration: TypeMeta<'_>) -> &mut Self {
        if Shallow.enabled() {
            return self;
        }

        self.type_metas.push(declaration.into_token_stream());

        self
    }

    #[inline(always)]
    pub fn prototype(&mut self, declaration: Prototype<'_>) -> &mut Self {
        if Shallow.enabled() {
            return self;
        }

        self.prototypes.push(declaration.into_token_stream());

        self
    }

    #[inline(always)]
    pub fn custom(&mut self, stream: impl ToTokens) -> &mut Self {
        if Shallow.enabled() {
            return self;
        }

        self.custom.push(stream.into_token_stream());

        self
    }
}

pub struct Package<'a> {
    pub ty: &'a Type,
    pub manifest: &'a ManifestMeta,
}

impl<'a> ToTokens for Package<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = Context.span();
        let origin = Context.primary_origin();

        let core = span.face_core();
        let intrinsics = span.face_intrinsics();
        let env = span.face_env();
        let format = span.face_format();
        let default = span.face_default();
        let option = span.face_option();
        let deref = span.face_deref();

        let ty = self.ty;
        let name = &self.manifest.name;
        let version = &self.manifest.version;

        let version_mismatch = LitStr::new(
            &format!(
                "Manifest package {}@{} is different to the current package {{}}@{{}}.",
                self.manifest.name.value(),
                self.manifest.version.value(),
            ),
            span,
        );

        let package_missing = LitStr::new(
            &format!(
                "Package {}@{} is missing in the global package repository.",
                self.manifest.name.value(),
                self.manifest.version.value(),
            ),
            span,
        );

        let package_instantiation = LitStr::new(
            &format!(
                "Package {}@{} instantiation failure.",
                self.manifest.name.value(),
                self.manifest.version.value(),
            ),
            span,
        );

        let doc = match &self.manifest.doc {
            Some(doc) => quote_spanned!(span=> #option::Some(#doc)),
            None => quote_spanned!(span=> #option::None),
        };

        quote_spanned!(span=> {
            fn package() -> #intrinsics::PackageDeclaration {
                static PACKAGE_NAME: &'static str = #env("CARGO_PKG_NAME");
                static PACKAGE_VERSION: &'static str = #env("CARGO_PKG_VERSION");

                #[allow(non_local_definitions)]
                impl #core::runtime::ScriptPackage for #ty {
                    fn meta() -> &'static #core::runtime::PackageMeta {
                        static META: #intrinsics::Lazy::<Option<&'static #core::runtime::PackageMeta>> =
                            #intrinsics::Lazy::new(|| {
                                let exact_version = #format("={}", PACKAGE_VERSION);

                                #core::runtime::PackageMeta::of(
                                    PACKAGE_NAME,
                                    &exact_version,
                                )
                            });

                        match <#intrinsics::Lazy::<
                            Option<&'static #core::runtime::PackageMeta>,
                        > as #deref>::deref(&META)
                        {
                            #option::Some(meta) => meta,

                            #option::None => #core::runtime::RustOrigin::blame(
                                &#origin,
                                #package_missing,
                            ),
                        }
                    }
                }

                if PACKAGE_NAME != #name || PACKAGE_VERSION != #version {
                    #core::runtime::RustOrigin::blame(
                        &#origin,
                        &#format(
                            #version_mismatch,
                            PACKAGE_NAME,
                            PACKAGE_VERSION,
                        ),
                    )
                }

                #intrinsics::PackageDeclaration {
                    name: PACKAGE_NAME,
                    version: PACKAGE_VERSION,
                    doc: #doc,
                    instance: #intrinsics::Lazy::<#core::runtime::Cell>::new(|| {
                        let instance: #ty = <#ty as #default>::default();

                        let result = #core::runtime::Cell::give(
                            #core::runtime::Origin::Rust(&#origin),
                            instance,
                        );

                        <#core::runtime::RuntimeResult::<#core::runtime::Cell>
                            as #core::runtime::RuntimeResultExt>::expect_blame(
                            result,
                            #package_instantiation,
                        )
                    }),
                }
            }

            package as fn() -> #intrinsics::PackageDeclaration
        })
        .to_tokens(tokens);
    }
}

pub struct TypeMeta<'a> {
    pub name: &'a LitStr,
    pub doc: Option<&'a LitStr>,
    pub ty: &'a Type,
    pub family: TypeFamily<'a>,
}

impl<'a> ToTokens for TypeMeta<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = Context.span();

        let intrinsics = span.face_intrinsics();
        let core = span.face_core();
        let type_id = span.face_type_id();
        let option = span.face_option();

        let name = self.name;
        let ty = self.ty;

        let doc = match &self.doc {
            Some(doc) => quote_spanned!(span=> #option::Some(#doc)),
            None => quote_spanned!(span=> #option::None),
        };

        let family = match &self.family {
            TypeFamily::Unique => {
                quote_spanned!(span=>
                    #option::<&'static #core::runtime::TypeFamily>::None)
            }

            TypeFamily::Package => {
                quote_spanned!(span=>
                    #option::<&'static #core::runtime::TypeFamily>::Some(&#intrinsics::PACKAGE_FAMILY))
            }

            TypeFamily::Function => {
                quote_spanned!(span=>
                    #option::<&'static #core::runtime::TypeFamily>::Some(&#intrinsics::FUNCTION_FAMILY))
            }

            TypeFamily::Custom(family) => {
                quote_spanned!(span=>
                    #option::<&'static #core::runtime::TypeFamily>::Some(#family))
            }
        };

        quote_spanned!(span=> {
            fn type_meta() -> #intrinsics::TypeMetaDeclaration {
                #intrinsics::TypeMetaDeclaration {
                    name: #name,
                    doc: #doc,
                    id: #type_id::of::<#ty>(),
                    family: #family,
                    size: <#ty as #intrinsics::SizeOf>::SIZE,
                }
            }

            type_meta as fn() -> #intrinsics::TypeMetaDeclaration
        })
        .to_tokens(tokens);
    }
}

#[derive(Clone, Copy)]
pub enum TypeFamily<'a> {
    Unique,
    Package,
    Function,
    Custom(&'a Expr),
}

pub struct Prototype<'a> {
    receiver_ty: Option<&'a Type>,
    receiver_id: TokenStream,
    manifest: Option<&'a ManifestMeta>,
    components: Vec<Component<'a>>,
    operators: Vec<(OperatorOrigin<'a>, Operator)>,
}

impl<'a> ToTokens for Prototype<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = Context.span();

        let intrinsics = span.face_intrinsics();
        let vec = span.face_vec();
        let vec_macro = span.face_vec_macro();

        let receiver_id = &self.receiver_id;

        let components = match self.manifest {
            None => {
                let components = &self.components;

                quote_spanned!(span=> #vec_macro[#(
                    #components,
                )*])
            }

            Some(manifest) => {
                let capacity = self.components.len() + manifest.dependencies.len();
                let components = &self.components;

                quote_spanned!(span=> {
                    let mut components = #vec::<#intrinsics::ComponentDeclaration>::with_capacity(#capacity);

                    #manifest

                    #( #vec::push(&mut components, #components); )*

                    components
                })
            }
        };

        let operators = match self.receiver_ty {
            Some(receiver) if !self.operators.is_empty() => {
                let operators = self.operators.iter().map(|(origin, operator)| {
                    let (span, origin) = origin.split();

                    operator.to_stream(span, origin, receiver)
                });

                quote_spanned!(span=> #vec_macro[#(
                    #operators,
                )*])
            }

            _ => quote_spanned!(span=> #vec::new()),
        };

        quote_spanned!(span=> {
            fn prototype() -> #intrinsics::PrototypeDeclaration {
                #intrinsics::PrototypeDeclaration {
                    receiver: #receiver_id,

                    components: #components,

                    operators: #operators,
                }
            }

            prototype as fn() -> #intrinsics::PrototypeDeclaration
        })
        .to_tokens(tokens);
    }
}

impl<'a> Prototype<'a> {
    pub fn for_type(receiver: &'a Type) -> Self {
        let span = receiver.span();
        let type_id = span.face_type_id();
        let receiver_id = quote_spanned!(span=> #type_id::of::<#receiver>());

        Self {
            receiver_ty: Some(receiver),
            receiver_id,
            manifest: None,
            components: Vec::new(),
            operators: Vec::with_capacity(1),
        }
    }

    pub fn for_package(span: Span) -> Self {
        let core = span.face_core();
        let env = span.face_env();
        let format = span.face_format();
        let option = span.face_option();
        let panic = span.face_panic();

        let receiver_id = quote_spanned!(span=> {
            let name = #env("CARGO_PKG_NAME");
            let version = #env("CARGO_PKG_VERSION");

            let package = match #core::runtime::PackageMeta::of(name, &#format("={}", version)) {
                #option::<&#core::runtime::PackageMeta>::Some(package) => package,
                #option::<&#core::runtime::PackageMeta>::None => {
                    #panic("Package {}@{} is not registered.", name, version);
                },
            };

            *#core::runtime::TypeMeta::id(&#core::runtime::PackageMeta::ty(package))
        });

        Self {
            receiver_ty: None,
            receiver_id,
            manifest: None,
            components: Vec::new(),
            operators: Vec::with_capacity(1),
        }
    }

    #[inline(always)]
    pub fn manifest(&mut self, manifest: &'a ManifestMeta) -> &mut Self {
        self.manifest = Some(manifest);

        self
    }

    #[inline(always)]
    pub fn component(&mut self, component: Component<'a>) -> &mut Self {
        self.components.push(component);

        self
    }

    #[inline(always)]
    pub fn operator(&mut self, origin: OperatorOrigin<'a>, operator: Operator) -> &mut Self {
        if self.receiver_ty.is_none() {
            unreachable!("Internal error. Operator without receiver.");
        }

        self.operators.push((origin, operator));

        self
    }
}

pub struct Component<'a> {
    pub name_ref: Cow<'a, IdRef>,
    pub constructor: TokenStream,
    pub hint: Cow<'a, Type>,
    pub doc: Option<LitStr>,
}

impl<'a> ToTokens for Component<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.name_ref.span();

        let intrinsics = span.face_intrinsics();
        let option = span.face_option();

        let name_ref = self.name_ref.as_ref();
        let constructor = &self.constructor;

        let hint = self.hint.type_hint();

        let doc = match &self.doc {
            Some(doc) => quote_spanned!(span=> #option::Some(#doc)),
            None => quote_spanned!(span=> #option::None),
        };

        quote_spanned!(span=> #intrinsics::ComponentDeclaration {
            name: &#name_ref,
            constructor: #constructor,
            hint: #hint,
            doc: #doc,
        })
        .to_tokens(tokens);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operator {
    Assign,
    Concat,
    Field,
    Clone,
    Debug,
    Display,
    PartialEq,
    Default,
    PartialOrd,
    Ord,
    Hash,
    Invocation,
    Binding,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    And,
    Or,
    Not,
    Neg,
    BitAnd,
    BitAndAssign,
    BitOr,
    BitOrAssign,
    BitXor,
    BitXorAssign,
    Shl,
    ShlAssign,
    Shr,
    ShrAssign,
    Rem,
    RemAssign,
    None,
}

impl Display for Operator {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, formatter)
    }
}

impl Operator {
    #[inline(always)]
    pub fn enumerate() -> impl Iterator<Item = &'static Self> {
        use Operator::*;

        static ALL: [Operator; 38] = [
            Assign,
            Concat,
            Field,
            Clone,
            Debug,
            Display,
            PartialEq,
            Default,
            PartialOrd,
            Ord,
            Hash,
            Invocation,
            Binding,
            Add,
            AddAssign,
            Sub,
            SubAssign,
            Mul,
            MulAssign,
            Div,
            DivAssign,
            And,
            Or,
            Not,
            Neg,
            BitAnd,
            BitAndAssign,
            BitOr,
            BitOrAssign,
            BitXor,
            BitXorAssign,
            Shl,
            ShlAssign,
            Shr,
            ShrAssign,
            Rem,
            RemAssign,
            None,
        ];

        ALL.iter()
    }

    #[inline]
    pub(super) fn describe(&self) -> OperatorDescription {
        let mut rhs = false;
        let mut result = false;

        match self {
            Self::Assign => {
                rhs = true;
            }

            Self::Concat => {
                result = true;
            }

            Self::Field => {
                rhs = false;
                result = true;
            }

            Self::Clone => {}

            Self::Debug => {}

            Self::Display => {}

            Self::PartialEq => {
                rhs = true;
            }

            Self::Default => {}

            Self::PartialOrd => {
                rhs = true;
            }

            Self::Ord => {}

            Self::Hash => {}

            Self::Invocation => {}

            Self::Binding => {
                rhs = true;
            }

            Self::Add => {
                rhs = true;
                result = true;
            }

            Self::AddAssign => {
                rhs = true;
                result = false;
            }

            Self::Sub => {
                rhs = true;
                result = true;
            }

            Self::SubAssign => {
                rhs = true;
                result = false;
            }

            Self::Mul => {
                rhs = true;
                result = true;
            }

            Self::MulAssign => {
                rhs = true;
                result = false;
            }

            Self::Div => {
                rhs = true;
                result = true;
            }

            Self::DivAssign => {
                rhs = true;
                result = false;
            }

            Self::And => {
                rhs = true;
                result = true;
            }

            Self::Or => {
                rhs = true;
                result = true;
            }

            Self::Not => {
                rhs = false;
                result = true;
            }

            Self::Neg => {
                rhs = false;
                result = true;
            }

            Self::BitAnd => {
                rhs = true;
                result = true;
            }

            Self::BitAndAssign => {
                rhs = true;
                result = false;
            }

            Self::BitOr => {
                rhs = true;
                result = true;
            }

            Self::BitOrAssign => {
                rhs = true;
                result = false;
            }

            Self::BitXor => {
                rhs = true;
                result = true;
            }

            Self::BitXorAssign => {
                rhs = true;
                result = false;
            }

            Self::Shl => {
                rhs = true;
                result = true;
            }

            Self::ShlAssign => {
                rhs = true;
                result = false;
            }

            Self::Shr => {
                rhs = true;
                result = true;
            }

            Self::ShrAssign => {
                rhs = true;
                result = false;
            }

            Self::Rem => {
                rhs = true;
                result = true;
            }

            Self::RemAssign => {
                rhs = true;
                result = false;
            }

            Self::None => {}
        }

        OperatorDescription { rhs, result }
    }

    fn to_stream(&self, span: Span, origin: TokenStream, lhs: &Type) -> TokenStream {
        let core = span.face_core();
        let intrinsics = span.face_intrinsics();

        let description = match self {
            Self::Concat => {
                let hint_result = quote_spanned!(span=>
                    <#lhs as #core::runtime::ops::ScriptConcat>::Result
                )
                .type_hint();

                return quote_spanned!(span=>
                    #intrinsics::OperatorDeclaration::Concat(
                        #intrinsics::ConcatOperator {
                            origin: #origin,

                            invoke: <#lhs as #core::runtime::ops::ScriptConcat>::script_concat as fn(
                                #core::runtime::Origin,
                                &mut [#core::runtime::Arg],
                            ) -> #core::runtime::RuntimeResult<#core::runtime::Cell>,

                            hint_result: #hint_result,
                        }
                    )
                );
            }

            Self::Field => {
                let hint_result = quote_spanned!(span=>
                    <#lhs as #core::runtime::ops::ScriptField>::Result
                )
                .type_hint();

                return quote_spanned!(span=>
                    #intrinsics::OperatorDeclaration::Field(
                        #intrinsics::FieldOperator {
                            origin: #origin,

                            invoke: <#lhs as #core::runtime::ops::ScriptField>::script_field as fn(
                                #core::runtime::Origin,
                                #core::runtime::Arg,
                                #core::runtime::Ident,
                            ) -> #core::runtime::RuntimeResult<#core::runtime::Cell>,

                            hint_result: #hint_result,
                        },
                    )
                );
            }

            Self::Clone => {
                let clone = span.face_clone();

                return quote_spanned!(span=>
                    #intrinsics::OperatorDeclaration::Clone(
                        #intrinsics::CloneOperator {
                            origin: #origin,

                            invoke: {
                                fn operator(
                                    origin: #core::runtime::Origin,
                                    mut lhs: #core::runtime::Arg,
                                ) -> #core::runtime::RuntimeResult<#core::runtime::Cell>
                                {
                                    let lhs = #core::runtime::Cell::borrow_ref::<#lhs>(
                                        &mut lhs.data,
                                        lhs.origin,
                                    )?;
                                    let cloned = #clone::clone(lhs);

                                    #core::runtime::Cell::give(origin, cloned)
                                }

                                operator as fn(
                                    #core::runtime::Origin,
                                    #core::runtime::Arg,
                                ) -> #core::runtime::RuntimeResult<#core::runtime::Cell>
                            },

                            clone_fn: #intrinsics::CloneFn::from_clone::<#lhs>(),
                        }
                    )
                );
            }

            Self::Debug => {
                let debug = span.face_debug();
                let formatter = span.face_formatter();
                let result = span.face_result();

                let borrowed = match is_str_type(lhs) {
                    false => quote_spanned!(span=>
                        #core::runtime::Cell::borrow_ref::<#lhs>(
                            &mut lhs.data,
                            lhs.origin,
                        )?
                    ),

                    true => quote_spanned!(span=>
                        #core::runtime::Cell::borrow_str(
                            &mut lhs.data,
                            lhs.origin,
                        )?
                    ),
                };

                return quote_spanned!(span=>
                    #intrinsics::OperatorDeclaration::Debug(
                        #intrinsics::DebugOperator {
                            origin: #origin,

                            invoke: {
                                fn operator(
                                    origin: #core::runtime::Origin,
                                    mut lhs: #core::runtime::Arg,
                                    formatter: &mut #formatter<'_>,
                                ) -> #core::runtime::RuntimeResult<()> {
                                    let borrowed = #borrowed;

                                    if #result::is_err(&#debug::fmt(borrowed, formatter)) {
                                        return Err(#core::runtime::RuntimeError::FormatError {
                                            access_origin: origin,
                                            receiver_origin: #core::runtime::Cell::origin(
                                                &lhs.data,
                                            ),
                                        });
                                    }

                                    #core::runtime::RuntimeResult::<()>::Ok(())
                                }

                                operator as fn(
                                    #core::runtime::Origin,
                                    #core::runtime::Arg,
                                    &mut #formatter<'_>,
                                ) -> #core::runtime::RuntimeResult<()>
                            },
                        }
                    )
                );
            }

            Self::Display => {
                let display = span.face_display();
                let formatter = span.face_formatter();
                let result = span.face_result();

                let borrowed = match is_str_type(lhs) {
                    false => quote_spanned!(span=>
                        #core::runtime::Cell::borrow_ref::<#lhs>(
                            &mut lhs.data,
                            lhs.origin,
                        )?
                    ),

                    true => quote_spanned!(span=>
                        #core::runtime::Cell::borrow_str(
                            &mut lhs.data,
                            lhs.origin,
                        )?
                    ),
                };

                return quote_spanned!(span=>
                    #intrinsics::OperatorDeclaration::Display(
                        #intrinsics::DisplayOperator {
                            origin: #origin,

                            invoke: {
                                fn operator(
                                    origin: #core::runtime::Origin,
                                    mut lhs: #core::runtime::Arg,
                                    formatter: &mut #formatter<'_>,
                                ) -> #core::runtime::RuntimeResult<()> {
                                    let borrowed = #borrowed;

                                    if #result::is_err(&#display::fmt(borrowed, formatter)) {
                                        return Err(#core::runtime::RuntimeError::FormatError {
                                            access_origin: origin,
                                            receiver_origin: #core::runtime::Cell::origin(
                                                &lhs.data,
                                            ),
                                        });
                                    }

                                    #core::runtime::RuntimeResult::<()>::Ok(())
                                }

                                operator as fn(
                                    #core::runtime::Origin,
                                    #core::runtime::Arg,
                                    &mut #formatter<'_>,
                                ) -> #core::runtime::RuntimeResult<()>
                            },
                        }
                    )
                );
            }

            Self::PartialEq => {
                let hint_rhs = quote_spanned!(span=>
                    <#lhs as #core::runtime::ops::ScriptPartialEq>::RHS
                )
                .type_hint();

                return quote_spanned!(span=>
                    #intrinsics::OperatorDeclaration::PartialEq(
                        #intrinsics::PartialEqOperator {
                            origin: #origin,

                            invoke: <#lhs as #core::runtime::ops::ScriptPartialEq>::script_eq as fn(
                                #core::runtime::Origin,
                                #core::runtime::Arg,
                                #core::runtime::Arg,
                            ) -> #core::runtime::RuntimeResult<bool>,

                            hint_rhs: #hint_rhs,
                        }
                    )
                );
            }

            Self::Default => {
                return quote_spanned!(span=>
                    #intrinsics::OperatorDeclaration::Default(
                        #intrinsics::DefaultOperator {
                            origin: #origin,

                            invoke: <#lhs as #core::runtime::ops::ScriptDefault>::script_default as fn(
                                #core::runtime::Origin,
                            ) -> #core::runtime::RuntimeResult<#core::runtime::Cell>,
                        },
                    )
                );
            }

            Self::PartialOrd => {
                let option = span.face_option();
                let ordering = span.face_ordering();

                let hint_rhs = quote_spanned!(span=>
                    <#lhs as #core::runtime::ops::ScriptPartialOrd>::RHS
                )
                .type_hint();

                return quote_spanned!(span=>
                    #intrinsics::OperatorDeclaration::PartialOrd(
                        #intrinsics::PartialOrdOperator {
                            origin: #origin,

                            invoke: <#lhs as #core::runtime::ops::ScriptPartialOrd>::script_partial_cmp as fn(
                                #core::runtime::Origin,
                                #core::runtime::Arg,
                                #core::runtime::Arg,
                            ) -> #core::runtime::RuntimeResult<#option<#ordering>>,

                            hint_rhs: #hint_rhs,
                        }
                    )
                );
            }

            Self::Ord => {
                let ordering = span.face_ordering();

                return quote_spanned!(span=>
                    #intrinsics::OperatorDeclaration::Ord(
                        #intrinsics::OrdOperator {
                            origin: #origin,

                            invoke: <#lhs as #core::runtime::ops::ScriptOrd>::script_cmp as fn(
                                #core::runtime::Origin,
                                #core::runtime::Arg,
                                #core::runtime::Arg,
                            ) -> #core::runtime::RuntimeResult<#ordering>,
                        }
                    )
                );
            }

            Self::Hash => {
                let hash = span.face_hash();

                let borrowed = match is_str_type(lhs) {
                    false => quote_spanned!(span=>
                        #core::runtime::Cell::borrow_ref::<#lhs>(
                            &mut lhs.data,
                            lhs.origin,
                        )?
                    ),

                    true => quote_spanned!(span=>
                        #core::runtime::Cell::borrow_str(
                            &mut lhs.data,
                            lhs.origin,
                        )?
                    ),
                };

                return quote_spanned!(span=>
                    #intrinsics::OperatorDeclaration::Hash(
                        #intrinsics::HashOperator {
                            origin: #origin,

                            invoke: {
                                fn operator(
                                    _origin: #core::runtime::Origin,
                                    mut lhs: #core::runtime::Arg,
                                    hasher: &mut #intrinsics::DynHasher<'_>,
                                ) -> #core::runtime::RuntimeResult<()> {
                                    let borrowed = #borrowed;

                                    <#lhs as #hash>::hash(borrowed, hasher);

                                    #core::runtime::RuntimeResult::<()>::Ok(())
                                }

                                operator as fn(
                                    origin: #core::runtime::Origin,
                                    #core::runtime::Arg,
                                    &mut #intrinsics::DynHasher<'_>,
                                ) -> #core::runtime::RuntimeResult<()>
                            }
                        }
                    )
                );
            }

            Self::Invocation => {
                let option = span.face_option();

                return quote_spanned!(span=>
                    #intrinsics::OperatorDeclaration::Invocation(
                        #intrinsics::InvocationOperator {
                            origin: #origin,

                            invoke: <#lhs as #core::runtime::ops::ScriptInvocation>::invoke as fn(
                                #core::runtime::Origin,
                                #core::runtime::Arg,
                                &mut [#core::runtime::Arg],
                            ) -> #core::runtime::RuntimeResult<#core::runtime::Cell>,

                            hint: <#lhs as #core::runtime::ops::ScriptInvocation>::hint
                                as fn() -> #option<&'static #core::runtime::InvocationMeta>,
                        }
                    )
                );
            }

            Self::None => {
                return quote_spanned!(span=>
                    #intrinsics::OperatorDeclaration::None(
                        #intrinsics::NoneOperator {
                            origin: #origin,
                        }
                    )
                );
            }

            _ => self.describe(),
        };

        let name = self.to_string();

        let intrinsics_variant_ident = Ident::new(name.as_str(), span);
        let intrinsic_operator_ident = Ident::new(&format!("{}Operator", name), span);
        let script_operator_ident = Ident::new(&format!("Script{}", name), span);
        let script_operator_invoke_ident =
            Ident::new(&format!("script_{}", name.to_case(Case::Snake)), span);

        let rhs_type;
        let rhs_type_hint;

        match description.rhs {
            true => {
                let type_hint =
                    quote_spanned!(span=> <#lhs as #core::runtime::ops::#script_operator_ident>::RHS).type_hint();

                rhs_type = Some(quote_spanned!(span=> #core::runtime::Arg,));
                rhs_type_hint = Some(quote_spanned!(span=> hint_rhs: #type_hint,));
            }

            false => {
                rhs_type = None;
                rhs_type_hint = None;
            }
        }

        let result_type;
        let result_type_hint;

        match description.result {
            true => {
                let type_hint =
                    quote_spanned!(span=> <#lhs as #core::runtime::ops::#script_operator_ident>::Result)
                        .type_hint();

                result_type = quote_spanned!(span=> #core::runtime::Cell);
                result_type_hint = Some(quote_spanned!(span=> hint_result: #type_hint,));
            }

            false => {
                result_type = quote_spanned!(span=> ());
                result_type_hint = None;
            }
        }

        quote_spanned!(span=>
            #intrinsics::OperatorDeclaration::#intrinsics_variant_ident(
                #intrinsics::#intrinsic_operator_ident {
                    origin: #origin,

                    invoke: <
                        #lhs as #core::runtime::ops::#script_operator_ident
                    >::#script_operator_invoke_ident as fn(
                        #core::runtime::Origin,
                        #core::runtime::Arg,
                        #rhs_type
                    ) -> #core::runtime::RuntimeResult<#result_type>,

                    #rhs_type_hint
                    #result_type_hint
                }
            )
        )
    }
}

pub struct OperatorDescription {
    pub rhs: bool,
    pub result: bool,
}

pub enum OperatorOrigin<'a> {
    Primary,
    Origin(&'a OriginRef),
    Id(&'a IdRef),
}

impl<'a> OperatorOrigin<'a> {
    #[inline]
    fn split(&self) -> (Span, TokenStream) {
        match self {
            Self::Primary => {
                let origin = Context.primary_origin();
                let span = origin.span();

                (span, quote_spanned!(span=> &#origin))
            }

            Self::Origin(origin) => {
                let span = origin.span();

                (span, quote_spanned!(span=> &#origin))
            }

            Self::Id(name_ref) => {
                let span = name_ref.span();

                (span, quote_spanned!(span=> #name_ref.origin))
            }
        }
    }
}
