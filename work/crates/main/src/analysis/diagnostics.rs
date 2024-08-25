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

use std::{collections::hash_set::Iter, iter::FusedIterator, ops::Deref};

use ahash::AHashSet;
use lady_deirdre::{
    analysis::Revision,
    arena::{Id, Identifiable},
    sync::Shared,
};

use crate::{
    analysis::{IssueCode, IssueSeverity, ModuleText, ScriptIssue},
    format::ScriptSnippet,
    runtime::ScriptOrigin,
};

/// A level indicating the depth of diagnostic analysis.
///
/// Each level represents an independent collection of issues related to the
/// script module. Currently, Ad Astra supports the following levels:
///
/// - Level `1`: All syntax parse errors. If the script module has no errors
///   at this level, the source code is syntactically well-formed. However, it
///   may still contain semantic errors or warnings.
/// - Level `2`: All semantic errors and warnings that can be inferred directly
///   from the local syntax context, without requiring deep source code
///   analysis.
/// - Level `3`: All warnings that require deep semantic analysis of the
///   interconnections between source code constructs.
///
/// Issues at lower diagnostic levels are more critical for the end user,
/// while issues at higher levels require more computational resources to infer.
///
/// Therefore, if a script module contains syntax errors (level `1`), you may
/// choose to display only these errors in the terminal, postponing deeper
/// diagnostic analysis until the end user addresses the issues at the lower
/// levels.

pub type DiagnosticsDepth = u8;

/// A collection of diagnostic issues (errors and warnings) in the script
/// module's source code.
///
/// Created by the [diagnostics](crate::analysis::ModuleRead::diagnostics)
/// function.
#[derive(Clone)]
pub struct ModuleDiagnostics {
    pub(super) id: Id,
    pub(super) issues: Shared<AHashSet<ScriptIssue>>,
    pub(super) depth: DiagnosticsDepth,
    pub(super) revision: Revision,
}

impl Identifiable for ModuleDiagnostics {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'a> IntoIterator for &'a ModuleDiagnostics {
    type Item = ModuleIssue<'a>;
    type IntoIter = DiagnosticsIter<'a>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl ModuleDiagnostics {
    /// Returns the number of issues in this collection that match the specified
    /// issue mask (`severity_mask`).
    ///
    /// For example, `len(IssueSeverity::Error as u8)` returns the number of
    /// errors, while `len(!0)` returns the total number of issues, including
    /// both errors and warnings.
    #[inline(always)]
    pub fn len(&self, severity_mask: u8) -> usize {
        let errors = severity_mask & (IssueSeverity::Error as u8) > 0;
        let warnings = severity_mask & (IssueSeverity::Warning as u8) > 0;

        match (errors, warnings) {
            (false, false) => 0,

            (true, false) => self
                .iter()
                .filter(|error| error.severity() == IssueSeverity::Error)
                .count(),

            (false, true) => self
                .iter()
                .filter(|error| error.severity() == IssueSeverity::Warning)
                .count(),

            (true, true) => self.issues.as_ref().len(),
        }
    }

    /// Returns true if this collection does not contain any diagnostic issues.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.issues.as_ref().is_empty()
    }

    /// Returns the diagnostic analysis depth at which this collection was
    /// constructed.
    ///
    /// See [DiagnosticsDepth] for details.
    #[inline(always)]
    pub fn depth(&self) -> DiagnosticsDepth {
        self.depth
    }

    /// Returns the revision number at which this collection was constructed.
    ///
    /// For a specific script module instance and [DiagnosticsDepth], this
    /// number always increases and never decreases.
    ///
    /// If two collections (of the same script module and the same diagnostic
    /// depth) have the same revision number, their content can be considered
    /// identical.
    ///
    /// However, if one collection has a higher revision number than the
    /// previous one, it indicates that the diagnostics at this level of depth
    /// have been updated.
    #[inline(always)]
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Returns an iterator that yields references to each diagnostic issue
    /// (error or warning) in this diagnostics collection.
    ///
    /// The issues are returned in an unspecified order. If you want to print
    /// each issue manually, you may consider sorting them (e.g., by issue type
    /// or by their position in the source code).
    #[inline(always)]
    pub fn iter(&self) -> DiagnosticsIter {
        DiagnosticsIter {
            id: self.id,
            inner: self.issues.as_ref().iter(),
        }
    }

