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

use std::{borrow::Cow, env::var};

use convert_case::{Case, Casing};
use proc_macro2::Span;
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Error,
    Expr,
    Ident,
    Lit,
    LitBool,
    LitByte,
    LitChar,
    LitInt,
    LitStr,
    Meta,
    Result,
    Token,
    Type,
};

use crate::utils::{seed_hash_set, DeriveMeta, PolymorphicScope, Printer, Shallow, TypeFamily};

pub const UNSPECIFIED: u16 = 1 << 0;
pub const DUMP: u16 = 1 << 1;
pub const INCLUDED: u16 = 1 << 2;
pub const EXCLUDED: u16 = 1 << 3;
pub const SHALLOW: u16 = 1 << 4;
pub const RENAME: u16 = 1 << 5;
pub const TYPE: u16 = 1 << 6;
pub const CONST: u16 = 1 << 7;
pub const ACCESS: u16 = 1 << 8;
pub const FAMILY: u16 = 1 << 9;
pub const PACKAGE: u16 = 1 << 10;
pub const COMPONENT: u16 = 1 << 11;

pub struct Attrs {
    span: Span,
    specified: bool,
    dump: Option<Span>,
    included: Option<Span>,
    excluded: Option<Span>,
    shallow: Option<Span>,
    name: Option<(Span, Vec<NameComponent>)>,
    types: Option<(Span, Vec<Type>)>,
    constants: Option<(Span, LitVec)>,
    readonly: Option<Span>,
    writeonly: Option<Span>,
    family: Option<(Span, Expr)>,
    package: Option<LitStr>,
    component: Option<(Span, Type)>,
    derive: DeriveMeta,
}

impl Attrs {
    pub fn check(&self, mask: u16) -> Result<()> {
        if mask & UNSPECIFIED == 0 {
            if !self.specified {
                return Err(self.error(mask));
            }
        }

        if mask & DUMP == 0 {
            if let Some(span) = &self.dump {
                return Err(Error::new(
                    *span,
                    "Export dump marker is not applicable here.",
                ));
            }
        }

        if mask & INCLUDED == 0 {
            if let Some(span) = &self.included {
                return Err(Error::new(
                    *span,
                    "Inclusion marker is not applicable here.",
                ));
            }
        }

        if mask & EXCLUDED == 0 {
            if let Some(span) = &self.excluded {
                return Err(Error::new(
                    *span,
                    "Exclusion marker is not applicable here.",
                ));
            }
        }

        if mask & SHALLOW == 0 {
            if let Some(span) = &self.shallow {
                return Err(Error::new(
                    *span,
                    "Shallow export marker is not applicable here.",
                ));
            }
        }

        if mask & ACCESS == 0 {
            if let Some(span) = &self.readonly {
                return Err(Error::new(*span, "Readonly marker is not applicable here."));
            }

            if let Some(span) = &self.writeonly {
                return Err(Error::new(
                    *span,
                    "Writeonly marker is not applicable here.",
                ));
            }
        }

        if mask & RENAME == 0 {
            if let Some((span, _)) = &self.name {
                return Err(Error::new(*span, "Renaming is not applicable here."));
            }
        }

        match mask & TYPE == 0 {
            true => {
                if let Some((span, _)) = &self.types {
                    return Err(Error::new(
                        *span,
                        "Type specialization is not applicable here.",
                    ));
                }
            }

            false => {
                if let Some((_, types)) = &self.types {
                    let mut checked = seed_hash_set();

                    for ty in types {
                        if !checked.insert(ty) {
                            return Err(Error::new(ty.span(), "Duplicate type."));
                        }
                    }
                }
            }
        }

        match mask & CONST == 0 {
            true => {
                if let Some((span, _)) = &self.constants {
                    return Err(Error::new(
                        *span,
                        "Constant specialization is not applicable here.",
                    ));
                }
            }

            false => {
                if let Some((_, constants)) = &self.constants {
                    constants.check_duplicates()?;
                }
            }
        }

        if mask & FAMILY == 0 {
            if let Some((span, _)) = &self.family {
                return Err(Error::new(*span, "Type family is not applicable here."));
            }
        }

        if mask & PACKAGE == 0 {
            if let Some(path) = &self.package {
                return Err(Error::new(
                    path.span(),
                    "Package declaration is not applicable here.",
                ));
            }
        }

        if mask & COMPONENT == 0 {
            if let Some((span, _)) = &self.component {
                return Err(Error::new(
                    *span,
                    "Component marker is not applicable here.",
                ));
            }
        }

        Ok(())
    }

    #[inline]
    pub fn has_rename_variables(&self) -> bool {
        match &self.name {
            None => false,
            Some((_, components)) => components.iter().any(|component| match component {
                NameComponent::Literal(..) => false,
                _ => true,
            }),
        }
    }

