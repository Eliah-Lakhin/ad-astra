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

use std::mem::take;

use ad_astra_export::export;
use lady_deirdre::sync::Shared;

use crate::{
    interpret::{engine::is_trusted, stack::Stack, Assembly},
    runtime::{
        ops::{DynamicType, ScriptBinding, ScriptClone, ScriptInvocation},
        Arg,
        Cell,
        Downcast,
        InvocationMeta,
        Origin,
        Provider,
        RuntimeError,
        RuntimeResult,
        ScriptType,
        TypeHint,
        Upcast,
        __intrinsics::FUNCTION_FAMILY,
    },
};

/// Assembly code for the Ad Astra Virtual Machine, ready for execution.
///
/// You can create this object using the
/// [compile](crate::analysis::ModuleRead::compile) function, and then run it
/// using the [ScriptFn::run] function.
///
/// The ScriptFn object is cheap to [Clone]. In the case of cloning, each clone
/// shares the same assembly code memory, but the execution
/// [context](ScriptFn::set_context) is unique to each clone.
///
/// ## Virtual Machine Design Overview
///
/// The assembly code design is currently an implementation detail and is
/// subject to continuous improvements, optimizations, and changes in future
/// minor versions of Ad Astra. For this reason, the crate API does not provide
/// direct access to manually alter the assembly code. However, for debugging
/// purposes, you can print the internals to the terminal using the [Debug]
/// implementation of the ScriptFn object.
///
/// The ScriptFn consists of Ad Astra assembly commands for the main script
/// module function (the top-level source code of a module itself serves as the
/// body of a function with zero parameters), as well as the assembly commands
/// for other script functions from this module.
///
/// The runtime executes each assembly command of the script function
/// sequentially. Some commands can conditionally or unconditionally jump to
/// other commands in the list.
///
/// The commands interact with the stack of the current thread by pulling some
/// [Cells](Cell) from the stack and pushing new Cells onto the stack.
/// Therefore, the Virtual Machine is a stack-based machine.
///
/// ## Isolation
///
/// Each assembly command is evaluated in a virtual environment. If for any
/// reason a command fails, the Virtual Machine immediately stops execution and
/// returns a [RuntimeError] from the [ScriptFn::run] function.
///
/// You can manually interrupt script function execution using the hook
/// mechanism. By setting a hook function with the
/// [set_runtime_hook](crate::interpret::set_runtime_hook) function, you enforce
/// the Virtual Machine to report every command execution to the hook. The hook,
/// in turn, can return `false` to signal the Virtual Machine to stop execution
/// and return from [ScriptFn::run] with a [RuntimeError::Interrupted] error.
///
/// ```rust
/// use ad_astra::interpret::set_runtime_hook;
///
/// set_runtime_hook(|_origin| true);
/// ```
///
/// The hook function is configured per OS process thread. By default, the
/// thread from which you call the [ScriptFn::run] function does not have a
/// configured hook, meaning that you trust the script to finish its job without
/// interruptions. In this trusting mode, script functions are executed slightly
/// faster than with a configured hook, but the downside is that you cannot
/// revoke control flow back to Rust until the Virtual Machine finishes its job.
/// This could be an issue, for example, if the script code contains
/// unconditional infinite loops.
///
/// Additionally, the hook function receives an [Origin] object as an argument
/// that roughly points to the original source code statements and expressions
/// of the script module that are about to be evaluated. You can use this
/// feature to organize interactive script evaluation.
///
/// ## Source Maps
///
/// In addition to the assembly commands, the ScriptFn object also holds a
/// mapping between the assembly commands and the source ranges from which these
/// commands were compiled.
///
/// The Virtual Machine uses this metadata to provide proper and descriptive
/// [runtime errors](RuntimeError) if a script execution flow ends with a script
/// evaluation error.
///
/// ## Concurrent Evaluation
///
/// Each script function is executed on the current OS thread from which
/// it was [run](ScriptFn::run).
///
/// The Ad Astra base language does not provide a built-in mechanism for
/// asynchronous script evaluation or thread management. However, you can
/// organize a multi-threaded execution environment depending on your design
/// goals using the export system.
///
/// For example, you can export a function from Rust to a script that takes
/// another function as a parameter (e.g., [Fn0](crate::runtime::ops::Fn0)).
/// In the script, the author can call this exported Rust function, passing
/// a script-defined function as an argument. The Rust function can then execute
/// the provided script function in another thread.
///
/// ```rust
/// # use std::thread::spawn;
/// #
/// # use ad_astra::{export, runtime::ops::Fn0};
/// #
/// # #[export(include)]
/// # #[export(package)]
/// # #[derive(Default)]
/// # struct Package;
/// #
/// #[export]
/// pub fn foo(f: Fn0<()>) {
///     spawn(move || {
///         let f = f;
///
///         let _ = f();
///     });
/// }
/// ```
#[derive(Clone)]
pub struct ScriptFn {
    pub(super) assembly: Shared<Assembly>,
    pub(super) closures: Vec<Cell>,
    pub(super) subroutines: Shared<Vec<ScriptFn>>,
}

impl Default for ScriptFn {
    #[inline(always)]
    fn default() -> Self {
        Self {
            assembly: Shared::default(),
            closures: Vec::new(),
            subroutines: Default::default(),
        }
    }
}

