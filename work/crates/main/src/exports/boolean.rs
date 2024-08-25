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

use std::cmp::Ordering;

use crate::{
    export,
    exports::utils::transparent_upcast,
    runtime::{
        ops::{
            ScriptAnd,
            ScriptAssign,
            ScriptClone,
            ScriptConcat,
            ScriptDebug,
            ScriptDefault,
            ScriptDisplay,
            ScriptHash,
            ScriptOr,
            ScriptOrd,
            ScriptPartialEq,
            ScriptPartialOrd,
        },
        Arg,
        Cell,
        Downcast,
        Origin,
        Provider,
        RuntimeResult,
        ScriptType,
        TypeHint,
        __intrinsics::canonicals::{script_assign, script_concat},
    },
};

/// A logical type. Possible values are `true` or `false`.
#[export(include)]
type BoolType = bool;

impl<'a> Downcast<'a> for BoolType {
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let mut type_match = provider.type_match();

        if type_match.is::<BoolType>() {
            return provider.to_owned().take::<BoolType>(origin);
        }

        return Err(type_match.mismatch(origin));
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(BoolType::type_meta())
    }
}

impl<'a> Downcast<'a> for &'a BoolType {
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let mut type_match = provider.type_match();

        if type_match.is::<BoolType>() {
            return provider.to_borrowed(&origin)?.borrow_ref(origin);
        }

        return Err(type_match.mismatch(origin));
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(BoolType::type_meta())
    }
}

impl<'a> Downcast<'a> for &'a mut BoolType {
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let mut type_match = provider.type_match();

        if type_match.is::<BoolType>() {
            return provider.to_borrowed(&origin)?.borrow_mut(origin);
        }

        return Err(type_match.mismatch(origin));
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(BoolType::type_meta())
    }
}

transparent_upcast!(BoolType);

#[export(include)]
impl ScriptAssign for BoolType {
    type RHS = Self;

    fn script_assign(_origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()> {
        script_assign::<Self>(lhs, rhs)
    }
}

#[export(include)]
impl ScriptConcat for BoolType {
    type Result = Self;

    fn script_concat(origin: Origin, items: &mut [Arg]) -> RuntimeResult<Cell> {
        script_concat::<Self>(origin, items)
    }
}

#[export(include)]
impl ScriptClone for BoolType {}

#[export(include)]
impl ScriptDebug for BoolType {}

#[export(include)]
impl ScriptDisplay for BoolType {}

#[export(include)]
impl ScriptPartialEq for BoolType {
    type RHS = BoolType;

    fn script_eq(_origin: Origin, mut lhs: Arg, mut rhs: Arg) -> RuntimeResult<bool> {
        let lhs = lhs.data.borrow_ref::<BoolType>(lhs.origin)?;
        let rhs = rhs.data.borrow_ref::<BoolType>(rhs.origin)?;

        Ok(lhs == rhs)
    }
}

#[export(include)]
impl ScriptDefault for BoolType {
    fn script_default(origin: Origin) -> RuntimeResult<Cell> {
        Cell::give(origin, BoolType::default())
    }
}

#[export(include)]
impl ScriptPartialOrd for BoolType {
    type RHS = BoolType;

    fn script_partial_cmp(
        _origin: Origin,
        mut lhs: Arg,
        mut rhs: Arg,
    ) -> RuntimeResult<Option<Ordering>> {
        let lhs = lhs.data.borrow_ref::<BoolType>(lhs.origin)?;
        let rhs = rhs.data.borrow_ref::<BoolType>(rhs.origin)?;

        Ok(lhs.partial_cmp(rhs))
    }
}

#[export(include)]
impl ScriptOrd for BoolType {
    fn script_cmp(_origin: Origin, mut lhs: Arg, mut rhs: Arg) -> RuntimeResult<Ordering> {
        let lhs = lhs.data.borrow_ref::<BoolType>(lhs.origin)?;
        let rhs = rhs.data.borrow_ref::<BoolType>(rhs.origin)?;

        Ok(lhs.cmp(rhs))
    }
}

#[export(include)]
impl ScriptHash for BoolType {}

#[export(include)]
impl ScriptAnd for BoolType {
    type RHS = BoolType;
    type Result = BoolType;

    fn script_and(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell> {
        let lhs = lhs.data.take::<BoolType>(lhs.origin)?;
        let rhs = rhs.data.take::<BoolType>(rhs.origin)?;

        Cell::give(origin, lhs && rhs)
    }
}

#[export(include)]
impl ScriptOr for BoolType {
    type RHS = BoolType;
    type Result = BoolType;

    fn script_or(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell> {
        let lhs = lhs.data.take::<BoolType>(lhs.origin)?;
        let rhs = rhs.data.take::<BoolType>(rhs.origin)?;

        Cell::give(origin, lhs || rhs)
    }
}