    /// Returns a [script snippet](ScriptSnippet) that highlights source code
    /// fragments associated with the underlying issues and annotates them with
    /// diagnostic messages.
    ///
    /// This function provides an easy way to print all diagnostic issues at
    /// once to the terminal.
    ///
    /// To construct the returned snippet object, you need access to the
    /// module's text, which can be obtained using the
    /// [text](crate::analysis::ModuleRead::text) function.
    ///
    /// The `severity_mask` allows you to filter issues by their severity:
    /// `IssueSeverity::Error as u8` shows only error issues, while `!0` shows
    /// both error and warning issues.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use ad_astra::{
    /// #     analysis::{ModuleRead, ScriptModule},
    /// #     export,
    /// #     lady_deirdre::analysis::TriggerHandle,
    /// #     runtime::ScriptPackage,
    /// # };
    /// #
    /// # #[export(package)]
    /// # #[derive(Default)]
    /// # struct Package;
    /// #
    /// let module = ScriptModule::new(Package::meta(), "let foo = ; let = 10;");
    /// module.rename("my_module.adastra");
    ///
    /// let handle = TriggerHandle::new();
    /// let module_read = module.read(&handle, 1).unwrap();
    ///
    /// let diagnostics = module_read.diagnostics(1).unwrap();
    /// let text = module_read.text();
    ///
    /// println!("{}", diagnostics.highlight(&text, !0));
    /// ```
    ///
    /// Outputs:
    ///
    /// ```text
    ///    ╭──╢ diagnostics [‹doctest›.‹my_module.adastra›] ╟──────────────────────────╮
    ///  1 │ let foo = ; let = 10;                                                     │
    ///    │          │     ╰╴ missing var name in 'let <var> = <expr>;'               │
    ///    │          ╰╴ missing expression in 'let <var> = <expr>;'                   │
    ///    ├───────────────────────────────────────────────────────────────────────────┤
    ///    │ Errors: 2                                                                 │
    ///    │ Warnings: 0                                                               │
    ///    ╰───────────────────────────────────────────────────────────────────────────╯
    /// ```
    pub fn highlight<'a>(&self, text: &'a ModuleText, severity_mask: u8) -> ScriptSnippet<'a> {
        let mut snippet = text.snippet();

        snippet.set_caption("diagnostics");

        let include_errors = severity_mask & (IssueSeverity::Error as u8) > 0;
        let include_warnings = severity_mask & (IssueSeverity::Warning as u8) > 0;

        let mut total_errors = 0;
        let mut total_warnings = 0;
        #[allow(unused)]
        let mut annotations = 0;

        for issue in self.iter() {
            match issue.severity() {
                IssueSeverity::Error => {
                    total_errors += 1;

                    if !include_errors {
                        continue;
                    }
                }

                IssueSeverity::Warning => {
                    total_warnings += 1;

                    if !include_warnings {
                        continue;
                    }
                }
            }

            annotations += 1;

            snippet.annotate(
                issue.origin(text),
                issue.severity().priority(),
                issue.verbose_message(text),
            );
        }

        let mut summary = String::with_capacity(1024);

        match total_errors == 0 && total_warnings == 0 {
            true => summary.push_str("No issues detected."),

            false => {
                summary.push_str(&format!("Errors: {}", total_errors));

                if !include_errors {
                    summary.push_str(" (omitted).");
                }

                summary.push('\n');

                summary.push_str(&format!("Warnings: {}", total_warnings));

                if !include_warnings {
                    summary.push_str(" (omitted).");
                }
            }
        };

        snippet.set_summary(summary);

        snippet
    }
}

/// An iterator over the diagnostic issues in the [ModuleDiagnostics]
/// collection.
///
/// Created by the [ModuleDiagnostics::iter] function and the [IntoIterator]
/// implementation of the ModuleDiagnostics.
pub struct DiagnosticsIter<'a> {
    id: Id,
    inner: Iter<'a, ScriptIssue>,
}

impl<'a> Iterator for DiagnosticsIter<'a> {
    type Item = ModuleIssue<'a>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let issue = self.inner.next()?;

        Some(ModuleIssue { id: self.id, issue })
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> FusedIterator for DiagnosticsIter<'a> {}

impl<'a> ExactSizeIterator for DiagnosticsIter<'a> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

/// An individual diagnostic issue in the [ModuleDiagnostics] collection.
///
/// Created by the [DiagnosticsIter] iterator, it represents a view into the
/// issue object owned by the collection.
pub struct ModuleIssue<'a> {
    id: Id,
    issue: &'a ScriptIssue,
}

impl<'a> Identifiable for ModuleIssue<'a> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'a> ModuleIssue<'a> {
    /// Returns an [IssueCode] object that describes the class of issues to
    /// which this issue belongs.
    ///
    /// From this IssueCode object, you can obtain additional metadata about
    /// this class, or convert the class into a numeric code value.
    #[inline(always)]
    pub fn code(&self) -> IssueCode {
        self.issue.code()
    }