    #[inline(always)]
    pub fn rename_checked<'a>(&self, scope: &impl PolymorphicScope) -> Result<Option<String>> {
        self.rename(scope, true)
    }

    #[inline(always)]
    pub fn rename_unchecked<'a>(&self, scope: &impl PolymorphicScope) -> Result<Option<String>> {
        self.rename(scope, false)
    }

    #[inline]
    pub fn has_types(&self) -> bool {
        self.types.is_some()
    }

    #[inline]
    pub fn types(&self) -> Result<&[Type]> {
        match &self.types {
            Some((_, types)) => Ok(&types[..]),
            None => Err(self.error(TYPE)),
        }
    }

    #[inline]
    pub fn constants(&self) -> Result<&LitVec> {
        match &self.constants {
            Some((_, constants)) => Ok(constants),
            None => Err(self.error(CONST)),
        }
    }

    #[inline]
    pub fn dump(&self) -> Option<Span> {
        self.dump
    }

    #[inline]
    pub fn specified(&self) -> bool {
        self.specified
    }

    #[inline]
    pub fn disabled(&self) -> bool {
        if self.included.is_some() {
            return false;
        }

        if self.excluded.is_some() {
            return true;
        }

        if self.shallow.is_some() {
            return false;
        }

        #[cfg(feature = "export")]
        {
            return false;
        }

        #[cfg(not(feature = "export"))]
        {
            return true;
        }
    }

    #[inline]
    pub fn included(&self) -> bool {
        self.included.is_some()
    }

    #[inline]
    pub fn excluded(&self) -> bool {
        self.excluded.is_some()
    }

    #[inline]
    pub fn shallow(&self) -> bool {
        #[cfg(not(debug_assertions))]
        {
            return false;
        }

        #[cfg(debug_assertions)]
        {
            if self.included.is_some() {
                return false;
            }

            if self.shallow.is_some() {
                return true;
            }

            #[cfg(feature = "shallow")]
            {
                return true;
            }

            #[cfg(not(feature = "shallow"))]
            {
                return false;
            }
        }
    }

    #[inline]
    pub fn readable(&self) -> bool {
        self.writeonly.is_none()
    }

    #[inline]
    pub fn writeable(&self) -> bool {
        self.readonly.is_none()
    }

    #[inline]
    pub fn family(&self) -> TypeFamily {
        match &self.family {
            None => TypeFamily::Unique,
            Some((_, family)) => TypeFamily::Custom(family),
        }
    }

    #[inline]
    pub fn package(&self) -> Option<&LitStr> {
        self.package.as_ref()
    }

    #[inline]
    pub fn derive(&self) -> &DeriveMeta {
        &self.derive
    }

    #[inline]
    pub fn component(&self) -> Option<&Type> {
        match &self.component {
            Some((_, ty)) => Some(ty),
            None => None,
        }
    }

    #[inline]
    fn rename<'a>(&self, scope: &impl PolymorphicScope, check: bool) -> Result<Option<String>> {
        match &self.name {
            None => Ok(None),

            Some((span, components)) => {
                let mut target = String::with_capacity(components.len() * 10);

                for component in components {
                    component.interpret(&mut target, scope, check)?;
                }

                if check {
                    NameComponent::check_validity(span, target.as_str())?;
                }

                Ok(Some(target))
            }
        }
    }

    fn append(&mut self, attr: Attr) -> Result<()> {
        match attr {
            Attr::None => {}

            Attr::Dump(span) => {
                if self.dump.is_some() {
                    return Err(Error::new(span, "Duplicate dump export mode marker."));
                }

                self.dump = Some(span);
            }

            Attr::Included(span) => {
                if self.included.is_some() {
                    return Err(Error::new(span, "Duplicate inclusion marker."));
                }

                if self.excluded.is_some() {
                    return Err(Error::new(
                        span,
                        "Inclusion marker conflicts with exclusion.",
                    ));
                }

                self.included = Some(span);
            }

            Attr::Shallow(span) => {
                if self.shallow.is_some() {
                    return Err(Error::new(span, "Duplicate shallow export marker."));
                }

                if self.excluded.is_some() {
                    return Err(Error::new(
                        span,
                        "Shallow export mode conflicts with exclusion.",
                    ));
                }

                self.shallow = Some(span);
            }

            Attr::Excluded(span) => {
                if self.excluded.is_some() {
                    return Err(Error::new(span, "Duplicate exclusion marker."));
                }

                if self.included.is_some() {
                    return Err(Error::new(
                        span,
                        "Exclusion marker conflicts with inclusion.",
                    ));
                }

                if self.shallow.is_some() {
                    return Err(Error::new(
                        span,
                        "Exclusion marker conflicts with shallow export mode.",
                    ));
                }

                self.excluded = Some(span);
            }

            Attr::Name((span, name)) => {
                if self.name.is_some() {
                    return Err(Error::new(span, "Duplicate rename."));
                }

                if self.package.is_some() {
                    return Err(Error::new(
                        span,
                        "Rename conflicts with package declaration.",
                    ));
                }

                self.name = Some((span, name));
            }

            Attr::Types((span, mut types)) => match &mut self.types {
                None => self.types = Some((span, types)),
                Some((_, previous)) => previous.append(&mut types),
            },

            Attr::Constants((span, mut constants)) => match &mut self.constants {
                None => self.constants = Some((span, constants)),
                Some((_, previous)) => previous.append(&mut constants)?,
            },

            Attr::Readonly(span) => {
                if self.readonly.is_some() {
                    return Err(Error::new(span, "Duplicate readonly marker."));
                }

                if self.writeonly.is_some() {
                    return Err(Error::new(
                        span,
                        "Readonly marker conflicts with writeonly maker.",
                    ));
                }

                self.readonly = Some(span);
            }

            Attr::Writeonly(span) => {
                if self.writeonly.is_some() {
                    return Err(Error::new(span, "Duplicate writeonly marker."));
                }

                if self.readonly.is_some() {
                    return Err(Error::new(
                        span,
                        "Writeonly marker conflicts with readonly maker.",
                    ));
                }

                self.writeonly = Some(span);
            }

            Attr::Family((span, family)) => {
                if self.family.is_some() {
                    return Err(Error::new(span, "Duplicate type family marker."));
                }

                if self.package.is_some() {
                    return Err(Error::new(
                        span,
                        "Type family marker conflicts with package declaration.",
                    ));
                }

                self.family = Some((span, family));
            }

            Attr::Package(path) => {
                if self.package.is_some() {
                    return Err(Error::new(
                        path.span(),
                        "Duplicate package declaration marker.",
                    ));
                }

                if self.family.is_some() {
                    return Err(Error::new(
                        path.span(),
                        "Package declaration conflicts with type family marker.",
                    ));
                }

                if self.name.is_some() {
                    return Err(Error::new(
                        path.span(),
                        "Package declaration conflicts with rename marker.",
                    ));
                }

                self.package = Some(path);
            }

            Attr::Component((span, ty)) => {
                if self.component.is_some() {
                    return Err(Error::new(span, "Duplicate component marker."));
                }

                self.component = Some((span, ty));
            }
        }

        Ok(())
    }

    fn error(&self, mask: u16) -> Error {
        let mut variants = Vec::new();

        if mask & INCLUDED > 0 {
            if mask & UNSPECIFIED > 0 {
                variants.push("#[export] inclusion");
            }
            variants.push("#[export(include)] inclusion");
        }

        if mask & EXCLUDED > 0 {
            variants.push("#[export(exclude)] exclusion");
        }

        if mask & SHALLOW > 0 {
            variants.push("#[export(shallow)] shallow export mode");
        }

        if mask & RENAME > 0 {
            variants.push("#[export(name \"<new name>\")] renaming");
        }

        if mask & TYPE > 0 {
            variants.push("#[export(type <type_1>, <type_2>, ...)] type specialization");
        }

        if mask & CONST > 0 {
            variants.push("#[export(const 1, 3..6, 'x', true ...)] const specialization");
        }

        if mask & ACCESS > 0 {
            variants.push("#[export(readonly)] readonly marker");
            variants.push("#[export(writeonly)] writeonly marker");
        }

        if mask & FAMILY > 0 {
            variants.push("#[export(family <reference to static type family>)] type family marker");
        }

        if mask & PACKAGE > 0 {
            variants.push("#[export(manifest)] package declaration");
            variants.push("#[export(manifest \"<Cargo.toml path>\")] package declaration");
        }

        if mask & COMPONENT > 0 {
            variants.push("#[export(component <type>)] component marker");
        }

        if variants.len() == 1 {
            return Error::new(self.span, format!("Missing {} attribute.", variants[0]));
        }

        let mut message = String::from("Missing:\n");

        let mut first = true;

        for variant in variants {
            match first {
                true => {
                    first = false;
                    message.push_str("  - ");
                }
                false => {
                    message.push_str(";\n  - or ");
                }
            }

            message.push_str(variant);
            message.push_str(" attribute");
        }

        message.push_str(".");

        Error::new(self.span, message)
    }
}

