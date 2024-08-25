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
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    hash::Hash,
};

pub(crate) use crate::runtime::ops::functions::{
    Fn0Repr,
    Fn1Repr,
    Fn2Repr,
    Fn3Repr,
    Fn4Repr,
    Fn5Repr,
    Fn6Repr,
    Fn7Repr,
};
pub use crate::runtime::ops::{
    functions::{Fn0, Fn1, Fn2, Fn3, Fn4, Fn5, Fn6, Fn7},
    types::{DynamicArgument, DynamicReturn, DynamicType},
};
use crate::runtime::{Arg, Cell, Ident, InvocationMeta, Origin, RuntimeResult};

/// A script assignment operator: `lhs = rhs`.
///
/// Implementing this trait enables the
/// [Object::assign](crate::runtime::Object::assign) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details,
/// see the [module documentation](crate::runtime::ops).
pub trait ScriptAssign {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::assign](crate::runtime::Object::assign) function.
    fn script_assign(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>;
}

/// A script concatenation operator: `[a, b, c]`.
///
/// Implementing this trait enables the
/// [TypeMeta::concat](crate::runtime::TypeMeta::concat) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptConcat {
    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [TypeMeta::concat](crate::runtime::TypeMeta::concat) function.
    fn script_concat(origin: Origin, items: &mut [Arg]) -> RuntimeResult<Cell>;
}

/// A dynamic resolver of the object fields: `foo.bar`.
///
/// Implementing this trait enables the
/// [Object::field](crate::runtime::Object::field) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptField {
    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    ///
    /// If the type is fully dynamic, consider using the [DynamicType] type as
    /// a `Result` specification.
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::field](crate::runtime::Object::field) function.
    fn script_field(origin: Origin, lhs: Arg, rhs: Ident) -> RuntimeResult<Cell>;
}

/// A script [cloning](Clone) operator: `*foo`.
///
/// Implementing this trait enables the
/// [Object::clone](crate::runtime::Object::clone) operation.
///
/// The underlying type on which this trait is implemented must also implement
/// the [Clone] trait, which provides the actual implementation of the script
/// cloning operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptClone: Clone {}

/// A script [debugging](Debug) formatting operator.
///
/// Implementing this trait enables the
/// [Object::debug](crate::runtime::Object::debug) operation.
///
/// The underlying type on which this trait is implemented must also implement
/// the [Debug] trait, which provides the actual implementation of the script
/// debugging operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptDebug: Debug {}

/// A script [displaying](Display) formatting operator.
///
/// Implementing this trait enables the
/// [Object::display](crate::runtime::Object::display) operation.
///
/// The underlying type on which this trait is implemented must also implement
/// the [Display] trait, which provides the actual implementation of the script
/// debugging operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptDisplay: Display {}

/// A script equality operator: `lhs == rhs`.
///
/// Implementing this trait enables the
/// [Object::partial_eq](crate::runtime::Object::partial_eq) operation.
///
/// Note that the implementation of [ScriptPartialEq::script_eq] serves both
/// partial and [full equality](Eq) purposes.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptPartialEq {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::partial_eq](crate::runtime::Object::partial_eq) function.
    fn script_eq(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<bool>;
}

/// A default constructor for a script object.
///
/// Implementing this trait enables the
/// [TypeMeta::instantiate](crate::runtime::TypeMeta::instantiate) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptDefault {
    /// Operation implementation.
    ///
    /// The parameter and return type of this function correspond to those of
    /// the [TypeMeta::instantiate](crate::runtime::TypeMeta::instantiate)
    /// function.
    fn script_default(origin: Origin) -> RuntimeResult<Cell>;
}

/// A script partial ordering operator: `lhs >= rhs`, `lhs < rhs`, etc.
///
/// Implementing this trait enables the
/// [Object::partial_ord](crate::runtime::Object::partial_ord) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptPartialOrd {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::partial_ord](crate::runtime::Object::partial_ord) function.
    fn script_partial_cmp(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Option<Ordering>>;
}

