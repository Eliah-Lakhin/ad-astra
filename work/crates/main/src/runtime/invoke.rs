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
    fmt::{Display, Formatter},
    hash::{Hash, Hasher},
    mem::take,
};

use crate::{
    report::system_panic,
    runtime::{Cell, Ident, Origin, Provider, TypeHint},
};

/// Metadata for a function-like object.
///
/// Typically, this object describes the signatures of exported Rust functions
/// and methods, but it can also describe a broader range of exported objects
/// that support the
/// [invocation operation](crate::runtime::Prototype::implements_invocation).
///
/// This object aids in the static semantic analysis of Script source code.
///
/// The [Display] implementation of this object renders a canonical, user-facing
/// view of the function's signature, such as
/// `fn foo(x: usize, y: bool) -> f32`.
///
/// Generally, you don't need to instantiate this object manually, unless your
/// crate introduces new types of invokable objects. In such cases, you should
/// store this object in a static context.
///
/// ```
/// use ad_astra::{
///     lady_deirdre::sync::Lazy,
///     runtime::{InvocationMeta, Origin, Param, ScriptType},
/// };
///
/// static INVOKE_META: Lazy<InvocationMeta> = Lazy::new(|| {
///     InvocationMeta {
///         name: Some("foo"),
///         inputs: Some(vec![Param {
///             name: None,
///             hint: <usize>::type_meta().into(),
///         }]),
///         ..InvocationMeta::new(Origin::nil())
///     }
/// });
///
/// assert_eq!(INVOKE_META.name, Some("foo"));
/// ```
#[derive(Clone, Debug)]
pub struct InvocationMeta {
    /// The source code range in Rust or Script where the function signature was
    /// introduced.
    ///
    /// Typically, the resulting [Origin] points to the Rust code.
    pub origin: Origin,

    /// The name of the function, if the function has a name.
    pub name: Option<&'static str>,

    /// The RustDoc documentation for this function, if available.
    pub doc: Option<&'static str>,

    /// The type of the method's receiver (e.g., `self`, `&self`, and
    /// similar Rust function parameters), if the function has a receiver.
    pub receiver: Option<TypeHint>,

    /// The signature of the function parameters, excluding the receiver,
    /// if the signature metadata is available.
    pub inputs: Option<Vec<Param>>,

    /// The type of the object returned after the function invocation.
    ///
    /// If the return type is not specified, the `output` corresponds to the
    /// [TypeHint::dynamic].
    pub output: TypeHint,
}

impl PartialEq for &'static InvocationMeta {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self as *const Self as usize == other as *const Self as usize
    }
}

impl Eq for &'static InvocationMeta {}

impl Hash for &'static InvocationMeta {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self as *const Self as usize).hash(state)
    }
}

impl Display for InvocationMeta {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match (&self.receiver, self.name) {
            (None, Some(name)) => formatter.write_str(&format!("fn {}", name))?,

            (Some(ty), Some(name)) => formatter.write_str(&format!("{}::{}", ty, name))?,

            _ => formatter.write_str("fn")?,
        };

        if let Some(parameters) = &self.inputs {
            formatter.write_str("(")?;

            let mut is_first = true;

            for parameter in parameters {
                match is_first {
                    true => is_first = false,
                    false => formatter.write_str(", ")?,
                }

                Display::fmt(parameter, formatter)?;
            }

            formatter.write_str(")")?;
        }

        if !self.output.is_nil() {
            formatter.write_str(" -> ")?;
            Display::fmt(&self.output, formatter)?;
        }

        Ok(())
    }
}

impl InvocationMeta {
    /// Creates a new invocation metadata object with all fields set to their
    /// default values.
    #[inline(always)]
    pub fn new(origin: Origin) -> Self {
        Self {
            origin,
            name: None,
            doc: None,
            receiver: None,
            inputs: None,
            output: TypeHint::dynamic(),
        }
    }