pub enum LitVec {
    Byte(Vec<LitByte>),
    Char(Vec<LitChar>),
    Int(Vec<LitInt>),
    Bool(Vec<LitBool>),
}

impl<'a> IntoIterator for &'a LitVec {
    type Item = Lit;
    type IntoIter = inner::LitVecIterator<'a>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        match self {
            LitVec::Byte(vector) => inner::LitVecIterator::Byte(vector.iter()),
            LitVec::Char(vector) => inner::LitVecIterator::Char(vector.iter()),
            LitVec::Int(vector) => inner::LitVecIterator::Int(vector.iter()),
            LitVec::Bool(vector) => inner::LitVecIterator::Bool(vector.iter()),
        }
    }
}

impl LitVec {
    #[inline(always)]
    pub fn len(&self) -> usize {
        match self {
            Self::Byte(vector) => vector.len(),
            Self::Char(vector) => vector.len(),
            Self::Int(vector) => vector.len(),
            Self::Bool(vector) => vector.len(),
        }
    }

    #[inline(always)]
    fn check_duplicates(&self) -> Result<()> {
        trait LitExt: Eq + Spanned {
            fn display(&self) -> Cow<'_, str>;
        }

        impl LitExt for LitByte {
            #[inline(always)]
            fn display(&self) -> Cow<'_, str> {
                Cow::from(self.value().to_string())
            }
        }

