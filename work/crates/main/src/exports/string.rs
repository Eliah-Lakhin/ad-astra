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

use std::{cell::RefCell, mem::take};

use crate::{
    export,
    exports::utils::Stringifier,
    runtime::{
        ops::{ScriptConcat, ScriptDisplay, ScriptPartialEq},
        Arg,
        Cell,
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

/// An immutable string type: `"Quick brown fox."`
#[export(include)]
pub(crate) type StringType = str;

impl<'a> Downcast<'a> for &'a str {
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let mut type_match = provider.type_match();

        if type_match.is::<str>() {
            return provider.to_borrowed(&origin)?.borrow_str(origin);
        }

        return Err(type_match.mismatch(origin));
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(StringType::type_meta())
    }
}

impl<'a> Downcast<'a> for Box<str> {
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let mut type_match = provider.type_match();

        if type_match.is::<str>() {
            return Ok(provider.to_owned().take_string(origin)?.into_boxed_str());
        }

        return Err(type_match.mismatch(origin));
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(StringType::type_meta())
    }
}

impl<'a> Upcast<'a> for &'a str {
    type Output = &'a str;

    #[inline(always)]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(this)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(StringType::type_meta())
    }
}

impl<'a> Upcast<'a> for Box<str> {
    type Output = String;

    #[inline(always)]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(this.into_string())
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(StringType::type_meta())
    }
}

#[export(include)]
impl ScriptDisplay for str {}

#[export(include)]
impl ScriptConcat for str {
    type Result = str;

    fn script_concat(origin: Origin, items: &mut [Arg]) -> RuntimeResult<Cell> {
        let mut result = String::new();

        for item in items {
            if item.data.is_nil() {
                continue;
            }

            let cell = take(&mut item.data);

            let stringifier = Stringifier {
                origin: item.origin,
                cell: &cell,
                error: RefCell::new(None),
                fallback_to_type: false,
            };

            let string = stringifier.to_string();

            if let Some(error) = stringifier.error.take() {
                return Err(error);
            }

            result.push_str(&string);
        }

        Cell::give(origin, result)
    }
}

#[export(include)]
impl ScriptPartialEq for str {
    type RHS = str;

    fn script_eq(_origin: Origin, mut lhs: Arg, mut rhs: Arg) -> RuntimeResult<bool> {
        let lhs = lhs.data.borrow_str(lhs.origin)?;
        let rhs = rhs.data.borrow_str(rhs.origin)?;

        Ok(lhs == rhs)
    }
}

impl<'a> Downcast<'a> for char {
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let mut type_match = provider.type_match();

        if type_match.is::<str>() {
            let string = provider.to_owned().take_string(origin)?;
            let mut chars = string.chars();

            let first = match chars.next() {
                Some(ch) => ch,
                None => {
                    return Err(RuntimeError::OutOfBounds {
                        access_origin: origin,
                        index: 0,
                        length: 0,
                    })
                }
            };

            if chars.next().is_some() {
                return Err(RuntimeError::NonSingleton {
                    access_origin: origin,
                    actual: string.len(),
                });
            }

            return Ok(first);
        }

        return Err(type_match.mismatch(origin));
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(StringType::type_meta())
    }
}

impl<'a> Upcast<'a> for char {
    type Output = String;

    #[inline]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(String::from(this))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(StringType::type_meta())
    }
}

impl<'a> Downcast<'a> for String {
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let mut type_match = provider.type_match();

        if type_match.is::<str>() {
            return provider.to_owned().take_string(origin);
        }

        if type_match.cell().ty().prototype().implements_display() {
            let stringifier = Stringifier {
                origin,
                cell: &provider.to_owned(),
                error: RefCell::new(None),
                fallback_to_type: false,
            };

            let string = stringifier.to_string();

            if let Some(error) = stringifier.error.take() {
                return Err(error);
            }

            return Ok(string);
        }

        return Err(type_match.mismatch(origin));
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(StringType::type_meta())
    }
}

impl<'a> Upcast<'a> for String {
    type Output = Self;

    #[inline(always)]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(this)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Family(StringType::type_meta().family())
    }
}

impl<'a> Upcast<'a> for &'a String {
    type Output = &'a str;

    #[inline(always)]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(this.as_str())
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(StringType::type_meta())
    }
}

impl<'a> Upcast<'a> for &'a mut String {
    type Output = &'a str;

    #[inline(always)]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(this.as_str())
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(StringType::type_meta())
    }
}
