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

mod borrow;
mod cell;
mod coercion;
mod error;
mod hints;
mod ident;
mod invoke;
mod memory;
mod object;
mod origin;
mod package;
mod ty;

// This module is hidden.
//
// You should never use it directly, as its API is not part of the official
// public API of the crate.
#[doc(hidden)]
pub mod __intrinsics;

/// Low-level exporting interfaces.
///
/// This module's API allows you to manually implement semantics for the
/// [exported types](ScriptType), which might not exist in the original
/// Rust API. This enables you to model more complex domain-specific APIs
/// within script environments.
///
/// Typically, the [export](crate::export) macro automatically exports
/// various elements using [ops] APIs based on item introspection.
///
/// You can override this behavior by manually exporting the type. In this case,
/// you need to export a type alias for the underlying type.
///
/// ```
/// use ad_astra::{export, runtime::ScriptType};
///
/// #[derive(Debug)]
/// struct Foo;
///
/// #[export]
/// type FooAlias = Foo;
///
/// // This Script type lacks debugging capabilities because the export
/// // macro does not export any semantics when applied to type aliases.
/// assert!(!<Foo>::type_meta().prototype().implements_debug());
/// ```
///
/// When the export macro is applied to a type alias, it only registers
/// the type in the Script Engine without inspecting it further. It does not
/// export any operators or runtime traits for this type. Specifically, the macro
/// does not implement [Downcast], [Upcast], or the
/// [assignment](Prototype::implements_assign) operator automatically. You can
/// implement them manually if needed.
///
/// To implement a low-level operator for the type, implement the corresponding
/// [ops] trait and export this implementation.
///
/// ```
/// use ad_astra::{
///     export,
///     runtime::{ops::ScriptDebug, ScriptType},
/// };
///
/// #[derive(Debug)]
/// struct Foo;
///
/// #[export]
/// type FooAlias = Foo;
///
/// #[export]
/// impl ScriptDebug for Foo {}
///
/// assert!(<Foo>::type_meta().prototype().implements_debug());
/// ```
///
/// Note that you can export low-level operations for any
/// registered type, not just type aliases. In such cases, you must
/// ensure that these operations do not conflict with higher-level exports.
/// The Script Engine will panic if there are conflicts between export points.
pub mod ops;

pub use crate::runtime::{
    cell::Cell,
    coercion::{Downcast, Either, Provider, TypeMatch, Upcast},
    error::{NumberCastCause, NumericOperationKind, RuntimeError, RuntimeResult, RuntimeResultExt},
    hints::{ComponentHint, TypeHint},
    ident::{Ident, RustIdent, ScriptIdent},
    invoke::{Arg, InvocationMeta, Param},
    object::{Object, Prototype},
    origin::{Origin, RustCode, RustOrigin, ScriptOrigin},
    package::{PackageMeta, ScriptPackage},
    ty::{ScriptType, TypeFamily, TypeMeta},
};
