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

use proc_macro2::{Span, TokenStream};
use quote::{quote_spanned, ToTokens};
use syn::Type;

use crate::utils::Facade;

pub trait DefaultScriptOperator {
    fn new_stream(span: Span, lhs: &Type, rhs: Option<&Type>) -> TokenStream;
}

pub struct ScriptAssign<'a> {
    pub span: Span,
    pub ty: &'a Type,
}

impl<'a> ToTokens for ScriptAssign<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;

        let core = span.face_core();
        let intrinsics = span.face_intrinsics();

        let ty = self.ty;

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #core::runtime::ops::ScriptAssign for #ty {
                type RHS = #ty;

                fn script_assign(
                    _origin: #core::runtime::Origin,
                    lhs: #core::runtime::Arg,
                    rhs: #core::runtime::Arg,
                ) -> #core::runtime::RuntimeResult<()> {
                    #intrinsics::canonicals::script_assign::<#ty>(lhs, rhs)
                }
            }
        )
        .to_tokens(tokens)
    }
}

pub struct ScriptConcat<'a> {
    pub span: Span,
    pub ty: &'a Type,
}

impl<'a> ToTokens for ScriptConcat<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;

        let core = span.face_core();
        let intrinsics = span.face_intrinsics();

        let ty = self.ty;

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #core::runtime::ops::ScriptConcat for #ty {
                type Result = #ty;

                fn script_concat(
                    origin: #core::runtime::Origin,
                    items: &mut [#core::runtime::Arg],
                ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                    #intrinsics::canonicals::script_concat::<#ty>(origin, items)
                }
            }
        )
        .to_tokens(tokens)
    }
}

pub struct ScriptClone<'a> {
    pub span: Span,
    pub lhs: &'a Type,
}

impl<'a> DefaultScriptOperator for ScriptClone<'a> {
    #[inline(always)]
    fn new_stream(span: Span, lhs: &Type, _rhs: Option<&Type>) -> TokenStream {
        ScriptClone { span, lhs }.to_token_stream()
    }
}

impl<'a> ToTokens for ScriptClone<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;

        let core = span.face_core();

        let lhs = self.lhs;

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #core::runtime::ops::ScriptClone for #lhs {}
        )
        .to_tokens(tokens)
    }
}

pub struct ScriptDebug<'a> {
    pub span: Span,
    pub lhs: &'a Type,
}

impl<'a> DefaultScriptOperator for ScriptDebug<'a> {
    #[inline(always)]
    fn new_stream(span: Span, lhs: &Type, _rhs: Option<&Type>) -> TokenStream {
        ScriptDebug { span, lhs }.to_token_stream()
    }
}

impl<'a> ToTokens for ScriptDebug<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;

        let core = span.face_core();

        let lhs = self.lhs;

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #core::runtime::ops::ScriptDebug for #lhs {}
        )
        .to_tokens(tokens)
    }
}

pub struct ScriptDisplay<'a> {
    pub span: Span,
    pub lhs: &'a Type,
}

impl<'a> DefaultScriptOperator for ScriptDisplay<'a> {
    #[inline(always)]
    fn new_stream(span: Span, lhs: &Type, _rhs: Option<&Type>) -> TokenStream {
        ScriptDisplay { span, lhs }.to_token_stream()
    }
}

impl<'a> ToTokens for ScriptDisplay<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;

        let core = span.face_core();

        let lhs = self.lhs;

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #core::runtime::ops::ScriptDisplay for #lhs {}
        )
        .to_tokens(tokens)
    }
}

pub struct ScriptPartialEq<'a> {
    pub span: Span,
    pub lhs: &'a Type,
    pub rhs: &'a Type,
}

impl<'a> DefaultScriptOperator for ScriptPartialEq<'a> {
    #[inline(always)]
    fn new_stream(span: Span, lhs: &Type, rhs: Option<&Type>) -> TokenStream {
        ScriptPartialEq {
            span,
            lhs,
            rhs: rhs.unwrap_or(lhs),
        }
        .to_token_stream()
    }
}