/// A script full ordering operator: `lhs >= rhs`, `lhs < rhs`, etc.
///
/// Implementing this trait enables the
/// [Object::ord](crate::runtime::Object::ord) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptOrd {
    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::ord](crate::runtime::Object::ord) function.
    fn script_cmp(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Ordering>;
}

/// A script data [hashing](Hash) operator.
///
/// Implementing this trait enables the
/// [Object::hash](crate::runtime::Object::hash) operation.
///
/// The underlying type on which this trait is implemented must also implement
/// the [Hash] trait, which provides the actual implementation of the script
/// data hashing operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptHash: Hash {}

/// A script invocation operator: `foo(arg1, arg2, arg3)`.
///
/// Implementing this trait enables the
/// [Object::invoke](crate::runtime::Object::invoke) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptInvocation {
    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::invoke](crate::runtime::Object::invoke) function.
    fn invoke(origin: Origin, lhs: Arg, arguments: &mut [Arg]) -> RuntimeResult<Cell>;

    /// Returns the invocation signature description.
    ///
    /// The function returns None if the signature is fully dynamic.
    fn hint() -> Option<&'static InvocationMeta>;
}

/// A script context binding operator.
///
/// Implementing this trait enables the
/// [Object::bind](crate::runtime::Object::bind) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptBinding {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::bind](crate::runtime::Object::bind) function.
    fn script_binding(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>;
}

/// A script addition operator: `lhs + rhs`.
///
/// Implementing this trait enables the
/// [Object::add](crate::runtime::Object::add) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptAdd {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::add](crate::runtime::Object::add) function.
    fn script_add(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>;
}

/// A script addition and assignment operator: `lhs += rhs`.
///
/// Implementing this trait enables the
/// [Object::add_assign](crate::runtime::Object::add_assign) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptAddAssign {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::add_assign](crate::runtime::Object::add_assign) function.
    fn script_add_assign(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>;
}

/// A script subtraction operator: `lhs - rhs`.
///
/// Implementing this trait enables the
/// [Object::sub](crate::runtime::Object::sub) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptSub {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::sub](crate::runtime::Object::sub) function.
    fn script_sub(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>;
}

/// A script subtraction and assignment operator: `lhs -= rhs`.
///
/// Implementing this trait enables the
/// [Object::sub_assign](crate::runtime::Object::sub_assign) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptSubAssign {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::sub_assign](crate::runtime::Object::sub_assign) function.
    fn script_sub_assign(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>;
}

/// A script multiplication operator: `lhs * rhs`.
///
/// Implementing this trait enables the
/// [Object::mul](crate::runtime::Object::mul) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptMul {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::mul](crate::runtime::Object::mul) function.
    fn script_mul(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>;
}

/// A script multiplication and assignment operator: `lhs *= rhs`.
///
/// Implementing this trait enables the
/// [Object::mul_assign](crate::runtime::Object::mul_assign) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptMulAssign {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::mul_assign](crate::runtime::Object::mul_assign) function.
    fn script_mul_assign(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>;
}

/// A script division operator: `lhs / rhs`.
///
/// Implementing this trait enables the
/// [Object::div](crate::runtime::Object::div) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptDiv {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::div](crate::runtime::Object::div) function.
    fn script_div(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>;
}

/// A script division and assignment operator: `lhs /= rhs`.
///
/// Implementing this trait enables the
/// [Object::div_assign](crate::runtime::Object::div_assign) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptDivAssign {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::div_assign](crate::runtime::Object::div_assign) function.
    fn script_div_assign(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>;
}

/// A script logical conjunction operator: `lhs && rhs`.
///
/// Implementing this trait enables the
/// [Object::and](crate::runtime::Object::and) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptAnd {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::and](crate::runtime::Object::and) function.
    fn script_and(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>;
}

