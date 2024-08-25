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

use std::{cell::UnsafeCell, cmp::Ordering, iter::repeat, ops::Range};

use crate::{
    exports::Struct,
    interpret::{
        stack::Stack,
        Assembly,
        BindCmd,
        Cmd,
        CmdIndex,
        ConcatCmd,
        DupCmd,
        FieldCmd,
        IfFalseCmd,
        IfTrueCmd,
        InvokeCmd,
        IterateCmd,
        JumpCmd,
        LiftCmd,
        OpCmd,
        PushClosureCmd,
        PushFloatCmd,
        PushFnCmd,
        PushIsizeCmd,
        PushPackageCmd,
        PushStringCmd,
        PushUsizeCmd,
        ScriptFn,
        ShrinkCmd,
        StackDepth,
        SwapCmd,
    },
    runtime::{Arg, Cell, Downcast, Origin, Provider, RuntimeError, RuntimeResult},
};

thread_local! {
    static THREAD_HOOK: UnsafeCell<Option<Box<dyn Fn(&Origin) -> bool>>> = const {
        UnsafeCell::new(None)
    };
}

/// Sets a script evaluation hook for the current OS thread.
///
/// The provided `hook` function will be called on each Ad Astra assembly
/// instruction, allowing the hook function to interrupt the script's execution
/// by returning `false`.
///
/// The hook function receives an [Origin] object as an argument, which roughly
/// indicates the Script or Rust source code text range about to be evaluated.
///
/// By default, the current OS thread does not have a configured runtime hook,
/// meaning scripts will be evaluated until the end of the script's control
/// flow. Setting up the runtime hook gives you control over script evaluation
/// but generally slows down the script's computational performance.
///
/// For additional information, see the [ScriptFn] documentation.
#[inline(always)]
pub fn set_runtime_hook(hook: impl Fn(&Origin) -> bool + 'static) {
    THREAD_HOOK.with(move |current| {
        // Safety: Access is localized.
        let current = unsafe { &mut *current.get() };

        *current = Some(Box::new(hook));
    })
}

/// Unsets the script evaluation hook previously set by the [set_runtime_hook]
/// function.
///
/// If a hook has not been configured for the current OS thread, this function
/// does nothing.
#[inline(always)]
pub fn remove_runtime_hook() {
    THREAD_HOOK.with(move |current| {
        // Safety: Access is localized.
        let current = unsafe { &mut *current.get() };

        *current = None;
    })
}

#[inline(always)]
pub(super) fn is_trusted() -> bool {
    THREAD_HOOK.with(move |current| {
        // Safety: Access is localized.
        let current = unsafe { &*current.get() };

        current.is_none()
    })
}

#[inline(always)]
fn proceed(origin: &Origin) -> bool {
    THREAD_HOOK.with(|hook| {
        // Safety: Access is localized.
        let hook = unsafe { &*hook.get() };

        let Some(hook) = hook else {
            return true;
        };

        hook(origin)
    })
}

impl ScriptFn {
    pub(super) fn execute<const TRUSTED: bool>(&self) -> RuntimeResult<()> {
        let assembly = self.assembly.as_ref();

        let origin = assembly.decl_origin();

        let Some(frame_begin) = Stack::enter_frame(assembly.frame, assembly.arity) else {
            return Err(RuntimeError::StackOverflow { origin });
        };

        let mut engine = Engine::<'_, TRUSTED> {
            assembly,
            cmd_origin: origin,
            closures: &self.closures,
            subroutines: self.subroutines.as_ref(),
            frame_begin,
            cmd_index: 0,
        };

