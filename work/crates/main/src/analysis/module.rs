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

use std::fmt::{Debug, Display, Formatter};

use ahash::RandomState;
use lady_deirdre::{
    analysis::{Analyzer, AnalyzerConfig, MutationAccess, TaskHandle, TaskPriority, TriggerHandle},
    arena::{Id, Identifiable},
};

use crate::{
    analysis::{ModuleReadGuard, ModuleResult, ModuleResultEx, ModuleWriteGuard},
    format::format_script_path,
    report::system_panic,
    runtime::PackageMeta,
    syntax::ScriptNode,
};

/// An in-memory representation of the Ad Astra script module.
///
/// This object owns the script's source code text, its syntax and semantics,
/// and is responsible for keeping this data in sync with source code edits,
/// ensuring the up-to-date semantics are available for query.
///
/// To execute the script's source code, you need to load it into the
/// ScriptModule object, compile it, and then run the compiled assembly.
///
/// ## Creation
///
/// To create a ScriptModule, you can load the source code text, for example,
/// from disk, and then pass it into the ScriptModule constructor:
/// [ScriptModule::new].
///
/// The constructor requires an additional parameter, which is a package
/// metadata object. The module will be analyzed under the Rust symbols exported
/// into this package object. For more details, see the
/// [ScriptPackage](crate::runtime::ScriptPackage) documentation.
///
/// ```rust
/// # use ad_astra::{
/// #     analysis::ScriptModule, export, lady_deirdre::analysis::TriggerHandle,
/// #     runtime::ScriptPackage,
/// # };
/// #
/// #[export(package)]
/// #[derive(Default)]
/// struct Package;
///
/// let _module = ScriptModule::<TriggerHandle>::new(
///     Package::meta(),
///     "let foo = 10;",
/// );
/// ```
///
/// ## Access
///
/// The ScriptModule is specifically designed for use in multi-threaded
/// environments. Although multi-threading is not a strict requirement, and
/// the ScriptModule can also be used in single-threaded applications, its
/// access API follows the read-write lock design pattern to address
/// concurrent access operations.
///
/// You access the ScriptModule's content using read and write access guards,
/// similar to [RwLock](std::sync::RwLock). The
/// [read](ScriptModule::read) and [write](ScriptModule::write) functions
/// provide read and write access guards, respectively. Both functions may block
/// if the ScriptModule is currently locked for the opposite type of access,
/// though non-blocking "try_" variants are available. Like RwLock, you can
/// have multiple read guards simultaneously, but at most one write guard.
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
/// // Module creation
/// let module = ScriptModule::new(Package::meta(), "let foo = 10;");
///
/// let handle = TriggerHandle::new();
/// let module_read = module.read(&handle, 1).unwrap(); // Acquiring read guard.
///
/// println!("{}", module_read.text()); // Prints module source code.
/// ```
///
/// ## Available Operations
///
/// The [ModuleReadGuard] object, created by the [read](ScriptModule::read)
/// function, implements the [ModuleRead](crate::analysis::ModuleRead) trait,
/// which provides the following operations:
///
/// - Reading the source code text via the
///   [text](crate::analysis::ModuleRead::text) function.
/// - Requesting source code diagnostics (errors and warnings) via the
///   [diagnostics](crate::analysis::ModuleRead::diagnostics) function.
/// - Querying for semantic metadata about specific syntax constructs within
///   specified source code ranges via the
///   [symbols](crate::analysis::ModuleRead::symbols) function.
/// - Compiling the module into Ad Astra assembly for execution via the
///   [compile](crate::analysis::ModuleRead::compile) function.
///
/// The [ModuleWriteGuard] object, created by the [write](ScriptModule::write)
/// function, represents exclusive access to the ScriptModule content. This
/// object implements both the ModuleRead and
/// [ModuleWrite](crate::analysis::ModuleWrite) traits. Through the ModuleRead
/// trait, you gain access to the operations listed above, and through the
/// ModuleWrite trait, you can perform content mutation operations:
///
/// - Editing the source code text within a specified range via the
///   [edit](crate::analysis::ModuleWrite::edit) function.
/// - Probing the source code for code-completion candidates via the
///   [completions](crate::analysis::ModuleWrite::completions) function. Even
///   though this function does not ultimately change the source code text, it
///   requires write access to probe the code through temporary mutation.
///
/// ## Multi-Threaded Design
///
/// A key difference from RwLock is that the ScriptModule's access
/// guards can be gracefully interrupted.
///
/// Both [read](ScriptModule::read) and [write](ScriptModule::write) access
/// functions (including their "try_" variants) require two additional
/// parameters: an access priority number and a handle object.
///
/// The handle object allows you to revoke previously granted read/write access
/// from another thread. The priority number indicates the priority of the task
/// you intend to perform with the access guard object.
///
/// For example, if several working threads are currently reading the
/// ScriptModule with one priority number, and another working thread
/// simultaneously attempts to acquire write access with a higher priority
/// number, the ScriptModule automatically revokes all read access grants to
/// prioritize the write access.
///
/// When the ScriptModule revokes an access grant, all guard access operations
/// will start yielding an
/// [Interrupted](crate::analysis::ModuleError::Interrupted) error. In this
/// case, the thread owning the guard should drop the guard object as soon as
/// possible to allow another working thread to proceed. The former thread can
/// later acquire a new access guard to continue its work.
///
/// ```rust
/// # use ad_astra::{
/// #     analysis::{ModuleError, ModuleRead, ScriptModule},
/// #     export,
/// #     lady_deirdre::analysis::{TaskHandle, TriggerHandle},
/// #     runtime::ScriptPackage,
/// # };
/// #
/// # #[export(package)]
/// # #[derive(Default)]
/// # struct Package;
/// #
/// let module = ScriptModule::new(Package::meta(), "let foo = 10;");
///
/// let handle = TriggerHandle::new();
/// let module_read = module.read(&handle, 1).unwrap(); // Acquiring read access.
///
/// // Revoking access manually.
/// // In a multi-threaded environment, you can clone and move this `handle`
/// // object into another working thread and trigger it there instead.
/// handle.trigger();
///
/// // Since the read access has been revoked, the diagnostics request function
/// // returns an Interrupted error.
/// assert!(matches!(
///     module_read.diagnostics(2),
///     Err(ModuleError::Interrupted(_)),
/// ));
/// ```
///
/// Although Ad Astra does not have a built-in worker manager and does not spawn
/// any threads, the above mechanism helps you organize highly concurrent
/// multi-threaded analysis tools with task priorities.
///
/// Note that analysis read operations (such as
/// [diagnostics](crate::analysis::ModuleRead::diagnostics) or
/// [symbols](crate::analysis::ModuleRead::symbols)) typically don't block each
/// other when requested from independent threads. Ad Astra's semantic analyzer
/// can infer module semantics concurrently.
///
/// ## Incremental Analysis
///
/// When you [edit](crate::analysis::ModuleWrite::edit) the source code of the
/// ScriptModule, the underlying algorithm does not reparse the entire module's
/// syntax. Instead, it typically reparses only a small fragment that includes
/// the edited text and updates the existing in-memory data structures. This
/// technique, known as incremental reparsing, allows for quick updates to
/// script modules with every keystroke, even when the source code text is
/// large.
///
/// Additionally, semantic analysis is demand-driven. The ScriptModule does not
/// compute the script's semantics until specific semantic facts are queried.
/// When these facts are queried, the underlying algorithm attempts to compute
/// (or update previously computed) the smallest subset of the inner semantic
/// representation required to fulfill the request. Thus, semantic analysis
/// is also incremental and usually localized to the specific query.
///
/// ## Identification
///
/// Each instance of the ScriptModule has a globally unique associated
/// identifier ([Id]). This Id object is Copy, Eq, Ord, and Hash, and is unique
/// per ScriptModule instance within the current process.
///
/// Most API objects related to script modules also expose their script module
/// ids. These identifiers can be retrieved using the [Identifiable::id]
/// function and compared for equality.
///
/// Additionally, in multi-script projects, you can use the identifier as a key
/// type in a hash map to store multiple ScriptModule instances within a single
/// hash map.
///
/// ## Naming
///
/// The API allows you to assign a potentially non-unique string name to a
/// ScriptModule instance using the [ScriptModule::rename] function. For
/// example, if you load a script from disk, you might consider assigning
/// the file name to the ScriptModule object as a module name.
///
/// API functions that print a module's content to the terminal will use the
/// assigned name of the ScriptModule as a content header, which helps
/// simplify script identification.
pub struct ScriptModule<H: TaskHandle = TriggerHandle> {
    id: Id,
    package: &'static PackageMeta,
    analyzer: Analyzer<ScriptNode, H, RandomState>,
}