/// A script logical disjunction operator: `lhs || rhs`.
///
/// Implementing this trait enables the
/// [Object::or](crate::runtime::Object::or) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptOr {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::or](crate::runtime::Object::or) function.
    fn script_or(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>;
}

/// A script logical negation operator: `!foo`.
///
/// Implementing this trait enables the
/// [Object::not](crate::runtime::Object::not) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptNot {
    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::not](crate::runtime::Object::not) function.
    fn script_not(origin: Origin, lhs: Arg) -> RuntimeResult<Cell>;
}

/// A script numeric negation operator: `-foo`.
///
/// Implementing this trait enables the
/// [Object::neg](crate::runtime::Object::neg) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptNeg {
    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::neg](crate::runtime::Object::neg) function.
    fn script_neg(origin: Origin, lhs: Arg) -> RuntimeResult<Cell>;
}

/// A script bitwise conjunction operator: `lhs & rhs`.
///
/// Implementing this trait enables the
/// [Object::bit_and](crate::runtime::Object::bit_and) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptBitAnd {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::bit_and](crate::runtime::Object::bit_and) function.
    fn script_bit_and(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>;
}

/// A script bitwise conjunction and assignment operator: `lhs &= rhs`.
///
/// Implementing this trait enables the
/// [Object::bit_and_assign](crate::runtime::Object::bit_and_assign) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptBitAndAssign {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::bit_and_assign](crate::runtime::Object::bit_and_assign)
    /// function.
    fn script_bit_and_assign(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>;
}

/// A script bitwise disjunction operator: `lhs | rhs`.
///
/// Implementing this trait enables the
/// [Object::bit_or](crate::runtime::Object::bit_or) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptBitOr {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::bit_or](crate::runtime::Object::bit_or) function.
    fn script_bit_or(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>;
}

/// A script bitwise disjunction and assignment operator: `lhs |= rhs`.
///
/// Implementing this trait enables the
/// [Object::bit_or_assign](crate::runtime::Object::bit_or_assign) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptBitOrAssign {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::bit_or_assign](crate::runtime::Object::bit_or_assign)
    /// function.
    fn script_bit_or_assign(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>;
}

/// A script bitwise exclusive disjunction operator: `lhs ^ rhs`.
///
/// Implementing this trait enables the
/// [Object::bit_xor](crate::runtime::Object::bit_xor) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptBitXor {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::bit_xor](crate::runtime::Object::bit_xor) function.
    fn script_bit_xor(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>;
}

/// A script bitwise exclusive disjunction and assignment operator:
/// `lhs ^= rhs`.
///
/// Implementing this trait enables the
/// [Object::bit_xor_assign](crate::runtime::Object::bit_xor_assign) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptBitXorAssign {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::bit_xor_assign](crate::runtime::Object::bit_xor_assign)
    /// function.
    fn script_bit_xor_assign(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>;
}

/// A script bitwise left shift operator: `lhs << rhs`.
///
/// Implementing this trait enables the
/// [Object::shl](crate::runtime::Object::shl) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptShl {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::shl](crate::runtime::Object::shl) function.
    fn script_shl(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>;
}

/// A script bitwise left shift and assignment operator: `lhs <<= rhs`.
///
/// Implementing this trait enables the
/// [Object::shl_assign](crate::runtime::Object::shl_assign) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptShlAssign {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::shl_assign](crate::runtime::Object::shl_assign) function.
    fn script_shl_assign(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>;
}

/// A script bitwise right shift operator: `lhs >> rhs`.
///
/// Implementing this trait enables the
/// [Object::shr](crate::runtime::Object::shr) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptShr {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::shr](crate::runtime::Object::shr) function.
    fn script_shr(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>;
}

/// A script bitwise right shift and assignment operator: `lhs >>= rhs`.
///
/// Implementing this trait enables the
/// [Object::shr_assign](crate::runtime::Object::shr_assign) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptShrAssign {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::shr_assign](crate::runtime::Object::shr_assign) function.
    fn script_shr_assign(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>;
}