        let result = loop {
            if !TRUSTED && !proceed(&engine.cmd_origin) {
                break Err(RuntimeError::Interrupted {
                    origin: engine.cmd_origin,
                });
            }

            let Some(cmd) = assembly.commands.get(engine.cmd_index) else {
                break Ok(());
            };

            let result = match cmd {
                Cmd::IfTrue(cmd) => engine.execute_if_true(cmd),
                Cmd::IfFalse(cmd) => engine.execute_if_false(cmd),
                Cmd::Jump(cmd) => engine.execute_jump(cmd),
                Cmd::Iterate(cmd) => engine.execute_iterate(cmd),
                Cmd::Lift(cmd) => engine.execute_lift(cmd),
                Cmd::Swap(cmd) => engine.execute_swap(cmd),
                Cmd::Dup(cmd) => engine.execute_dup(cmd),
                Cmd::Shrink(cmd) => engine.execute_shrink(cmd),
                Cmd::PushNil(..) => engine.execute_push_nil(),
                Cmd::PushTrue(..) => engine.execute_push_true(),
                Cmd::PushFalse(..) => engine.execute_push_false(),
                Cmd::PushUsize(cmd) => engine.execute_push_usize(cmd),
                Cmd::PushIsize(cmd) => engine.execute_push_isize(cmd),
                Cmd::PushFloat(cmd) => engine.execute_push_float(cmd),
                Cmd::PushString(cmd) => engine.execute_push_string(cmd),
                Cmd::PushPackage(cmd) => engine.execute_push_package(cmd),
                Cmd::PushClosure(cmd) => engine.execute_push_closure(cmd),
                Cmd::PushFn(cmd) => engine.execute_push_fn(cmd),
                Cmd::PushStruct(..) => engine.execute_push_struct(),
                Cmd::Range(..) => engine.execute_range(),
                Cmd::Bind(cmd) => engine.execute_bind(cmd),
                Cmd::Concat(cmd) => engine.execute_concat(cmd),
                Cmd::Field(cmd) => engine.execute_field(cmd),
                Cmd::Len(..) => engine.execute_len(),
                Cmd::Query(..) => engine.execute_query(),
                Cmd::Op(OpCmd::Clone) => engine.execute_op_clone(),
                Cmd::Op(OpCmd::Neg) => engine.execute_op_neg(),
                Cmd::Op(OpCmd::Not) => engine.execute_op_not(),
                Cmd::Op(OpCmd::Assign) => engine.execute_op_assign(),
                Cmd::Op(OpCmd::AddAssign) => engine.execute_op_add_assign(),
                Cmd::Op(OpCmd::SubAssign) => engine.execute_op_sub_assign(),
                Cmd::Op(OpCmd::MulAssign) => engine.execute_op_mul_assign(),
                Cmd::Op(OpCmd::DivAssign) => engine.execute_op_div_assign(),
                Cmd::Op(OpCmd::BitAndAssign) => engine.execute_op_bit_and_assign(),
                Cmd::Op(OpCmd::BitOrAssign) => engine.execute_op_bit_or_assign(),
                Cmd::Op(OpCmd::BitXorAssign) => engine.execute_op_bit_xor_assign(),
                Cmd::Op(OpCmd::ShlAssign) => engine.execute_op_shl_assign(),
                Cmd::Op(OpCmd::ShrAssign) => engine.execute_op_shr_assign(),
                Cmd::Op(OpCmd::RemAssign) => engine.execute_op_rem_assign(),
                Cmd::Op(OpCmd::Equal) => engine.execute_op_equal(),
                Cmd::Op(OpCmd::NotEqual) => engine.execute_op_not_equal(),
                Cmd::Op(OpCmd::Greater) => engine.execute_op_greater(),
                Cmd::Op(OpCmd::GreaterOrEqual) => engine.execute_op_greater_or_equal(),
                Cmd::Op(OpCmd::Lesser) => engine.execute_op_lesser(),
                Cmd::Op(OpCmd::LesserOrEqual) => engine.execute_op_lesser_or_equal(),
                Cmd::Op(OpCmd::And) => engine.execute_op_and(),
                Cmd::Op(OpCmd::Or) => engine.execute_op_or(),
                Cmd::Op(OpCmd::Add) => engine.execute_op_add(),
                Cmd::Op(OpCmd::Sub) => engine.execute_op_sub(),
                Cmd::Op(OpCmd::Mul) => engine.execute_op_mul(),
                Cmd::Op(OpCmd::Div) => engine.execute_op_div(),
                Cmd::Op(OpCmd::BitAnd) => engine.execute_op_bit_and(),
                Cmd::Op(OpCmd::BitOr) => engine.execute_op_bit_or(),
                Cmd::Op(OpCmd::BitXor) => engine.execute_op_bit_xor(),
                Cmd::Op(OpCmd::Shl) => engine.execute_op_shl(),
                Cmd::Op(OpCmd::Shr) => engine.execute_op_shr(),
                Cmd::Op(OpCmd::Rem) => engine.execute_op_rem(),
                Cmd::Invoke(cmd) => engine.execute_invoke(cmd),
                Cmd::Index(..) => engine.execute_index(),
            };

            if let Err(error) = result {
                break Err(error);
            }
        };

