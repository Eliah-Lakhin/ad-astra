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

use crate::runtime::{Downcast, Origin, Provider, RuntimeResult, ScriptType, TypeHint, Upcast};

impl<'a, T> Downcast<'a> for &'a [T]
where
    T: ScriptType,
{
    #[inline]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        provider.to_borrowed(&origin)?.borrow_slice_ref::<T>(origin)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(T::type_meta())
    }
}

impl<'a, T> Downcast<'a> for &'a mut [T]
where
    T: ScriptType,
{
    #[inline]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        provider.to_borrowed(&origin)?.borrow_slice_mut::<T>(origin)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(T::type_meta())
    }
}

impl<'a, T> Upcast<'a> for &'a [T]
where
    T: ScriptType,
{
    type Output = Self;

    #[inline]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(this)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(T::type_meta())
    }
}

impl<'a, T> Upcast<'a> for &'a mut [T]
where
    T: ScriptType,
{
    type Output = Self;

    #[inline]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(this)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(T::type_meta())
    }
}
