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
    collections::hash_map::Entry,
    error::Error as StdError,
    fmt::{Debug, Display, Formatter},
    ops::Range,
    result::Result as StdResult,
    str::Utf8Error,
    sync::Arc,
};

use ahash::{AHashMap, AHashSet};
use lady_deirdre::{arena::Identifiable, format::AnnotationPriority, lexis::ToSpan};

use crate::{
    analysis::ModuleTextResolver,
    format::{format_script_path, ScriptSnippet},
    runtime::{ops::OperatorKind, Origin, TypeMeta},
};

/// A result of a runtime API call, which can either be a normal value or a
/// [RuntimeError].
pub type RuntimeResult<T> = StdResult<T, RuntimeError>;

/// A helper trait for the [RuntimeResult] object.
///
/// This trait is automatically implemented for RuntimeResult and provides the
/// [expect_blame](Self::expect_blame) function, which either unwraps the
/// underlying value or panics if the result is [Err], indicating where the
/// RuntimeError [originated](RuntimeError::primary_origin).
pub trait RuntimeResultExt {
    /// The [Ok] type of the underlying [Result].
    type OkType;

    /// If the result is [Ok], returns the underlying data; otherwise, panics
    /// at the location where the RuntimeError
    /// [originated](RuntimeError::primary_origin).
    fn expect_blame(self, message: &str) -> Self::OkType;
}

impl<T> RuntimeResultExt for RuntimeResult<T> {
    type OkType = T;

    #[inline(always)]
    fn expect_blame(self, message: &str) -> Self::OkType {
        match self {
            Ok(ok) => ok,

            Err(error) => {
                let origin = *error.primary_origin();

                match origin {
                    Origin::Rust(origin) => origin.blame(&format!("{message}\n{error}")),

                    Origin::Script(origin) => {
                        panic!(
                            "{}: {message}\n{error}",
                            format_script_path(origin.id(), None),
                        );
                    }
                }
            }
        }
    }
}

