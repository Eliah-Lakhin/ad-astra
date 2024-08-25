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

use crate::runtime::{Downcast, Either, Origin, Provider, RuntimeResult, TypeHint, Upcast};

impl<'a, T> Downcast<'a> for Cow<'a, T>
where
    &'a T: Downcast<'a>,
    T: ToOwned,
    <T as ToOwned>::Owned: Downcast<'a>,
{
    #[inline(always)]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        match provider {
            Provider::Borrowed(cell) => Downcast::<'a>::downcast(origin, Provider::Borrowed(cell)),
            Provider::Owned(cell) => Downcast::<'a>::downcast(origin, Provider::Owned(cell)),
        }
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        <<T as ToOwned>::Owned as Downcast<'a>>::hint()
    }
}

impl<'a, T> Upcast<'a> for Cow<'a, T>
where
    &'a T: Upcast<'a>,
    T: ToOwned,
    <T as ToOwned>::Owned: Upcast<'static>,
{
    type Output =
        Either<<&'a T as Upcast<'a>>::Output, <<T as ToOwned>::Owned as Upcast<'static>>::Output>;

    #[inline(always)]
    fn upcast(origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        match this {
            Cow::Borrowed(borrowed) => Ok(Either::Left(Upcast::<'a>::upcast(origin, borrowed)?)),
            Cow::Owned(owned) => Ok(Either::Right(Upcast::<'static>::upcast(origin, owned)?)),
        }
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        <<T as ToOwned>::Owned as Upcast<'static>>::hint()
    }
}
