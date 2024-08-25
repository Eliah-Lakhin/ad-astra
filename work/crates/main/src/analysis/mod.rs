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

mod closeness;
mod completions;
mod description;
mod diagnostics;
mod error;
mod issues;
mod module;
mod read;
mod text;
mod write;

/// A set of script language constructions that can be retrieved from the
/// [symbols](ModuleRead::symbols) function. It also includes the semantic
/// analysis queries you can use to further explore the script's syntax and
/// semantics.
///
/// The entry-point type for exploring this module is the
/// [ModuleSymbol](symbols::ModuleSymbol) enum.
pub mod symbols;

pub use crate::analysis::{
    closeness::{Closeness, StringEstimation},
    completions::{CompletionItem, CompletionScope, Completions},
    description::Description,
    diagnostics::{
        DiagnosticsDepth,
        DiagnosticsIter,
        IssueQuickfix,
        ModuleDiagnostics,
        ModuleIssue,
    },
    error::{ModuleError, ModuleResult},
    issues::{IssueCode, IssueSeverity},
    module::ScriptModule,
    read::{ModuleRead, ModuleReadGuard},
    text::{ModuleText, ModuleTextResolver},
    write::{ModuleWrite, ModuleWriteGuard},
};
pub(crate) use crate::analysis::{error::ModuleResultEx, issues::ScriptIssue};
