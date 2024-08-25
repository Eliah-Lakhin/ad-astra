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

use std::cell::RefCell;

use ahash::{AHashMap, AHashSet};
use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote_spanned, ToTokens};
use syn::{spanned::Spanned, Type};

use crate::utils::{
    seed_hash_map,
    seed_hash_set,
    ty::is_str_type,
    Coercion,
    Context,
    Facade,
    Operator,
    TypeUtils,
};

#[derive(Clone, Copy)]
pub struct Shallow;

impl ToTokens for Shallow {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.borrow(|inner| inner.to_tokens(tokens));
    }
}

impl Shallow {
    #[inline(always)]
    pub fn init(self, shallow_mode: bool) {
        INNER.with_borrow_mut(move |inner| {
            if inner.is_some() {
                panic!("Internal error. Inner shallow interface already initialized.");
            }

            *inner = Some({
                let mut shallow_inner = ShallowInner::default();

                shallow_inner.shallow_mode = shallow_mode;

                shallow_inner
            });
        })
    }

    #[inline(always)]
    pub fn release(self, enforce: bool) {
        INNER.with_borrow_mut(|inner| {
            if enforce && inner.is_none() {
                panic!("Internal error. Inner shallow interface already released.");
            }

            *inner = None;
        })
    }

    #[inline(always)]
    pub fn enabled(self) -> bool {
        self.borrow(|inner| inner.shallow_mode)
    }

    pub fn assert_type_impls_downcast(self, ty: &Type, span: Span) {
        if !Shallow.enabled() {
            return;
        }

        self.borrow(|inner| {
            let _ = inner.assert_type_impls_downcast.insert(ty.clone(), span);
        });
    }

    pub fn assert_type_impls_upcast(self, ty: &Type, span: Span) {
        if !Shallow.enabled() {
            return;
        }

        self.borrow(|inner| {
            let _ = inner.assert_type_impls_upcast.insert(ty.clone(), span);
        });
    }

    pub fn assert_ref_type_impls_static_upcast(self, ty: &Type, span: Span) {
        if !Shallow.enabled() {
            return;
        }

        self.borrow(|inner| {
            let _ = inner
                .assert_ref_type_impls_static_upcast
                .insert(ty.clone(), span);
        });
    }

    pub fn assert_type_impls_script_type(self, ty: &Type, span: Span) {
        if !Shallow.enabled() {
            return;
        }

        self.borrow(|inner| {
            if inner.impl_registered_type.contains(ty) {
                return;
            }

            let _ = inner.assert_type_impls_script_type.insert(ty.clone(), span);
        });
    }

    pub fn assert_type_meets_op_requirements(self, ty: &Type, operator: Operator, span: Span) {
        if !Shallow.enabled() {
            return;
        }

        let key = (operator, ty.clone());

        if self.borrow(|inner| inner.impl_operator.contains_key(&key)) {
            return;
        }

        if let Operator::Debug | Operator::Display | Operator::Hash = operator {
            if !is_str_type(ty) {
                self.assert_type_impls_script_type(ty, span);
            }
        }

        self.borrow(|inner| {
            let _ = inner.assert_type_meets_op_requirements.insert(key, span);
        });
    }

    pub fn impl_registered_type(self, ty: &Type) {
        if !Shallow.enabled() {
            return;
        }

        self.borrow(|inner| {
            let _ = inner.assert_type_impls_script_type.remove(ty);
            let _ = inner.impl_registered_type.insert(ty.clone());
        });
    }

    pub fn impl_coercion(self, ty: &Type, coercion: Coercion) {
        if !Shallow.enabled() {
            return;
        }

        self.assert_type_impls_script_type(ty, ty.span());

        self.borrow(|inner| match inner.impl_coercion.get_mut(ty) {
            Some(previous) => previous.enrich(coercion),
            None => {
                let _ = inner.impl_coercion.insert(ty.clone(), coercion);
            }
        });
    }

    pub fn impl_operator(self, lhs: &Type, rhs: Option<&Type>, operator: Operator, span: Span) {
        if !Shallow.enabled() {
            return;
        }

        self.assert_type_impls_script_type(lhs, span);

        if let Some(rhs) = rhs {
            self.assert_type_impls_script_type(rhs, span);
        }

        self.borrow(|inner| {
            let _ = inner
                .impl_operator
                .insert((operator, lhs.clone()), (rhs.cloned(), span));

            let _ = inner
                .assert_type_meets_op_requirements
                .remove(&(operator, lhs.clone()));
        });
    }