impl<'a> ToTokens for ScriptPartialEq<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;

        let core = span.face_core();
        let partial_eq = span.face_partial_eq();

        let lhs = self.lhs;
        let rhs = self.rhs;

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #core::runtime::ops::ScriptPartialEq for #lhs {
                type RHS = #rhs;

                fn script_eq(
                    _origin: #core::runtime::Origin,
                    mut lhs: #core::runtime::Arg,
                    mut rhs: #core::runtime::Arg,
                ) -> #core::runtime::RuntimeResult<bool> {
                    let lhs = #core::runtime::Cell::borrow_ref::<#lhs>(
                        &mut lhs.data,
                        lhs.origin,
                    )?;

                    let rhs = #core::runtime::Cell::borrow_ref::<#rhs>(
                        &mut rhs.data,
                        rhs.origin,
                    )?;

                    #core::runtime::RuntimeResult::<bool>::Ok(
                        <#lhs as #partial_eq::<#rhs>>::eq(lhs, rhs),
                    )
                }
            }
        )
        .to_tokens(tokens)
    }
}

pub struct ScriptDefault<'a> {
    pub span: Span,
    pub lhs: &'a Type,
}

impl<'a> DefaultScriptOperator for ScriptDefault<'a> {
    #[inline(always)]
    fn new_stream(span: Span, lhs: &Type, _rhs: Option<&Type>) -> TokenStream {
        ScriptDefault { span, lhs }.to_token_stream()
    }
}

impl<'a> ToTokens for ScriptDefault<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;

        let core = span.face_core();
        let default = span.face_default();

        let lhs = self.lhs;

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #core::runtime::ops::ScriptDefault for #lhs {
                fn script_default(
                    origin: #core::runtime::Origin,
                ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                    #core::runtime::Cell::give(
                        origin,
                        <#lhs as #default>::default(),
                    )
                }
            }
        )
        .to_tokens(tokens)
    }
}

pub struct ScriptPartialOrd<'a> {
    pub span: Span,
    pub lhs: &'a Type,
    pub rhs: &'a Type,
}

impl<'a> DefaultScriptOperator for ScriptPartialOrd<'a> {
    #[inline(always)]
    fn new_stream(span: Span, lhs: &Type, rhs: Option<&Type>) -> TokenStream {
        ScriptPartialOrd {
            span,
            lhs,
            rhs: rhs.unwrap_or(lhs),
        }
        .to_token_stream()
    }
}

impl<'a> ToTokens for ScriptPartialOrd<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;

        let core = span.face_core();
        let option = span.face_option();
        let ordering = span.face_ordering();
        let partial_ord = span.face_partial_ord();

        let lhs = self.lhs;
        let rhs = self.rhs;

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #core::runtime::ops::ScriptPartialOrd for #lhs {
                type RHS = #rhs;

                fn script_partial_cmp(
                    _origin: #core::runtime::Origin,
                    mut lhs: #core::runtime::Arg,
                    mut rhs: #core::runtime::Arg,
                ) -> #core::runtime::RuntimeResult<#option<#ordering>> {
                    let lhs = #core::runtime::Cell::borrow_ref::<#lhs>(
                        &mut lhs.data,
                        lhs.origin,
                    )?;

                    let rhs = #core::runtime::Cell::borrow_ref::<#rhs>(
                        &mut rhs.data,
                        rhs.origin,
                    )?;

                    #core::runtime::RuntimeResult::<#option<#ordering>>::Ok(
                        <#lhs as #partial_ord::<#rhs>>::partial_cmp(lhs, rhs),
                    )
                }
            }
        )
        .to_tokens(tokens)
    }
}

pub struct ScriptOrd<'a> {
    pub span: Span,
    pub lhs: &'a Type,
}

impl<'a> DefaultScriptOperator for ScriptOrd<'a> {
    #[inline(always)]
    fn new_stream(span: Span, lhs: &Type, _rhs: Option<&Type>) -> TokenStream {
        ScriptOrd { span, lhs }.to_token_stream()
    }
}

impl<'a> ToTokens for ScriptOrd<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;

        let core = span.face_core();
        let ordering = span.face_ordering();
        let ord = span.face_ord();

        let lhs = self.lhs;

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #core::runtime::ops::ScriptOrd for #lhs {
                fn script_cmp(
                    _origin: #core::runtime::Origin,
                    mut lhs: #core::runtime::Arg,
                    mut rhs: #core::runtime::Arg,
                ) -> #core::runtime::RuntimeResult<#ordering> {
                    let lhs = #core::runtime::Cell::borrow_ref::<#lhs>(
                        &mut lhs.data,
                        lhs.origin,
                    )?;

                    let rhs = #core::runtime::Cell::borrow_ref::<#lhs>(
                        &mut rhs.data,
                        rhs.origin,
                    )?;

                    #core::runtime::RuntimeResult::<#ordering>::Ok(
                        <#lhs as #ord>::cmp(lhs, rhs),
                    )
                }
            }
        )
        .to_tokens(tokens)
    }
}

