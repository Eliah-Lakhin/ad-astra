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

use std::ops::Deref;

use ahash::RandomState;
use lady_deirdre::{
    analysis::{
        AbstractTask,
        AnalysisTask,
        DocumentReadGuard,
        SemanticAccess,
        TaskHandle,
        TriggerHandle,
    },
    arena::{Id, Identifiable},
    lexis::ToSpan,
    sync::Shared,
    syntax::SyntaxTree,
};

use crate::{
    analysis::{
        symbols::{LookupOptions, ModuleSymbol, SymbolsLookup},
        DiagnosticsDepth,
        ModuleDiagnostics,
        ModuleError,
        ModuleResult,
        ModuleResultEx,
        ModuleText,
    },
    interpret::ScriptFn,
    report::system_panic,
    runtime::{PackageMeta, ScriptOrigin},
    syntax::{PolyRefOrigin, ScriptNode, SpanBounds},
};

/// An object that grants non-exclusive access to the
/// [ScriptModule](crate::analysis::ScriptModule) content.
///
/// Created by the [read](crate::analysis::ScriptModule::read) and
/// [try_read](crate::analysis::ScriptModule::try_read) functions.
///
/// Implements the [ModuleRead] trait, which provides content read functions.
pub struct ModuleReadGuard<'a, H: TaskHandle = TriggerHandle> {
    pub(super) id: Id,
    pub(super) package: &'static PackageMeta,
    pub(super) task: AnalysisTask<'a, ScriptNode, H, RandomState>,
}

impl<'a, H: TaskHandle> Identifiable for ModuleReadGuard<'a, H> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'a, H: TaskHandle> ModuleRead<H> for ModuleReadGuard<'a, H> {
    #[inline(always)]
    fn package(&self) -> &'static PackageMeta {
        self.package
    }
}

impl<'a, H: TaskHandle> ModuleReadSealed<H> for ModuleReadGuard<'a, H> {
    type Task = AnalysisTask<'a, ScriptNode, H, RandomState>;

    #[inline(always)]
    fn task(&self) -> &Self::Task {
        &self.task
    }
}

/// A set of read functions for the
/// [ScriptModule](crate::analysis::ScriptModule) content.
///
/// This trait is implemented by both the [ModuleReadGuard] and
/// [ModuleWriteGuard](crate::analysis::ModuleWriteGuard) objects. The trait is
/// sealed, meaning it cannot be implemented outside of this crate.
pub trait ModuleRead<H: TaskHandle>: Identifiable + ModuleReadSealed<H> {
    /// Returns the metadata object of the script package under which the
    /// underlying [ScriptModule](crate::analysis::ScriptModule) is being
    /// analyzed.
    ///
    /// This value is the same as the one provided to the module's
    /// [constructor](crate::analysis::ScriptModule::new) function.
    ///
    /// See [ScriptPackage](crate::runtime::ScriptPackage) for details.
    fn package(&self) -> &'static PackageMeta;

    /// Returns true if the underlying access guard has been revoked.
    ///
    /// If the function returns true, it indicates that the guard object needs
    /// to be dropped as soon as possible.
    #[inline(always)]
    fn is_interrupted(&self) -> bool {
        self.task().handle().is_triggered()
    }

    /// Gets access to the script module's source code text.
    ///
    /// See [ModuleText] for details.
    fn text(&self) -> ModuleText {
        ModuleText {
            package: self.package(),
            doc_read: self.read_doc(),
        }
    }

    /// Computes script module diagnostics (errors and warnings).
    ///
    /// The returned [ModuleDiagnostics] object is a collection of inferred
    /// issues that you can iterate through. Using this object, you can also
    /// print the source code text annotated with diagnostic messages to the
    /// terminal.
    ///
    /// The `depth` numeric argument specifies the level of diagnostic analysis
    /// depth. Available values are 1 (syntax errors), 2 (shallow semantic
    /// analysis), and 3 (deep semantic analysis). For details, see the
    /// [DiagnosticsDepth] documentation.
    fn diagnostics(&self, depth: DiagnosticsDepth) -> ModuleResult<ModuleDiagnostics> {
        let doc_read = self.read_doc();

        let ScriptNode::Root { semantics, .. } = doc_read.deref().root() else {
            system_panic!("Incorrect root variant.");
        };

        let id = self.id();

        let root_semantics = semantics.get().into_module_result(id)?;

        match depth {
            1 => {
                let (revision, snapshot) = root_semantics
                    .diagnostics_cross_1
                    .snapshot(self.task())
                    .into_module_result(id)?;

                Ok(ModuleDiagnostics {
                    id,
                    issues: snapshot.issues.clone(),
                    depth,
                    revision,
                })
            }

            2 => {
                let (revision, snapshot) = root_semantics
                    .diagnostics_cross_2
                    .snapshot(self.task())
                    .into_module_result(id)?;

                Ok(ModuleDiagnostics {
                    id,
                    issues: snapshot.issues.clone(),
                    depth,
                    revision,
                })
            }

            3 => {
                let (revision, snapshot) = root_semantics
                    .diagnostics_cross_3
                    .snapshot(self.task())
                    .into_module_result(id)?;

                Ok(ModuleDiagnostics {
                    id,
                    issues: snapshot.issues.clone(),
                    depth,
                    revision,
                })
            }

            _ => Ok(ModuleDiagnostics {
                id,
                issues: Shared::default(),
                depth,
                revision: 0,
            }),
        }
    }

