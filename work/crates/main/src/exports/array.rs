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

use crate::{
    report::system_panic,
    runtime::{
        Downcast,
        Origin,
        Provider,
        RuntimeError,
        RuntimeResult,
        ScriptType,
        TypeHint,
        Upcast,
    },
};

impl<'a, const N: usize, T> Downcast<'a> for [T; N]
where
    T: ScriptType,
{
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let mut vector = provider.to_owned().take_vec::<T>(origin)?;

        let length = vector.len();

        if length < N {
            return Err(RuntimeError::ShortSlice {
                access_origin: origin,
                minimum: N,
                actual: length,
            });
        }

        if length > N {
            vector.truncate(N);
        }

        match vector.try_into() {
            Ok(array) => Ok(array),

            Err(_) => {
                system_panic!("Downcasting of vector to array failure.")
            }
        }
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(T::type_meta())
    }
}

impl<'a, const N: usize, T> Downcast<'a> for &'a [T; N]
where
    T: ScriptType,
{
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let slice = provider
            .to_borrowed(&origin)?
            .borrow_slice_ref::<T>(origin)?;

        let length = slice.len();

        if length < N {
            return Err(RuntimeError::ShortSlice {
                access_origin: origin,
                minimum: N,
                actual: length,
            });
        }

        match slice.try_into() {
            Ok(array) => Ok(array),

            Err(_) => {
                system_panic!("Downcasting of slice ref to array failure.")
            }
        }
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(T::type_meta())
    }
}

impl<'a, const N: usize, T> Downcast<'a> for &'a mut [T; N]
where
    T: ScriptType,
{
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let slice = provider
            .to_borrowed(&origin)?
            .borrow_slice_mut::<T>(origin)?;

        let length = slice.len();

        if length < N {
            return Err(RuntimeError::ShortSlice {
                access_origin: origin,
                minimum: N,
                actual: length,
            });
        }

        match slice.try_into() {
            Ok(array) => Ok(array),

            Err(_) => {
                system_panic!("Downcasting of slice ref to array failure.")
            }
        }
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(T::type_meta())
    }
}

impl<'a, const N: usize, T> Upcast<'a> for [T; N]
where
    T: ScriptType,
{
    type Output = Vec<T>;

    #[inline]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(Vec::from(this))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(T::type_meta())
    }
}

impl<'a, const N: usize, T> Upcast<'a> for &'a [T; N]
where
    T: ScriptType,
{
    type Output = &'a [T];

    #[inline]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(this)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(T::type_meta())
    }
}

impl<'a, const N: usize, T> Upcast<'a> for &'a mut [T; N]
where
    T: ScriptType,
{
    type Output = &'a mut [T];

    #[inline]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(this)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(T::type_meta())
    }
}