/// A script reminder of division operator: `lhs % rhs`.
///
/// Implementing this trait enables the
/// [Object::rem](crate::runtime::Object::rem) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptRem {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// A rough estimation of the result type of this operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type Result: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::rem](crate::runtime::Object::rem) function.
    fn script_rem(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>;
}

/// A script reminder of division and assignment operator: `lhs %= rhs`.
///
/// Implementing this trait enables the
/// [Object::rem_assign](crate::runtime::Object::rem_assign) operation.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptRemAssign {
    /// A rough estimation of the type of the right-hand side of the operation.
    ///
    /// This type must implement [ScriptType](crate::runtime::ScriptType).
    type RHS: ?Sized;

    /// Operation implementation.
    ///
    /// The parameters and return type of this function correspond to those of
    /// the [Object::rem_assign](crate::runtime::Object::rem_assign) function.
    fn script_rem_assign(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>;
}

/// A marker trait indicating that the underlying type represents void data.
///
/// When this trait is implemented for a script type, the
/// [Prototype::implements_none](crate::runtime::Prototype::implements_none)
/// function will return `true`.
///
/// The trait must be implemented for the
/// [registered type](crate::runtime::ScriptType), and the implementation must
/// be exported using the [export](crate::export) macro. For more details, see
/// the [module documentation](crate::runtime::ops).
pub trait ScriptNone {}

/// A type of the script operator.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum OperatorKind {
    /// An assignment operator: `lhs = rhs`.
    Assign,

    /// An array constructor: `[a, b, c]`.
    Concat,

    /// A field access operator: `foo.bar`.
    Field,

    /// A cloning operator: `*foo`.
    Clone,

    /// A [Debug] formatting operator.
    Debug,

    /// A [Display] formatting operator.
    Display,

    /// An equality operator: `lhs == rhs`.
    PartialEq,

    /// An object's default constructor.
    Default,

    /// A partial ordering operator: `lhs >= rhs`, `lhs < rhs`, etc.
    PartialOrd,

    /// A full ordering operator: `lhs >= rhs`, `lhs < rhs`, etc.
    Ord,

    /// A data [Hash] operator.
    Hash,

    /// An invocation operator: `foo(arg1, arg2, arg3)`.
    Invocation,

    /// A context binding operator.
    Binding,

    /// An addition operator: `lhs + rhs`.
    Add,

    /// An addition and assignment operator: `lhs += rhs`.
    AddAssign,

    /// An subtraction operator: `lhs - rhs`.
    Sub,

    /// An subtraction and assignment operator: `lhs -= rhs`.
    SubAssign,

    /// A multiplication operator: `lhs * rhs`.
    Mul,

    /// A multiplication and assignment operator: `lhs *= rhs`.
    MulAssign,

    /// A division operator: `lhs / rhs`.
    Div,

    /// A division and assignment operator: `lhs /= rhs`.
    DivAssign,

    /// A logical conjunction operator: `lhs && rhs`.
    And,

    /// A logical disjunction operator: `lhs || rhs`.
    Or,

    /// A logical negation operator: `!foo`.
    Not,

    /// A numeric negation operator: `-foo`.
    Neg,

    /// A bitwise conjunction operator: `lhs & rhs`.
    BitAnd,

    /// A bitwise conjunction and assignment operator: `lhs &= rhs`.
    BitAndAssign,

    /// A bitwise disjunction operator: `lhs | rhs`.
    BitOr,

    /// A bitwise disjunction and assignment operator: `lhs |= rhs`.
    BitOrAssign,

    /// A bitwise exclusive disjunction operator: `lhs ^ rhs`.
    BitXor,

    /// A bitwise exclusive disjunction and assignment operator: `lhs ^= rhs`.
    BitXorAssign,

    /// A bitwise left shift operator: `lhs << rhs`.
    Shl,

    /// A bitwise left shift and assignment operator: `lhs <<= rhs`.
    ShlAssign,

    /// A bitwise right shift operator: `lhs >> rhs`.
    Shr,

    /// A bitwise right shift and assignment operator: `lhs >>= rhs`.
    ShrAssign,

    /// A reminder of division operator: `lhs % rhs`.
    Rem,

    /// A reminder of division and assignment operator: `lhs %= rhs`.
    RemAssign,
}

