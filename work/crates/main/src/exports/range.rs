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
    any::type_name,
    ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
    ptr::{addr_of, addr_of_mut},
    sync::Arc,
};

use crate::{
    export,
    exports::utils::transparent_upcast,
    runtime::{
        ops::{ScriptClone, ScriptDebug, ScriptHash, ScriptPartialEq},
        Arg,
        Cell,
        Downcast,
        NumericOperationKind,
        Origin,
        Provider,
        RuntimeError,
        RuntimeResult,
        ScriptType,
        TypeHint,
        Upcast,
    },
};

/// A range of integer numbers: `10..125`.
///
/// This range includes all integer numbers starting from the lower value
/// (inclusive) up to the upper value (exclusive).
#[export(include)]
#[export(name "range")]
pub(crate) type RangeType = Range<usize>;

impl<'a> Downcast<'a> for RangeType {
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let mut type_match = provider.type_match();

        if type_match.is::<Self>() {
            return provider.to_owned().take(origin);
        }

        Err(type_match.mismatch(origin))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(Self::type_meta())
    }
}

impl<'a> Downcast<'a> for &'a RangeType {
    #[inline]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let mut type_match = provider.type_match();

        if type_match.is::<RangeType>() {
            return provider.to_borrowed(&origin)?.borrow_ref(origin);
        }

        Err(type_match.mismatch(origin))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(RangeType::type_meta())
    }
}

impl<'a> Downcast<'a> for &'a mut RangeType {
    #[inline]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let mut type_match = provider.type_match();

        if type_match.is::<RangeType>() {
            return provider.to_borrowed(&origin)?.borrow_mut(origin);
        }

        Err(type_match.mismatch(origin))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(RangeType::type_meta())
    }
}

transparent_upcast!(RangeType);

#[export(include)]
impl ScriptClone for RangeType {}

#[export(include)]
impl ScriptHash for RangeType {}

#[export(include)]
impl ScriptDebug for RangeType {}

#[export(include)]
impl ScriptPartialEq for RangeType {
    type RHS = Self;

    fn script_eq(_origin: Origin, mut lhs: Arg, mut rhs: Arg) -> RuntimeResult<bool> {
        let lhs = lhs.data.borrow_ref::<Self>(lhs.origin)?;

        let mut type_match = rhs.data.type_match();

        if type_match.is::<Self>() {
            let rhs = rhs.data.borrow_ref::<Self>(rhs.origin)?;

            return Ok(lhs == rhs);
        }

        if type_match.belongs_to::<usize>() {
            let singleton =
                <usize as Downcast>::downcast(rhs.origin, Provider::Borrowed(&mut rhs.data))?;

            return match singleton == usize::MAX {
                true => Ok(false),

                false => Ok(lhs.start == singleton && lhs.end == singleton + 1),
            };
        }

        Err(type_match.mismatch(rhs.origin))
    }
}

trait RangeImpl {
    fn start(origin: Origin, lhs: Arg) -> RuntimeResult<Cell>;
    fn end(origin: Origin, lhs: Arg) -> RuntimeResult<Cell>;
}

#[export(include)]
impl RangeImpl for RangeType {
    #[export(component RangeType)]
    fn start(origin: Origin, lhs: Arg) -> RuntimeResult<Cell> {
        unsafe fn by_ref(range: *const RangeType) -> *const usize {
            // Safety: Upheld by the Cell::map_component specification.
            unsafe { addr_of!((*range).start) }
        }

        unsafe fn by_mut(range: *mut RangeType) -> *mut usize {
            // Safety: Upheld by the Cell::map_component specification.
            unsafe { addr_of_mut!((*range).start) }
        }

        lhs.data
            .map_ptr::<Self, usize>(origin, Some(by_ref), Some(by_mut))
    }

    #[export(component RangeType)]
    fn end(origin: Origin, lhs: Arg) -> RuntimeResult<Cell> {
        unsafe fn by_ref(range: *const RangeType) -> *const usize {
            // Safety: Upheld by the Cell::map_component specification.
            unsafe { addr_of!((*range).end) }
        }

        unsafe fn by_mut(range: *mut RangeType) -> *mut usize {
            // Safety: Upheld by the Cell::map_component specification.
            unsafe { addr_of_mut!((*range).end) }
        }

        lhs.data
            .map_ptr::<Self, usize>(origin, Some(by_ref), Some(by_mut))
    }
}