        impl LitExt for LitChar {
            #[inline(always)]
            fn display(&self) -> Cow<'_, str> {
                Cow::from(format!("{:?}", self.value()))
            }
        }

        impl LitExt for LitInt {
            #[inline(always)]
            fn display(&self) -> Cow<'_, str> {
                Cow::from(self.base10_digits())
            }
        }

        impl LitExt for LitBool {
            #[inline(always)]
            fn display(&self) -> Cow<'_, str> {
                Cow::from(self.value.to_string())
            }
        }

        fn check_vector<T: LitExt>(vector: &Vec<T>) -> Result<()> {
            let mut iterator = vector.iter();

            let mut previous = match iterator.next() {
                None => return Ok(()),
                Some(value) => value,
            };

            while let Some(current) = iterator.next() {
                if previous == current {
                    let display = current.display();
                    return Err(Error::new(
                        current.span(),
                        format!("Duplicate value {display}"),
                    ));
                }

                previous = current;
            }

            Ok(())
        }

        match self {
            Self::Byte(vector) => check_vector(vector),
            Self::Char(vector) => check_vector(vector),
            Self::Int(vector) => check_vector(vector),
            Self::Bool(vector) => check_vector(vector),
        }
    }

    #[inline]
    fn sort(&mut self) {
        match self {
            Self::Byte(vector) => vector.sort_unstable_by(|a, b| a.value().cmp(&b.value())),
            Self::Char(vector) => vector.sort_unstable_by(|a, b| a.value().cmp(&b.value())),
            Self::Int(vector) => {
                vector.sort_unstable_by(|a, b| a.base10_digits().cmp(&b.base10_digits()))
            }
            Self::Bool(vector) => vector.sort_unstable_by(|a, b| a.value().cmp(&b.value())),
        }
    }

    #[inline]
    fn append(&mut self, other: &mut Self) -> Result<()> {
        let self_kind = self.kind();
        let other_kind = other.kind();

        if self_kind != other_kind {
            return Err(Error::new(
                other.span(),
                format!("Constant types inconsistency. Expected {self_kind} value."),
            ));
        }

        match (self, other) {
            (Self::Byte(dest), Self::Byte(src)) => dest.append(src),
            (Self::Char(dest), Self::Char(src)) => dest.append(src),
            (Self::Int(dest), Self::Int(src)) => dest.append(src),
            (Self::Bool(dest), Self::Bool(src)) => dest.append(src),

            _ => (),
        }

        Ok(())
    }

    fn kind(&self) -> &str {
        match self {
            Self::Byte(..) => "byte",

            Self::Char(..) => "char",

            Self::Int(vector) => {
                let first = &vector[0];

                match first.suffix() {
                    "" if first.base10_digits().chars().next() == Some('-') => "isize",

                    "" => "usize",

                    other => other,
                }
            }

            Self::Bool(..) => "bool",
        }
    }

    #[inline(always)]
    fn span(&self) -> Span {
        match self {
            Self::Byte(vector) => vector[0].span(),
            Self::Char(vector) => vector[0].span(),
            Self::Int(vector) => vector[0].span(),
            Self::Bool(vector) => vector[0].span(),
        }
    }
}

pub trait Exportable: inner::WithAttributes {
    #[inline]
    fn drain_attrs(&mut self) -> Result<Attrs> {
        let span = self.span();
        let attributes = self.attributes_mut();

        let mut export_attributes = Vec::with_capacity(attributes.len().min(1));

        let mut derive = DeriveMeta::default();

        for attribute in attributes.iter() {
            derive.enrich(attribute)?;
        }

        attributes.retain(|attribute| {
            if attribute.path().is_ident("export") {
                export_attributes.push(attribute.clone());
                return false;
            }

            true
        });

        let mut result = Attrs {
            span,
            specified: !export_attributes.is_empty(),
            dump: None,
            included: None,
            excluded: None,
            shallow: None,
            name: None,
            types: None,
            constants: None,
            readonly: None,
            writeonly: None,
            family: None,
            package: None,
            component: None,
            derive,
        };

        for attribute in export_attributes {
            let attr = match &attribute.meta {
                Meta::List(meta) => meta.parse_args::<Attr>()?,
                Meta::NameValue(meta) => {
                    return Err(Error::new(
                        meta.eq_token.span,
                        "Name-value attribute format is not supported.",
                    ))
                }
                Meta::Path(..) => continue,
            };

            result.append(attr)?;
        }

        Ok(result)
    }

    fn rust_doc(&self) -> Option<LitStr> {
        if Shallow.enabled() {
            return None;
        }

        let mut result = None;

        for attribute in self.attributes() {
            let Meta::NameValue(meta) = &attribute.meta else {
                continue;
            };

            let Some(ident) = meta.path.get_ident() else {
                continue;
            };

            let Expr::Lit(value) = &meta.value else {
                continue;
            };

            let Lit::Str(value) = &value.lit else {
                continue;
            };

            if ident != "doc" {
                continue;
            }

            match &mut result {
                None => result = Some((value.value(), value.span())),

                Some((result, _)) => {
                    result.push('\n');
                    result.push_str(&value.value());
                }
            }
        }

        result.map(|(text, span)| LitStr::new(&text, span))
    }

    fn deny_export(&mut self) -> Result<()> {
        for attribute in self.attributes_mut() {
            if attribute.path().is_ident("export") {
                return Err(Error::new(self.span(), "Export attribute denied here."));
            }
        }

        Ok(())
    }
}

impl<T: inner::WithAttributes> Exportable for T {}

mod inner {
    use std::slice::Iter;

    use syn::{
        spanned::Spanned,
        Attribute,
        ConstParam,
        Field,
        ImplItemConst,
        ImplItemFn,
        ItemConst,
        ItemFn,
        ItemImpl,
        ItemStatic,
        ItemStruct,
        ItemTrait,
        ItemType,
        LifetimeParam,
        Lit,
        LitBool,
        LitByte,
        LitChar,
        LitInt,
        PatType,
        TraitItemConst,
        TraitItemFn,
        TypeParam,
    };

    pub trait WithAttributes: Spanned {
        fn attributes(&self) -> &Vec<Attribute>;

        fn attributes_mut(&mut self) -> &mut Vec<Attribute>;
    }

    impl WithAttributes for ItemType {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for ItemStatic {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for ItemStruct {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for LifetimeParam {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for ConstParam {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for TypeParam {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for ItemConst {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for ItemTrait {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for ItemFn {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for ItemImpl {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for Field {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for PatType {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for ImplItemConst {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for ImplItemFn {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for TraitItemConst {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    impl WithAttributes for TraitItemFn {
        #[inline(always)]
        fn attributes(&self) -> &Vec<Attribute> {
            &self.attrs
        }