/// Represents any error that may occur during the evaluation of Script code.
///
/// This object implements the [Debug] and [Display] traits. The Display
/// implementation provides a brief description of the underlying error.
///
/// However, it is recommended to use the [RuntimeError::display] function
/// instead, as it renders the script's source code and annotates it with
/// detailed error messages.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum RuntimeError {
    /// The script code attempts to access [Nil](crate::runtime::Cell::nil)
    /// data.
    Nil {
        /// The range in Rust or Script source code where the data was accessed.
        access_origin: Origin,
    },

    /// The script code attempts to access an object representing an array
    /// with zero or more than one element.
    NonSingleton {
        /// The range in Rust or Script source code where the data was accessed.
        access_origin: Origin,

        /// The actual length of the array.
        actual: usize,
    },

    /// The script array is too short and cannot be interpreted as an array
    /// with the requested number of elements.
    ShortSlice {
        /// The range in Rust or Script source code where the data was accessed.
        access_origin: Origin,

        /// The required length of the array.
        minimum: usize,

        /// The actual length of the array.
        actual: usize,
    },

    /// The script code attempts to index into an array or string, but the index
    /// is out of bounds.
    OutOfBounds {
        /// The range in Rust or Script source code where the data was accessed.
        access_origin: Origin,

        /// The requested index for the array or string.
        index: usize,

        /// The actual length of the array or string.
        length: usize,
    },

    /// The script code attempts to mutate an object that only provides
    /// read-only access.
    ReadOnly {
        /// The range in Rust or Script source code where the data was accessed.
        access_origin: Origin,

        /// The range in Rust or Script source code where the data was created.
        data_origin: Origin,
    },

    /// The script code attempts to read an object that only provides mutation
    /// access.
    WriteOnly {
        /// The range in Rust or Script source code where the data was accessed.
        access_origin: Origin,

        /// The range in Rust or Script source code where the data was created.
        data_origin: Origin,
    },

    /// The script code attempts to mutate an object that is currently borrowed
    /// for reading.
    ReadToWrite {
        /// The range in Rust or Script source code where the data was accessed.
        access_origin: Origin,

        /// The range in Rust or Script source code where the data was
        /// previously borrowed.
        borrow_origin: Origin,
    },

    /// The script code attempts to read an object that is currently borrowed
    /// for writing.
    WriteToRead {
        /// The range in Rust or Script source code where the data was accessed.
        access_origin: Origin,

        /// The range in Rust or Script source code where the data was
        /// previously borrowed.
        borrow_origin: Origin,
    },

    /// The script code attempts to borrow data for mutation more than once
    /// at a time.
    WriteToWrite {
        /// The range in Rust or Script source code where the data was accessed.
        access_origin: Origin,

        /// The range in Rust or Script source code where the data was
        /// previously borrowed.
        borrow_origin: Origin,
    },

    /// The script attempts to decode a byte array that is not a valid UTF-8
    /// encoding.
    Utf8Decoding {
        /// The range in Rust or Script source code where the data was accessed.
        access_origin: Origin,

        /// An error that occurred during UTF-8 decoding.
        cause: Box<Utf8Error>,
    },

    /// The script attempts to borrow data too many times simultaneously.
    BorrowLimit {
        /// The range in Rust or Script source code where the data was accessed.
        access_origin: Origin,

        /// The maximum number of allowed simultaneous active borrows.
        limit: usize,
    },

    /// The script attempts to use a data object as an argument for a function
    /// or an operator, but the data type does not meet the requirements.
    TypeMismatch {
        /// The range in Rust or Script source code where the data was accessed.
        access_origin: Origin,

        /// The type of the data object being provided as an argument.
        data_type: &'static TypeMeta,

        /// A list of expected types acceptable for this operation.
        expected_types: Vec<&'static TypeMeta>,
    },

    /// The script attempts to dereference a data object, but the data object
    /// does not live long enough.
    DowncastStatic {
        /// The range in Rust or Script source code where the data was accessed.
        access_origin: Origin,
    },

    /// The script calls a Rust function that results in [Result::Err].
    UpcastResult {
        /// The range in Rust or Script source code where the function was
        /// invoked.
        access_origin: Origin,

        /// The inner value of the [Err] variant.
        cause: Arc<dyn StdError + Send + Sync + 'static>,
    },

    /// The script attempts to cast one numeric type into another, but the
    /// conversion is not possible for the specified source number value and the
    /// requested destination type.
    NumberCast {
        /// The range in Rust or Script source code where the numeric object was
        /// accessed for conversion.
        access_origin: Origin,

        /// The source type of the value from which the conversion was
        /// requested.
        from: &'static TypeMeta,

        /// The destination type into which the source number should be
        /// converted.
        to: &'static TypeMeta,

        /// The cause of the failure.
        cause: NumberCastCause,

        /// The source numeric value. This object implements [Display].
        value: Arc<dyn NumValue>,
    },

    /// The script attempts to perform an operation between two primitive
    /// numeric values (or on a single numeric value), but the operation results
    /// in an error due to specific reasons (e.g., numeric overflow).
    NumericOperation {
        /// The range in Rust or Script source code where the operation was
        /// requested.
        invoke_origin: Origin,

        /// The type of numeric operation.
        kind: NumericOperationKind,

        /// The left-hand side of the operation.
        lhs: (&'static TypeMeta, Arc<dyn NumValue>),

        /// The right-hand side of the operation. If omitted, the operation has
        /// only one argument.
        rhs: Option<(&'static TypeMeta, Arc<dyn NumValue>)>,

        /// The numeric type expected to represent the result of the operation.
        target: &'static TypeMeta,
    },

    /// The script attempts to cast a [Range] object into another range type
    /// (e.g., `100..200` into `100..=199`), but such casting is not possible
    /// due to unsatisfied bounds.
    RangeCast {
        /// The range in Rust or Script source code where the casting was
        /// requested.
        access_origin: Origin,

        /// The original value of the range from which the casting was supposed
        /// to happen.
        from: Range<usize>,

        /// The name of the target range type.
        to: &'static str,
    },

    /// The script attempts to use a malformed range (e.g., `200..100`).
    MalformedRange {
        /// The range in Rust or Script source code where the range was
        /// accessed.
        access_origin: Origin,

        /// The lower bound of the [Range].
        start_bound: usize,

        /// The upper bound of the [Range].
        end_bound: usize,
    },

    /// The script attempts to parse a string into a primitive type, but the
    /// string is malformed and cannot be interpreted as the requested
    /// primitive type.
    PrimitiveParse {
        /// The range in Rust or Script source code where the string was
        /// accessed for parsing.
        access_origin: Origin,

        /// The content of the string being parsed.
        from: String,

        /// The primitive type into which the string was supposed to be parsed.
        to: &'static TypeMeta,

        /// A description of the parse error.
        cause: Arc<dyn StdError + Send + Sync + 'static>,
    },

    /// The script attempts to call a function with an incorrect number of
    /// arguments, either too few or too many.
    ArityMismatch {
        /// The range in Rust or Script source code where the function was
        /// invoked.
        invocation_origin: Origin,

        /// The range in Rust or Script source code where the function was
        /// declared.
        function_origin: Origin,

        /// The expected number of parameters for the function.
        parameters: usize,

        /// The actual number of arguments that were passed during the
        /// invocation.
        arguments: usize,
    },

    /// The script attempts to apply an operator to an object, but the object's
    /// type does not support this operator.
    UndefinedOperator {
        /// The range in Rust or Script source code where the operator was
        /// applied.
        access_origin: Origin,

        /// The range in Rust or Script source code where the object was
        /// created. If omitted, the object instance has not been created yet
        /// (e.g., the operator is an instantiation operator).
        receiver_origin: Option<Origin>,

        /// The type of the object to which the operator was applied.
        receiver_type: &'static TypeMeta,

        /// The type of operator.
        operator: OperatorKind,
    },

    /// The script attempts to access a field of an object, but the object does
    /// not have the specified field.
    UnknownField {
        /// A Rust or Script source code range, where the field was accessed.
        access_origin: Origin,

        /// The range in Rust or Script source code where the field was
        /// accessed.
        receiver_origin: Origin,

        /// The type of the receiver object.
        receiver_type: &'static TypeMeta,

        /// The name of the field.
        field: String,
    },

    /// The script attempts to format a data object using the [Debug] or
    /// [Display] implementations, but the formatter returns an error.
    FormatError {
        /// The range in Rust or Script source code where the object was
        /// accessed for formatting.
        access_origin: Origin,

        /// The origin of the receiver object.
        receiver_origin: Origin,
    },

    /// The script attempts to access a package that is not fully registered
    /// (e.g., the export system has been switched to a shallow mode).
    UnknownPackage {
        /// The range in Rust or Script source code where the package metadata
        /// was accessed.
        access_origin: Origin,

        /// The name of the package.
        name: &'static str,

        /// The version of the package.
        version: &'static str,
    },

    /// The script evaluation has been interrupted by the thread's
    /// [runtime hook](crate::interpret::set_runtime_hook).
    Interrupted {
        /// The range in Rust or Script source code where the interruption
        /// occurred.
        origin: Origin,
    },

    /// The script has been interrupted because the interpreter's memory stack
    /// for the thread overflowed.
    StackOverflow {
        /// The range in Rust or Script source code where the interruption
        /// occurred.
        origin: Origin,
    },
}

impl Display for RuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nil { .. } => formatter.write_str("inaccessible data"),

            Self::NonSingleton { actual, .. } => formatter.write_fmt(format_args!(
                "expected a single data instance, but the array with {actual} \
                elements provided",
            )),

            Self::ShortSlice {
                minimum, actual, ..
            } => formatter.write_fmt(format_args!(
                "expected an array with at least {minimum} elements, but the \
                array with {actual} elements provided",
            )),

            Self::OutOfBounds { index, length, .. } => {
                formatter.write_fmt(format_args!("index {index} out of 0..{length} bounds",))
            }

            Self::ReadOnly { .. } => formatter.write_str("read-only data"),

            Self::WriteOnly { .. } => formatter.write_str("write-only data"),

            Self::ReadToWrite { .. } => {
                formatter.write_str("cannot access data for write while it is being read")
            }

            Self::WriteToRead { .. } => {
                formatter.write_str("cannot access data for read while it is being written")
            }

            Self::WriteToWrite { .. } => {
                formatter.write_str("cannot access data for write more than once")
            }

            Self::Utf8Decoding { .. } => formatter.write_str("invalid utf-8 encoding"),

            Self::BorrowLimit { .. } => formatter.write_str("too many simultaneous data accesses"),

            Self::TypeMismatch {
                data_type,
                expected_types,
                ..
            } => {
                let partition = Self::partition_types(expected_types);

                match partition.is_empty() {
                    true => formatter.write_fmt(format_args!("unexpected '{data_type}' data type")),

                    false => {
                        let partition = partition.join(", or ");

                        formatter.write_fmt(format_args!(
                            "expected {partition}, but '{data_type}' data type provided"
                        ))
                    }
                }
            }

            Self::DowncastStatic { .. } => formatter
                .write_str("cannot get static reference to the data owned by the script engine"),

            Self::UpcastResult { .. } => {
                formatter.write_str("the function returned explicit error")
            }

            Self::NumberCast {
                from,
                to,
                cause,
                value,
                ..
            } => {
                use NumberCastCause::*;

                match cause {
                    Infinite => formatter.write_fmt(format_args!(
                        "cannot cast infinity value of {from} type to {to}"
                    )),

                    NAN => formatter
                        .write_fmt(format_args!("cannot cast NAN value of {from} type to {to}")),

                    Overflow => {
                        formatter.write_fmt(format_args!("cannot cast {value}{from} to {to} type"))
                    }

                    Underflow => {
                        formatter.write_fmt(format_args!("cannot cast {value}{from} to {to} type"))
                    }
                }
            }

            Self::NumericOperation { kind, lhs, rhs, .. } => {
                use NumericOperationKind::*;

                match (lhs, kind, rhs) {
                    ((lhs_ty, lhs_value), Add, Some((rhs_ty, rhs_value))) => formatter.write_fmt(
                        format_args!("cannot add {rhs_value}{rhs_ty} to {lhs_value}{lhs_ty}"),
                    ),

                    ((lhs_ty, lhs_value), Sub, Some((rhs_ty, rhs_value))) => {
                        formatter.write_fmt(format_args!(
                            "cannot subtract {rhs_value}{rhs_ty} from {lhs_value}{lhs_ty}"
                        ))
                    }

                    ((lhs_ty, lhs_value), Mul, Some((rhs_ty, rhs_value))) => formatter.write_fmt(
                        format_args!("cannot multiply {rhs_value}{rhs_ty} by {lhs_value}{lhs_ty}"),
                    ),

                    ((lhs_ty, lhs_value), Div, Some((rhs_ty, rhs_value))) => formatter.write_fmt(
                        format_args!("cannot divide {rhs_value}{rhs_ty} by {lhs_value}{lhs_ty}"),
                    ),

                    ((lhs_ty, lhs_value), Neg, None) => formatter.write_fmt(format_args!(
                        "cannot get negative number of {lhs_value}{lhs_ty}"
                    )),

                    ((lhs_ty, lhs_value), Shl, Some((rhs_ty, rhs_value))) => {
                        formatter.write_fmt(format_args!(
                            "cannot shift {lhs_value}{lhs_ty} left by {rhs_value}{rhs_ty} bits"
                        ))
                    }

                    ((lhs_ty, lhs_value), Shr, Some((rhs_ty, rhs_value))) => {
                        formatter.write_fmt(format_args!(
                            "cannot shift {lhs_value}{lhs_ty} right by {rhs_value}{rhs_ty} bits"
                        ))
                    }

                    ((lhs_ty, lhs_value), Rem, Some((_rhs_ty, _rhs_value))) => {
                        formatter.write_fmt(format_args!(
                            "cannot get {lhs_value}{lhs_ty} reminder of \
                            division by {lhs_value}{lhs_ty}"
                        ))
                    }

                    _ => formatter.write_str("invalid numeric operation"),
                }
            }

            Self::RangeCast { from, to, .. } => formatter.write_fmt(format_args!(
                "cannot cast {from:?} range to {to} type. target type bounds mismatch"
            )),

            Self::MalformedRange {
                start_bound,
                end_bound,
                ..
            } => formatter.write_fmt(format_args!(
                "range {start_bound} start bound is greater than the range end bound {end_bound}"
            )),

            Self::PrimitiveParse { from, to, .. } => {
                formatter.write_fmt(format_args!("failed to parse {from:?} as {to}"))
            }

            Self::ArityMismatch {
                parameters,
                arguments,
                ..
            } => match *parameters == 1 {
                true => formatter.write_fmt(format_args!(
                    "the function requires 1 argument, but {arguments} provided"
                )),
                false => formatter.write_fmt(format_args!(
                    "the function requires {parameters} arguments, but {arguments} provided"
                )),
            },

            Self::UndefinedOperator {
                receiver_type,
                operator,
                ..
            } => formatter.write_fmt(format_args!(
                "type '{receiver_type}' does not implement {operator}"
            )),

            Self::UnknownField {
                receiver_type,
                field,
                ..
            } => formatter.write_fmt(format_args!(
                "type '{receiver_type}' does not have field '{field}'"
            )),

            Self::FormatError { .. } => formatter
                .write_str("an error occurred during Debug or Display format function call"),

            Self::UnknownPackage { name, version, .. } => {
                formatter.write_fmt(format_args!("unknown {name}@{version} script package"))
            }

            Self::Interrupted { .. } => formatter.write_str("script evaluation interrupted"),

            Self::StackOverflow { .. } => formatter.write_str("script engine stack overflow"),
        }
    }
}

