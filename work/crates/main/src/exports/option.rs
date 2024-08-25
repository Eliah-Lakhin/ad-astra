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

use crate::runtime::{Downcast, Either, Origin, Provider, RuntimeResult, TypeHint, Upcast};

impl<'a, T> Downcast<'a> for Option<T>
where
    T: Downcast<'a>,
{
    #[inline(always)]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        match &provider {
            Provider::Owned(cell) if cell.is_nil() => return Ok(None),
            Provider::Borrowed(cell) if cell.is_nil() => return Ok(None),

            _ => (),
        }

        let inner = <T as Downcast<'a>>::downcast(origin, provider)?;

        Ok(Some(inner))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        <T as Downcast<'a>>::hint()
    }
}

impl<'a, T> Upcast<'a> for Option<T>
where
    T: Upcast<'a>,
{
    type Output = Either<(), <T as Upcast<'a>>::Output>;

    #[inline(always)]
    fn upcast(origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        match this {
            Some(inner) => Ok(Either::Right(<T as Upcast<'a>>::upcast(origin, inner)?)),
            None => Ok(Either::Left(())),
        }
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        <T as Upcast<'a>>::hint()
    }
}

impl<'a, T> Upcast<'a> for &'a Option<T>
where
    &'a T: Upcast<'a>,
{
    type Output = Either<(), <&'a T as Upcast<'a>>::Output>;

    #[inline(always)]
    fn upcast(origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        match this {
            Some(inner) => Ok(Either::Right(<&'a T as Upcast<'a>>::upcast(origin, inner)?)),
            None => Ok(Either::Left(())),
        }
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        <&'a T as Upcast<'a>>::hint()
    }
}

impl<'a, T> Upcast<'a> for &'a mut Option<T>
where
    &'a mut T: Upcast<'a>,
{
    type Output = Either<(), <&'a mut T as Upcast<'a>>::Output>;

    #[inline(always)]
    fn upcast(origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        match this {
            Some(inner) => Ok(Either::Right(<&'a mut T as Upcast<'a>>::upcast(
                origin, inner,
            )?)),
            None => Ok(Either::Left(())),
        }
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        <&'a mut T as Upcast<'a>>::hint()
    }
}