        match result {
            Ok(()) => {
                Stack::leave_frame(frame_begin + 1);
            }

            Err(..) => {
                Stack::leave_frame(frame_begin);
                Stack::push_nil();
            }
        }

        result
    }
}

struct Engine<'a, const TRUSTED: bool> {
    assembly: &'a Assembly,
    cmd_origin: Origin,
    closures: &'a [Cell],
    subroutines: &'a [ScriptFn],
    frame_begin: StackDepth,
    cmd_index: CmdIndex,
}

impl<'a, const TRUSTED: bool> Engine<'a, TRUSTED> {
    fn execute_if_true(&mut self, cmd: &IfTrueCmd) -> RuntimeResult<()> {
        let IfTrueCmd { otherwise } = cmd;

        let condition_origin = self.cmd_1_source();

        if !TRUSTED {
            self.cmd_origin = condition_origin;
        }

        let condition_cell = self.pop_1();

        let value = condition_cell.take::<bool>(condition_origin)?;

        match value {
            true => self.cmd_index += 1,
            false => self.cmd_index = *otherwise,
        }

        Ok(())
    }

    fn execute_if_false(&mut self, cmd: &IfFalseCmd) -> RuntimeResult<()> {
        let IfFalseCmd { otherwise } = cmd;

        let condition_origin = self.cmd_1_source();

        if !TRUSTED {
            self.cmd_origin = condition_origin;
        }

        let condition_cell = self.pop_1();

        let value = condition_cell.take::<bool>(condition_origin)?;

        match value {
            false => self.cmd_index += 1,
            true => self.cmd_index = *otherwise,
        }

        Ok(())
    }

    fn execute_jump(&mut self, cmd: &JumpCmd) -> RuntimeResult<()> {
        let JumpCmd { command } = cmd;

        self.cmd_index = *command;

        Ok(())
    }