impl StdError for RuntimeError {
    #[inline]
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Utf8Decoding { cause, .. } => Some(cause),
            Self::UpcastResult { cause, .. } => Some(cause),
            Self::PrimitiveParse { cause, .. } => Some(cause),
            _ => None,
        }
    }
}

impl RuntimeError {
    /// Returns a printable object that renders the script's source code and
    /// annotates it with error messages describing the underlying error object
    /// and pointing to the source code location(s) where the error occurred.
    ///
    /// This function provides a canonical way to print end-user-facing script
    /// evaluation error messages.
    ///
    /// The `resolver` parameter specifies an object through which the returned
    /// printer accesses the [ModuleText](crate::analysis::ModuleText).
    ///
    /// In a multi-module environment, a RuntimeError may occur in any of these
    /// modules and could potentially relate to several scripts. The resolver
    /// allows the printer to access their source code texts when formatting the
    /// message.
    ///
    /// If your runtime configuration consists of just one script module, or if
    /// you are confident that the modules are semantically isolated from each
    /// other (by default, modules are isolated), you can directly use the
    /// ModuleText of the [script module](crate::analysis::ScriptModule) as the
    /// `resolver` argument.
    pub fn display<'a>(&self, resolver: &'a impl ModuleTextResolver) -> impl Display + 'a {
        enum DisplayError<'a> {
            Snippet(ScriptSnippet<'a>),
            String(String),
        }

        impl<'a> Display for DisplayError<'a> {
            #[inline(always)]
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::Snippet(display) => Display::fmt(display, formatter),
                    Self::String(display) => Display::fmt(display, formatter),
                }
            }
        }

        let primary_description = self.primary_description();

        let Origin::Script(primary_origin) = self.primary_origin() else {
            return DisplayError::String(primary_description);
        };

        let Some(primary_text) = resolver.resolve(primary_origin.id()) else {
            return DisplayError::String(primary_description);
        };

        if !primary_origin.is_valid_span(primary_text) {
            return DisplayError::String(primary_description);
        }

        let secondary_description = self.secondary_description();

        let mut snippet = primary_text.snippet();

        snippet.set_caption("runtime error");
        snippet.annotate(
            primary_origin,
            AnnotationPriority::Primary,
            primary_description,
        );

        let mut summary = self.summary();

        match self.secondary_origin() {
            Some(Origin::Script(secondary_origin))
                if secondary_origin.id() == primary_origin.id() =>
            {
                snippet.annotate(
                    secondary_origin,
                    AnnotationPriority::Secondary,
                    secondary_description,
                );
            }

            Some(Origin::Script(secondary_origin)) => {
                if let Some(secondary_text) = resolver.resolve(primary_origin.id()) {
                    if secondary_origin.is_valid_span(secondary_text) {
                        summary.push_str("\n\n");

                        let mut inner_snippet = secondary_text.snippet();

                        inner_snippet.annotate(
                            secondary_origin,
                            AnnotationPriority::Secondary,
                            secondary_description,
                        );

                        summary.push_str(&inner_snippet.to_string());
                    }
                }
            }

            Some(Origin::Rust(secondary_origin)) => {
                if let Some(code) = secondary_origin.code {
                    summary.push_str(&format!("\n\n{code}: {secondary_description}"));
                }
            }

            None => (),
        }

        snippet.set_summary(summary);

        DisplayError::Snippet(snippet)
    }

    /// Returns the Rust or Script source code range where the error occurred.
    pub fn primary_origin(&self) -> &Origin {
        match self {
            Self::Nil { access_origin, .. } => access_origin,

            Self::NonSingleton { access_origin, .. } => access_origin,

            Self::ShortSlice { access_origin, .. } => access_origin,

            Self::OutOfBounds { access_origin, .. } => access_origin,

            Self::ReadOnly { access_origin, .. } => access_origin,

            Self::WriteOnly { access_origin, .. } => access_origin,

            Self::ReadToWrite { access_origin, .. } => access_origin,

            Self::WriteToRead { access_origin, .. } => access_origin,

            Self::WriteToWrite { access_origin, .. } => access_origin,

            Self::Utf8Decoding { access_origin, .. } => access_origin,

            Self::BorrowLimit { access_origin, .. } => access_origin,

            Self::TypeMismatch { access_origin, .. } => access_origin,

            Self::DowncastStatic { access_origin, .. } => access_origin,

            Self::UpcastResult { access_origin, .. } => access_origin,

            Self::NumberCast { access_origin, .. } => access_origin,

            Self::RangeCast { access_origin, .. } => access_origin,

            Self::MalformedRange { access_origin, .. } => access_origin,

            Self::NumericOperation { invoke_origin, .. } => invoke_origin,

            Self::PrimitiveParse { access_origin, .. } => access_origin,

            Self::ArityMismatch {
                invocation_origin, ..
            } => invocation_origin,

            Self::UndefinedOperator { access_origin, .. } => access_origin,

            Self::UnknownField { access_origin, .. } => access_origin,

            Self::FormatError { access_origin, .. } => access_origin,

            Self::UnknownPackage { access_origin, .. } => access_origin,

            Self::Interrupted { origin } => origin,

            Self::StackOverflow { origin, .. } => origin,
        }
    }

    /// Returns an additional Rust or Script source code range that hints at an
    /// extra location related to the cause of the error.
    ///
    /// For example, the [ReadToWrite](Self::ReadToWrite) error variant includes
    /// an additional location that indicates the previous borrowing site.
    ///
    /// Most error variants do not have an extra location, and in such cases,
    /// this function will return None.
    pub fn secondary_origin(&self) -> Option<&Origin> {
        match self {
            Self::Nil { .. } => None,

            Self::NonSingleton { .. } => None,

            Self::ShortSlice { .. } => None,

            Self::OutOfBounds { .. } => None,

            Self::ReadOnly { data_origin, .. } => Some(data_origin),

            Self::WriteOnly { data_origin, .. } => Some(data_origin),

            Self::ReadToWrite { borrow_origin, .. } => Some(borrow_origin),

            Self::WriteToRead { borrow_origin, .. } => Some(borrow_origin),

            Self::WriteToWrite { borrow_origin, .. } => Some(borrow_origin),

            Self::Utf8Decoding { .. } => None,

            Self::BorrowLimit { .. } => None,

            Self::TypeMismatch { .. } => None,

            Self::DowncastStatic { .. } => None,

            Self::UpcastResult { .. } => None,

            Self::NumberCast { .. } => None,

            Self::NumericOperation { .. } => None,

            Self::RangeCast { .. } => None,

            Self::MalformedRange { .. } => None,

            Self::PrimitiveParse { .. } => None,

            Self::ArityMismatch {
                function_origin, ..
            } => Some(function_origin),

            Self::UndefinedOperator {
                receiver_origin, ..
            } => receiver_origin.as_ref(),

            Self::UnknownField {
                receiver_origin, ..
            } => Some(receiver_origin),

            Self::FormatError {
                receiver_origin, ..
            } => Some(receiver_origin),

            Self::UnknownPackage { .. } => None,

            Self::Interrupted { .. } => None,

            Self::StackOverflow { .. } => None,
        }
    }

    /// Returns an error message string related to the
    /// [primary_origin](Self::primary_origin).
    ///
    /// This function returns the same message string that you would get by
    /// formatting RuntimeError using the Display implementation.
    #[inline(always)]
    pub fn primary_description(&self) -> String {
        self.to_string()
    }

    /// Returns an error message string related to the
    /// [secondary_origin](Self::secondary_origin).
    ///
    /// If the error variant does not have an extra location, this function
    /// returns an empty string.
    pub fn secondary_description(&self) -> String {
        match self {
            Self::Nil { .. } => String::new(),

            Self::NonSingleton { .. } => String::new(),

            Self::ShortSlice { .. } => String::new(),

            Self::OutOfBounds { .. } => String::new(),

            Self::ReadOnly { .. } => String::from("data object origin"),

            Self::WriteOnly { .. } => String::from("data object origin"),

            Self::ReadToWrite { .. } => String::from("active read access"),

            Self::WriteToRead { .. } => String::from("active write access"),

            Self::WriteToWrite { .. } => String::from("active write access"),

            Self::Utf8Decoding { .. } => String::new(),

            Self::BorrowLimit { .. } => String::new(),

            Self::TypeMismatch { .. } => String::new(),

            Self::DowncastStatic { .. } => String::new(),

            Self::UpcastResult { .. } => String::new(),

            Self::NumberCast { .. } => String::new(),

            Self::NumericOperation { .. } => String::new(),

            Self::RangeCast { .. } => String::new(),

            Self::MalformedRange { .. } => String::new(),

            Self::PrimitiveParse { .. } => String::new(),

            Self::ArityMismatch { .. } => String::from("function origin"),

            Self::UndefinedOperator {
                receiver_origin, ..
            } if receiver_origin.is_some() => String::from("receiver origin"),

            Self::UndefinedOperator { .. } => String::new(),

            Self::UnknownField { .. } => String::from("receiver origin"),

            Self::FormatError { .. } => String::from("receiver object"),

            Self::UnknownPackage { .. } => String::new(),

            Self::Interrupted { .. } => String::new(),

            Self::StackOverflow { .. } => String::new(),
        }
    }

    /// Returns a detailed summary of this error.
    ///
    /// This message is the same one that would be printed in the footer of the
    /// [RuntimeError::display] object.
    pub fn summary(&self) -> String {
        let result = match self {
            Self::Nil { .. } => {
                r#"The requested operation has been applied on the void data.

The source of the void data could be:
    - an empty array "[]",
    - a struct field that does not exists in the struct,
    - a function or operator that does not return any value,
    - a function that returns "Option::None",
    - or any other source.

Use the ? operator to check if the value is void: "if a? {...}"."#
            }

            Self::NonSingleton { .. } => {
                r#"Most script operations require singleton objects (just normal objects)
and cannot be applied to arrays with zero or more than one element.

Consider using the index operator to retrieve a single element from the array:
    "my_array[3] + 10" instead of "my_array + 10"."#
            }

            Self::ShortSlice { .. } => {
                r#"The underlying operation requires an array of longer length."#
            }

            Self::OutOfBounds { .. } => {
                r#"The specified range or an index is out of the array bounds."#
            }

            Self::ReadOnly { .. } => {
                r#"The underlying operation requires write access to one of its arguments,
but the argument reference provides read-only access."#
            }

            Self::WriteOnly { .. } => {
                r#"The underlying operation requires read access to one of its arguments,
but the argument reference provides write-only access."#
            }

            Self::ReadToWrite { .. } => {
                r#"The underlying operation requires write access to one of its arguments,
but the argument object is currently being read.

The script engine does not allow simultaneous read and write access
to the same data.

For instance, if the script calls a function (or an operator) with this object
as an argument and the function returns a reference that indirectly points
to the argument's data, the object is blocked for writing until the reference's
lifetime ends."#
            }

            Self::WriteToRead { .. } => {
                r#"The underlying operation requires read access to one of its arguments,
but the argument object is currently being written.

The script engine does not allow simultaneous read and write access
to the same data.

For instance, if the script calls a function (or an operator) with this object
as an argument and the function returns a reference that indirectly modifies
argument's data, the object is blocked for reading until the reference's
lifetime ends."#
            }

            Self::WriteToWrite { .. } => {
                r#"The underlying operation requires write access to one of its arguments,
but the argument object is currently being written.

The script engine mandates that the ongoing write access to the data object
is exclusive.

For instance, if the script calls a function (or an operator) with this object
as an argument and the function returns a reference that indirectly modifies
argument's data, the object is blocked from another write access until
the reference's lifetime ends."#
            }

            Self::Utf8Decoding { cause, .. } => {
                let mut result = String::from(
                    r#"The underlying operation is attempting to reinterpret an array of bytes
as a UTF-8 encoding of the Unicode text, but the script engine has detected
that this encoding is invalid.

Error description:"#,
                );

                for line in cause.to_string().split("\n") {
                    result.push_str("\n    ");
                    result.push_str(line);
                }

                return result;
            }

            Self::BorrowLimit { .. } => {
                r#"The script engine has a predefined limit for active references
to the same data object.

This limit may be exceeded, for example, if a recursive function attempts
to access the same object too many times."#
            }

            Self::TypeMismatch { .. } => {
                r#"The underlying function (or operator) requires an argument of a different type
than the one being provided."#
            }

            Self::DowncastStatic { .. } => {
                r#"The underlying function (or operator) requested a data object
with a lifetime that may be different from the actual object's lifetime."#
            }

            Self::UpcastResult { cause, .. } => {
                let mut result = String::from(
                    r#"The invoked function (or operator) returned explicit error.

Error description:"#,
                );

                for line in cause.to_string().split("\n") {
                    result.push_str("\n    ");
                    result.push_str(line);
                }

                return result;
            }

            Self::NumberCast { cause, .. } => match cause {
                NumberCastCause::Infinite | NumberCastCause::NAN => {
                    r#"Failed to cast primitive numeric type to another numeric type."#
                }

                NumberCastCause::Overflow => {
                    r#"Failed to cast primitive numeric type to another numeric type.

The source value is bigger than the target type upper bound.
"#
                }

                NumberCastCause::Underflow => {
                    r#"Failed to cast primitive numeric type to another numeric type.

The source value is lesser than the target type lower bound.
"#
                }
            },

            Self::NumericOperation { target, .. } => {
                let mut result = String::from(
                    r#"Failed to perform numeric operation between two primitive types

The result overflows "#,
                );

                result.push_str(&format!("{target}"));

                result.push_str(" bounds.");

                return result;
            }

            Self::RangeCast { .. } => r#"Failed to cast a range to another range type."#,

            Self::MalformedRange { .. } => r#"Malformed range bounds."#,

            Self::PrimitiveParse { cause, .. } => {
                let mut result = String::from(r#"String parse error:"#);

                for line in cause.to_string().split("\n") {
                    result.push_str("\n    ");
                    result.push_str(line);
                }

                return result;
            }

            Self::ArityMismatch {
                parameters,
                arguments,
                ..
            } => match *parameters > *arguments {
                true => r#"Not enough arguments."#,
                false => r#"Too many arguments."#,
            },

            Self::UndefinedOperator { .. } => {
                r#"The object's type that is responsible to perform specified operation does not
implement this operator."#
            }

            Self::UnknownField { .. } => r#"The object does not have specified field."#,

            Self::FormatError { .. } => r#"Failed to turn the object into string representation."#,

            Self::UnknownPackage { .. } => r#"Package lookup failure."#,

            Self::Interrupted { .. } => r#"The script explicitly terminated by the host request."#,

            Self::StackOverflow { .. } => {
                r#"The script engine failed to invoke the function,
because the engine's thread stack exceeded its limit.
                
This situation may occur in functions with unlimited recursion."#
            }
        };

        String::from(result)
    }

    fn partition_types(types: &[&'static TypeMeta]) -> Vec<String> {
        let mut result = Vec::new();
        let type_metas = types.iter().copied().collect::<AHashSet<_>>();
        let mut type_families = AHashMap::new();

        for ty in type_metas {
            let family = ty.family();

            if family.is_fn() || family.len() <= 1 {
                result.push(format!("'{}'", ty.name()));
                continue;
            }

            match type_families.entry(family) {
                Entry::Vacant(entry) => {
                    let _ = entry.insert(AHashSet::from([ty]));
                }

                Entry::Occupied(mut entry) => {
                    let _ = entry.get_mut().insert(ty);
                }
            }
        }

        for (family, types) in type_families {
            if family.len() == types.len() {
                result.push(format!("'{}'", family.name()));
                continue;
            }

            for ty in types {
                result.push(format!("'{}'", ty.name()));
            }
        }

        result.sort();

        result
    }
}

/// A type of the [RuntimeError::NumberCast] error.
///
/// This object describes the reason why the source numeric value cannot be
/// converted into the destination numeric value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NumberCastCause {
    /// The target type does not support representation of infinite numbers.
    Infinite,

    /// The target type does not support representation of NaN numbers.
    NAN,

    /// The source numeric value is too large for the range of the target type.
    Overflow,

    /// The source numeric value is too small for the range of the target type.
    Underflow,
}

/// A type of the [RuntimeError::NumericOperation] error.
///
/// This object describes the type of operation that caused the error.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NumericOperationKind {
    /// The sum of two numbers: `10 + 20`.
    Add,

    /// The subtraction of two numbers: `10 - 20`.
    Sub,

    /// The multiplication of two numbers: `10 * 20`.
    Mul,

    /// The division of two numbers: `10 / 20`.
    Div,

    /// The negative value of a number: `-10`.
    Neg,

    /// A bitwise shift to the left: `10 << 20`.
    Shl,

    /// A bitwise shift to the right: `10 >> 20`.
    Shr,

    /// The remainder of division: `10 % 20`.
    Rem,
}

pub trait NumValue: Debug + Display + Send + Sync + 'static {}

impl<T: Debug + Display + Send + Sync + 'static> NumValue for T {}
