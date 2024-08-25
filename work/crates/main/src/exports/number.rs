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

use std::{
    any::TypeId,
    cmp::Ordering,
    fmt::{Debug, Display},
    mem::{take, transmute, transmute_copy},
    ops::*,
    result::Result as StdResult,
    str::FromStr,
    sync::Arc,
};

use crate::{
    export,
    exports::utils::transparent_upcast,
    report::system_panic,
    runtime::{
        ops::{
            ScriptAdd,
            ScriptAssign,
            ScriptBitAnd,
            ScriptBitOr,
            ScriptBitXor,
            ScriptClone,
            ScriptConcat,
            ScriptDebug,
            ScriptDefault,
            ScriptDisplay,
            ScriptDiv,
            ScriptHash,
            ScriptMul,
            ScriptNeg,
            ScriptOrd,
            ScriptPartialEq,
            ScriptPartialOrd,
            ScriptRem,
            ScriptShl,
            ScriptShr,
            ScriptSub,
        },
        Arg,
        Cell,
        Downcast,
        NumberCastCause,
        NumericOperationKind,
        Origin,
        Provider,
        RuntimeError,
        RuntimeResult,
        ScriptType,
        TypeHint,
    },
    type_family,
};

type_family!(
    /// A numeric type: an integer or a real number, with or without a sign.
    pub(crate) static NUMBER_FAMILY = "number";
);