    pub fn impl_package(self, ty: &Type, span: Span) {
        if !Shallow.enabled() {
            return;
        }

        self.borrow(|inner| {
            let _ = inner.impl_package.insert(ty.clone(), span);
        });
    }

    #[inline(always)]
    fn borrow<R>(&self, f: impl FnOnce(&mut ShallowInner) -> R) -> R {
        INNER.with_borrow_mut(|inner| {
            let Some(inner) = inner else {
                panic!("Internal error. Inner shallow interface is not initialized.");
            };

            f(inner)
        })
    }
}

thread_local! {
    static INNER: RefCell<Option<ShallowInner>> = RefCell::new(None)
}

struct ShallowInner {
    shallow_mode: bool,
    assert_type_impls_downcast: AHashMap<Type, Span>,
    assert_type_impls_upcast: AHashMap<Type, Span>,
    assert_ref_type_impls_static_upcast: AHashMap<Type, Span>,
    assert_type_impls_script_type: AHashMap<Type, Span>,
    assert_type_meets_op_requirements: AHashMap<(Operator, Type), Span>,
    impl_registered_type: AHashSet<Type>,
    impl_coercion: AHashMap<Type, Coercion>,
    impl_operator: AHashMap<(Operator, Type), (Option<Type>, Span)>,
    impl_package: AHashMap<Type, Span>,
}

impl Default for ShallowInner {
    fn default() -> Self {
        Self {
            shallow_mode: false,
            assert_type_impls_downcast: seed_hash_map(),
            assert_type_impls_upcast: seed_hash_map(),
            assert_ref_type_impls_static_upcast: seed_hash_map(),
            assert_type_impls_script_type: seed_hash_map(),
            assert_type_meets_op_requirements: seed_hash_map(),
            impl_registered_type: seed_hash_set(),
            impl_coercion: seed_hash_map(),
            impl_operator: seed_hash_map(),
            impl_package: seed_hash_map(),
        }
    }
}

impl ToTokens for ShallowInner {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = Context.span();
        let name = Context.name();

        let section = Context.make_section_name(name.as_str(), span);

        let mut body = TokenStream::new();

        for (ty, span) in &self.assert_type_impls_downcast {
            let core = span.face_core();

            quote_spanned!(*span=> let _ = <#ty as #core::runtime::Downcast>::downcast;)
                .to_tokens(&mut body);
        }

        for (ty, span) in &self.assert_type_impls_upcast {
            let core = span.face_core();

            quote_spanned!(*span=> let _ = <#ty as #core::runtime::Upcast>::upcast;)
                .to_tokens(&mut body);
        }

        for (ty, span) in &self.assert_ref_type_impls_static_upcast {
            let core = span.face_core();

            quote_spanned!(*span=> let _ = <&#ty as #core::runtime::Upcast<'static>>::upcast;)
                .to_tokens(&mut body);
        }

        for (ty, span) in &self.assert_type_impls_script_type {
            let core = span.face_core();

            quote_spanned!(*span=> let _ = <#ty as #core::runtime::ScriptType>::type_meta;)
                .to_tokens(&mut body);
        }

        for ((operator, lhs), span) in &self.assert_type_meets_op_requirements {
            Self::assert_operator(&mut body, operator, lhs, *span);
        }

        for ty in &self.impl_registered_type {
            ty.impl_registered_type().to_tokens(&mut body);
        }

        for (ty, coercion) in &self.impl_coercion {
            ty.impl_shallow_coercion(*coercion).to_tokens(&mut body);
        }

        for ((operator, lhs), (rhs, span)) in &self.impl_operator {
            Self::impl_operator(&mut body, operator, lhs, rhs, *span);
        }

        for (ty, span) in &self.impl_package {
            let core = span.face_core();
            let type_name = span.face_type_name();
            let panic = span.face_panic();

            quote_spanned!(*span=>
                #[allow(non_local_definitions)]
                impl #core::runtime::ScriptPackage for #ty {
                    fn meta() -> &'static #core::runtime::PackageMeta {
                        let name = #type_name::<Self>();
                        #panic("{name} type was not registered. Probably because export has been \
                        disabled for this type.");
                    }
                }
            )
            .to_tokens(&mut body);
        }

        if body.is_empty() {
            return;
        }

        quote_spanned!(span=> static #section: () = {
            #body
        };)
        .to_tokens(tokens);
    }
}