        #[inline(always)]
        fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
            &mut self.attrs
        }
    }

    pub enum LitVecIterator<'a> {
        Byte(Iter<'a, LitByte>),
        Char(Iter<'a, LitChar>),
        Int(Iter<'a, LitInt>),
        Bool(Iter<'a, LitBool>),
    }

    impl<'a> Iterator for LitVecIterator<'a> {
        type Item = Lit;

        #[inline(always)]
        fn next(&mut self) -> Option<Self::Item> {
            match self {
                Self::Byte(iterator) => Some(Lit::Byte(iterator.next()?.clone())),
                Self::Char(iterator) => Some(Lit::Char(iterator.next()?.clone())),
                Self::Int(iterator) => Some(Lit::Int(iterator.next()?.clone())),
                Self::Bool(iterator) => Some(Lit::Bool(iterator.next()?.clone())),
            }
        }
    }
}

enum Attr {
    None,
    Dump(Span),
    Included(Span),
    Excluded(Span),
    Shallow(Span),
    Name((Span, Vec<NameComponent>)),
    Types((Span, Vec<Type>)),
    Constants((Span, LitVec)),
    Readonly(Span),
    Writeonly(Span),
    Family((Span, Expr)),
    Package(LitStr),
    Component((Span, Type)),
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(Self::None);
        }

        if input.peek(keyword::dump) {
            let keyword = input.parse::<keyword::dump>()?;

            if !input.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            return Ok(Self::Dump(keyword.span));
        }

        let lookahead = input.lookahead1();

        if lookahead.peek(keyword::include) {
            let keyword = input.parse::<keyword::include>()?;

            if !input.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            return Ok(Self::Included(keyword.span));
        }

        if lookahead.peek(keyword::exclude) {
            let keyword = input.parse::<keyword::exclude>()?;

            if !input.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            return Ok(Self::Excluded(keyword.span));
        }

        if lookahead.peek(keyword::shallow) {
            let keyword = input.parse::<keyword::shallow>()?;

            if !input.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            return Ok(Self::Shallow(keyword.span));
        }

        if lookahead.peek(keyword::readonly) {
            let keyword = input.parse::<keyword::readonly>()?;

            if !input.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            return Ok(Self::Readonly(keyword.span));
        }

        if lookahead.peek(keyword::writeonly) {
            let keyword = input.parse::<keyword::writeonly>()?;

            if !input.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            return Ok(Self::Writeonly(keyword.span));
        }

        if lookahead.peek(keyword::name) {
            let keyword = input.parse::<keyword::name>()?;

            let mut components = Vec::with_capacity(1);

            while !input.is_empty() {
                components.push(input.parse::<NameComponent>()?)
            }

            return Ok(Self::Name((keyword.span, components)));
        }

        if lookahead.peek(Token![const]) {
            let keyword = input.parse::<Token![const]>()?;

            let ranges = Punctuated::<_, Token![,]>::parse_separated_nonempty_with(
                input,
                parse_constant_entry,
            )?;

            if !input.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            let mut buffer = None;

            for mut range in ranges {
                match &mut buffer {
                    None => {
                        buffer = Some(range);
                    }

                    Some(buffer) => {
                        buffer.append(&mut range)?;
                    }
                }
            }

            let mut buffer = match buffer {
                Some(buffer) => buffer,
                None => {
                    return Err(Error::new(input.span(), "Expected constant literal."));
                }
            };

            buffer.sort();

            return Ok(Self::Constants((keyword.span, buffer)));
        }

        if lookahead.peek(Token![type]) {
            let keyword = input.parse::<Token![type]>()?;

            let types = Punctuated::<Type, Token![,]>::parse_separated_nonempty(input)?
                .into_iter()
                .collect::<Vec<_>>();

            if !input.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            return Ok(Self::Types((keyword.span, types)));
        }

        if lookahead.peek(keyword::family) {
            let keyword = input.parse::<keyword::family>()?;

            let family = input.parse::<Expr>()?;

            if !input.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            return Ok(Self::Family((keyword.span, family)));
        }

        if lookahead.peek(keyword::package) {
            let keyword = input.parse::<keyword::package>()?;

            let manifest_path = match input.is_empty() {
                true => match var("CARGO_MANIFEST_DIR") {
                    Ok(directory) => LitStr::new(&format!("{directory}/Cargo.toml"), keyword.span),

                    Err(error) => {
                        return Err(Error::new(
                            keyword.span,
                            format!("CARGO_MANIFEST_DIR environment variable read error.\n{error}"),
                        ))
                    }
                },

                false => input.parse::<LitStr>()?,
            };

            if !input.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            return Ok(Self::Package(manifest_path));
        }

        if lookahead.peek(keyword::component) {
            let keyword = input.parse::<keyword::component>()?;

            let ty = input.parse::<Type>()?;

            if !input.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            return Ok(Self::Component((keyword.span, ty)));
        }

        return Err(lookahead.error());
    }
}

enum NameComponent {
    Literal(Lit),
    Expr(Expr, NameCase),
    Type(Type, NameCase),
    Arg(Ident, NameCase),
    Ret(Span, NameCase),
}

impl Parse for NameComponent {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Lit) {
            let literal = input.parse::<Lit>()?;

            return Ok(NameComponent::Literal(literal));
        }