    /// Indicates whether this issue is a hard error or a warning.
    ///
    /// Equivalent to `issue.code().severity()`.
    #[inline(always)]
    pub fn severity(&self) -> IssueSeverity {
        self.code().severity()
    }

    /// Returns a short description of the class of issues to which this issue
    /// belongs.
    ///
    /// Equivalent to `issue.code().to_string()`.
    #[inline(always)]
    pub fn short_message(&self) -> String {
        self.code().to_string()
    }

    /// Returns an issue-specific text message.
    ///
    /// Unlike [ModuleIssue::short_message], this message considers the full
    /// context of the issue and its relation to the source code. Both
    /// [ModuleDiagnostics::highlight] and [ModuleIssue::highlight] use this
    /// text when annotating the source code snippet.
    #[inline(always)]
    pub fn verbose_message(&self, text: &ModuleText) -> String {
        self.issue.message(text.doc_read.deref()).to_string()
    }

    /// Returns a reference to the source code fragment where the issue appears.
    ///
    /// You can use the
    /// [to_site_span](lady_deirdre::lexis::ToSpan::to_site_span) function to
    /// convert the returned object into an absolute character index range, or
    /// [to_position_span](lady_deirdre::lexis::ToSpan::to_position_span) to
    /// convert it into a line-column range.
    #[inline(always)]
    pub fn origin(&self, text: &ModuleText) -> ScriptOrigin {
        self.issue.span(text.doc_read.deref())
    }

    /// Returns a quick-fix suggestion that could potentially resolve the
    /// underlying issue.
    ///
    /// Some diagnostic issues can be addressed directly based on heuristic
    /// assumptions. For example, if the user misspells a variable name, the
    /// [IssueQuickfix] might suggest a replacement for the identifier.
    ///
    /// If the function returns `None`, it means there is no quick fix for this
    /// issue that can currently be inferred heuristically. Future versions of
    /// Ad Astra may provide more quick-fix options.
    ///
    /// This function is primarily useful for code editor extensions, such as
    /// refactoring/quick-fix actions.
    pub fn quickfix(&self) -> Option<IssueQuickfix> {
        match self.issue {
            ScriptIssue::UnresolvedPackage { quickfix, .. } if !quickfix.is_empty() => {
                Some(IssueQuickfix {
                    set_text_to_origin: Some(quickfix.to_string()),
                    implement_use_of: None,
                })
            }

            ScriptIssue::UnresolvedIdent {
                quickfix, import, ..
            } if !quickfix.is_empty() || !import.is_empty() => Some(IssueQuickfix {
                set_text_to_origin: (!quickfix.is_empty()).then(|| quickfix.to_string()),
                implement_use_of: (!import.is_empty()).then(|| import.to_string()),
            }),

            ScriptIssue::UnknownComponent { quickfix, .. } if !quickfix.is_empty() => {
                Some(IssueQuickfix {
                    set_text_to_origin: Some(quickfix.to_string()),
                    implement_use_of: None,
                })
            }

            _ => None,
        }
    }

    /// Returns a [script snippet](ScriptSnippet) that highlights a source code
    /// fragment related to the underlying issue, annotating it with the
    /// diagnostic message.
    ///
    /// This function is similar to [ModuleDiagnostics::highlight], but it
    /// highlights only a single issue and does not include a footer with
    /// summary metadata.
    #[inline(always)]
    pub fn highlight<'b>(&self, text: &'b ModuleText) -> ScriptSnippet<'b> {
        let mut snippet = text.snippet();

        snippet.set_caption(self.severity().to_string()).annotate(
            self.origin(text),
            self.severity().priority(),
            self.verbose_message(text),
        );

        snippet
    }

    /// Returns a numeric value indicating the level of diagnostic analysis
    /// depth to which this issue belongs.
    ///
    /// Equivalent to `issue.code().depth()`.
    ///
    /// See [DiagnosticsDepth] for more details.
    #[inline(always)]
    pub fn depth(&self) -> DiagnosticsDepth {
        self.code().depth()
    }
}

/// A heuristic suggestion that could potentially fix a module diagnostics
/// issue.
///
/// Created by the [ModuleIssue::quickfix] function (see the function's
/// documentation for details).
///
/// To fully implement this suggestion, each struct field should be addressed.
#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct IssueQuickfix {
    /// To resolve this issue, the [ModuleIssue::origin] fragment of the source
    /// code must be replaced with this text.
    pub set_text_to_origin: Option<String>,

    /// To resolve this issue, the `use <implement_use_of>;` import statement
    /// must be added to the source code.
    pub implement_use_of: Option<String>,
}