impl Display for OperatorKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Assign => formatter.write_str("= operator"),
            Self::Concat => formatter.write_str("[] operator"),
            Self::Field => formatter.write_str("field access operator"),
            Self::Clone => formatter.write_str("clone operator"),
            Self::Debug => formatter.write_str("debug operator"),
            Self::Display => formatter.write_str("display operator"),
            Self::PartialEq => formatter.write_str("== operator"),
            Self::Default => formatter.write_str("default constructor"),
            Self::PartialOrd => formatter.write_str("ordering"),
            Self::Ord => formatter.write_str("ordering"),
            Self::Hash => formatter.write_str("hash interface"),
            Self::Invocation => formatter.write_str("invocation"),
            Self::Binding => formatter.write_str("binding"),
            Self::Add => formatter.write_str("+ operator"),
            Self::AddAssign => formatter.write_str("+= operator"),
            Self::Sub => formatter.write_str("- operator"),
            Self::SubAssign => formatter.write_str("-= operator"),
            Self::Mul => formatter.write_str("* operator"),
            Self::MulAssign => formatter.write_str("*= operator"),
            Self::Div => formatter.write_str("/ operator"),
            Self::DivAssign => formatter.write_str("/= operator"),
            Self::And => formatter.write_str("&& operator"),
            Self::Or => formatter.write_str("|| operator"),
            Self::Not => formatter.write_str("! operator"),
            Self::Neg => formatter.write_str("negative - operator"),
            Self::BitAnd => formatter.write_str("& operator"),
            Self::BitAndAssign => formatter.write_str("&= operator"),
            Self::BitOr => formatter.write_str("| operator"),
            Self::BitOrAssign => formatter.write_str("|= operator"),
            Self::BitXor => formatter.write_str("^ operator"),
            Self::BitXorAssign => formatter.write_str("^= operator"),
            Self::Shl => formatter.write_str("<< operator"),
            Self::ShlAssign => formatter.write_str("<<= operator"),
            Self::Shr => formatter.write_str(">> operator"),
            Self::ShrAssign => formatter.write_str(">>= operator"),
            Self::Rem => formatter.write_str("% operator"),
            Self::RemAssign => formatter.write_str("%= operator"),
        }
    }
}

mod types {
    use std::marker::PhantomData;

    use crate::{
        export,
        runtime::{
            Arg,
            Cell,
            Downcast,
            Origin,
            Provider,
            RuntimeResult,
            ScriptType,
            TypeHint,
            Upcast,
        },
    };

    /// A type that cannot be inferred during static semantic analysis.
    ///
    /// Normally, you should not instantiate or use this object in
    /// implementations, but you can use this type as a hint for the analyzer
    /// when the implementation nature is fully dynamic.
    ///
    /// ```
    /// # use ad_astra::{
    /// #     export,
    /// #     runtime::{
    /// #         ops::{DynamicType, ScriptAssign},
    /// #         Arg,
    /// #         Origin,
    /// #         RuntimeResult,
    /// #     },
    /// # };
    /// #
    /// struct Foo;
    ///
    /// #[export]
    /// type FooAlias = Foo;
    ///
    /// #[export]
    /// impl ScriptAssign for Foo {
    ///     // `type RHS = Foo;` would be more preferable in this case.
    ///     type RHS = DynamicType;
    ///
    ///     fn script_assign(_origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()> {
    ///         let rhs = rhs.data.take::<Foo>(rhs.origin)?;
    ///
    ///         let (lhs_origin, mut lhs_cell) = lhs.split();
    ///
    ///         let lhs = lhs_cell.borrow_mut::<Foo>(lhs_origin)?;
    ///
    ///         *lhs = rhs;
    ///
    ///         Ok(())
    ///     }
    /// }
    /// ```
    ///
    /// Generally, it is recommended to use this type as sparingly as possible,
    /// preferring more specific types even if they provide an imprecise
    /// description.
    ///
    /// If you need more control over the exported function's arguments and
    /// result type casting, consider using [DynamicArgument] and
    /// [DynamicReturn] wrappers instead.
    pub struct DynamicType;