impl ScriptFn {
    /// Evaluates the script.
    ///
    /// The function returns the evaluation result in the form of a [Cell],
    /// representing an object returned by the script using the `return 100;`
    /// statement. If the script does not return any value, the function returns
    /// [Cell::nil].
    ///
    /// If the script encounters a runtime error during execution, the function
    /// halts script execution immediately and returns a [RuntimeError].
    ///
    /// If the script execution is interrupted by the execution hook (configured
    /// via [set_runtime_hook](crate::interpret::set_runtime_hook)), this
    /// function returns a [RuntimeError::Interrupted] error.
    ///
    /// By default, the current OS thread does not have a script hook, meaning
    /// that the Virtual Machine will execute the script until the end of the
    /// script's control flow.
    #[inline(always)]
    pub fn run(&self) -> RuntimeResult<Cell> {
        let assembly = self.assembly.as_ref();

        let parameters = assembly.arity;

        if parameters > 0 {
            return Err(RuntimeError::ArityMismatch {
                invocation_origin: Origin::nil(),
                function_origin: assembly.decl_origin(),
                parameters,
                arguments: 0,
            });
        }

        match is_trusted() {
            true => self.execute::<true>()?,
            false => self.execute::<false>()?,
        }

        Ok(Stack::pop_1(0))
    }

    /// Sets the value of the `self` script variable, allowing the module's
    /// source code to read script input data.
    ///
    /// You can create the [Cell] using the [Cell::give] constructor by passing
    /// a value of any type known to the Ad Astra Runtime (either any built-in
    /// Rust type or any type exported using the [export](crate::export) macro).
    ///
    /// Each clone of the ScriptFn object may have a unique context value, but
    /// the context can only be set once per ScriptFn instance. Subsequent calls
    /// to the `set_context` function will not change the previously set
    /// context.
    ///
    /// By default, the ScriptFn instance does not have an evaluation context,
    /// and the `self` variable is interpreted as "nil" within the script code.
    ///
    /// Note that the script's `self` variable is generally mutable if the type
    /// of the value supports mutations (e.g., number types are mutable). Thus,
    /// the `self` script variable can serve as both a data input and output
    /// channel. If the value you set as the context is intended as the script's
    /// output, consider reading this value after the script
    /// [evaluation](Self::run) using the [get_context](Self::get_context)
    /// function.
    #[inline(always)]
    pub fn set_context(&mut self, context: Cell) {
        let Some(this) = self.closures.get_mut(0) else {
            return;
        };

        *this = context;
    }

    /// Provides access to the value of the script's `self` variable, as
    /// previously set by the [set_context](Self::set_context) function.
    ///
    /// By default, if the ScriptFn instance does not have an evaluation context
    /// value, this function returns [Cell::nil].
    #[inline(always)]
    pub fn get_context(&self) -> &Cell {
        static NIL: Cell = Cell::nil();

        let Some(this) = self.closures.get(0) else {
            return &NIL;
        };

        this
    }
}

/// A script function.
#[export(include)]
#[export(name "fn")]
#[export(family &FUNCTION_FAMILY)]
pub(crate) type ScriptFnType = ScriptFn;

impl<'a> Downcast<'a> for ScriptFnType {
    #[inline(always)]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let mut type_match = provider.type_match();

        if type_match.is::<ScriptFnType>() {
            return provider.to_owned().take::<ScriptFnType>(origin);
        }

        return Err(type_match.mismatch(origin));
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(ScriptFnType::type_meta())
    }
}

impl<'a> Upcast<'a> for ScriptFnType {
    type Output = Box<ScriptFnType>;

    #[inline(always)]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(Box::new(this))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(ScriptFnType::type_meta())
    }
}

#[export(include)]
impl ScriptClone for ScriptFnType {}

#[export(include)]
impl ScriptInvocation for ScriptFnType {
    fn invoke(origin: Origin, lhs: Arg, arguments: &mut [Arg]) -> RuntimeResult<Cell> {
        let function = lhs.data.take::<ScriptFnType>(origin)?;
        let assembly = function.assembly.as_ref();

        let arguments_count = arguments.len();
        let parameters_count = assembly.arity;

        if arguments_count != parameters_count {
            return Err(RuntimeError::ArityMismatch {
                invocation_origin: origin,
                function_origin: assembly.decl_origin(),
                parameters: parameters_count,
                arguments: arguments_count,
            });
        }

        for arg in arguments {
            let cell = take(&mut arg.data);

            Stack::push(cell);
        }

        match is_trusted() {
            true => function.execute::<true>()?,
            false => function.execute::<false>()?,
        }

        Ok(Stack::pop_1(0))
    }

    #[inline(always)]
    fn hint() -> Option<&'static InvocationMeta> {
        None
    }
}

#[export(include)]
impl ScriptBinding for ScriptFnType {
    type RHS = DynamicType;

    fn script_binding(_origin: Origin, mut lhs: Arg, rhs: Arg) -> RuntimeResult<()> {
        if rhs.data.is_nil() {
            return Ok(());
        }

        let function = lhs.data.borrow_mut::<ScriptFnType>(lhs.origin)?;

        let Some(this) = function.closures.get_mut(0) else {
            return Ok(());
        };

        if this.is_nil() {
            *this = rhs.data;
        }

        Ok(())
    }
}
