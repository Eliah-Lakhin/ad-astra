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

use std::borrow::Cow;

use proc_macro2::{Ident, Span};
use quote::{quote_spanned, ToTokens};
use syn::{
    spanned::Spanned,
    Error,
    ImplItem,
    ItemImpl,
    LitStr,
    Result,
    Signature,
    TraitItem,
    Type,
    Visibility,
};

use crate::{
    export::ExportConfig,
    utils::{
        Attrs,
        Component,
        Context,
        DefaultScriptOperator,
        EmptyPolymorphism,
        Exportable,
        Facade,
        FunctionPolymorphism,
        Group,
        IdRef,
        ImplPolymorphism,
        Invocation,
        Operator,
        OperatorOrigin,
        PathMeta,
        PathUtils,
        PolymorphicScope,
        Prototype,
        ScriptAdd,
        ScriptAddAssign,
        ScriptBitAnd,
        ScriptBitAndAssign,
        ScriptBitOr,
        ScriptBitOrAssign,
        ScriptBitXor,
        ScriptBitXorAssign,
        ScriptClone,
        ScriptDebug,
        ScriptDefault,
        ScriptDisplay,
        ScriptDiv,
        ScriptDivAssign,
        ScriptHash,
        ScriptMul,
        ScriptMulAssign,
        ScriptNeg,
        ScriptNot,
        ScriptOrd,
        ScriptPartialEq,
        ScriptPartialOrd,
        ScriptRem,
        ScriptRemAssign,
        ScriptShl,
        ScriptShlAssign,
        ScriptShr,
        ScriptShrAssign,
        ScriptSub,
        ScriptSubAssign,
        Shallow,
        SignaturePolymorphism,
        COMPONENT,
        DUMP,
        EXCLUDED,
        INCLUDED,
        RENAME,
        SHALLOW,
        UNSPECIFIED,
    },
};

