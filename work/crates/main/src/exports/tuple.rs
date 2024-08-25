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
    exports::Struct,
    runtime::{
        Cell,
        Downcast,
        Ident,
        Origin,
        Provider,
        RuntimeError,
        RuntimeResult,
        ScriptType,
        TypeHint,
        Upcast,
    },
};

macro_rules! impl_tuple {
    ($($arg:ident: $index:tt),+) => {
        impl<'a $(, $arg)+> Downcast<'a> for ($($arg, )+)
        where
            $(
            $arg: Downcast<'static>,
            )+
        {
            fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
                let structure_origin = provider.as_ref().origin();

                let structure = provider
                    .to_borrowed(&origin)?
                    .borrow_ref::<Struct>(origin)?;

                Ok((
                $(
                    match structure.map.get(&Ident::from_string(stringify!($index))) {
                        Some(entry) => <$arg as Downcast>::downcast(
                            origin,
                            Provider::Owned(entry.clone()),
                        )?,
                        None => {
                            return Err(RuntimeError::UnknownField {
                                access_origin: origin,
                                receiver_origin: structure_origin,
                                receiver_type: Struct::type_meta(),
                                field: String::from(stringify!($index)),
                            })
                        }
                    },
                )+
                ))
            }

            #[inline(always)]
            fn hint() ->TypeHint {
                TypeHint::Type(Struct::type_meta())
            }
        }

        impl<'a $(, $arg)+> Upcast<'a> for ($($arg, )+)
        where
            $(
            $arg: Upcast<'static>,
            )+
        {
            type Output = Box<Struct>;

            fn upcast(origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
                Ok(Box::new(Struct {
                    map: [
                        $((
                            Ident::from_string(stringify!($index)),
                            Cell::give(origin, this.$index)?,
                        ),)+
                    ].into()
                }))
            }

            #[inline(always)]
            fn hint() -> TypeHint {
                TypeHint::Type(Struct::type_meta())
            }
        }
    };
}

impl_tuple!(A: 0);
impl_tuple!(A: 0, B: 1);
impl_tuple!(A: 0, B: 1, C: 2);
impl_tuple!(A: 0, B: 1, C: 2, D: 3);
impl_tuple!(A: 0, B: 1, C: 2, D: 3, E: 4);
impl_tuple!(A: 0, B: 1, C: 2, D: 3, E: 4, F: 5);
impl_tuple!(A: 0, B: 1, C: 2, D: 3, E: 4, F: 5, G: 6);