macro_rules! impl_num {
    (type $alias:ident($name:expr) = $ty:ty $( as $bool:ident)?) => {
        #[export(include)]
        #[export(family(&NUMBER_FAMILY))]
        #[export(name $name)]
        pub(crate) type $alias = $ty;

        impl<'a> Downcast<'a> for $ty {
            fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
                let mut type_match = provider.type_match();

                if type_match.is::<f32>() {
                    let from = provider.to_owned().take::<f32>(origin)?;

                    return from.cast_to(&origin);
                }

                if type_match.is::<f64>() {
                    let from = provider.to_owned().take::<f64>(origin)?;

                    return from.cast_to(&origin);
                }

                if type_match.is::<i8>() {
                    let from = provider.to_owned().take::<i8>(origin)?;

                    return from.cast_to(&origin);
                }

                if type_match.is::<i16>() {
                    let from = provider.to_owned().take::<i16>(origin)?;

                    return from.cast_to(&origin);
                }

                if type_match.is::<i32>() {
                    let from = provider.to_owned().take::<i32>(origin)?;

                    return from.cast_to(&origin);
                }

                if type_match.is::<i64>() {
                    let from = provider.to_owned().take::<i64>(origin)?;

                    return from.cast_to(&origin);
                }

                if type_match.is::<i128>() {
                    let from = provider.to_owned().take::<i128>(origin)?;

                    return from.cast_to(&origin);
                }

                if type_match.is::<isize>() {
                    let from = provider.to_owned().take::<isize>(origin)?;

                    return from.cast_to(&origin);
                }

                if type_match.is::<str>() {
                    let mut cell = provider.to_owned();
                    let string = cell.borrow_str(origin)?;

                    return match <$ty>::from_str(string) {
                        Ok(number) => Ok(number),

                        Err(error) => Err(RuntimeError::PrimitiveParse {
                            access_origin: origin,
                            from: string.to_string(),
                            to: <$ty>::type_meta(),
                            cause: Arc::new(error)
                        })
                    };
                }

                if type_match.is::<u8>() {
                    let from = provider.to_owned().take::<u8>(origin)?;

                    return from.cast_to(&origin);
                }

                if type_match.is::<u16>() {
                    let from = provider.to_owned().take::<u16>(origin)?;

                    return from.cast_to(&origin);
                }

                if type_match.is::<u32>() {
                    let from = provider.to_owned().take::<u32>(origin)?;

                    return from.cast_to(&origin);
                }

                if type_match.is::<u64>() {
                    let from = provider.to_owned().take::<u64>(origin)?;

                    return from.cast_to(&origin);
                }

                if type_match.is::<u128>() {
                    let from = provider.to_owned().take::<u128>(origin)?;

                    return from.cast_to(&origin);
                }

                if type_match.is::<usize>() {
                    let from = provider.to_owned().take::<usize>(origin)?;

                    return from.cast_to(&origin);
                }

                $(if type_match.is::<$bool>() {
                    let from = provider.to_owned().take::<bool>(origin)?;

                    return Ok(from as Self);
                })?

                return Err(type_match.mismatch(origin));
            }

            #[inline(always)]
            fn hint() -> TypeHint {
                TypeHint::Type(<$ty>::type_meta())
            }
        }

        impl<'a> Downcast<'a> for &'a $ty {
            fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
                let mut type_match = provider.type_match();

                if type_match.is::<$ty>() {
                    return provider.to_borrowed(&origin)?.borrow_ref(origin);
                }

                return Err(type_match.mismatch(origin));
            }

            #[inline(always)]
            fn hint() -> TypeHint {
                TypeHint::Type(<$ty>::type_meta())
            }
        }

        impl<'a> Downcast<'a> for &'a mut $ty {
            fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
                let mut type_match = provider.type_match();

                if type_match.is::<$ty>() {
                    return provider.to_borrowed(&origin)?.borrow_mut(origin);
                }

                return Err(type_match.mismatch(origin));
            }

            #[inline(always)]
            fn hint() -> TypeHint {
                TypeHint::Type(<$ty>::type_meta())
            }
        }

        transparent_upcast!($ty);

        #[export(include)]
        impl ScriptAssign for $ty {
            type RHS = $ty;

            fn script_assign(
                _origin: Origin,
                mut lhs: Arg,
                mut rhs: Arg,
            ) -> RuntimeResult<()> {
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;
                let lhs = lhs.data.borrow_mut::<$ty>(lhs.origin)?;

                *lhs = rhs;

                Ok(())
            }
        }

        #[export(include)]
        impl ScriptConcat for $ty {
            type Result = $ty;

            fn script_concat(
                origin: Origin,
                items: &mut [Arg],
            ) -> RuntimeResult<Cell> {
                canonical_num_concat(origin, items)
            }
        }

        #[export(include)]
        impl ScriptClone for $ty {}

        #[export(include)]
        impl ScriptDebug for $ty {}

        #[export(include)]
        impl ScriptDisplay for $ty {}

        #[export(include)]
        impl ScriptPartialEq for $ty {
            type RHS = $ty;

            fn script_eq(
                _origin: Origin,
                lhs: Arg,
                mut rhs: Arg,
            ) -> RuntimeResult<bool> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;

                Ok(lhs == rhs)
            }
        }

        #[export(include)]
        impl ScriptDefault for $ty {
            fn script_default(origin: Origin) -> RuntimeResult<Cell> {
                Cell::give(origin, <$ty>::default())
            }
        }

        impl NumConcat for $ty {
            fn num_concat(origin: Origin, items: &mut [Arg]) -> RuntimeResult<Cell>
            {
                let mut result = Vec::<Self>::new();

                for item in items {
                    if item.data.is_nil() {
                        continue;
                    }

                    let mut type_match = item.data.type_match();

                    if type_match.is::<f32>() {
                        let item_slice = take(&mut item.data).take_vec::<f32>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<f32>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<f32>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    if type_match.is::<f64>() {
                        let item_slice = take(&mut item.data).take_vec::<f64>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<f64>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<f64>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    if type_match.is::<i128>() {
                        let item_slice = take(&mut item.data).take_vec::<i128>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<i128>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<i128>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    if type_match.is::<i16>() {
                        let item_slice = take(&mut item.data).take_vec::<i16>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<i16>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<i16>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    if type_match.is::<i32>() {
                        let item_slice = take(&mut item.data).take_vec::<i32>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<i32>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<i32>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    if type_match.is::<i64>() {
                        let item_slice = take(&mut item.data).take_vec::<i64>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<i64>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<i64>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    if type_match.is::<i8>() {
                        let item_slice = take(&mut item.data).take_vec::<i8>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<i8>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<i8>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    if type_match.is::<isize>() {
                        let item_slice = take(&mut item.data).take_vec::<isize>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<isize>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<isize>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    if type_match.is::<u128>() {
                        let item_slice = take(&mut item.data).take_vec::<u128>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<u128>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<u128>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    if type_match.is::<u16>() {
                        let item_slice = take(&mut item.data).take_vec::<u16>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<u16>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<u16>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    if type_match.is::<u32>() {
                        let item_slice = take(&mut item.data).take_vec::<u32>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<u32>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<u32>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    if type_match.is::<u64>() {
                        let item_slice = take(&mut item.data).take_vec::<u64>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<u64>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<u64>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    if type_match.is::<str>() {
                        let string = item.data.borrow_str(item.origin)?;

                        match Self::from_str(string) {
                            Ok(number) => result.push(number),

                            Err(error) => {
                                return Err(RuntimeError::PrimitiveParse {
                                    access_origin: item.origin,
                                    from: string.to_string(),
                                    to: Self::type_meta(),
                                    cause: Arc::new(error),
                                });
                            }
                        };

                        continue;
                    }

                    if type_match.is::<u8>() {
                        let item_slice = take(&mut item.data).take_vec::<u8>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<u8>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<u8>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    if type_match.is::<usize>() {
                        let item_slice = take(&mut item.data).take_vec::<usize>(item.origin)?;

                        if TypeId::of::<Self>() == TypeId::of::<usize>() {
                            // Safety: Types checked above.
                            let mut item_slice = unsafe { transmute::<Vec<usize>, Vec<Self>>(item_slice) };

                            result.append(&mut item_slice);
                            continue;
                        }

                        CastTo::append_to(item_slice, &item.origin, &mut result)?;
                        continue;
                    }

                    $(if type_match.is::<$bool>() {
                        let item_slice = take(&mut item.data).take_vec::<bool>(item.origin)?;

                        for value in item_slice {
                            result.push(value as Self);
                        }

                        continue;
                    })?

                    return Err(type_match.mismatch(item.origin));
                }

                if result.is_empty() {
                    return Ok(Cell::nil());
                };

                Cell::give_vec(origin, result)
            }
        }
    };
}

macro_rules! impl_int {
    ($ty:ty) => {
        #[export(include)]
        impl ScriptPartialOrd for $ty {
            type RHS = $ty;

            fn script_partial_cmp(
                _origin: Origin,
                mut lhs: Arg,
                mut rhs: Arg,
            ) -> RuntimeResult<Option<Ordering>> {
                let lhs = lhs.data.borrow_ref::<$ty>(lhs.origin)?;
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;

                Ok(lhs.partial_cmp(&rhs))
            }
        }

        #[export(include)]
        impl ScriptOrd for $ty {
            fn script_cmp(_origin: Origin, mut lhs: Arg, mut rhs: Arg) -> RuntimeResult<Ordering> {
                let lhs = lhs.data.borrow_ref::<$ty>(lhs.origin)?;
                let rhs = rhs.data.borrow_ref::<$ty>(rhs.origin)?;

                Ok(lhs.cmp(&rhs))
            }
        }

        #[export(include)]
        impl ScriptHash for $ty {}

        #[export(include)]
        impl ScriptAdd for $ty {
            type RHS = Self;
            type Result = Self;

            fn script_add(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs_type = rhs.data.ty();
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;

                match lhs.checked_add(rhs) {
                    Some(result) => Cell::give(origin, result),

                    None => Err(RuntimeError::NumericOperation {
                        invoke_origin: origin,
                        kind: NumericOperationKind::Add,
                        lhs: (<$ty>::type_meta(), Arc::new(lhs)),
                        rhs: Some((rhs_type, Arc::new(rhs))),
                        target: <$ty>::type_meta(),
                    }),
                }
            }
        }

        #[export(include)]
        impl ScriptSub for $ty {
            type RHS = Self;
            type Result = Self;

            fn script_sub(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs_type = rhs.data.ty();
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;

                match lhs.checked_sub(rhs) {
                    Some(result) => Cell::give(origin, result),

                    None => Err(RuntimeError::NumericOperation {
                        invoke_origin: origin,
                        kind: NumericOperationKind::Sub,
                        lhs: (<$ty>::type_meta(), Arc::new(lhs)),
                        rhs: Some((rhs_type, Arc::new(rhs))),
                        target: <$ty>::type_meta(),
                    }),
                }
            }
        }

        #[export(include)]
        impl ScriptMul for $ty {
            type RHS = Self;
            type Result = Self;

            fn script_mul(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs_type = rhs.data.ty();
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;

                match lhs.checked_mul(rhs) {
                    Some(result) => Cell::give(origin, result),

                    None => Err(RuntimeError::NumericOperation {
                        invoke_origin: origin,
                        kind: NumericOperationKind::Mul,
                        lhs: (<$ty>::type_meta(), Arc::new(lhs)),
                        rhs: Some((rhs_type, Arc::new(rhs))),
                        target: <$ty>::type_meta(),
                    }),
                }
            }
        }

        #[export(include)]
        impl ScriptDiv for $ty {
            type RHS = Self;
            type Result = Self;

            fn script_div(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs_type = rhs.data.ty();
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;

                match lhs.checked_div(rhs) {
                    Some(result) => Cell::give(origin, result),

                    None => Err(RuntimeError::NumericOperation {
                        invoke_origin: origin,
                        kind: NumericOperationKind::Div,
                        lhs: (<$ty>::type_meta(), Arc::new(lhs)),
                        rhs: Some((rhs_type, Arc::new(rhs))),
                        target: <$ty>::type_meta(),
                    }),
                }
            }
        }

        #[export(include)]
        impl ScriptNeg for $ty {
            type Result = Self;

            fn script_neg(origin: Origin, lhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;

                match lhs.checked_neg() {
                    Some(result) => Cell::give(origin, result),

                    None => Err(RuntimeError::NumericOperation {
                        invoke_origin: origin,
                        kind: NumericOperationKind::Neg,
                        lhs: (<$ty>::type_meta(), Arc::new(lhs)),
                        rhs: None,
                        target: <$ty>::type_meta(),
                    }),
                }
            }
        }

        #[export(include)]
        impl ScriptBitAnd for $ty {
            type RHS = Self;
            type Result = Self;

            fn script_bit_and(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;
                let result = lhs.bitand(rhs);

                Cell::give(origin, result)
            }
        }

        #[export(include)]
        impl ScriptBitOr for $ty {
            type RHS = Self;
            type Result = Self;

            fn script_bit_or(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;
                let result = lhs.bitor(rhs);

                Cell::give(origin, result)
            }
        }

        #[export(include)]
        impl ScriptBitXor for $ty {
            type RHS = Self;
            type Result = Self;

            fn script_bit_xor(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;
                let result = lhs.bitxor(rhs);

                Cell::give(origin, result)
            }
        }

        #[export(include)]
        impl ScriptShl for $ty {
            type RHS = u32;
            type Result = Self;

            fn script_shl(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs_type = rhs.data.ty();
                let rhs = <u32>::downcast(rhs.origin, rhs.provider())?;

                match lhs.checked_shl(rhs) {
                    Some(result) => Cell::give(origin, result),

                    None => Err(RuntimeError::NumericOperation {
                        invoke_origin: origin,
                        kind: NumericOperationKind::Shl,
                        lhs: (<$ty>::type_meta(), Arc::new(lhs)),
                        rhs: Some((rhs_type, Arc::new(rhs))),
                        target: <$ty>::type_meta(),
                    }),
                }
            }
        }

        #[export(include)]
        impl ScriptShr for $ty {
            type RHS = u32;
            type Result = Self;

            fn script_shr(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs_type = rhs.data.ty();
                let rhs = <u32>::downcast(rhs.origin, rhs.provider())?;

                match lhs.checked_shr(rhs) {
                    Some(result) => Cell::give(origin, result),

                    None => Err(RuntimeError::NumericOperation {
                        invoke_origin: origin,
                        kind: NumericOperationKind::Shr,
                        lhs: (<$ty>::type_meta(), Arc::new(lhs)),
                        rhs: Some((rhs_type, Arc::new(rhs))),
                        target: <$ty>::type_meta(),
                    }),
                }
            }
        }

        #[export(include)]
        impl ScriptRem for $ty {
            type RHS = Self;
            type Result = Self;

            fn script_rem(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs_type = rhs.data.ty();
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;

                match lhs.checked_rem(rhs) {
                    Some(result) => Cell::give(origin, result),

                    None => Err(RuntimeError::NumericOperation {
                        invoke_origin: origin,
                        kind: NumericOperationKind::Rem,
                        lhs: (<$ty>::type_meta(), Arc::new(lhs)),
                        rhs: Some((rhs_type, Arc::new(rhs))),
                        target: <$ty>::type_meta(),
                    }),
                }
            }
        }
    };
}

macro_rules! impl_float {
    ($ty:ty) => {
        #[export(include)]
        impl ScriptPartialOrd for $ty {
            type RHS = $ty;

            fn script_partial_cmp(
                _origin: Origin,
                mut lhs: Arg,
                mut rhs: Arg,
            ) -> RuntimeResult<Option<Ordering>> {
                let lhs = lhs.data.borrow_ref::<$ty>(lhs.origin)?;
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;

                Ok(lhs.partial_cmp(&rhs))
            }
        }

        #[export(include)]
        impl ScriptAdd for $ty {
            type RHS = Self;
            type Result = Self;

            fn script_add(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;

                Cell::give(origin, lhs + rhs)
            }
        }

        #[export(include)]
        impl ScriptSub for $ty {
            type RHS = Self;
            type Result = Self;

            fn script_sub(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;

                Cell::give(origin, lhs - rhs)
            }
        }

        #[export(include)]
        impl ScriptMul for $ty {
            type RHS = Self;
            type Result = Self;

            fn script_mul(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;

                Cell::give(origin, lhs * rhs)
            }
        }

        #[export(include)]
        impl ScriptDiv for $ty {
            type RHS = Self;
            type Result = Self;

            fn script_div(origin: Origin, lhs: Arg, mut rhs: Arg) -> RuntimeResult<Cell> {
                let lhs = lhs.data.take::<$ty>(lhs.origin)?;
                let rhs = <$ty>::downcast(rhs.origin, rhs.provider())?;

                Cell::give(origin, lhs / rhs)
            }
        }
    };
}

impl_num!(type F32("f32") = f32);
impl_float!(f32);

impl_num!(type F64("f64") = f64);
impl_float!(f64);

impl_num!(type I128("i128") = i128 as bool);
impl_int!(i128);

impl_num!(type I16("i16") = i16 as bool);
impl_int!(i16);

impl_num!(type I32("i32") = i32 as bool);
impl_int!(i32);

impl_num!(type I64("i64") = i64 as bool);
impl_int!(i64);

impl_num!(type I8("i8") = i8 as bool);
impl_int!(i8);

impl_num!(type ISIZE("isize") = isize as bool);
impl_int!(isize);

impl_num!(type U128("u128") = u128 as bool);
impl_int!(u128);

impl_num!(type U16("u16") = u16 as bool);
impl_int!(u16);

impl_num!(type U32("u32") = u32 as bool);
impl_int!(u32);

impl_num!(type U64("u64") = u64 as bool);
impl_int!(u64);

impl_num!(type U8("u8") = u8 as bool);
impl_int!(u8);

impl_num!(type USIZE("usize") = usize as bool);
impl_int!(usize);

#[inline(always)]
fn canonical_num_concat(origin: Origin, items: &mut [Arg]) -> RuntimeResult<Cell> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum ExactDepth {
        Depth8,
        Depth16,
        Depth32,
        Depth64,
    }

    impl PartialOrd for ExactDepth {
        #[inline(always)]
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for ExactDepth {
        #[inline(always)]
        fn cmp(&self, other: &Self) -> Ordering {
            self.depth().cmp(&other.depth())
        }
    }

    impl ExactDepth {
        #[inline(always)]
        fn platform() -> Self {
            #[cfg(target_pointer_width = "64")]
            {
                return Self::Depth64;
            }

            #[cfg(target_pointer_width = "32")]
            {
                return Self::Depth32;
            }

            #[cfg(target_pointer_width = "16")]
            {
                return Self::Depth16;
            }

            #[allow(unreachable_code)]
            {
                return Self::Depth64;
            }
        }

        #[inline(always)]
        fn depth(&self) -> u8 {
            match self {
                ExactDepth::Depth8 => 8,
                ExactDepth::Depth16 => 16,
                ExactDepth::Depth32 => 32,
                ExactDepth::Depth64 => 64,
            }
        }
    }

    #[derive(Clone, Copy)]
    enum BitDepth {
        Unknown,
        Platform,
        Exact(ExactDepth),
    }

    impl BitDepth {
        #[inline(always)]
        fn impose_platform(&mut self) {
            match self {
                Self::Unknown => *self = Self::Platform,
                Self::Platform => (),
                Self::Exact(current) => *current = (*current).max(ExactDepth::platform()),
            }
        }

        #[inline(always)]
        fn impose_exact(&mut self, depth: ExactDepth) {
            match self {
                Self::Unknown => *self = Self::Exact(depth),
                Self::Platform => *self = Self::Exact(ExactDepth::platform().max(depth)),
                Self::Exact(current) => *current = (*current).max(depth),
            }
        }
    }

    struct Features {
        signed: bool,
        float: bool,
        bit_depth: BitDepth,
    }

    impl Features {
        #[inline(always)]
        fn new() -> Self {
            Self {
                signed: false,
                float: false,
                bit_depth: BitDepth::Unknown,
            }
        }

        #[inline(always)]
        fn impose_f32(&mut self) {
            self.signed = true;
            self.float = true;
            self.bit_depth.impose_exact(ExactDepth::Depth32);
        }

        #[inline(always)]
        fn impose_f64(&mut self) {
            self.signed = true;
            self.float = true;
            self.bit_depth.impose_exact(ExactDepth::Depth64);
        }

        #[inline(always)]
        fn impose_i128(&mut self) {
            self.signed = true;
            self.bit_depth.impose_exact(ExactDepth::Depth64);
        }

        #[inline(always)]
        fn impose_i16(&mut self) {
            self.signed = true;
            self.bit_depth.impose_exact(ExactDepth::Depth16);
        }

        #[inline(always)]
        fn impose_i32(&mut self) {
            self.signed = true;
            self.bit_depth.impose_exact(ExactDepth::Depth32);
        }

        #[inline(always)]
        fn impose_i64(&mut self) {
            self.signed = true;
            self.bit_depth.impose_exact(ExactDepth::Depth64);
        }

        #[inline(always)]
        fn impose_i8(&mut self) {
            self.signed = true;
            self.bit_depth.impose_exact(ExactDepth::Depth8);
        }

        #[inline(always)]
        fn impose_isize(&mut self) {
            self.signed = true;
            self.bit_depth.impose_platform();
        }

        #[inline(always)]
        fn impose_u128(&mut self) {
            self.bit_depth.impose_exact(ExactDepth::Depth64);
        }

        #[inline(always)]
        fn impose_u16(&mut self) {
            self.bit_depth.impose_exact(ExactDepth::Depth16);
        }

        #[inline(always)]
        fn impose_u32(&mut self) {
            self.bit_depth.impose_exact(ExactDepth::Depth32);
        }

        #[inline(always)]
        fn impose_u64(&mut self) {
            self.bit_depth.impose_exact(ExactDepth::Depth64);
        }

        #[inline(always)]
        fn impose_u8(&mut self) {
            self.bit_depth.impose_exact(ExactDepth::Depth8);
        }

        #[inline(always)]
        fn impose_usize(&mut self) {
            self.bit_depth.impose_platform();
        }
    }

    let mut features = Features::new();

    for item in items.iter() {
        let id = *item.data.ty().id();

        if id == TypeId::of::<f32>() {
            features.impose_f32();
            continue;
        }

        if id == TypeId::of::<f64>() {
            features.impose_f64();
            continue;
        }

        if id == TypeId::of::<i128>() {
            features.impose_i128();
            continue;
        }

        if id == TypeId::of::<i16>() {
            features.impose_i16();
            continue;
        }

        if id == TypeId::of::<i32>() {
            features.impose_i32();
            continue;
        }

        if id == TypeId::of::<i64>() {
            features.impose_i64();
            continue;
        }

        if id == TypeId::of::<i8>() {
            features.impose_i8();
            continue;
        }

        if id == TypeId::of::<isize>() {
            features.impose_isize();
            continue;
        }

        if id == TypeId::of::<u128>() {
            features.impose_u128();
            continue;
        }

        if id == TypeId::of::<u16>() {
            features.impose_u16();
            continue;
        }

        if id == TypeId::of::<u32>() {
            features.impose_u32();
            continue;
        }

        if id == TypeId::of::<u64>() {
            features.impose_u64();
            continue;
        }

        if id == TypeId::of::<u8>() {
            features.impose_u8();
            continue;
        }

        if id == TypeId::of::<usize>() {
            features.impose_usize();
            continue;
        }
    }

    match (features.float, features.signed, features.bit_depth) {
        (
            true,
            _,
            BitDepth::Exact(ExactDepth::Depth8 | ExactDepth::Depth16 | ExactDepth::Depth32),
        ) => f32::num_concat(origin, items),

        (true, _, _) => f64::num_concat(origin, items),

        (false, true, BitDepth::Unknown | BitDepth::Platform) => isize::num_concat(origin, items),

        (false, true, BitDepth::Exact(ExactDepth::Depth8)) => i8::num_concat(origin, items),

        (false, true, BitDepth::Exact(ExactDepth::Depth16)) => i16::num_concat(origin, items),

        (false, true, BitDepth::Exact(ExactDepth::Depth32)) => i32::num_concat(origin, items),

        (false, true, BitDepth::Exact(ExactDepth::Depth64)) => i64::num_concat(origin, items),

        (false, false, BitDepth::Unknown | BitDepth::Platform) => usize::num_concat(origin, items),

        (false, false, BitDepth::Exact(ExactDepth::Depth8)) => u8::num_concat(origin, items),

        (false, false, BitDepth::Exact(ExactDepth::Depth16)) => u16::num_concat(origin, items),

        (false, false, BitDepth::Exact(ExactDepth::Depth32)) => u32::num_concat(origin, items),

        (false, false, BitDepth::Exact(ExactDepth::Depth64)) => u64::num_concat(origin, items),
    }
}

trait NumConcat {
    fn num_concat(origin: Origin, items: &mut [Arg]) -> RuntimeResult<Cell>;
}

trait CastTo<To> {
    fn cast_to(self, origin: &Origin) -> RuntimeResult<To>;

    fn append_to(from: Vec<Self>, origin: &Origin, to: &mut Vec<To>) -> RuntimeResult<()>
    where
        Self: Sized;
}

impl<From, To> CastTo<To> for From
where
    From: ScriptType + Debug + Display + Copy,
    To: cast::From<From> + ScriptType,
    <To as cast::From<From>>::Output: 'static,
{
    fn cast_to(self, origin: &Origin) -> RuntimeResult<To> {
        let to = <To as cast::From<From>>::cast(self);

        let to_id = TypeId::of::<To>();
        let to_result_id = TypeId::of::<StdResult<To, cast::Error>>();

        return match TypeId::of::<<To as cast::From<From>>::Output>() {
            id if id == to_id => {
                // Safety: TypeId is checked.
                Ok(unsafe { transmute_copy::<<To as cast::From<From>>::Output, To>(&to) })
            }

            id if id == to_result_id => {
                // Safety: TypeId is checked.
                let result = unsafe {
                    transmute_copy::<<To as cast::From<From>>::Output, StdResult<To, cast::Error>>(
                        &to,
                    )
                };

                match result {
                    Ok(to) => Ok(to),
                    Err(cause) => {
                        let cause = match cause {
                            cast::Error::Infinite => NumberCastCause::Infinite,
                            cast::Error::NaN => NumberCastCause::NAN,
                            cast::Error::Overflow => NumberCastCause::Overflow,
                            cast::Error::Underflow => NumberCastCause::Underflow,
                        };

                        Err(RuntimeError::NumberCast {
                            access_origin: *origin,
                            from: From::type_meta(),
                            to: To::type_meta(),
                            cause,
                            value: Arc::new(self),
                        })
                    }
                }
            }

            _ => {
                system_panic!("Cast Output format has been changed.")
            }
        };
    }

    #[inline(always)]
    fn append_to(from: Vec<Self>, origin: &Origin, to: &mut Vec<To>) -> RuntimeResult<()>
    where
        Self: Sized,
    {
        for item in from {
            to.push(item.cast_to(origin)?);
        }

        Ok(())
    }
}