pub fn export_item_impl(item: &mut ItemImpl) -> Result<ExportConfig> {
    let attrs = item.drain_attrs()?;

    attrs.check(DUMP | INCLUDED | EXCLUDED | SHALLOW)?;

    Shallow.init(attrs.shallow());

    let mut impl_polymorphism = ImplPolymorphism::new(item)?;

    let mut group = Group::default();

    loop {
        if let Some((_, trait_path, _)) = &item.trait_ {
            if let Some(operator) = trait_path.matches_operator() {
                export_custom_operator(&mut group, &mut impl_polymorphism, operator)?;
                break;
            }

            if let Some(meta) = trait_path.matches_clone() {
                export_default_operator::<ScriptClone>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Clone,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_copy() {
                export_default_operator::<ScriptClone>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Clone,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_debug() {
                export_default_operator::<ScriptDebug>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Debug,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_display() {
                export_default_operator::<ScriptDisplay>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Display,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_partial_eq() {
                export_default_operator::<ScriptPartialEq>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::PartialEq,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_eq() {
                export_default_operator::<ScriptPartialEq>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::PartialEq,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_eq() {
                export_default_operator::<ScriptPartialEq>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::PartialEq,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_default() {
                export_default_operator::<ScriptDefault>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Default,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_partial_ord() {
                export_default_operator::<ScriptPartialOrd>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::PartialOrd,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_ord() {
                export_default_operator::<ScriptOrd>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Ord,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_hash() {
                export_default_operator::<ScriptHash>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Hash,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_add() {
                export_default_operator::<ScriptAdd>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Add,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_add_assign() {
                export_default_operator::<ScriptAddAssign>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::AddAssign,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_sub() {
                export_default_operator::<ScriptSub>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Sub,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_sub_assign() {
                export_default_operator::<ScriptSubAssign>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::SubAssign,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_mul() {
                export_default_operator::<ScriptMul>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Mul,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_mul_assign() {
                export_default_operator::<ScriptMulAssign>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::SubAssign,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_div() {
                export_default_operator::<ScriptDiv>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Div,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_div_assign() {
                export_default_operator::<ScriptDivAssign>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::DivAssign,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_not() {
                export_default_operator::<ScriptNot>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Not,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_neg() {
                export_default_operator::<ScriptNeg>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Neg,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_bit_and() {
                export_default_operator::<ScriptBitAnd>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::BitAnd,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_bit_and_assign() {
                export_default_operator::<ScriptBitAndAssign>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::BitAndAssign,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_bit_or() {
                export_default_operator::<ScriptBitOr>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::BitOr,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_bit_or_assign() {
                export_default_operator::<ScriptBitOrAssign>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::BitOrAssign,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_bit_xor() {
                export_default_operator::<ScriptBitXor>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::BitXor,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_bit_xor_assign() {
                export_default_operator::<ScriptBitXorAssign>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::BitXorAssign,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_shl() {
                export_default_operator::<ScriptShl>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Shl,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_shl_assign() {
                export_default_operator::<ScriptShlAssign>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::ShlAssign,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_shr() {
                export_default_operator::<ScriptShr>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Shr,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_shr_assign() {
                export_default_operator::<ScriptShrAssign>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::ShrAssign,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_rem() {
                export_default_operator::<ScriptRem>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::Rem,
                    meta,
                )?;
                break;
            }

            if let Some(meta) = trait_path.matches_rem_assign() {
                export_default_operator::<ScriptRemAssign>(
                    &mut group,
                    &mut impl_polymorphism,
                    Operator::RemAssign,
                    meta,
                )?;
                break;
            }
        }

        export_impl_body(item, &mut group, &mut impl_polymorphism)?;
        break;
    }

    Ok(ExportConfig {
        dump: attrs.dump(),
        stream: match attrs.disabled() {
            true => None,
            false => match attrs.shallow() {
                true => Some(Shallow.to_token_stream()),
                false => Some(group.to_token_stream()),
            },
        },
    })
}

fn export_custom_operator(
    group: &mut Group,
    impl_polymorphism: &mut ImplPolymorphism,
    operator: Operator,
) -> Result<()> {
    loop {
        let ty = impl_polymorphism
            .get_self_type()?
            .expect("Internal error. Missing self type.");

        let mut prototype = Prototype::for_type(ty);

        prototype.operator(OperatorOrigin::Primary, operator);

        group.prototype(prototype);

        Shallow.assert_type_meets_op_requirements(ty, operator, Context.span());

        if !impl_polymorphism.rotate()? {
            break;
        }
    }

    Ok(())
}

fn export_default_operator<D: DefaultScriptOperator>(
    group: &mut Group,
    impl_polymorphism: &mut ImplPolymorphism,
    operator: Operator,
    meta: PathMeta<'_>,
) -> Result<()> {
    loop {
        let ty = impl_polymorphism
            .get_self_type()?
            .expect("Internal error. Missing self type.");

        let arg = match meta.args.first() {
            None => None,
            Some(arg) => {
                let mut arg = (*arg).clone();

                impl_polymorphism.specialize_type(&mut arg)?;

                Some(arg)
            }
        };

        group.custom(D::new_stream(meta.span, ty, arg.as_ref()));

        let mut prototype = Prototype::for_type(ty);

        prototype.operator(OperatorOrigin::Primary, operator);

        group.prototype(prototype);

        Shallow.impl_operator(ty, arg.as_ref(), operator, meta.span);

        if !impl_polymorphism.rotate()? {
            break;
        }
    }

    Ok(())
}

fn export_impl_body(
    item: &mut ItemImpl,
    group: &mut Group,
    impl_polymorphism: &mut ImplPolymorphism,
) -> Result<()> {
    let implicitly_included = impl_polymorphism.get_trait_type()?.is_some();

    let mut item_set = ItemSet::from_impl_items(&mut item.items, implicitly_included)?;

    let mut package_prototype = match item_set.has_package_items {
        false => None,
        true => Some(Prototype::for_package(item.impl_token.span)),
    };

    loop {
        let mut self_prototype = match item_set.has_self_items {
            false => None,

            true => {
                let ty = impl_polymorphism
                    .get_self_type()?
                    .expect("Internal error. Missing self type.");

                Some(Prototype::for_type(ty))
            }
        };

        item_set.export::<ImplPolymorphism>(
            &impl_polymorphism,
            group,
            &mut self_prototype,
            &mut package_prototype,
        )?;

        if let Some(prototype) = self_prototype {
            group.prototype(prototype);
        }

        if !impl_polymorphism.rotate()? {
            break;
        }
    }

    if let Some(prototype) = package_prototype {
        group.prototype(prototype);
    }

    Ok(())
}

pub(super) struct ItemSet<'a> {
    has_self_items: bool,
    has_package_items: bool,
    items: Vec<ItemMeta<'a>>,
}

impl<'a> ItemSet<'a> {
    pub(super) fn from_trait_items(source: &'a mut [TraitItem]) -> Result<Self> {
        let mut items = Vec::with_capacity(source.len());
        let mut has_self_items = false;
        let mut has_package_items = false;

        for source in source {
            let item = match source {
                TraitItem::Const(source) => {
                    if !source.generics.params.is_empty() {
                        return Err(Error::new(
                            source.generics.span(),
                            "Constants with generics are not supported by the introspection system.",
                        ));
                    }

                    let attrs = source.drain_attrs()?;

                    match ConstMeta::new(attrs, true, &source.ident, &source.ty, source.rust_doc())?
                    {
                        Some(item) => {
                            has_package_items = true;
                            ItemMeta::Const(item)
                        }

                        None => continue,
                    }
                }

                TraitItem::Fn(source) => {
                    let attrs = source.drain_attrs()?;

                    match FnMeta::new(attrs, true, source.rust_doc(), &mut source.sig)? {
                        Some(item) => {
                            match &item.kind {
                                FnKind::Component(..) => has_self_items = true,
                                FnKind::Invocation(invocation) if invocation.uses_receiver() => {
                                    has_self_items = true;
                                }
                                _ => has_package_items = true,
                            }

                            ItemMeta::Fn(item)
                        }
                        None => continue,
                    }
                }

                _ => continue,
            };

            items.push(item);
        }

        Ok(Self {
            has_self_items,
            has_package_items,
            items,
        })
    }

    fn from_impl_items(source: &'a mut [ImplItem], implicitly_included: bool) -> Result<Self> {
        let mut items = Vec::with_capacity(source.len());
        let mut has_self_items = false;
        let mut has_package_items = false;

        for source in source {
            let item = match source {
                ImplItem::Const(source) => {
                    if !source.generics.params.is_empty() {
                        return Err(Error::new(
                            source.generics.span(),
                            "Constants with generics are not supported by the introspection system.",
                        ));
                    }

                    let attrs = source.drain_attrs()?;

                    let implicitly_included = implicitly_included
                        || match source.vis {
                            Visibility::Public(..) => true,
                            _ => false,
                        };

                    match ConstMeta::new(
                        attrs,
                        implicitly_included,
                        &source.ident,
                        &source.ty,
                        source.rust_doc(),
                    )? {
                        Some(item) => {
                            has_package_items = true;
                            ItemMeta::Const(item)
                        }

                        None => continue,
                    }
                }

                ImplItem::Fn(source) => {
                    let attrs = source.drain_attrs()?;

                    let implicitly_included = implicitly_included
                        || match source.vis {
                            Visibility::Public(..) => true,
                            _ => false,
                        };

                    match FnMeta::new(
                        attrs,
                        implicitly_included,
                        source.rust_doc(),
                        &mut source.sig,
                    )? {
                        Some(item) => {
                            match &item.kind {
                                FnKind::Component(..) => has_self_items = true,
                                FnKind::Invocation(invocation) if invocation.uses_receiver() => {
                                    has_self_items = true;
                                }
                                _ => has_package_items = true,
                            }

                            ItemMeta::Fn(item)
                        }
                        None => continue,
                    }
                }

                _ => continue,
            };

            items.push(item);
        }

        Ok(Self {
            has_self_items,
            has_package_items,
            items,
        })
    }

    #[inline(always)]
    pub(super) fn has_self_items(&self) -> bool {
        self.has_self_items
    }

    #[inline(always)]
    pub(super) fn has_package_items(&self) -> bool {
        self.has_package_items
    }

    pub(super) fn export<S: PolymorphicScope>(
        &mut self,
        scope: &S,
        group: &mut Group,
        self_prototype: &mut Option<Prototype<'_>>,
        package_prototype: &mut Option<Prototype<'_>>,
    ) -> Result<()> {
        for item in &mut self.items {
            match item {
                ItemMeta::Const(item) => {
                    let prototype = package_prototype
                        .as_mut()
                        .expect("Internal error. Missing package PrototypeDeclaration.");

                    item.export(scope, item.doc.clone(), prototype)?;
                }

                ItemMeta::Fn(item) => match &item.kind {
                    FnKind::Component(..) => {
                        let prototype = self_prototype
                            .as_mut()
                            .expect("Internal error. Missing self PrototypeDeclaration.");

                        item.export(scope, group, prototype)?;
                    }

                    FnKind::Invocation(invocation) if invocation.uses_receiver() => {
                        let prototype = self_prototype
                            .as_mut()
                            .expect("Internal error. Missing self PrototypeDeclaration.");

                        item.export(scope, group, prototype)?;
                    }

                    _ => {
                        let prototype = package_prototype
                            .as_mut()
                            .expect("Internal error. Missing package PrototypeDeclaration.");

                        item.export(scope, group, prototype)?;
                    }
                },
            }
        }

        Ok(())
    }
}

enum ItemMeta<'a> {
    Const(ConstMeta<'a>),
    Fn(FnMeta<'a>),
}

struct ConstMeta<'a> {
    attrs: Attrs,
    name_ref: Option<IdRef>,
    ident: &'a Ident,
    ty: &'a Type,
    doc: Option<LitStr>,
}

impl<'a> ConstMeta<'a> {
    #[inline]
    fn new(
        attrs: Attrs,
        implicitly_included: bool,
        ident: &'a Ident,
        ty: &'a Type,
        doc: Option<LitStr>,
    ) -> Result<Option<Self>> {
        attrs.check(UNSPECIFIED | INCLUDED | EXCLUDED | RENAME)?;

        if attrs.excluded() {
            return Ok(None);
        }

        if !implicitly_included && !attrs.included() && !attrs.specified() {
            return Ok(None);
        }

        let name_ref = match attrs.has_rename_variables() {
            true => None,

            false => Some(
                attrs
                    .rename_checked(&EmptyPolymorphism)?
                    .map(|name| Context.make_unique_identifier(name.as_str(), ident.span()))
                    .unwrap_or_else(|| {
                        Context.make_unique_identifier(ident.to_string().as_str(), ident.span())
                    }),
            ),
        };

        Ok(Some(Self {
            attrs,
            name_ref,
            ident,
            ty,
            doc,
        }))
    }

    #[inline]
    fn export<S: PolymorphicScope>(
        &self,
        scope: &S,
        doc: Option<LitStr>,
        prototype: &mut Prototype<'_>,
    ) -> Result<()> {
        let ident = self.ident;
        let span = ident.span();

        let name_ref = match &self.name_ref {
            Some(name_ref) => name_ref.clone(),

            None => {
                let name = self
                    .attrs
                    .rename_checked(scope)?
                    .expect("Internal error. Missing constant name.");

                Context.make_shared_identifier(name.as_str(), span)
            }
        };

        let constructor = {
            let core = span.face_core();

            let self_type = scope.get_self_type()?;
            let trait_type = scope.get_trait_type()?;

            let component = match trait_type {
                None => quote_spanned!(span=> #self_type::#ident),
                Some(trait_type) => quote_spanned!(span=> <#self_type as #trait_type>::#ident),
            };

            quote_spanned!(span=> {
                fn component(
                    origin: #core::runtime::Origin,
                    _lhs: #core::runtime::Arg,
                ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                    #core::runtime::Cell::give(origin, &#component)
                }

                component as fn(
                    #core::runtime::Origin,
                    #core::runtime::Arg,
                ) -> #core::runtime::RuntimeResult::<#core::runtime::Cell>
            })
        };

        let mut ty = self.ty.clone();

        scope.specialize_type(&mut ty)?;

        Shallow.assert_ref_type_impls_static_upcast(&ty, span);

        prototype.component(Component {
            name_ref: Cow::Owned(name_ref),
            constructor,
            hint: Cow::Owned(ty),
            doc,
        });

        Ok(())
    }
}

struct FnMeta<'a> {
    attrs: Attrs,
    span: Span,
    name: Option<(String, IdRef)>,
    doc: Option<LitStr>,
    signature_polymorphism: SignaturePolymorphism<'a>,
    kind: FnKind<'a>,
}

impl<'a> FnMeta<'a> {
    #[inline]
    fn new(
        attrs: Attrs,
        implicitly_included: bool,
        doc: Option<LitStr>,
        sig: &'a mut Signature,
    ) -> Result<Option<Self>> {
        attrs.check(UNSPECIFIED | INCLUDED | EXCLUDED | RENAME | COMPONENT)?;

        if attrs.excluded() {
            return Ok(None);
        }

        if !attrs.included() && !implicitly_included && !attrs.specified() {
            return Ok(None);
        }

        let span = sig.ident.span();

        let name = match attrs.has_rename_variables() {
            true => None,

            false => attrs
                .rename_checked(&EmptyPolymorphism)?
                .map(|name| {
                    let name_ref = Context.make_unique_identifier(name.as_str(), span);

                    Some((name, name_ref))
                })
                .unwrap_or_else(|| {
                    let name = sig.ident.to_string();
                    let name_ref = Context.make_unique_identifier(name.as_str(), span);

                    Some((name, name_ref))
                }),
        };

        let signature_polymorphism = SignaturePolymorphism::new(
            &sig.ident,
            &mut sig.generics,
            &mut sig.inputs,
            &sig.output,
        )?;

        let kind = match attrs.component().is_some() {
            false => FnKind::Invocation(Invocation::new(sig)?),

            true => {
                if !sig.generics.params.is_empty() {
                    return Err(Error::new(
                        sig.generics.span(),
                        "Component functions with generic parameters are not allowed.",
                    ));
                }

                FnKind::Component(&sig.ident)
            }
        };

        Ok(Some(Self {
            attrs,
            span,
            name,
            doc,
            signature_polymorphism,
            kind,
        }))
    }

    #[inline]
    fn export<S: PolymorphicScope>(
        &mut self,
        scope: &S,
        group: &mut Group,
        prototype: &mut Prototype<'_>,
    ) -> Result<()> {
        let span = self.span;

        let core = span.face_core();

        loop {
            let function_polymorphism = FunctionPolymorphism {
                scope,
                signature: &self.signature_polymorphism,
            };

            let name;
            let name_ref;

            match &self.name {
                Some((name_string, name_reference)) => {
                    name = Cow::Borrowed(name_string);
                    name_ref = name_reference.clone();
                }

                None => {
                    let name_string = self
                        .attrs
                        .rename_checked(&function_polymorphism)?
                        .expect("Internal error. Missing function name.");

                    name_ref = Context.make_shared_identifier(name_string.as_str(), span);
                    name = Cow::Owned(name_string)
                }
            };

            match &self.kind {
                FnKind::Invocation(invocation) => {
                    let function_type = invocation.make_function_type(
                        group,
                        &function_polymorphism,
                        name.as_str(),
                        &name_ref,
                        self.doc.clone(),
                    )?;

                    let component = match invocation.uses_receiver() {
                        true => quote_spanned!(span=> fn component(
                            origin: #core::runtime::Origin,
                            lhs: #core::runtime::Arg,
                        ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                            #core::runtime::Cell::give(origin, #function_type(lhs))
                        }),

                        false => quote_spanned!(span=> fn component(
                            origin: #core::runtime::Origin,
                            _lhs: #core::runtime::Arg,
                        ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                            #core::runtime::Cell::give(origin, #function_type)
                        }),
                    };

                    let constructor = quote_spanned!(span=> {
                        #component

                        component as fn(
                            #core::runtime::Origin,
                            #core::runtime::Arg,
                        ) -> #core::runtime::RuntimeResult::<#core::runtime::Cell>
                    });

                    prototype.component(Component {
                        name_ref: Cow::Owned(name_ref),
                        constructor,
                        hint: Cow::Owned(function_type),
                        doc: self.doc.clone(),
                    });
                }

                FnKind::Component(ident) => {
                    let mut ty = self
                        .attrs
                        .component()
                        .expect("Internal error. Missing component type.")
                        .clone();

                    function_polymorphism.specialize_type(&mut ty)?;

                    let self_type = function_polymorphism
                        .get_self_type()?
                        .expect("Internal error. Missing self type.");
                    let trait_type = function_polymorphism.get_trait_type()?;

                    let component = match trait_type {
                        None => quote_spanned!(span=> <#self_type>::#ident),

                        Some(trait_type) => quote_spanned!(span=>
                            <#self_type as #trait_type>::#ident),
                    };

                    let constructor = quote_spanned!(span=>
                        #component as fn(
                            #core::runtime::Origin,
                            #core::runtime::Arg,
                        ) -> #core::runtime::RuntimeResult::<#core::runtime::Cell>
                    );

                    Shallow.assert_ref_type_impls_static_upcast(&ty, span);

                    prototype.component(Component {
                        name_ref: Cow::Owned(name_ref),
                        constructor,
                        hint: Cow::Owned(ty),
                        doc: self.doc.clone(),
                    });
                }
            }

            if !self.signature_polymorphism.rotate() {
                break;
            }
        }

        Ok(())
    }
}

enum FnKind<'a> {
    Invocation(Invocation<'a>),
    Component(&'a Ident),
}