impl<'a> Downcast<'a> for RangeFrom<usize> {
    #[inline]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let range = provider.to_owned().take::<RangeType>(origin)?;

        if range.end != usize::MAX {
            return Err(RuntimeError::RangeCast {
                access_origin: origin,
                from: range,
                to: type_name::<RangeFrom<usize>>(),
            });
        }

        Ok(RangeFrom { start: range.start })
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(RangeType::type_meta())
    }
}

impl<'a> Upcast<'a> for RangeFrom<usize> {
    type Output = Box<RangeType>;

    #[inline(always)]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(Box::new(this.start..usize::MAX))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(RangeType::type_meta())
    }
}

impl<'a> Downcast<'a> for RangeFull {
    #[inline]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let range = provider.to_owned().take::<RangeType>(origin)?;

        if range.start != 0 || range.end != usize::MAX {
            return Err(RuntimeError::RangeCast {
                access_origin: origin,
                from: range,
                to: type_name::<RangeFrom<usize>>(),
            });
        }

        Ok(RangeFull)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(RangeType::type_meta())
    }
}

impl<'a> Upcast<'a> for RangeFull {
    type Output = Box<RangeType>;

    #[inline(always)]
    fn upcast(_origin: Origin, _this: Self) -> RuntimeResult<Self::Output> {
        Ok(Box::new(0..usize::MAX))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(RangeType::type_meta())
    }
}

impl<'a> Downcast<'a> for RangeInclusive<usize> {
    #[inline]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let mut type_match = provider.type_match();

        if type_match.is::<RangeType>() {
            let range = provider.to_owned().take::<RangeType>(origin)?;

            let start = range.start;

            let end = match start.checked_add(1) {
                Some(bound) => bound,

                None => {
                    return Err(RuntimeError::NumericOperation {
                        invoke_origin: origin,
                        kind: NumericOperationKind::Add,
                        lhs: (usize::type_meta(), Arc::new(start)),
                        rhs: Some((usize::type_meta(), Arc::new(1))),
                        target: usize::type_meta(),
                    })
                }
            };

            return Ok(start..=end);
        }

        if type_match.belongs_to::<usize>() {
            let start = <usize as Downcast>::downcast(origin, provider)?;

            return Ok(start..=start);
        }

        Err(type_match.mismatch(origin))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(RangeType::type_meta())
    }
}

impl<'a> Upcast<'a> for RangeInclusive<usize> {
    type Output = Box<RangeType>;

    #[inline]
    fn upcast(origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        let start = *this.start();

        let end = match start.checked_add(1) {
            Some(bound) => bound,

            None => {
                return Err(RuntimeError::NumericOperation {
                    invoke_origin: origin,
                    kind: NumericOperationKind::Add,
                    lhs: (usize::type_meta(), Arc::new(start)),
                    rhs: Some((usize::type_meta(), Arc::new(1))),
                    target: usize::type_meta(),
                })
            }
        };

        Ok(Box::new(start..end))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(RangeType::type_meta())
    }
}

impl<'a> Downcast<'a> for RangeTo<usize> {
    #[inline]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let range = provider.to_owned().take::<RangeType>(origin)?;

        if range.start != 0 {
            return Err(RuntimeError::RangeCast {
                access_origin: origin,
                from: range,
                to: type_name::<RangeTo<usize>>(),
            });
        }

        Ok(..range.end)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(RangeType::type_meta())
    }
}

impl<'a> Upcast<'a> for RangeTo<usize> {
    type Output = Box<RangeType>;

    #[inline(always)]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(Box::new(0..this.end))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(RangeType::type_meta())
    }
}

impl<'a> Downcast<'a> for RangeToInclusive<usize> {
    #[inline]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let range = provider.to_owned().take::<RangeType>(origin)?;

        if range.start != 0 {
            return Err(RuntimeError::RangeCast {
                access_origin: origin,
                from: range,
                to: type_name::<RangeToInclusive<usize>>(),
            });
        }

        let end = match range.end.checked_add(1) {
            Some(bound) => bound,

            None => {
                return Err(RuntimeError::NumericOperation {
                    invoke_origin: origin,
                    kind: NumericOperationKind::Add,
                    lhs: (usize::type_meta(), Arc::new(range.end)),
                    rhs: Some((usize::type_meta(), Arc::new(1))),
                    target: usize::type_meta(),
                })
            }
        };

        Ok(..=end)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(RangeType::type_meta())
    }
}