    /// Looks up syntax constructions within the specified `span` (source code
    /// range) based on the `options` filter.
    ///
    /// This function allows you to find specific source code constructions
    /// (called "symbols") and then explore their syntax and semantics.
    ///
    /// For example, you can find a variable reference in the source code and
    /// then determine where this variable was introduced.
    ///
    /// The `span` argument specifies the source code range for the symbol
    /// lookup. You can use an absolute Unicode character range like `10..20`,
    /// a [line-column](lady_deirdre::lexis::Position) range like
    /// `Position::new(10, 3)..Position::new(12, 4)`, or a [ScriptOrigin]
    /// instance.
    ///
    /// The `options` argument specifies the lookup filter. The
    /// [LookupOptions::default] implementation searches for all kinds of
    /// language constructions within the specified span, but you can
    /// restrict the lookup to a particular set of symbol types.
    ///
    /// The function returns a [ModuleError::Cursor] error if the provided
    /// `span` is not [valid](ToSpan::is_valid_span) for this module.
    fn symbols(
        &self,
        span: impl ToSpan,
        options: LookupOptions,
    ) -> ModuleResult<Vec<ModuleSymbol>> {
        let doc_read = self.read_doc();

        let span = {
            let doc_read = self.read_doc();

            match span.to_site_span(doc_read.deref()) {
                Some(span) => span,
                None => return Err(ModuleError::Cursor(self.id())),
            }
        };

        Ok(SymbolsLookup::lookup(doc_read.deref(), span, options))
    }

    /// Returns a range of the source code without the header and footer
    /// comments.
    ///
    /// ```text
    /// // header comment
    ///
    /// <content range start>let x = 10;
    /// let y = 20;<content range end>
    ///
    /// // footer comment
    /// ```
    ///
    /// This range is useful, for example, to determine where to place a new
    /// global `use foo;` import statement in the source code.
    ///
    /// Note that the returned [ScriptOrigin] can be converted into an absolute
    /// range of Unicode characters using the
    /// [to_site_span](ToSpan::to_site_span) function:
    /// `script_origin.to_site_span(&module_guard.text())`.
    fn content_origin(&self) -> ScriptOrigin {
        let doc_read = self.read_doc();

        let ScriptNode::Root { statements, .. } = doc_read.root() else {
            system_panic!("Incorrect root variant.");
        };

        let (Some(first), Some(last)) = (statements.first(), statements.last()) else {
            return ScriptOrigin::eoi(self.id());
        };

        let mut start = first.script_origin(doc_read.deref(), SpanBounds::Header);
        let end = last.script_origin(doc_read.deref(), SpanBounds::Footer);

        start.union(&end);

        start
    }

    /// Compiles the source code into the Ad Astra assembly, making it available
    /// for execution. To execute the resulting ScriptFn object, use the
    /// [ScriptFn::run] function.
    ///
    /// Source code compilation is typically a fast and robust process.
    /// In general, the compiler can compile any source code text, even if it
    /// contains [diagnostic](Self::diagnostics) errors and warnings. However,
    /// if the code has diagnostic errors, the correctness of the resulting
    /// ScriptFn execution flow is not guaranteed. Running such a ScriptFn
    /// object does not result in undefined behavior, as Ad Astra's virtual
    /// machine fully controls the assembly execution. In the worst case,
    /// running such an assembly will result in a
    /// [RuntimeError](crate::runtime::RuntimeError).
    ///
    /// The compiler attempts to compile script code with diagnostic errors in a
    /// way that best matches the author's original intentions. However, it is
    /// recommended to avoid running ScriptFn objects in production that have
    /// been compiled from script modules with diagnostic errors.
    fn compile(&self) -> ModuleResult<ScriptFn> {
        let task = self.task();
        let doc_read = self.read_doc();

        ScriptFn::compile(task, doc_read.deref(), &doc_read.root_node_ref())
            .into_module_result(self.id())
    }
}

pub trait ModuleReadSealed<H: TaskHandle>: Identifiable {
    type Task: SemanticAccess<ScriptNode, H, RandomState>;

    fn task(&self) -> &Self::Task;

    #[track_caller]
    #[inline(always)]
    fn read_doc(&self) -> DocumentReadGuard<ScriptNode, RandomState> {
        let id = self.id();

        match self.task().read_doc(id) {
            Ok(doc_read) => doc_read,
            Err(error) => system_panic!("Analysis internal error. {error}",),
        }
    }
}