impl<H: TaskHandle> Drop for ScriptModule<H> {
    fn drop(&mut self) {
        // Safety: Module was attached during creation.
        unsafe { self.package.detach_module(self.id) }
    }
}

impl<H: TaskHandle> Debug for ScriptModule<H> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_fmt(format_args!(
            "ScriptModule({})",
            format_script_path(self.id, Some(self.package))
        ))
    }
}

impl<H: TaskHandle> Display for ScriptModule<H> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&format_script_path(self.id, Some(self.package)))
    }
}

impl<H: TaskHandle> PartialEq for ScriptModule<H> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl<H: TaskHandle> Identifiable for ScriptModule<H> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<H: TaskHandle> ScriptModule<H> {
    /// Constructs a ScriptObject.
    ///
    /// The `package` argument specifies the package under which the source code
    /// will be analyzed (see [ScriptPackage](crate::runtime::ScriptPackage) for
    /// details).
    ///
    /// The `text` argument is the source code of the script.
    pub fn new(package: &'static PackageMeta, text: impl AsRef<str>) -> Self {
        let mut config = AnalyzerConfig::default();

        config.single_document = true;

        let analyzer = Analyzer::new(config);

        let id = {
            let handle = H::default();

            let mut task = match analyzer.mutate(&handle, 0) {
                Ok(task) => task,
                Err(error) => system_panic!("Script creation failure. {error}",),
            };

            task.add_mutable_doc(text)
        };

        // Safety: Ids are globally unique.
        unsafe { package.attach_module(id) };

        Self {
            id,
            package,
            analyzer,
        }
    }

