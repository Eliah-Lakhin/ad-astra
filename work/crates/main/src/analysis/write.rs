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
    analysis::{ExclusiveTask, MutationAccess, TaskHandle, TriggerHandle},
    arena::{Id, Identifiable},
    lexis::{ToSite, ToSpan},
};

use crate::{
    analysis::{
        completions::PROMPT_STRING,
        read::ModuleReadSealed,
        Completions,
        ModuleError,
        ModuleRead,
        ModuleResult,
        ModuleResultEx,
    },
    runtime::PackageMeta,
    syntax::ScriptNode,
};

/// An object that grants exclusive access to the
/// [ScriptModule](crate::analysis::ScriptModule) content.
///
/// Created by the [write](crate::analysis::ScriptModule::write) and
/// [try_write](crate::analysis::ScriptModule::try_write) functions.
///
/// Implements the [ModuleRead] trait, which provides content read functions,
/// and the [ModuleWrite] trait, which provides content write functions and
/// read functions that require exclusive access.
pub struct ModuleWriteGuard<'a, H: TaskHandle = TriggerHandle> {
    pub(super) id: Id,
    pub(super) package: &'static PackageMeta,
    pub(super) task: ExclusiveTask<'a, ScriptNode, H, RandomState>,
}

impl<'a, H: TaskHandle> Identifiable for ModuleWriteGuard<'a, H> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'a, H: TaskHandle> ModuleRead<H> for ModuleWriteGuard<'a, H> {
    #[inline(always)]
    fn package(&self) -> &'static PackageMeta {
        self.package
    }
}

impl<'a, H: TaskHandle> ModuleWrite<H> for ModuleWriteGuard<'a, H> {}

impl<'a, H: TaskHandle> ModuleReadSealed<H> for ModuleWriteGuard<'a, H> {
    type Task = ExclusiveTask<'a, ScriptNode, H, RandomState>;

    #[inline(always)]
    fn task(&self) -> &Self::Task {
        &self.task
    }
}

impl<'a, H: TaskHandle> ModuleWriteSealed<H> for ModuleWriteGuard<'a, H> {
    #[inline(always)]
    fn task_mut(&mut self) -> &mut Self::Task {
        &mut self.task
    }
}

/// A set of write functions and exclusive read functions for the
/// [ScriptModule](crate::analysis::ScriptModule) content.
///
/// This trait is implemented by the [ModuleWriteGuard] object. The trait is
/// sealed, meaning it cannot be implemented outside of this crate.
pub trait ModuleWrite<H: TaskHandle>: ModuleRead<H> + ModuleWriteSealed<H>
where
    Self::Task: MutationAccess<ScriptNode, H, RandomState>,
{
    /// Mutates the source code text of the script module.
    ///
    /// The `span` argument specifies the source code range that you want to
    /// rewrite. It can be an absolute Unicode character range, such as `10..20`,
    /// a [line-column](lady_deirdre::lexis::Position) range like
    /// `Position::new(10, 3)..Position::new(12, 4)`, or a
    /// [ScriptOrigin](crate::runtime::ScriptOrigin) instance.
    ///
    /// The `text` argument specifies the string you want to insert in place
    /// of the spanned range. It can be an empty string if you want to erase
    /// a fragment of the source code.
    ///
    /// The underlying algorithm incrementally patches the internal
    /// representation of the script module, localized to the spanned fragment.
    /// In most cases, this process is quite fast, even with large source code.
    /// Therefore, it is acceptable to call this function on each end-user
    /// keystroke.
    ///
    /// The function returns a [ModuleError::Cursor] error if the provided
    /// `span` is not [valid](ToSpan::is_valid_span) for this module.
    fn edit(&mut self, span: impl ToSpan, text: impl AsRef<str>) -> ModuleResult<()> {
        let id = self.id();

        let span = {
            let doc_read = self.read_doc();

            match span.to_site_span(doc_read.deref()) {
                Some(span) => span,
                None => return Err(ModuleError::Cursor(id)),
            }
        };

        self.task_mut()
            .write_to_doc(id, span, text)
            .into_module_result(id)
    }

    /// Returns a [Completions] description object that describes potential
    /// completions for the script module's source code at the specified
    /// `site` position.
    ///
    /// The `site` argument specifies the location of the end-user cursor. It
    /// can be an absolute Unicode character offset, such as `10`, or a
    /// [line-column](lady_deirdre::lexis::Position) offset.
    ///
    /// The function returns a [ModuleError::Cursor] error if the provided
    /// `site` is not [valid](ToSite::is_valid_site) for this module.
    fn completions(&mut self, site: impl ToSite) -> ModuleResult<Completions> {
        let id = self.id();

        let site = {
            let doc_read = self.read_doc();

            match site.to_site(doc_read.deref()) {
                Some(site) => site,
                None => return Err(ModuleError::Cursor(id)),
            }
        };

        let _ = self
            .task_mut()
            .write_to_doc(id, site..site, PROMPT_STRING)
            .into_module_result(id)?;

        let task = self.task();
        let result = Completions::analyze(id, site, task).forward();

        let _ = self
            .task_mut()
            .write_to_doc(id, site..(site + PROMPT_STRING.len()), "")
            .into_module_result(id)?;

        result.into_module_result(id)
    }
}

pub trait ModuleWriteSealed<H: TaskHandle>: ModuleReadSealed<H>
where
    <Self as ModuleReadSealed<H>>::Task: MutationAccess<ScriptNode, H, RandomState>,
{
    fn task_mut(&mut self) -> &mut Self::Task;
}