    /// Unknown type.
    ///
    /// This type cannot be inferred statically.
    #[export(include)]
    #[export(name "?")]
    type DynamicTypeExport = DynamicType;

    /// A type of a function's parameter that may have a more dynamic casting
    /// nature than usual.
    ///
    /// The analyzer will assume that the argument's type belongs to the
    /// [type family](crate::runtime::TypeFamily) of the `T` type, but the
    /// Script Engine will not automatically [downcast](Downcast) the script's
    /// argument, allowing the implementation to manually cast the argument's
    /// [Cell].
    ///
    /// ```
    /// # use ad_astra::{
    /// #     export,
    /// #     runtime::{ops::DynamicArgument, Downcast, Provider, RuntimeResult},
    /// # };
    /// #
    /// #[export]
    /// fn plus_10(mut arg: DynamicArgument<usize>) -> RuntimeResult<usize> {
    ///     let mut arg_ty = arg.data.type_match();
    ///
    ///     if arg_ty.belongs_to::<usize>() {
    ///         let arg = <usize>::downcast(arg.origin, Provider::Borrowed(&mut arg.data))?;
    ///
    ///         return Ok(arg + 10);
    ///     }
    ///
    ///     if arg_ty.is::<str>() {
    ///         let arg = arg.data.borrow_str(arg.origin)?;
    ///
    ///         return Ok(arg.len() + 10);
    ///     }
    ///
    ///     Err(arg_ty.mismatch(arg.origin))
    /// }
    /// ```
    pub struct DynamicArgument<T> {
        /// A Rust or Script source code range where the argument has been
        /// provided.
        pub origin: Origin,

        /// The actual data of the argument.
        pub data: Cell,

        phantom: PhantomData<T>,
    }

    impl<T> DynamicArgument<T> {
        /// A helper function that converts this instance into an operator's
        /// [argument](Arg).
        #[inline(always)]
        pub fn into_argument(self) -> Arg {
            Arg {
                origin: self.origin,
                data: self.data,
            }
        }
    }

    impl<'a, T: ScriptType> Downcast<'a> for DynamicArgument<T> {
        #[inline(always)]
        fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
            Ok(DynamicArgument {
                origin,
                data: provider.to_owned(),
                phantom: PhantomData,
            })
        }

        #[inline(always)]
        fn hint() -> TypeHint {
            TypeHint::Type(T::type_meta())
        }
    }

    /// A type for a function's return value that may have a more dynamic
    /// casting nature than usual.
    ///
    /// The analyzer will assume that the return type belongs to the
    /// [type family](crate::runtime::TypeFamily) of the `T` type, but the
    /// Script Engine will not automatically [upcast](Upcast) the Rust value,
    /// allowing the implementation to manually cast the result into a [Cell].
    ///
    /// ```
    /// # use ad_astra::{
    /// #     export,
    /// #     runtime::{ops::DynamicReturn, Cell, Origin, RuntimeResult},
    /// # };
    /// #
    /// #[export]
    /// fn string_or_number(is_string: bool) -> RuntimeResult<DynamicReturn<usize>> {
    ///     match is_string {
    ///         true => Ok(DynamicReturn::new(Cell::give(Origin::nil(), "string")?)),
    ///         false => Ok(DynamicReturn::new(Cell::give(Origin::nil(), 100usize)?)),
    ///     }
    /// }
    /// ```
    #[repr(transparent)]
    pub struct DynamicReturn<T> {
        /// The underlying return object.
        pub data: Cell,
        phantom: PhantomData<T>,
    }

    impl<T> DynamicReturn<T> {
        /// A constructor that wraps the provided `data` [Cell] into a
        /// DynamicReturn.
        #[inline(always)]
        pub fn new(data: Cell) -> Self {
            DynamicReturn {
                data,
                phantom: PhantomData,
            }
        }
    }

    impl<'a, T: ScriptType> Upcast<'a> for DynamicReturn<T> {
        type Output = Cell;

        #[inline(always)]
        fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
            Ok(this.data)
        }

        #[inline(always)]
        fn hint() -> TypeHint {
            TypeHint::Type(T::type_meta())
        }
    }
}