        if lookahead.peek(names::Expr) {
            let _ = input.parse::<names::Expr>()?;

            let case = input.parse::<NameCase>()?;

            let content;

            bracketed!(content in input);

            let value = content.parse::<Expr>()?;

            if !content.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            return Ok(NameComponent::Expr(value, case));
        }

        if lookahead.peek(names::Type) {
            let _ = input.parse::<names::Type>()?;

            let case = input.parse::<NameCase>()?;

            let content;

            bracketed!(content in input);

            let value = content.parse::<Type>()?;

            if !content.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            return Ok(NameComponent::Type(value, case));
        }

        if lookahead.peek(names::Arg) {
            let _ = input.parse::<names::Arg>()?;

            let case = input.parse::<NameCase>()?;

            let content;

            bracketed!(content in input);

            let value = content.parse::<Ident>()?;

            if !content.is_empty() {
                return Err(input.error("Unexpected token."));
            }

            return Ok(NameComponent::Arg(value, case));
        }

        if lookahead.peek(names::Ret) {
            let span = input.parse::<names::Ret>()?.span;

            let case = input.parse::<NameCase>()?;

            return Ok(NameComponent::Ret(span, case));
        }

        return Err(lookahead.error());
    }
}

impl NameComponent {
    fn interpret(
        &self,
        target: &mut String,
        scope: &impl PolymorphicScope,
        sanitize: bool,
    ) -> Result<()> {
        fn interpret_literal(literal: &Lit, target: &mut String) -> Result<()> {
            match literal {
                Lit::Str(literal) => target.push_str(&literal.value()),
                Lit::Byte(literal) => target.push_str(&literal.value().to_string()),
                Lit::Char(literal) => target.push(literal.value()),
                Lit::Int(literal) => target.push_str(literal.base10_digits()),
                Lit::Float(literal) => target.push_str(literal.base10_digits()),
                Lit::Bool(literal) if literal.value => target.push_str("true"),
                Lit::Bool(literal) if !literal.value => target.push_str("false"),

                _ => return Err(Error::new(literal.span(), "Unsupported literal type.")),
            }

            Ok(())
        }

        match self {
            Self::Literal(literal) => interpret_literal(literal, target)?,

            Self::Expr(value, case) => {
                let mut value = value.clone();

                scope.specialize_expr(&mut value)?;

                match &value {
                    Expr::Lit(lit) => interpret_literal(&lit.lit, target)?,

                    _ => {
                        target.push_str(
                            case.apply(value.to_display_string().as_str(), sanitize)
                                .as_str(),
                        );
                    }
                }
            }

            Self::Type(value, case) => {
                let mut value = value.clone();

                scope.specialize_type(&mut value)?;

                target.push_str(
                    case.apply(value.to_display_string().as_str(), sanitize)
                        .as_str(),
                );
            }

            Self::Arg(value, case) => {
                let value = match scope.get_arg_type(value)? {
                    Some(ty) => ty,
                    None => {
                        return Err(Error::new(value.span(), "Unknown argument."));
                    }
                };

                target.push_str(
                    case.apply(value.to_display_string().as_str(), sanitize)
                        .as_str(),
                );
            }

            Self::Ret(span, case) => {
                let value = match scope.get_return_type()? {
                    Some(ty) => ty,
                    None => {
                        return Err(Error::new(*span, "Unknown return type."));
                    }
                };

                target.push_str(
                    case.apply(value.to_display_string().as_str(), sanitize)
                        .as_str(),
                );
            }
        }

        Ok(())
    }

    fn check_validity(span: &Span, string: &str) -> Result<()> {
        for char in string.chars() {
            if char.is_ascii_alphanumeric() || char == '_' {
                continue;
            }

            return Err(Error::new(
                *span,
                format!(
                    "The identifier {string:?} contains character \
                    '{char}'.\nExported identifiers must be built of ['a'..'z', \
                    'A'..'Z', '0'..'9', '_'] characters only.",
                ),
            ));
        }

        Ok(())
    }
}

#[repr(transparent)]
struct NameCase(Case);

impl Parse for NameCase {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<Token![:]>()?;

        let lookahead = input.lookahead1();

        if lookahead.peek(cases::Upper) {
            let _ = input.parse::<cases::Upper>()?;
            return Ok(Self(Case::Upper));
        }

        if lookahead.peek(cases::Lower) {
            let _ = input.parse::<cases::Lower>()?;
            return Ok(Self(Case::Lower));
        }

        if lookahead.peek(cases::Title) {
            let _ = input.parse::<cases::Title>()?;
            return Ok(Self(Case::Title));
        }

        if lookahead.peek(cases::Camel) {
            let _ = input.parse::<cases::Camel>()?;
            return Ok(Self(Case::Camel));
        }

        if lookahead.peek(cases::UpperCamel) {
            let _ = input.parse::<cases::UpperCamel>()?;
            return Ok(Self(Case::UpperCamel));
        }

        if lookahead.peek(cases::Snake) {
            let _ = input.parse::<cases::Snake>()?;
            return Ok(Self(Case::Snake));
        }

        if lookahead.peek(cases::UpperSnake) {
            let _ = input.parse::<cases::UpperSnake>()?;
            return Ok(Self(Case::UpperSnake));
        }

        if lookahead.peek(cases::Kebab) {
            let _ = input.parse::<cases::Kebab>()?;
            return Ok(Self(Case::Kebab));
        }

        if lookahead.peek(cases::UpperKebab) {
            let _ = input.parse::<cases::UpperKebab>()?;
            return Ok(Self(Case::UpperKebab));
        }