impl ShallowInner {
    fn assert_operator(body: &mut TokenStream, operator: &Operator, lhs: &Type, span: Span) {
        if let Operator::Clone = operator {
            let intrinsics = span.face_intrinsics();

            quote_spanned!(span=> let _ = #intrinsics::CloneFn::from_clone::<#lhs>;)
                .to_tokens(body);

            return;
        };

        let core = span.face_core();

        let description = operator.describe();

        let name = operator.to_string();
        let script_operator_ident = Ident::new(&format!("Script{}", name), span);

        if description.rhs {
            quote_spanned!(span=> let _ = <
                <#lhs as #core::runtime::ops::#script_operator_ident>::RHS
                as #core::runtime::ScriptType
            >::type_meta;)
            .to_tokens(body);
        }

        if description.result {
            quote_spanned!(span=> let _ = <
                <#lhs as #core::runtime::ops::#script_operator_ident>::Result
                as #core::runtime::ScriptType
            >::type_meta;)
            .to_tokens(body);
        }
    }

    fn impl_operator(
        body: &mut TokenStream,
        operator: &Operator,
        lhs: &Type,
        rhs: &Option<Type>,
        span: Span,
    ) {
        let result_type: Option<TokenStream>;

        let core = span.face_core();
        let type_name = span.face_type_name();
        let panic = span.face_panic();

        match operator {
            Operator::Field
            | Operator::Invocation
            | Operator::Binding
            | Operator::And
            | Operator::Or
            | Operator::None => return,

            Operator::Assign => {
                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::ScriptAssign for #lhs {
                        type RHS = #lhs;

                        fn script_assign(
                            _origin: #core::runtime::Origin,
                            _lhs: #core::runtime::Arg,
                            _rhs: #core::runtime::Arg,
                        ) -> #core::runtime::RuntimeResult<()> {
                            let name = #type_name::<Self>();
                            #panic("{name} type was not registered. Probably because export has been \
                            disabled for this type.");
                        }
                    }
                )
                .to_tokens(body);

                return;
            }

            Operator::Concat => {
                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::ScriptConcat for #lhs {
                        type Result = #lhs;

                        fn script_concat(
                            _origin: #core::runtime::Origin,
                            _items: &mut [#core::runtime::Arg],
                        ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                            let name = #type_name::<Self>();
                            #panic("{name} type was not registered. Probably because export has been \
                            disabled for this type.");
                        }
                    }
                )
                .to_tokens(body);