    /// Returns the arity of the function, which is the number of function
    /// parameters excluding the [receiver](Self::receiver).
    ///
    /// This is a helper function similar in semantics to
    /// `self.inputs.as_ref().map(|inputs| inputs.len())`.
    #[inline(always)]
    pub fn arity(&self) -> Option<usize> {
        self.inputs.as_ref().map(|inputs| inputs.len())
    }
}

/// A description of a parameter in the [InvocationMeta].
#[derive(Clone, Debug)]
pub struct Param {
    /// The name of the parameter. If omitted, the parameter is anonymous.
    pub name: Option<Ident>,

    /// The type of the parameter.
    pub hint: TypeHint,
}

impl Display for Param {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self.hint.is_dynamic() {
            true => match &self.name {
                None => formatter.write_str("_"),
                Some(name) => formatter.write_fmt(format_args!("{name}")),
            },

            false => {
                if let Some(name) = &self.name {
                    formatter.write_fmt(format_args!("{name}: "))?;
                }

                formatter.write_fmt(format_args!("{}", self.hint))
            }
        }
    }
}

/// A wrapper for a [Cell] that is intended to be an argument of a function or
/// an operator.
#[derive(Clone, Default)]
pub struct Arg {
    /// The range in the Rust or Script source code that spans the location of
    /// the argument's application.
    pub origin: Origin,

    /// The actual argument data.
    pub data: Cell,
}

impl Arg {
    /// A convenient constructor for creating an Arg.
    #[inline(always)]
    pub fn new(origin: Origin, data: Cell) -> Self {
        Self { origin, data }
    }

    /// Returns a borrowed [Provider] of the argument's `data` Cell.
    ///
    /// This function is useful for [downcasting](crate::runtime::Downcast) the
    /// argument into a Rust reference:
    ///
    /// `<&usize>::downcast(origin, arg.provider())`.
    ///
    /// Note that downcasting may render the argument's `data` obsolete.
    /// Therefore, you shouldn't use this Cell again. If needed, clone the [Arg]
    /// or the `data` Cell of this Arg object.
    #[inline(always)]
    pub fn provider(&mut self) -> Provider {
        Provider::Borrowed(&mut self.data)
    }

    /// Returns an owned [Provider] of the argument's `data` Cell.
    ///
    /// This function is useful for [downcasting](crate::runtime::Downcast) the
    /// argument into a Rust-owned data:
    ///
    /// `<usize>::downcast(origin, arg.into_provider())`.
    #[inline(always)]
    pub fn into_provider<'a>(self) -> Provider<'a> {
        Provider::Owned(self.data)
    }

    /// A helper function that splits an `Arg` into a tuple of `origin` and
    /// `data`.
    #[inline(always)]
    pub fn split(self) -> (Origin, Cell) {
        (self.origin, self.data)
    }

    /// A helper function that retrieves an argument by index from an array of
    /// arguments.
    ///
    /// This function returns an Arg object at `arguments[index]`, replacing the
    /// original Cell with [Cell::nil].
    ///
    /// ## Panics
    ///
    /// This function panics if the index is greater than or equal to the length
    /// of `arguments`.
    #[inline(always)]
    pub fn take(arguments: &mut [Self], index: usize) -> Self {
        take(&mut arguments[index])
    }

    /// An unsafe version of the [Arg::take] function.
    ///
    /// Unlike the safe `take` function, the `take_unchecked` function does not
    /// check the `arguments` boundaries in production builds. As a result, it
    /// does not panic if the `index` exceeds the length of the input array.
    ///
    /// ## Safety
    ///
    /// The `index` must be less than `arguments.len()`.
    #[inline(always)]
    pub unsafe fn take_unchecked(arguments: &mut [Self], index: usize) -> Self {
        #[cfg(debug_assertions)]
        {
            if index >= arguments.len() {
                let bounds = arguments.len();

                system_panic!("Argument {index} index out of bounds({bounds}).",);
            }
        }

        // Safety: Upheld by the caller.
        take(unsafe { arguments.get_unchecked_mut(index) })
    }
}