        if lookahead.peek(cases::Train) {
            let _ = input.parse::<cases::Train>()?;
            return Ok(Self(Case::Train));
        }

        if lookahead.peek(cases::Flat) {
            let _ = input.parse::<cases::Flat>()?;
            return Ok(Self(Case::Flat));
        }

        if lookahead.peek(cases::UpperFlat) {
            let _ = input.parse::<cases::UpperFlat>()?;
            return Ok(Self(Case::UpperFlat));
        }

        Err(lookahead.error())
    }
}

impl NameCase {
    #[inline]
    fn apply(&self, string: &str, sanitize: bool) -> String {
        let string = string.to_case(self.0);

        match sanitize {
            true => self.sanitize(&string),
            false => string,
        }
    }

    fn sanitize(&self, string: &str) -> String {
        let underscore_placeholder = match self.0 {
            Case::Camel | Case::Pascal | Case::UpperCamel | Case::Flat | Case::UpperFlat => false,
            _ => true,
        };

        let mut result = String::with_capacity(string.len());

        let mut pending_underscore = false;

        for character in string.chars() {
            if !character.is_ascii_alphanumeric() {
                pending_underscore = underscore_placeholder;
                continue;
            }

            if pending_underscore {
                pending_underscore = false;

                if !result.is_empty() {
                    result.push('_');
                }
            }

            result.push(character);
        }

        if result.is_empty() {
            result.push('_');
        }

        result
    }
}

fn parse_constant_entry(input: ParseStream) -> Result<LitVec> {
    match input.parse::<Lit>()? {
        Lit::Str(start) => Err(Error::new(start.span(), "String constants not supported.")),

        Lit::ByteStr(start) => Err(Error::new(
            start.span(),
            "Byte string constants not supported.",
        )),

        Lit::Byte(start) => {
            if input.peek(Token![..=]) {
                let span = input.parse::<Token![..=]>()?.span();
                let end = input.parse::<LitByte>()?;

                let start_value = start.value();
                let end_value = end.value();

                if start_value > end_value {
                    return Err(Error::new(
                        end.span(),
                        "End byte must be greater or equal to start byte.",
                    ));
                }

                let mut result = Vec::with_capacity((end_value - start_value) as usize + 1);

                for value in start_value..=end_value {
                    result.push(LitByte::new(value, span))
                }

                return Ok(LitVec::Byte(result));
            }

            if input.peek(Token![..]) {
                let span = input.parse::<Token![..]>()?.span();
                let end = input.parse::<LitByte>()?;

                let start_value = start.value();
                let end_value = end.value();

                if start_value >= end_value {
                    return Err(Error::new(
                        end.span(),
                        "End byte must be greater than start byte.",
                    ));
                }

                let mut result = Vec::with_capacity((end_value - start_value) as usize);

                for value in start_value..end_value {
                    result.push(LitByte::new(value, span))
                }

                return Ok(LitVec::Byte(result));
            }

            Ok(LitVec::Byte(vec![start]))
        }

        Lit::Char(start) => {
            if input.peek(Token![..=]) {
                let span = input.parse::<Token![..=]>()?.span();
                let end = input.parse::<LitChar>()?;

                let start_value = start.value();
                let end_value = end.value();

                if start_value > end_value {
                    return Err(Error::new(
                        end.span(),
                        "End character must be greater or equal to start character.",
                    ));
                }

                let mut result =
                    Vec::with_capacity(end_value as usize - (start_value as usize) + 1);

                for value in start_value..=end_value {
                    result.push(LitChar::new(value, span))
                }

                return Ok(LitVec::Char(result));
            }

            if input.peek(Token![..]) {
                let span = input.parse::<Token![..]>()?.span();
                let end = input.parse::<LitChar>()?;

                let start_value = start.value();
                let end_value = end.value();

                if start_value >= end_value {
                    return Err(Error::new(
                        end.span(),
                        "End character must be greater than start character.",
                    ));
                }

                let mut result = Vec::with_capacity(end_value as usize - (start_value as usize));

                for value in start_value..end_value {
                    result.push(LitChar::new(value, span))
                }

                return Ok(LitVec::Char(result));
            }

            Ok(LitVec::Char(vec![start]))
        }

        Lit::Int(start) => {
            if input.peek(Token![..=]) {
                let span = input.parse::<Token![..=]>()?.span();
                let end = input.parse::<LitInt>()?;
                let suffix = start.suffix();

                if suffix != end.suffix() {
                    return Err(Error::new(span, "Literal suffixes must match."));
                }

                macro_rules! match_suffix_exclusive {
                    ($($suffix:expr => $ty:ty,)+) => {
                        return match suffix {
                            "" if start.base10_digits().chars().next() == Some('-') => {
                                let start_value = start.base10_parse::<isize>()?;
                                let end_value = end.base10_parse::<isize>()?;

                                if start_value > end_value {
                                    return Err(Error::new(
                                        end.span(),
                                        "End number must be greater or equal to start number.",
                                    ));
                                }

                                let mut result = Vec::with_capacity((end_value - start_value) as usize + 1);

                                for value in start_value..=end_value {
                                    result.push(LitInt::new(&format!("{value}usize"), span))
                                }

                                Ok(LitVec::Int(result))
                            }

                            "" => {
                                let start_value = start.base10_parse::<usize>()?;
                                let end_value = end.base10_parse::<usize>()?;

                                if start_value > end_value {
                                    return Err(Error::new(
                                        end.span(),
                                        "End number must be greater or equal to start number.",
                                    ));
                                }

                                let mut result = Vec::with_capacity(end_value - start_value + 1);

                                for value in start_value..=end_value {
                                    result.push(LitInt::new(&format!("{value}usize"), span))
                                }

                                Ok(LitVec::Int(result))
                            }

                            $($suffix => {
                                let start_value = start.base10_parse::<$ty>()?;
                                let end_value = end.base10_parse::<$ty>()?;

                                if start_value > end_value {
                                    return Err(Error::new(
                                        end.span(),
                                        "End number must be greater or equal to start number.",
                                    ));
                                }

                                let mut result = Vec::with_capacity((end_value - start_value) as usize + 1);

                                for value in start_value..=end_value {
                                    result.push(LitInt::new(&format!("{value}{suffix}"), span))
                                }

                                Ok(LitVec::Int(result))
                            })*

                            other => Err(Error::new(start.span(), format!("Unsupported suffix {other}"))),
                        }
                    };
                }

                match_suffix_exclusive!(
                    "u8" => u8,
                    "u16" => u16,
                    "u32" => u32,
                    "u64" => u64,
                    "u128" => u128,
                    "usize" => usize,
                    "i8" => i8,
                    "i16" => i16,
                    "i32" => i32,
                    "i64" => i64,
                    "i128" => i128,
                    "isize" => isize,
                )
            }

            if input.peek(Token![..]) {
                let span = input.parse::<Token![..]>()?.span();
                let end = input.parse::<LitInt>()?;
                let suffix = start.suffix();

                if suffix != end.suffix() {
                    return Err(Error::new(span, "Literal suffixes must match."));
                }

                macro_rules! match_suffix_inclusive {
                    ($($suffix:expr => $ty:ty,)+) => {
                        return match suffix {
                            "" if start.base10_digits().chars().next() == Some('-') => {
                                let start_value = start.base10_parse::<isize>()?;
                                let end_value = end.base10_parse::<isize>()?;

                                if start_value >= end_value {
                                    return Err(Error::new(
                                        end.span(),
                                        "End number must be greater than start number.",
                                    ));
                                }

                                let mut result = Vec::with_capacity((end_value - start_value) as usize);

                                for value in start_value..end_value {
                                    result.push(LitInt::new(&format!("{value}usize"), span))
                                }

                                Ok(LitVec::Int(result))
                            }

                            "" => {
                                let start_value = start.base10_parse::<usize>()?;
                                let end_value = end.base10_parse::<usize>()?;

                                if start_value >= end_value {
                                    return Err(Error::new(
                                        end.span(),
                                        "End number must be greater than start number.",
                                    ));
                                }

                                let mut result = Vec::with_capacity(end_value - start_value);

                                for value in start_value..end_value {
                                    result.push(LitInt::new(&format!("{value}usize"), span))
                                }

                                Ok(LitVec::Int(result))
                            }

                            $($suffix => {
                                let start_value = start.base10_parse::<$ty>()?;
                                let end_value = end.base10_parse::<$ty>()?;

                                if start_value >= end_value {
                                    return Err(Error::new(
                                        end.span(),
                                        "End number must be greater than start number.",
                                    ));
                                }

                                let mut result = Vec::with_capacity((end_value - start_value) as usize);

                                for value in start_value..end_value {
                                    result.push(LitInt::new(&format!("{value}{suffix}"), span))
                                }

                                Ok(LitVec::Int(result))
                            })*

                            other => Err(Error::new(start.span(), format!("Unsupported suffix {other}"))),
                        }
                    };
                }

                match_suffix_inclusive!(
                    "u8" => u8,
                    "u16" => u16,
                    "u32" => u32,
                    "u64" => u64,
                    "u128" => u128,
                    "usize" => usize,
                    "i8" => i8,
                    "i16" => i16,
                    "i32" => i32,
                    "i64" => i64,
                    "i128" => i128,
                    "isize" => isize,
                )
            }

            Ok(LitVec::Int(vec![start]))
        }

        Lit::Float(start) => Err(Error::new(start.span(), "Float constants not supported.")),

        Lit::Bool(start) => Ok(LitVec::Bool(vec![start])),

        other => Err(Error::new(
            other.span(),
            "Unsupported constant literal syntax.",
        )),
    }
}

mod keyword {
    syn::custom_keyword!(dump);
    syn::custom_keyword!(include);
    syn::custom_keyword!(exclude);
    syn::custom_keyword!(shallow);
    syn::custom_keyword!(name);
    syn::custom_keyword!(readonly);
    syn::custom_keyword!(writeonly);
    syn::custom_keyword!(family);
    syn::custom_keyword!(package);
    syn::custom_keyword!(component);
}

mod names {
    syn::custom_keyword!(Expr);
    syn::custom_keyword!(Type);
    syn::custom_keyword!(Arg);
    syn::custom_keyword!(Ret);
}

mod cases {
    syn::custom_keyword!(Upper);
    syn::custom_keyword!(Lower);
    syn::custom_keyword!(Title);
    syn::custom_keyword!(Camel);
    syn::custom_keyword!(UpperCamel);
    syn::custom_keyword!(Snake);
    syn::custom_keyword!(UpperSnake);
    syn::custom_keyword!(Kebab);
    syn::custom_keyword!(UpperKebab);
    syn::custom_keyword!(Train);
    syn::custom_keyword!(Flat);
    syn::custom_keyword!(UpperFlat);
}