                return;
            }

            Operator::Clone => {
                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::ScriptClone for #lhs {}
                )
                .to_tokens(body);
                return;
            }

            Operator::Debug => {
                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::ScriptDebug for #lhs {}
                )
                .to_tokens(body);
                return;
            }

            Operator::Display => {
                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::ScriptDisplay for #lhs {}
                )
                .to_tokens(body);
                return;
            }

            Operator::PartialEq => {
                let rhs = rhs.as_ref().unwrap_or(lhs);

                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::ScriptPartialEq for #lhs {
                        type RHS = #rhs;

                        fn script_eq(
                            _origin: #core::runtime::Origin,
                            _lhs: #core::runtime::Arg,
                            _rhs: #core::runtime::Arg,
                        ) -> #core::runtime::RuntimeResult<bool> {
                            let name = #type_name::<Self>();
                            #panic("{name} type was not registered. Probably because export has been \
                            disabled for this type.");
                        }
                    }
                )
                .to_tokens(body);

                return;
            }

            Operator::Default => {
                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::ScriptDefault for #lhs {
                        fn script_default(
                            origin: #core::runtime::Origin,
                        ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                            let name = #type_name::<Self>();
                            #panic("{name} type was not registered. Probably because export has been \
                            disabled for this type.");
                        }
                    }
                )
                .to_tokens(body);
                return;
            }

            Operator::PartialOrd => {
                let rhs = rhs.as_ref().unwrap_or(lhs);

                let option = span.face_option();
                let ordering = span.face_ordering();

                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::ScriptPartialOrd for #lhs {
                        type RHS = #rhs;

                        fn script_partial_cmp(
                            _origin: #core::runtime::Origin,
                            _lhs: #core::runtime::Arg,
                            _rhs: #core::runtime::Arg,
                        ) -> #core::runtime::RuntimeResult<#option<#ordering>> {
                            let name = #type_name::<Self>();
                            #panic("{name} type was not registered. Probably because export has been \
                            disabled for this type.");
                        }
                    }
                )
                .to_tokens(body);

                return;
            }

            Operator::Ord => {
                let ordering = span.face_ordering();

                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::ScriptOrd for #lhs {
                        fn script_cmp(
                            _origin: #core::runtime::Origin,
                            _lhs: #core::runtime::Arg,
                            _rhs: #core::runtime::Arg,
                        ) -> #core::runtime::RuntimeResult<#ordering> {
                            let name = #type_name::<Self>();
                            #panic("{name} type was not registered. Probably because export has been \
                            disabled for this type.");
                        }
                    }
                )
                .to_tokens(body);

                return;
            }

            Operator::Hash => {
                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::ScriptHash for #lhs {}
                )
                .to_tokens(body);
                return;
            }

            Operator::Not => {
                let not = span.face_not();

                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::ScriptNot for #lhs {
                        type Result = <#lhs as #not>::Output;

                        fn script_not(
                            _origin: #core::runtime::Origin,
                            _lhs: #core::runtime::Arg,
                        ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                            let name = #type_name::<Self>();
                            #panic("{name} type was not registered. Probably because export has been \
                            disabled for this type.");
                        }
                    }
                )
                .to_tokens(body);

                return;
            }

            Operator::Neg => {
                let neg = span.face_neg();

                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::ScriptNeg for #lhs {
                        type Result = <#lhs as #neg>::Output;

                        fn script_neg(
                            _origin: #core::runtime::Origin,
                            _lhs: #core::runtime::Arg,
                        ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                            let name = #type_name::<Self>();
                            #panic("{name} type was not registered. Probably because export has been \
                            disabled for this type.");
                        }
                    }
                )
                .to_tokens(body);

                return;
            }

            Operator::Add => {
                result_type = Some(span.face_add());
            }

            Operator::Sub => {
                result_type = Some(span.face_sub());
            }

            Operator::Mul => {
                result_type = Some(span.face_mul());
            }

            Operator::Div => {
                result_type = Some(span.face_div());
            }

            Operator::BitAnd => {
                result_type = Some(span.face_bit_and());
            }

            Operator::BitOr => {
                result_type = Some(span.face_bit_or());
            }

            Operator::BitXor => {
                result_type = Some(span.face_bit_xor());
            }

            Operator::Shl => {
                result_type = Some(span.face_shl());
            }

            Operator::Shr => {
                result_type = Some(span.face_shr());
            }

            Operator::Rem => {
                result_type = Some(span.face_rem());
            }

            Operator::AddAssign
            | Operator::SubAssign
            | Operator::MulAssign
            | Operator::DivAssign
            | Operator::BitAndAssign
            | Operator::BitOrAssign
            | Operator::BitXorAssign
            | Operator::ShlAssign
            | Operator::ShrAssign
            | Operator::RemAssign => {
                result_type = None;
            }
        }

        let rhs = rhs.as_ref().unwrap_or(lhs);

        let operator_name = operator.to_string();
        let script_operator = Ident::new(&format!("Script{operator_name}"), span);
        let script_function = Ident::new(
            &format!("script_{}", operator_name.to_case(Case::Snake)),
            span,
        );

        let output;
        let assertion;

        match result_type {
            None => {
                output = None;
                assertion = None;
            }

            Some(ty) => {
                output = Some(quote_spanned!(span=> type Result = <#lhs as #ty::<#rhs>>::Output;));
                assertion = Some(quote_spanned!(span=>
                    let _ = <Self::Output as #core::runtime::Upcast<'static>>::upcast;
                ));
            }
        };

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #core::runtime::ops::#script_operator for #lhs {
                type RHS = #rhs;
                #output

                fn #script_function(
                    _origin: #core::runtime::Origin,
                    _lhs: #core::runtime::Arg,
                    _rhs: #core::runtime::Arg,
                ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                    #assertion

                    let name = #type_name::<Self>();
                    #panic("{name} type was not registered. Probably because export has been \
                    disabled for this type.");
                }
            }
        )
        .to_tokens(body)
    }
}