    /// Returns the metadata object of the script package under which this
    /// script module is being analyzed.
    ///
    /// This value is equal to the one provided to the [constructor](Self::new)
    /// function.
    ///
    /// See [ScriptPackage](crate::runtime::ScriptPackage) for details.
    #[inline(always)]
    pub fn package(&self) -> &'static PackageMeta {
        self.package
    }

    /// Sets the user-facing string name of the script module.
    ///
    /// This name will be used by the crate API as a header for script snippets
    /// when they are printed to the terminal. For example, if you print module
    /// diagnostics using the
    /// [ModuleDiagnostics::highlight](crate::analysis::ModuleDiagnostics::highlight)
    /// function, the snippet printer will use the name specified by this
    /// function.
    ///
    /// For instance, you can use the file name as the module name if the
    /// script's source code was loaded from disk.
    ///
    /// Unlike the module's [Id], which is globally unique per ScriptModule
    /// instance, the string name is not required to be unique (although it is
    /// generally preferable).
    ///
    /// To get a copy of the name set previously, use the [Id::name] function:
    /// `module.id().name()`.
    ///
    /// To unset the name, you can supply an empty string to this function.
    /// By default, script modules do not have names (their names are empty
    /// strings).
    #[inline(always)]
    pub fn rename(&self, name: impl AsRef<str>) {
        self.id.set_name(String::from(name.as_ref()))
    }

    /// Requests access for [read operations](ScriptModule#available-operations).
    ///
    /// This function may block the current thread if read access cannot be
    /// granted instantly. It may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// read operation cannot be granted, for example, if another thread
    /// simultaneously attempts to acquire write access with a higher priority.
    ///
    /// The `handle` argument specifies a reference to the handle object
    /// through which granted access can be manually revoked from another
    /// thread.
    ///
    /// The `priority` argument specifies the grant priority. If there are
    /// conflicting grants with a lower priority, they will be revoked.
    #[inline(always)]
    pub fn read<'a>(
        &'a self,
        handle: &'a H,
        priority: TaskPriority,
    ) -> ModuleResult<ModuleReadGuard<H>> {
        let task = self
            .analyzer
            .analyze(handle, priority)
            .into_module_result(self.id)?;

        Ok(ModuleReadGuard {
            id: self.id,
            package: self.package,
            task,
        })
    }

    /// A non-blocking alternative to the [read](Self::read) function. If read
    /// access cannot be granted instantly, this function will not block the
    /// current thread. Instead, it will immediately return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error.
    #[inline(always)]
    pub fn try_read<'a>(
        &'a self,
        handle: &'a H,
        priority: TaskPriority,
    ) -> ModuleResult<ModuleReadGuard<H>> {
        let task = self
            .analyzer
            .try_analyze(handle, priority)
            .into_module_result(self.id)?;

        Ok(ModuleReadGuard {
            id: self.id,
            package: self.package,
            task,
        })
    }

    /// Requests access for [write operations](ScriptModule#available-operations).
    ///
    /// This function may block the current thread if write access cannot be
    /// granted instantly. It may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// write operation cannot be granted, for example, if another thread
    /// simultaneously attempts to acquire read or write access with a higher
    /// priority.
    ///
    /// The `handle` argument specifies a reference to the handle object
    /// through which granted access can be manually revoked from another
    /// thread.
    ///
    /// The `priority` argument specifies the grant priority. If there are
    /// conflicting grants with a lower priority, they will be revoked.
    #[inline(always)]
    pub fn write<'a>(
        &'a self,
        handle: &'a H,
        priority: TaskPriority,
    ) -> ModuleResult<ModuleWriteGuard<H>> {
        let task = self
            .analyzer
            .exclusive(handle, priority)
            .into_module_result(self.id)?;

        Ok(ModuleWriteGuard {
            id: self.id,
            package: self.package,
            task,
        })
    }

    /// A non-blocking alternative to the [write](Self::write) function. If
    /// write access cannot be granted instantly, this function will not block
    /// the current thread. Instead, it will immediately return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error.
    #[inline(always)]
    pub fn try_write<'a>(
        &'a self,
        handle: &'a H,
        priority: TaskPriority,
    ) -> ModuleResult<ModuleWriteGuard<H>> {
        let task = self
            .analyzer
            .try_exclusive(handle, priority)
            .into_module_result(self.id)?;

        Ok(ModuleWriteGuard {
            id: self.id,
            package: self.package,
            task,
        })
    }

    /// Reverts the [deny_access](Self::deny_access) action back to its default
    /// state, enabling read/write operation requests.
    #[inline(always)]
    pub fn allow_access(&self) {
        self.analyzer.set_access_level(0);
    }

    /// Immediately revokes all previously granted read/write access guards and
    /// prevents any new incoming read/write access requests.
    ///
    /// This function is useful when you want to gracefully shut down a
    /// continuous compilation/analysis process.
    ///
    /// You can revert this action by calling the
    /// [allow_access](Self::allow_access) function to enable access requests
    /// again.
    #[inline(always)]
    pub fn deny_access(&self) {
        self.analyzer.set_access_level(TaskPriority::MAX);
    }

    /// Returns true if the ScriptModule allows read/write access to its
    /// content.
    ///
    /// By default, the ScriptModule allows read and write access, but this can
    /// be prevented using the [deny_access](Self::deny_access) function. In
    /// such a case, is_access_allowed would return false.
    #[inline(always)]
    pub fn is_access_allowed(&self) -> bool {
        self.analyzer.get_access_level() < TaskPriority::MAX
    }
}