mod functions {
    use lady_deirdre::sync::Lazy;

    use crate::{
        export,
        report::system_panic,
        runtime::{
            ops::{OperatorKind, ScriptConcat, ScriptInvocation},
            ty::ScriptType,
            Arg,
            Cell,
            Downcast,
            InvocationMeta,
            Origin,
            Param,
            Provider,
            RuntimeError,
            RuntimeResult,
            TypeHint,
            Upcast,
        },
    };

    #[repr(transparent)]
    pub struct FnRepr<const ARGS: usize, F: ?Sized> {
        callable: Box<F>,
    }

    macro_rules! impl_fn {
        (
            $(#[doc = $fn_doc:expr])*
            $fn_ty:ident;
            $fn_repr_ty:ident [$arity:expr] as $name:expr => $($arg:ident: $index:expr),*;
        ) => {
            $(#[doc = $fn_doc])*
            pub type $fn_ty<$($arg, )* R> = Box<dyn Fn($($arg),*) -> RuntimeResult<R> + Send + Sync + 'static>;

            $(#[doc = $fn_doc])*
            #[export(include)]
            #[export(name $name)]
            #[export(family &crate::runtime::__intrinsics::FUNCTION_FAMILY)]
            pub(crate) type $fn_repr_ty = FnRepr<
                0,
                dyn Fn(Origin, &mut [Arg; $arity]) -> RuntimeResult<Cell> + Send + Sync + 'static,
            >;

            impl<'a $(, $arg)*, R> Downcast<'a> for $fn_ty<$($arg, )* R>
            where
                $(
                $arg: Upcast<'static>,
                )*
                R: Downcast<'static>,
            {
                #[inline]
                fn downcast(downcast_origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
                    let cell = provider.to_owned();
                    let ty = cell.ty();

                    if !ty.prototype().implements_invocation() {
                        return Err(RuntimeError::UndefinedOperator {
                            access_origin: downcast_origin,
                            receiver_origin: Some(cell.origin()),
                            receiver_type: ty,
                            operator: OperatorKind::Invocation,
                        });
                    }

                    Ok(Box::new(move |$(#[allow(non_snake_case)] $arg: $arg),*| {
                        let object = cell.clone().into_object();

                        let mut arguments: [Arg; $arity] = [
                            $(Arg {
                                origin: downcast_origin,
                                data: Cell::give(downcast_origin, $arg)?
                            }),*
                        ];

                        let result = object.invoke(
                            downcast_origin,
                            downcast_origin,
                            &mut arguments,
                        )?;

                        <R as Downcast<'static>>::downcast(
                            downcast_origin,
                            Provider::Owned(result),
                        )
                    }))
                }

                #[inline(always)]
                fn hint() -> TypeHint {
                    TypeHint::Type(<$fn_repr_ty>::type_meta())
                }
            }

            impl<'a $(, $arg)*, R> Upcast<'a> for $fn_ty<$($arg, )* R>
            where
                $($arg: Downcast<'static>,)*
                R: Upcast<'static>,
            {
                type Output = Box<$fn_repr_ty>;

                #[inline]
                fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
                    Ok(Box::new($fn_repr_ty {
                        callable: Box::new(
                            move |
                                origin: Origin,
                                #[allow(unused)] arguments: &mut [Arg; $arity],
                            | {
                                $(
                                // Safety: Index is lesser than array length.
                                #[allow(non_snake_case)]
                                let $arg = unsafe {
                                    Arg::take_unchecked(arguments, $index)
                                };
                                #[allow(non_snake_case)]
                                let $arg = <$arg as Downcast<'static>>::downcast(
                                    $arg.origin,
                                    $arg.into_provider(),
                                )?;
                                )*

                                let result = this($($arg),*)?;

                                Cell::give(origin, result)
                            },
                        ),
                    }))
                }

                #[inline(always)]
                fn hint() -> TypeHint {
                    TypeHint::Type(<$fn_repr_ty>::type_meta())
                }
            }

            #[export(include)]
            impl ScriptConcat for $fn_repr_ty {
                type Result = Self;

                fn script_concat(origin: Origin, items: &mut [Arg]) -> RuntimeResult<Cell> {
                    $crate::runtime::__intrinsics::canonicals::script_concat::<Self>(origin, items)
                }
            }

            #[export(include)]
            impl ScriptInvocation for $fn_repr_ty {
                fn invoke(origin: Origin, lhs: Arg, arguments: &mut [Arg]) -> RuntimeResult<Cell> {
                    let function = lhs.data.take::<Self>(origin)?;

                    let arguments_count = arguments.len();

                    if arguments_count != $arity {
                        return Err(RuntimeError::ArityMismatch {
                            invocation_origin: origin,
                            function_origin: Origin::default(),
                            parameters: $arity,
                            arguments: arguments_count,
                        });
                    }

                    let arguments = match arguments.try_into() {
                        Ok(array) => array,

                        Err(_) => {
                            system_panic!("Argument slice to array casting failure.")
                        }
                    };

                    (function.callable)(origin, arguments)
                }

                fn hint() -> Option<&'static InvocationMeta> {
                    static META: Lazy<InvocationMeta> = Lazy::new(|| {
                        InvocationMeta {
                            origin: Origin::default(),
                            name: None,
                            doc: Some(concat!($($fn_doc),*)),
                            receiver: None,
                            inputs: Some(vec![
                                $(
                                Param {
                                    name: {
                                        #[allow(unused)]
                                        #[allow(non_snake_case)]
                                        let $arg = ();
                                        None
                                    },
                                    hint: TypeHint::dynamic(),
                                },
                                )*
                            ]),
                            output: TypeHint::dynamic(),
                        }
                    });

                    Some(&META)
                }
            }
        };
    }