pub struct ScriptHash<'a> {
    pub span: Span,
    pub lhs: &'a Type,
}

impl<'a> DefaultScriptOperator for ScriptHash<'a> {
    #[inline(always)]
    fn new_stream(span: Span, lhs: &Type, _rhs: Option<&Type>) -> TokenStream {
        ScriptHash { span, lhs }.to_token_stream()
    }
}

impl<'a> ToTokens for ScriptHash<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;

        let core = span.face_core();

        let lhs = self.lhs;

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #core::runtime::ops::ScriptHash for #lhs {}
        )
        .to_tokens(tokens)
    }
}

macro_rules! binary {
    (
        impl $script_operator:ident($script_function:ident)
        for $facade:ident($facade_function:ident)
    ) => {
        pub struct $script_operator<'a> {
            pub span: Span,
            pub lhs: &'a Type,
            pub rhs: &'a Type,
        }

        impl<'a> DefaultScriptOperator for $script_operator<'a> {
            #[inline(always)]
            fn new_stream(span: Span, lhs: &Type, rhs: Option<&Type>) -> TokenStream {
                $script_operator {
                    span,
                    lhs,
                    rhs: rhs.unwrap_or(lhs),
                }
                .to_token_stream()
            }
        }

        impl<'a> ToTokens for $script_operator<'a> {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                let span = self.span;

                let core = span.face_core();
                let $facade = span.$facade();

                let lhs = self.lhs;
                let rhs = self.rhs;

                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::$script_operator for #lhs {
                        type RHS = #rhs;
                        type Result = <#lhs as #$facade::<#rhs>>::Output;

                        fn $script_function(
                            origin: #core::runtime::Origin,
                            lhs: #core::runtime::Arg,
                            rhs: #core::runtime::Arg,
                        ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                            let lhs = #core::runtime::Cell::take::<#lhs>(
                                lhs.data,
                                lhs.origin,
                            )?;

                            let rhs = #core::runtime::Cell::take::<#rhs>(
                                rhs.data,
                                rhs.origin,
                            )?;

                            let result = <#lhs as #$facade::<#rhs>>::$facade_function(
                                lhs,
                                rhs,
                            );

                            #core::runtime::Cell::give(origin, result)
                        }
                    }
                )
                .to_tokens(tokens)
            }
        }
    };
}

macro_rules! binary_assign {
    (
        impl $script_operator:ident($script_function:ident)
        for $facade:ident($facade_function:ident)
    ) => {
        pub struct $script_operator<'a> {
            pub span: Span,
            pub lhs: &'a Type,
            pub rhs: &'a Type,
        }

        impl<'a> DefaultScriptOperator for $script_operator<'a> {
            #[inline(always)]
            fn new_stream(span: Span, lhs: &Type, rhs: Option<&Type>) -> TokenStream {
                $script_operator {
                    span,
                    lhs,
                    rhs: rhs.unwrap_or(lhs),
                }
                .to_token_stream()
            }
        }

        impl<'a> ToTokens for $script_operator<'a> {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                let span = self.span;

                let core = span.face_core();
                let $facade = span.$facade();

                let lhs = self.lhs;
                let rhs = self.rhs;

                quote_spanned!(span=>
                    #[allow(non_local_definitions)]
                    impl #core::runtime::ops::$script_operator for #lhs {
                        type RHS = #rhs;

                        fn $script_function(
                            _origin: #core::runtime::Origin,
                            mut lhs: #core::runtime::Cell,
                            rhs: #core::runtime::Arg,
                        ) -> #core::runtime::RuntimeResult<()> {
                            let rhs = #core::runtime::Cell::take::<#rhs>(
                                rhs.data,
                                rhs.origin,
                            )?;

                            let lhs = #core::runtime::Cell::borrow_mut::<#lhs>(
                                &mut lhs.data,
                                lhs.origin,
                            )?;

                            <#lhs as #$facade::<#rhs>>::$facade_function(
                                lhs,
                                rhs,
                            );

                            #core::RuntimeResult::<()>::Ok(())
                        }
                    }
                )
                .to_tokens(tokens)
            }
        }
    };
}

