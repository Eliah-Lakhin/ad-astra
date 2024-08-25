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
    error::Error,
    fmt::{Display, Formatter},
};

use lady_deirdre::{
    analysis::{AnalysisError, AnalysisResult},
    arena::{Id, Identifiable},
};

use crate::report::system_panic;

/// An alias type for analysis results.
pub type ModuleResult<T> = Result<T, ModuleError>;

/// An error type for script module analysis.
///
/// Some variants of this enum (such as [Cursor](ModuleError::Cursor)) represent
/// errors indicating that the arguments supplied to the function are not
/// valid. Other variants indicate that the result cannot be computed due to
/// specific reasons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ModuleError {
    /// Indicates that the function cannot fulfill the request because another
    /// thread is attempting to revoke the current read or write access to this
    /// script module in a multi-threaded environment.
    ///
    /// If you encounter this error in a worker thread, you should drop the
    /// module's [read](crate::analysis::ModuleReadGuard) or
    /// [write](crate::analysis::ModuleWriteGuard) access object to give
    /// priority to another thread. After dropping the access object, it is
    /// recommended to pause the worker thread for a short amount of time before
    /// retrying the operation by acquiring a new module access guard.
    ///
    /// In single-threaded programs, this error should never occur unless access
    /// handles are manually triggered.
    ///
    /// See the [ScriptModule](crate::analysis::ScriptModule) documentation for
    /// more details about multi-threaded analysis tools.
    Interrupted(Id),

    /// Indicates that the analysis operation cannot be completed within
    /// the predefined amount of time.
    ///
    /// This type of error is rare and may occur only in specific edge cases.
    /// The internal semantic analysis algorithm allocates generous timeout
    /// limits for semantic operations, which should be sufficient to handle a
    /// wide range of semantic requests in large source code texts, even on
    /// low-end machines. However, if a request operation takes too long, the
    /// analyzer may decide to give up and return a Timeout error.
    ///
    /// In such cases, rerunning the operation typically has no benefit, so you
    /// can either display an error message to the user or silently ignore the
    /// request. If implementing a hand-written LSP server, you may choose to
    /// return an empty response to the language client.
    Timeout(Id),

    /// Indicates that the addressed source code character or range of
    /// characters is not valid for the underlying script module.
    Cursor(Id),
}

impl Error for ModuleError {}

impl Identifiable for ModuleError {
    #[inline(always)]
    fn id(&self) -> Id {
        match self {
            Self::Interrupted(id) => *id,
            Self::Timeout(id) => *id,
            Self::Cursor(id) => *id,
        }
    }
}

impl Display for ModuleError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Interrupted(id) => formatter.write_fmt(format_args!(
                "Cannot complete module {id} analysis request because the \
                operation was interrupted.",
            )),

            Self::Timeout(id) => {
                formatter.write_fmt(format_args!("Module {id} analysis request timed out.",))
            }

            Self::Cursor(id) => formatter.write_fmt(format_args!(
                "The specified source code site or range of sites is not valid for module {id}.",
            )),
        }
    }
}

pub(crate) trait ModuleResultEx<T>: Sized {
    fn into_module_result(self, id: Id) -> ModuleResult<T>;

    fn forward(self) -> AnalysisResult<T>;
}

impl<T> ModuleResultEx<T> for AnalysisResult<T> {
    #[track_caller]
    #[inline(always)]
    fn into_module_result(self, id: Id) -> ModuleResult<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(error) => match error {
                AnalysisError::Interrupted => Err(ModuleError::Interrupted(id)),
                AnalysisError::Timeout if cfg!(not(debug_assertions)) => {
                    Err(ModuleError::Timeout(id))
                }
                _ => system_panic!("Analysis internal error. {error}",),
            },
        }
    }

    #[track_caller]
    #[inline(always)]
    fn forward(self) -> AnalysisResult<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(error) if !error.is_abnormal() => Err(error),
            Err(error) => system_panic!("Analysis internal error. {error}",),
        }
    }
}