    impl_fn!(
        /// A function with 0 arguments.
        Fn0;
        Fn0Repr[0] as "fn(0)" =>;
    );

    impl_fn!(
        /// A function with 1 argument.
        Fn1;
        Fn1Repr[1] as "fn(1)" => A: 0;
    );

    impl_fn!(
        /// A function with 2 arguments.
        Fn2;
        Fn2Repr[2] as "fn(2)" => A: 0, B: 1;
    );

    impl_fn!(
        /// A function with 3 arguments.
        Fn3;
        Fn3Repr[3] as "fn(3)" => A: 0, B: 1, C: 2;
    );

    impl_fn!(
        /// A function with 4 arguments.
        Fn4;
        Fn4Repr[4] as "fn(4)" => A: 0, B: 1, C: 2, D: 3;
    );

    impl_fn!(
        /// A function with 5 arguments.
        Fn5;
        Fn5Repr[5] as "fn(5)" => A: 0, B: 1, C: 2, D: 3, E: 4;
    );

    impl_fn!(
        /// A function with 6 arguments.
        Fn6;
        Fn6Repr[6] as "fn(6)" => A: 0, B: 1, C: 2, D: 3, E: 4, F: 5;
    );

    impl_fn!(
        /// A function with 7 arguments.
        Fn7;
        Fn7Repr[7] as "fn(7)" => A: 0, B: 1, C: 2, D: 3, E: 4, F: 5, G: 6;
    );
}