    fn execute_iterate(&mut self, cmd: &IterateCmd) -> RuntimeResult<()> {
        let IterateCmd { finish } = cmd;

        let range_origin = self.cmd_1_source();

        if !TRUSTED {
            self.cmd_origin = range_origin;
        }

        let mut range_cell = self.peek_1();

        let range = range_cell.borrow_mut::<Range<usize>>(range_origin)?;

        let Some(next) = range.next() else {
            self.cmd_index = *finish;
            return Ok(());
        };

        self.push(Cell::give(range_origin, next)?);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_lift(&mut self, cmd: &LiftCmd) -> RuntimeResult<()> {
        let LiftCmd { depth } = cmd;

        self.lift(*depth);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_swap(&mut self, cmd: &SwapCmd) -> RuntimeResult<()> {
        let SwapCmd { depth } = cmd;

        self.swap(*depth);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_dup(&mut self, cmd: &DupCmd) -> RuntimeResult<()> {
        let DupCmd { depth } = cmd;

        self.dup(*depth);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_shrink(&mut self, cmd: &ShrinkCmd) -> RuntimeResult<()> {
        let ShrinkCmd { depth } = cmd;

        self.shrink(*depth);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_push_nil(&mut self) -> RuntimeResult<()> {
        self.push_nil();

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_push_true(&mut self) -> RuntimeResult<()> {
        let const_origin = self.cmd_1_source();

        if !TRUSTED {
            self.cmd_origin = const_origin;
        }

        let const_cell = Cell::give(const_origin, true)?;

        self.push(const_cell);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_push_false(&mut self) -> RuntimeResult<()> {
        let const_origin = self.cmd_1_source();

        if !TRUSTED {
            self.cmd_origin = const_origin;
        }

        let const_cell = Cell::give(const_origin, false)?;

        self.push(const_cell);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_push_usize(&mut self, cmd: &PushUsizeCmd) -> RuntimeResult<()> {
        let PushUsizeCmd { value } = cmd;

        let const_origin = self.cmd_1_source();

        if !TRUSTED {
            self.cmd_origin = const_origin;
        }

        let const_cell = Cell::give(const_origin, *value)?;

        self.push(const_cell);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_push_isize(&mut self, cmd: &PushIsizeCmd) -> RuntimeResult<()> {
        let PushIsizeCmd { value } = cmd;

        let const_origin = self.cmd_1_source();

        if !TRUSTED {
            self.cmd_origin = const_origin;
        }

        let const_cell = Cell::give(const_origin, *value)?;

        self.push(const_cell);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_push_float(&mut self, cmd: &PushFloatCmd) -> RuntimeResult<()> {
        let PushFloatCmd { value } = cmd;

        let const_origin = self.cmd_1_source();

        if !TRUSTED {
            self.cmd_origin = const_origin;
        }

        let const_cell = Cell::give(const_origin, value.0)?;

        self.push(const_cell);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_push_string(&mut self, cmd: &PushStringCmd) -> RuntimeResult<()> {
        let PushStringCmd { string_index } = cmd;

        let const_origin = self.cmd_1_source();

        if !TRUSTED {
            self.cmd_origin = const_origin;
        }

        let Some(value) = self.assembly.strings.get(*string_index) else {
            self.push_nil();

            self.cmd_index += 1;

            return Ok(());
        };

        let const_cell = Cell::give(const_origin, value.clone().into_string())?;

        self.push(const_cell);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_push_package(&mut self, cmd: &PushPackageCmd) -> RuntimeResult<()> {
        let PushPackageCmd { package } = cmd;

        let const_cell = package.instance();

        self.push(const_cell);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_push_closure(&mut self, cmd: &PushClosureCmd) -> RuntimeResult<()> {
        let PushClosureCmd { index } = cmd;

        let Some(const_cell) = self.closures.get(*index) else {
            self.push_nil();

            self.cmd_index += 1;

            return Ok(());
        };

        self.push(const_cell.clone());

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_push_fn(&mut self, cmd: &PushFnCmd) -> RuntimeResult<()> {
        let PushFnCmd { index } = cmd;

        let const_origin = self.cmd_1_source();

        if !TRUSTED {
            self.cmd_origin = const_origin;
        }

        let Some(subroutine) = self.subroutines.get(*index) else {
            self.push_nil();

            self.cmd_index += 1;

            return Ok(());
        };

        let const_cell = Cell::give(const_origin, subroutine.clone())?;

        self.push(const_cell);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_push_struct(&mut self) -> RuntimeResult<()> {
        let const_origin = self.cmd_1_source();

        if !TRUSTED {
            self.cmd_origin = const_origin;
        }

        let const_cell = Cell::give(const_origin, Struct::default())?;

        self.push(const_cell);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_range(&mut self) -> RuntimeResult<()> {
        let (range_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = range_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let lhs = <usize as Downcast<'static>>::downcast(lhs_origin, Provider::Owned(lhs_cell))?;
        let rhs = <usize as Downcast<'static>>::downcast(rhs_origin, Provider::Owned(rhs_cell))?;

        let range = lhs..rhs;

        let range_cell = Cell::give(range_origin, range)?;

        self.push(range_cell);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_bind(&mut self, cmd: &BindCmd) -> RuntimeResult<()> {
        let BindCmd { index } = cmd;

        let closure_cell = self.pop_1();
        let mut fn_cell = self.peek_1();

        let script_fn = fn_cell.borrow_mut::<ScriptFn>(self.assembly.decl_origin())?;

        let Some(current) = script_fn.closures.get_mut(*index) else {
            self.cmd_index += 1;

            return Ok(());
        };

        *current = closure_cell;

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_concat(&mut self, cmd: &ConcatCmd) -> RuntimeResult<()> {
        let ConcatCmd { items } = cmd;

        let mut origins = self.cmd_many_source();

        let array_origin = origins.pop().unwrap_or_else(|| self.assembly.decl_origin());

        if !TRUSTED {
            self.cmd_origin = array_origin;
        }

        let item_cells = self.pop_many(*items);

        let mut item_args = Vec::with_capacity(item_cells.len());

        let mut receiver = None;

        let origins_iter = origins.into_iter().chain(repeat(Origin::nil()));

        for (origin, data) in origins_iter.zip(item_cells.into_iter()) {
            if receiver.is_none() {
                if !data.is_nil() {
                    receiver = Some(data.ty());
                }
            }

            item_args.push(Arg { origin, data });
        }

        let Some(receiver) = receiver else {
            self.push_nil();

            self.cmd_index += 1;

            return Ok(());
        };

        let result = receiver.concat(array_origin, &mut item_args)?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_field(&mut self, cmd: &FieldCmd) -> RuntimeResult<()> {
        let FieldCmd { field_index } = cmd;

        let (lhs_origin, field_origin) = self.cmd_2_source();

        if !TRUSTED {
            self.cmd_origin = field_origin;
        }

        let lhs_cell = self.pop_1();

        let Some(field_string) = self.assembly.strings.get(*field_index) else {
            self.push_nil();

            self.cmd_index += 1;

            return Ok(());
        };

        let result = lhs_cell.into_object().component_or_field(
            lhs_origin,
            lhs_origin,
            field_origin.into_ident(field_string.clone()),
        )?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_len(&mut self) -> RuntimeResult<()> {
        let (lhs_origin, field_origin) = self.cmd_2_source();

        if !TRUSTED {
            self.cmd_origin = field_origin;
        }

        let mut lhs_cell = self.pop_1();

        let length = match lhs_cell.is::<str>() {
            true => lhs_cell.borrow_str(lhs_origin)?.chars().count(),
            false => lhs_cell.length(),
        };

        self.push(Cell::give(field_origin, length)?);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_query(&mut self) -> RuntimeResult<()> {
        let op_origin = self.cmd_1_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let lhs_cell = self.pop_1();

        let ty = lhs_cell.ty();

        let result = !ty.prototype().implements_none();

        self.push(Cell::give(op_origin, result)?);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_clone(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin) = self.cmd_2_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let rhs_cell = self.pop_1();

        let result = rhs_cell.into_object().clone(op_origin, rhs_origin)?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_neg(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin) = self.cmd_2_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let rhs_cell = self.pop_1();

        let result = rhs_cell.into_object().neg(op_origin, rhs_origin)?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_not(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin) = self.cmd_2_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let rhs_cell = self.pop_1();

        let result = rhs_cell.into_object().not(op_origin, rhs_origin)?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_assign(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin, lhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (rhs_cell, lhs_cell) = self.pop_2();

        lhs_cell.into_object().assign(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_add_assign(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin, lhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (rhs_cell, lhs_cell) = self.pop_2();

        lhs_cell.into_object().add_assign_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_sub_assign(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin, lhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (rhs_cell, lhs_cell) = self.pop_2();

        lhs_cell.into_object().sub_assign_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_mul_assign(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin, lhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (rhs_cell, lhs_cell) = self.pop_2();

        lhs_cell.into_object().mul_assign_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_div_assign(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin, lhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (rhs_cell, lhs_cell) = self.pop_2();

        lhs_cell.into_object().div_assign_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_bit_and_assign(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin, lhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (rhs_cell, lhs_cell) = self.pop_2();

        lhs_cell.into_object().bit_and_assign_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_bit_or_assign(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin, lhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (rhs_cell, lhs_cell) = self.pop_2();

        lhs_cell.into_object().bit_or_assign_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_bit_xor_assign(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin, lhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (rhs_cell, lhs_cell) = self.pop_2();

        lhs_cell.into_object().bit_xor_assign_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_shl_assign(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin, lhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (rhs_cell, lhs_cell) = self.pop_2();

        lhs_cell.into_object().shl_assign_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_shr_assign(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin, lhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (rhs_cell, lhs_cell) = self.pop_2();

        lhs_cell.into_object().shr_assign_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_rem_assign(&mut self) -> RuntimeResult<()> {
        let (op_origin, rhs_origin, lhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (rhs_cell, lhs_cell) = self.pop_2();

        lhs_cell.into_object().rem_assign_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_equal(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().partial_eq(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(Cell::give(op_origin, result)?);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_not_equal(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().partial_eq(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(Cell::give(op_origin, !result)?);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_greater(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().ord_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(Cell::give(op_origin, result == Ordering::Greater)?);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_greater_or_equal(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().ord_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(Cell::give(
            op_origin,
            result == Ordering::Greater || result == Ordering::Equal,
        )?);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_lesser(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().ord_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(Cell::give(op_origin, result == Ordering::Less)?);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_lesser_or_equal(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().ord_fallback(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(Cell::give(
            op_origin,
            result == Ordering::Less || result == Ordering::Equal,
        )?);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_and(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().and(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_or(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().or(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_add(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().add(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_sub(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().sub(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_mul(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().mul(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_div(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().div(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_bit_and(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().bit_and(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_bit_or(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().bit_or(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_bit_xor(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().bit_xor(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_shl(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().shl(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_shr(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().shr(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_op_rem(&mut self) -> RuntimeResult<()> {
        let (op_origin, lhs_origin, rhs_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (lhs_cell, rhs_cell) = self.pop_2();

        let result = lhs_cell.into_object().rem(
            op_origin,
            lhs_origin,
            Arg {
                origin: rhs_origin,
                data: rhs_cell,
            },
        )?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    fn execute_invoke(&mut self, cmd: &InvokeCmd) -> RuntimeResult<()> {
        let InvokeCmd { arity } = cmd;

        let mut origins = self.cmd_many_source();

        let invocation_origin = origins.pop().unwrap_or_else(|| self.assembly.decl_origin());
        let lhs_origin = origins.pop().unwrap_or_else(|| self.assembly.decl_origin());

        if !TRUSTED {
            self.cmd_origin = invocation_origin;
        }

        let mut lhs_cell = self.pop_1();

        if lhs_cell.is::<ScriptFn>() {
            let script_fn = lhs_cell.borrow_ref::<ScriptFn>(lhs_origin)?;

            let parameters = script_fn.assembly.as_ref().arity;

            if parameters.ne(arity) {
                return Err(RuntimeError::ArityMismatch {
                    invocation_origin,
                    function_origin: script_fn.assembly.as_ref().decl_origin(),
                    parameters,
                    arguments: *arity,
                });
            }

            script_fn.execute::<TRUSTED>()?;

            self.cmd_index += 1;

            return Ok(());
        }

        let arg_origins = origins.into_iter().chain(repeat(invocation_origin));
        let arg_cells = self.pop_many(*arity);

        let mut args = Vec::with_capacity(arg_cells.len());

        for (origin, data) in arg_origins.zip(arg_cells) {
            args.push(Arg { origin, data });
        }

        let result = lhs_cell
            .into_object()
            .invoke(invocation_origin, lhs_origin, &mut args)?;

        self.cmd_index += 1;

        self.push(result);

        Ok(())
    }

    fn execute_index(&mut self) -> RuntimeResult<()> {
        let (op_origin, slice_origin, range_origin) = self.cmd_3_source();

        if !TRUSTED {
            self.cmd_origin = op_origin;
        }

        let (range_cell, slice_cell) = self.pop_2();

        let bounds = Self::slice_bounds(range_origin, range_cell)?;

        if slice_cell.is::<str>() {
            let bounds =
                Self::string_range(range_origin, slice_origin, slice_cell.clone(), bounds)?;

            let result = slice_cell
                .map_slice(op_origin, bounds)?
                .map_str(op_origin)?;

            self.push(result);

            self.cmd_index += 1;

            return Ok(());
        }

        let range = match bounds {
            SliceBounds::Range(mut range) => {
                let length = slice_cell.length();

                if range.start < range.end && range.start <= length && range.end > length {
                    range.end = length;
                }

                range
            }

            SliceBounds::Index(index) => index..index.checked_add(1).unwrap_or(usize::MAX),
        };

        let result = slice_cell.map_slice(op_origin, range)?;

        self.push(result);

        self.cmd_index += 1;

        Ok(())
    }

    #[inline(always)]
    fn slice_bounds(range_origin: Origin, range_cell: Cell) -> RuntimeResult<SliceBounds> {
        let provider = Provider::Owned(range_cell);

        let mut type_match = provider.type_match();

        if type_match.belongs_to::<usize>() || type_match.is::<bool>() || type_match.is::<str>() {
            let index = <usize as Downcast<'static>>::downcast(range_origin, provider)?;

            return Ok(SliceBounds::Index(index));
        }

        if type_match.is::<Range<usize>>() {
            let range = provider.to_owned().take::<Range<usize>>(range_origin)?;

            return Ok(SliceBounds::Range(range));
        }

        Err(type_match.mismatch(range_origin))
    }

    fn string_range(
        range_origin: Origin,
        string_origin: Origin,
        mut string_cell: Cell,
        bounds: SliceBounds,
    ) -> RuntimeResult<Range<usize>> {
        let (is_index, range) = match bounds {
            SliceBounds::Range(range) => (false, range),
            SliceBounds::Index(index) => (true, index..index.checked_add(1).unwrap_or(usize::MAX)),
        };

        if range.start > range.end {
            return Err(RuntimeError::MalformedRange {
                access_origin: range_origin,
                start_bound: range.start,
                end_bound: range.end,
            });
        }

        let string = string_cell.borrow_str(string_origin)?;

        let mut chars_iter = string.char_indices();
        let mut chars_consumed = 0;

        let mut start = None;

        while let Some((byte, _)) = chars_iter.next() {
            if chars_consumed == range.start {
                start = Some(byte);
                chars_consumed += 1;
                break;
            }

            chars_consumed += 1;
        }

        let start = match start {
            Some(bound) => bound,

            None if range.start == chars_consumed && !is_index => string.len(),

            None => {
                return Err(RuntimeError::OutOfBounds {
                    access_origin: range_origin,
                    index: range.start,
                    length: chars_consumed,
                })
            }
        };

        if range.start == range.end {
            return Ok(start..start);
        }

        while let Some((byte, _)) = chars_iter.next() {
            if chars_consumed == range.end {
                return Ok(start..byte);
            }

            chars_consumed += 1;
        }

        Ok(start..string.len())
    }

    #[inline(always)]
    fn cmd_1_source(&self) -> Origin {
        self.assembly.cmd_1_source(self.cmd_index)
    }

    #[inline(always)]
    fn cmd_2_source(&self) -> (Origin, Origin) {
        self.assembly.cmd_2_source(self.cmd_index)
    }

    #[inline(always)]
    fn cmd_3_source(&self) -> (Origin, Origin, Origin) {
        self.assembly.cmd_3_source(self.cmd_index)
    }

    #[inline(always)]
    fn cmd_many_source(&self) -> Vec<Origin> {
        self.assembly.cmd_many_source(self.cmd_index)
    }

    #[inline(always)]
    fn push_nil(&self) {
        Stack::push_nil()
    }

    #[inline(always)]
    fn push(&self, cell: Cell) {
        Stack::push(cell)
    }

    #[inline(always)]
    fn pop_1(&self) -> Cell {
        Stack::pop_1(self.frame_begin)
    }

    #[inline(always)]
    fn pop_2(&self) -> (Cell, Cell) {
        Stack::pop_2(self.frame_begin)
    }

    #[inline(always)]
    fn pop_many(&self, length: StackDepth) -> Vec<Cell> {
        Stack::pop_many(self.frame_begin, length)
    }

    #[inline(always)]
    fn peek_1(&self) -> Cell {
        Stack::peek_1(self.frame_begin)
    }

    #[inline(always)]
    fn lift(&self, depth: StackDepth) {
        Stack::lift(self.frame_begin, depth);
    }

    #[inline(always)]
    fn swap(&self, depth: StackDepth) {
        Stack::swap(self.frame_begin, depth);
    }

    #[inline(always)]
    fn dup(&self, depth: StackDepth) {
        Stack::dup(self.frame_begin, depth);
    }

    #[inline(always)]
    fn shrink(&self, depth: StackDepth) {
        Stack::shrink(self.frame_begin, depth);
    }
}

enum SliceBounds {
    Range(Range<usize>),
    Index(usize),
}