binary!(impl ScriptAdd(script_add) for face_add(add));

binary_assign!(impl ScriptAddAssign(script_add_assign) for face_add_assign(add_assign));

binary!(impl ScriptSub(script_sub) for face_sub(sub));

binary_assign!(impl ScriptSubAssign(script_sub_assign) for face_sub_assign(sub_assign));

binary!(impl ScriptMul(script_mul) for face_mul(mul));

binary_assign!(impl ScriptMulAssign(script_mul_assign) for face_mul_assign(mul_assign));

binary!(impl ScriptDiv(script_div) for face_div(div));

binary_assign!(impl ScriptDivAssign(script_div_assign) for face_div_assign(div_assign));

pub struct ScriptNot<'a> {
    pub span: Span,
    pub lhs: &'a Type,
}

impl<'a> DefaultScriptOperator for ScriptNot<'a> {
    #[inline(always)]
    fn new_stream(span: Span, lhs: &Type, _rhs: Option<&Type>) -> TokenStream {
        ScriptNot { span, lhs }.to_token_stream()
    }
}

impl<'a> ToTokens for ScriptNot<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;

        let core = span.face_core();
        let not = span.face_not();

        let lhs = self.lhs;

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #core::runtime::ops::ScriptNot for #lhs {
                type Result = <#lhs as #not>::Output;

                fn script_not(
                    origin: #core::runtime::Origin,
                    lhs: #core::runtime::Arg,
                ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                    let lhs = #core::runtime::Cell::take::<#lhs>(
                        lhs.data,
                        lhs.origin,
                    )?;

                    #core::runtime::Cell::give(origin, <#lhs as #not>::not(lhs))
                }
            }
        )
        .to_tokens(tokens)
    }
}

pub struct ScriptNeg<'a> {
    pub span: Span,
    pub lhs: &'a Type,
}

impl<'a> DefaultScriptOperator for ScriptNeg<'a> {
    #[inline(always)]
    fn new_stream(span: Span, lhs: &Type, _rhs: Option<&Type>) -> TokenStream {
        ScriptNeg { span, lhs }.to_token_stream()
    }
}

impl<'a> ToTokens for ScriptNeg<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.span;

        let core = span.face_core();
        let neg = span.face_neg();

        let lhs = self.lhs;

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #core::runtime::ops::ScriptNeg for #lhs {
                type Result = <#lhs as #neg>::Output;

                fn script_neg(
                    origin: #core::runtime::Origin,
                    lhs: #core::runtime::Arg,
                ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                    let lhs = #core::runtime::Cell::take::<#lhs>(
                        lhs.data,
                        lhs.origin,
                    )?;

                    #core::runtime::Cell::give(origin, <#lhs as #neg>::neg(lhs))
                }
            }
        )
        .to_tokens(tokens)
    }
}

binary!(impl ScriptBitAnd(script_bit_and) for face_bit_and(bitand));

binary_assign!(impl ScriptBitAndAssign(script_bit_and_assign) for face_bit_and_assign(bitand_assign));

binary!(impl ScriptBitOr(script_bit_or) for face_bit_or(bitor));

binary_assign!(impl ScriptBitOrAssign(script_bit_or_assign) for face_bit_or_assign(bitor_assign));

binary!(impl ScriptBitXor(script_bit_xor) for face_bit_xor(bitxor));

binary_assign!(impl ScriptBitXorAssign(script_bit_xor_assign) for face_bit_xor_assign(bitxor_assign));

binary!(impl ScriptShl(script_shl) for face_shl(shl));

binary_assign!(impl ScriptShlAssign(script_shl_assign) for face_shl_assign(shl_assign));

binary!(impl ScriptShr(script_shr) for face_shr(shr));

binary_assign!(impl ScriptShrAssign(script_shr_assign) for face_shr_assign(shr_assign));

binary!(impl ScriptRem(script_rem) for face_rem(rem));

binary_assign!(impl ScriptRemAssign(script_rem_assign) for face_rem_assign(rem_assign));
