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

use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::spanned::Spanned;

pub trait Facade: Spanned {
    #[inline(always)]
    fn face_core(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::ad_astra)
    }

    #[inline(always)]
    fn face_intrinsics(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::ad_astra::runtime::__intrinsics)
    }

    #[inline(always)]
    fn face_module_path(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::module_path!)
    }

    #[inline(always)]
    fn face_line(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::line!)
    }

    #[inline(always)]
    fn face_column(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::column!)
    }

    #[inline(always)]
    fn face_panic(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::panic!)
    }

    #[inline(always)]
    fn face_type_id(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::any::TypeId)
    }

    #[inline(always)]
    fn face_type_name(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::any::type_name)
    }

    #[inline(always)]
    fn face_env(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::env!)
    }

    #[inline(always)]
    fn face_result(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::result::Result)
    }

    #[inline(always)]
    fn face_option(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::option::Option)
    }

    #[inline(always)]
    fn face_deref(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::Deref)
    }

    #[inline(always)]
    fn face_format(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::format!)
    }

    #[inline(always)]
    fn face_box(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::boxed::Box)
    }

    #[inline(always)]
    fn face_vec(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::vec::Vec)
    }

    #[inline(always)]
    fn face_vec_macro(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::vec!)
    }

    #[inline(always)]
    fn face_addr_of(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ptr::addr_of!)
    }

    #[inline(always)]
    fn face_addr_of_mut(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ptr::addr_of_mut!)
    }

    #[inline(always)]
    fn face_fn_once(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::FnOnce)
    }

    #[inline(always)]
    fn face_clone(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::clone::Clone)
    }

    #[inline(always)]
    fn face_formatter(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::fmt::Formatter)
    }

    #[inline(always)]
    fn face_debug(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::fmt::Debug)
    }

    #[inline(always)]
    fn face_display(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::fmt::Display)
    }

    #[inline(always)]
    fn face_partial_eq(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::cmp::PartialEq)
    }

    #[inline(always)]
    fn face_default(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::default::Default)
    }

    #[inline(always)]
    fn face_ordering(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::cmp::Ordering)
    }

    #[inline(always)]
    fn face_partial_ord(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::cmp::PartialOrd)
    }

    #[inline(always)]
    fn face_ord(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::cmp::Ord)
    }

    #[inline(always)]
    fn face_hash(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::hash::Hash)
    }

    #[inline(always)]
    fn face_add(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::Add)
    }

    #[inline(always)]
    fn face_add_assign(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::AddAssign)
    }

    #[inline(always)]
    fn face_sub(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::Sub)
    }

    #[inline(always)]
    fn face_sub_assign(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::SubAssign)
    }

    #[inline(always)]
    fn face_mul(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::Mul)
    }

    #[inline(always)]
    fn face_mul_assign(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::MulAssign)
    }

    #[inline(always)]
    fn face_div(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::Div)
    }

    #[inline(always)]
    fn face_div_assign(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::DivAssign)
    }

    #[inline(always)]
    fn face_not(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::Not)
    }

    #[inline(always)]
    fn face_neg(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::Neg)
    }

    #[inline(always)]
    fn face_bit_and(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::BitAnd)
    }

    #[inline(always)]
    fn face_bit_and_assign(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::BitAndAssign)
    }

    #[inline(always)]
    fn face_bit_or(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::BitOr)
    }

    #[inline(always)]
    fn face_bit_or_assign(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::BitOrAssign)
    }

    #[inline(always)]
    fn face_bit_xor(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::BitXor)
    }

    #[inline(always)]
    fn face_bit_xor_assign(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::BitXorAssign)
    }

    #[inline(always)]
    fn face_shl(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::Shl)
    }

    #[inline(always)]
    fn face_shl_assign(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::ShlAssign)
    }

    #[inline(always)]
    fn face_shr(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::Shr)
    }

    #[inline(always)]
    fn face_shr_assign(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::ShrAssign)
    }

    #[inline(always)]
    fn face_rem(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::Rem)
    }

    #[inline(always)]
    fn face_rem_assign(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::ops::RemAssign)
    }
}

impl<T: Spanned> Facade for T {}
