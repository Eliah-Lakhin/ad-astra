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

use std::{borrow::Cow, collections::hash_map::Entry, mem::replace, ops::Deref};

use ahash::{AHashMap, AHashSet};
use compact_str::CompactString;
use lady_deirdre::{
    analysis::{AnalysisResult, AttrContext, Semantics, SharedComputable, TaskHandle},
    arena::Identifiable,
    lexis::{SourceCode, ToSpan, TokenRef},
    sync::{Shared, SyncBuildHasher},
    syntax::{NodeRef, PolyRef, NIL_NODE_REF},
};

use crate::{
    analysis::ModuleResultEx,
    interpret::{
        Assembly,
        BindCmd,
        ClosureIndex,
        Cmd,
        CmdIndex,
        ConcatCmd,
        DupCmd,
        FieldCmd,
        IfFalseCmd,
        IfTrueCmd,
        IndexCmd,
        InvokeCmd,
        IterateCmd,
        JumpCmd,
        LenCmd,
        LiftCmd,
        OpCmd,
        OriginIndex,
        PushClosureCmd,
        PushFalseCmd,
        PushFloatCmd,
        PushFnCmd,
        PushIsizeCmd,
        PushNilCmd,
        PushPackageCmd,
        PushStringCmd,
        PushStructCmd,
        PushTrueCmd,
        PushUsizeCmd,
        QueryCmd,
        RangeCmd,
        ShrinkCmd,
        StackDepth,
        StringIndex,
        SwapCmd,
        RET,
    },
    report::system_panic,
    runtime::{Origin, PackageMeta, ScriptOrigin},
    semantics::{setup::log_attr, *},
    syntax::{PolyRefOrigin, ScriptDoc, ScriptNode, ScriptToken, SpanBounds},
};

impl SharedComputable for Assembly {
    type Node = ScriptNode;

    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;
        let doc = doc_read.deref();

        match node_ref.deref(doc) {
            Some(ScriptNode::Root {
                statements,
                semantics,
                ..
            }) => {
                let root_semantics = semantics.get().forward()?;

                let origin = node_ref.script_origin(doc, SpanBounds::Header);

                let ident_desc_map = root_semantics.compilation.ident_desc_map.read(context)?;
                let closure_vec = root_semantics.compilation.closure_vec.read(context)?;
                let lifetimes = root_semantics.compilation.lifetimes.read(context)?;

                let mut assembler = Assembler::new(
                    doc,
                    context,
                    origin,
                    &NIL_NODE_REF,
                    &ident_desc_map,
                    &closure_vec,
                    &lifetimes,
                );

                assembler.assemble_statements(statements)?;

                if let FlowExecution::Normal = assembler.flow_state.execution {
                    assembler.assemble_return(&NIL_NODE_REF)?;
                }

                assembler.shrink_ret();

                Ok(Shared::new(assembler.assembly))
            }

            Some(ScriptNode::Fn {
                params,
                body,
                semantics,
                ..
            }) => {
                let fn_semantics = semantics.get().forward()?;

                let origin = node_ref.script_origin(doc, SpanBounds::Header);

                let ident_desc_map = fn_semantics.compilation.ident_desc_map.read(context)?;
                let closure_vec = fn_semantics.compilation.closure_vec.read(context)?;
                let lifetimes = fn_semantics.compilation.lifetimes.read(context)?;

                let mut assembler = Assembler::new(
                    doc,
                    context,
                    origin,
                    params,
                    &ident_desc_map,
                    &closure_vec,
                    &lifetimes,
                );

                match body.deref(doc) {
                    Some(ScriptNode::Block { statements, .. }) => {
                        assembler.assemble_statements(statements)?;
                    }

                    Some(ScriptNode::Expr { inner, .. }) => {
                        assembler.assemble_return(inner)?;
                    }

                    _ => (),
                }

                if let FlowExecution::Normal = assembler.flow_state.execution {
                    assembler.assemble_return(&NIL_NODE_REF)?;
                }

                assembler.shrink_ret();

                Ok(Shared::new(assembler.assembly))
            }

            _ => Ok(Shared::default()),
        }
    }
}

struct Assembler<'doc, 'ctx, 'ctx_param, H: TaskHandle, S: SyncBuildHasher> {
    doc: &'doc ScriptDoc,
    context: &'ctx mut AttrContext<'ctx_param, ScriptNode, H, S>,
    package: &'static PackageMeta,
    ident_desc_map: &'doc AHashMap<NodeRef, IdentDesc>,
    closure_index: AHashMap<&'doc str, ClosureIndex>,
    ident_drops: &'doc AHashSet<NodeRef>,
    closure_drops: &'doc AHashMap<NodeRef, AHashSet<CompactString>>,
    unused_vars: &'doc AHashSet<NodeRef>,
    assembly: Assembly,
    committed: CmdIndex,
    flow_state: FlowState<'doc>,
    loop_state: Option<LoopState<'doc>>,
    origin_index: AHashMap<Origin, OriginIndex>,
    string_index: AHashMap<Cow<'doc, str>, StringIndex>,
    string_rev_index: AHashMap<StringIndex, (usize, Cow<'doc, str>)>,
}

impl<'doc, 'ctx, 'ctx_param, H: TaskHandle, S: SyncBuildHasher>
    Assembler<'doc, 'ctx, 'ctx_param, H, S>
{
    #[inline(always)]
    fn new(
        doc: &'doc ScriptDoc,
        context: &'ctx mut AttrContext<'ctx_param, ScriptNode, H, S>,
        origin: impl Into<Origin>,
        params: &'doc NodeRef,
        ident_desc_map: &'doc IdentsDescMap,
        closure_vec: &'doc ClosureVec,
        lifetimes: &'doc Lifetimes,
    ) -> Self {
        let Some(package) = PackageMeta::by_id(doc.id()) else {
            system_panic!("Missing package.");
        };

        let ident_desc_map = &ident_desc_map.map;

        let mut closure_index = AHashMap::with_capacity(closure_vec.vec.len());

        for (index, closure_name) in closure_vec.vec.iter().enumerate() {
            let _ = closure_index.insert(closure_name.as_str(), index + 1);
        }

        let ident_drops = &lifetimes.ident_drops;
        let closure_drops = &lifetimes.closure_drops;
        let unused_vars = &lifetimes.unused_vars;

        let mut namespace = AHashMap::new();
        let arity;

        match params.deref(doc) {
            Some(ScriptNode::FnParams { params, .. }) => {
                arity = params.len();

                for (depth, param_ref) in params.iter().enumerate() {
                    let Some(ScriptNode::Var { token, .. }) = param_ref.deref(doc) else {
                        continue;
                    };

                    let Some(param_string) = token.string(doc) else {
                        continue;
                    };

                    let _ = namespace.insert(param_string, depth);
                }
            }
            _ => {
                arity = 0;
            }
        }

        let origin = origin.into();

        let assembly = Assembly::new::<true>(arity, closure_vec.vec.len(), 0, origin);

        let flow_state = FlowState {
            namespace,
            stack_depth: arity,
            execution: FlowExecution::Normal,
        };

        let origin_index = AHashMap::from([(origin, 0)]);

        Self {
            doc,
            context,
            package,
            ident_desc_map,
            closure_index,
            ident_drops,
            closure_drops,
            unused_vars,
            assembly,
            committed: 0,
            flow_state,
            loop_state: None,
            origin_index,
            string_index: AHashMap::new(),
            string_rev_index: AHashMap::new(),
        }
    }

    fn assemble_clause(&mut self, expr: &NodeRef) -> AnalysisResult<()> {
        self.assemble_expr(expr)?;
        self.shrink_stack(1);

        Ok(())
    }

    fn assemble_if(&mut self, condition: &NodeRef, body: &NodeRef) -> AnalysisResult<()> {
        match ScriptNode::extract_bool(self.doc, *condition) {
            Some(true) => {
                self.assemble_st(body)?;

                return Ok(());
            }

            Some(false) => return Ok(()),

            _ => (),
        }

        let before_condition = self.save_flow();

        let condition_origin = condition.script_origin(self.doc, SpanBounds::Cover);

        self.assemble_expr(condition)?;

        let if_true_cmd = self.cmd_if_true(condition_origin);

        if let Some(ScriptNode::Block { statements, .. }) = body.deref(self.doc) {
            let before_handler = self.save_flow();

            self.assemble_statements(statements)?;

            let _ = self.restore_flow(before_handler);
        }

        let next_cmd = self.reserve_cmd();

        let _ = self.restore_flow(before_condition);

        self.set_cmd_jump_target(if_true_cmd, next_cmd);

        Ok(())
    }

    fn assemble_match(&mut self, subject: &NodeRef, body: &NodeRef) -> AnalysisResult<()> {
        struct ArmMeta {
            condition_cmd: Option<CmdIndex>,
            break_cmd: Option<CmdIndex>,
            end_cmd: CmdIndex,
        }

        let before_subject = self.save_flow();

        let mut subject_origin = ScriptOrigin::nil();
        let mut subject_depth = 0;

        if !subject.is_nil() {
            subject_origin = subject.script_origin(self.doc, SpanBounds::Cover);

            self.assemble_expr(subject)?;

            subject_depth = self.stack_top();
        }

        let Some(ScriptNode::MatchBody { arms, .. }) = body.deref(self.doc) else {
            return Ok(());
        };

        let mut true_arm = false;
        let mut false_arm = false;
        let mut default_arm = false;
        let mut exec_after = None;
        let mut arm_meta = Vec::with_capacity(arms.len());

        for arm_ref in arms {
            let Some(ScriptNode::MatchArm { case, handler, .. }) = arm_ref.deref(self.doc) else {
                continue;
            };

            let before_condition = self.save_flow();

            let condition_cmd = match case.deref(self.doc) {
                Some(ScriptNode::Else { .. }) => {
                    default_arm = true;
                    None
                }

                Some(ScriptNode::Expr { inner, .. }) => {
                    match (subject.is_nil(), ScriptNode::extract_bool(self.doc, *inner)) {
                        (true, Some(true)) => {
                            default_arm = true;
                            None
                        }

                        (true, Some(false)) => continue,

                        (true, None) => {
                            let object_origin = case.script_origin(self.doc, SpanBounds::Cover);

                            self.assemble_expr(inner)?;

                            Some(self.cmd_if_true(object_origin))
                        }

                        (false, Some(true)) => {
                            true_arm = true;

                            match false_arm {
                                true => None,

                                false => {
                                    let _ = self.cmd_dup(subject_depth);

                                    Some(self.cmd_if_true(subject_origin))
                                }
                            }
                        }

                        (false, Some(false)) => {
                            false_arm = true;

                            match true_arm {
                                true => None,

                                false => {
                                    let _ = self.cmd_dup(subject_depth);

                                    Some(self.cmd_if_false(subject_origin))
                                }
                            }
                        }

                        (false, None) => {
                            let object_origin = case.script_origin(self.doc, SpanBounds::Cover);

                            let _ = self.cmd_dup(subject_depth);
                            self.assemble_expr(inner)?;

                            let _ = self.cmd_op_binary(
                                object_origin,
                                subject_origin,
                                object_origin,
                                OpCmd::Equal,
                            );

                            Some(self.cmd_if_true(object_origin))
                        }
                    }
                }

                _ => continue,
            };

            let before_handler = self.save_flow();

            match handler.deref(self.doc) {
                Some(ScriptNode::Expr { inner, .. }) => self.assemble_clause(inner)?,

                Some(ScriptNode::Block { statements, .. }) => {
                    self.assemble_statements(statements)?;
                }

                _ => (),
            }

            let handler_exec = self.restore_flow(before_handler);

            let break_cmd = match (handler_exec, condition_cmd.is_some()) {
                (FlowExecution::Normal, true) => Some(self.cmd_jump_to_the_future()),
                _ => None,
            };

            let end_cmd = self.reserve_cmd();

            self.restore_flow(before_condition);

            arm_meta.push(ArmMeta {
                condition_cmd,
                break_cmd,
                end_cmd,
            });

            match (exec_after, handler_exec) {
                (None, exec) => {
                    exec_after = Some(exec);
                }

                (Some(current_exec), arm_exec) if (current_exec as u8) > (arm_exec as u8) => {
                    exec_after = Some(arm_exec)
                }

                _ => (),
            }

            if condition_cmd.is_none() {
                break;
            }
        }

        let exhaustive = true_arm && false_arm || default_arm;

        let exec_after = exec_after
            .filter(|_| exhaustive)
            .unwrap_or(FlowExecution::Normal);

        let match_end = self.reserve_cmd();

        if let FlowExecution::Normal = exec_after {
            let _ = self.restore_flow(before_subject);
        }

        self.flow_state.execution = exec_after;

        for meta in arm_meta {
            if let Some(condition_cmd) = meta.condition_cmd {
                self.set_cmd_jump_target(condition_cmd, meta.end_cmd);
            }

            if let Some(break_cmd) = meta.break_cmd {
                self.set_cmd_jump_target(break_cmd, match_end);
            }
        }

        Ok(())
    }

    fn assemble_let(&mut self, name: &NodeRef, value: &NodeRef) -> AnalysisResult<()> {
        if self.unused_vars.contains(name) {
            if !value.is_nil() {
                self.assemble_clause(value)?;
            }

            return Ok(());
        }

        let Some(ScriptNode::Var { token, .. }) = name.deref(self.doc) else {
            if !value.is_nil() {
                self.assemble_clause(value)?;
            }

            return Ok(());
        };

        let Some(var_string) = token.string(self.doc) else {
            if !value.is_nil() {
                self.assemble_clause(value)?;
            }

            return Ok(());
        };

        match value.is_nil() {
            true => {
                let _ = self.cmd_push_nil();
            }

            false => {
                self.assemble_expr(value)?;
            }
        }

        let var_depth = self.stack_top();

        let _ = self.flow_state.namespace.insert(var_string, var_depth);

        Ok(())
    }

    fn assemble_for(
        &mut self,
        iterator: &NodeRef,
        range: &NodeRef,
        body: &NodeRef,
    ) -> AnalysisResult<()> {
        let before_range = self.save_flow();

        self.assemble_expr(range)?;

        let before_loop = {
            let entry_point = self.reserve_cmd();
            let entry_state = self.save_flow();

            replace(
                &mut self.loop_state,
                Some(LoopState {
                    entry_point,
                    entry_state,
                    breaks: Vec::new(),
                }),
            )
        };

        let range_origin = range.script_origin(self.doc, SpanBounds::Cover);

        let iterate_cmd = self.cmd_iterate(range_origin);

        if let Some(ScriptNode::Var { token, .. }) = iterator.deref(self.doc) {
            if let Some(var_string) = token.string(self.doc) {
                let iterator_depth = self.stack_top();

                let _ = self.flow_state.namespace.insert(var_string, iterator_depth);
            }
        }

        if let Some(ScriptNode::Block { statements, .. }) = body.deref(self.doc) {
            self.assemble_statements(statements)?;
        }

        let Some(loop_state) = replace(&mut self.loop_state, before_loop) else {
            return Ok(());
        };

        match self.restore_flow(loop_state.entry_state) {
            FlowExecution::Normal => {
                let _ = self.cmd_jump_to_the_past(loop_state.entry_point);
            }

            FlowExecution::Break
            | FlowExecution::Continue
            | FlowExecution::Return
            | FlowExecution::Unreachable => (),
        }

        self.flow_state.execution = FlowExecution::Normal;

        let loop_end = self.reserve_cmd();

        self.set_cmd_jump_target(iterate_cmd, loop_end);

        for break_cmd in loop_state.breaks {
            self.set_cmd_jump_target(break_cmd, loop_end);
        }

        self.restore_flow(before_range);

        Ok(())
    }

    fn assemble_loop(&mut self, body: &NodeRef) -> AnalysisResult<()> {
        let before_loop = {
            let entry_point = self.reserve_cmd();
            let entry_state = self.save_flow();

            replace(
                &mut self.loop_state,
                Some(LoopState {
                    entry_point,
                    entry_state,
                    breaks: Vec::new(),
                }),
            )
        };

        if let Some(ScriptNode::Block { statements, .. }) = body.deref(self.doc) {
            self.assemble_statements(statements)?;
        }

        let Some(loop_state) = replace(&mut self.loop_state, before_loop) else {
            return Ok(());
        };

        match self.restore_flow(loop_state.entry_state) {
            FlowExecution::Normal => {
                let _ = self.cmd_jump_to_the_past(loop_state.entry_point);

                self.flow_state.execution = match loop_state.breaks.is_empty() {
                    true => FlowExecution::Unreachable,
                    false => FlowExecution::Normal,
                };
            }

            FlowExecution::Continue | FlowExecution::Return | FlowExecution::Unreachable => {
                self.flow_state.execution = match loop_state.breaks.is_empty() {
                    true => FlowExecution::Unreachable,
                    false => FlowExecution::Normal,
                };
            }

            FlowExecution::Break => {
                self.flow_state.execution = FlowExecution::Normal;
            }
        }

        if let FlowExecution::Normal = self.flow_state.execution {
            let loop_end = self.reserve_cmd();

            for break_cmd in loop_state.breaks {
                self.set_cmd_jump_target(break_cmd, loop_end);
            }
        }

        Ok(())
    }

    fn assemble_block(&mut self, statements: &[NodeRef]) -> AnalysisResult<()> {
        let state_before = self.save_flow();

        self.assemble_statements(statements)?;

        let exec = self.restore_flow(state_before);

        self.flow_state.execution = exec;

        Ok(())
    }

    fn assemble_statements(&mut self, statements: &[NodeRef]) -> AnalysisResult<()> {
        self.context.proceed()?;

        for st in statements {
            self.assemble_st(st)?;

            let FlowExecution::Normal = self.flow_state.execution else {
                break;
            };
        }

        Ok(())
    }

    fn assemble_st(&mut self, st: &NodeRef) -> AnalysisResult<()> {
        let Some(script_node) = st.deref(self.doc) else {
            return Ok(());
        };

        match script_node {
            ScriptNode::Clause { expr, .. } => self.assemble_clause(expr),

            ScriptNode::If {
                condition, body, ..
            } => self.assemble_if(condition, body),

            ScriptNode::Match { subject, body, .. } => self.assemble_match(subject, body),

            ScriptNode::Let { name, value, .. } => self.assemble_let(name, value),

            ScriptNode::For {
                iterator,
                range,
                body,
                ..
            } => self.assemble_for(iterator, range, body),

            ScriptNode::Loop { body, .. } => self.assemble_loop(body),

            ScriptNode::Block { statements, .. } => self.assemble_block(statements),

            ScriptNode::Break { .. } => self.assemble_break(),

            ScriptNode::Continue { .. } => self.assemble_continue(),

            ScriptNode::Return { result, .. } => self.assemble_return(result),

            _ => Ok(()),
        }
    }

    fn assemble_break(&mut self) -> AnalysisResult<()> {
        let Some(loop_state) = &self.loop_state else {
            return Ok(());
        };

        let entry_state = loop_state.entry_state.clone();

        self.restore_flow(entry_state);

        let break_cmd = self.cmd_jump_to_the_future();

        let Some(loop_state) = &mut self.loop_state else {
            system_panic!("Malformed loop state.");
        };

        loop_state.breaks.push(break_cmd);

        self.flow_state.execution = FlowExecution::Break;

        Ok(())
    }

    fn assemble_continue(&mut self) -> AnalysisResult<()> {
        let Some(loop_state) = &self.loop_state else {
            return Ok(());
        };

        let entry_state = loop_state.entry_state.clone();
        let entry_point = loop_state.entry_point;

        self.restore_flow(entry_state);

        let _ = self.cmd_jump_to_the_past(entry_point);

        self.flow_state.execution = FlowExecution::Continue;

        Ok(())
    }

    fn assemble_return(&mut self, result: &NodeRef) -> AnalysisResult<()> {
        self.assemble_expr(result)?;

        let top = self.stack_top();

        if top > 0 {
            let _ = self.cmd_swap(0);
            self.shrink_stack(top);
            let _ = self.cmd_jump_to_the_future();
        }

        self.flow_state.execution = FlowExecution::Return;

        Ok(())
    }

    fn assemble_expr(&mut self, expr: &NodeRef) -> AnalysisResult<()> {
        let Some(script_node) = expr.deref(self.doc) else {
            let _ = self.cmd_push_nil();
            return Ok(());
        };

        match script_node {
            ScriptNode::Fn {
                keyword, semantics, ..
            } => self.assemble_fn(expr, keyword, semantics)?,

            ScriptNode::Struct { keyword, body, .. } => self.assemble_struct(keyword, body)?,

            ScriptNode::Array { items, .. } => self.assemble_array(expr, items)?,

            ScriptNode::String { start, end, .. } => self.assemble_string(start, end)?,

            ScriptNode::Crate { token, .. } => self.assemble_crate(token)?,

            ScriptNode::This { .. } => self.assemble_this()?,

            ScriptNode::Ident { token, .. } => self.assemble_ident(expr, token)?,

            ScriptNode::Number {
                token, semantics, ..
            } => self.assemble_number(token, semantics)?,

            ScriptNode::Max { token, .. } => self.assemble_max(token)?,

            ScriptNode::Bool { token, .. } => self.assemble_bool(token)?,

            ScriptNode::UnaryLeft { op, right, .. } => self.assembly_unary_left(op, right)?,

            ScriptNode::Binary {
                left, op, right, ..
            } => self.assembly_binary(left, op, right)?,

            ScriptNode::Query { left, op, .. } => self.assemble_query(op, left)?,

            ScriptNode::Call { left, args, .. } => self.assemble_call(left, args)?,

            ScriptNode::Index { left, arg, .. } => self.assemble_index(left, arg)?,

            ScriptNode::Expr { inner, .. } => self.assemble_expr(inner)?,

            _ => {
                let _ = self.cmd_push_nil();
            }
        }

        Ok(())
    }

    fn assemble_fn(
        &mut self,
        fn_ref: &NodeRef,
        keyword: &TokenRef,
        semantics: &Semantics<FnSemantics>,
    ) -> AnalysisResult<()> {
        let origin = ScriptOrigin::from(keyword);

        let _ = self.cmd_push_fn(origin, fn_ref);

        let fn_semantics = semantics.get().forward()?;

        let closure_vec = fn_semantics
            .compilation
            .closure_vec
            .read(self.context)
            .forward()?;

        let closure_drops = self.closure_drops.get(fn_ref);

        for (index, closure_name) in closure_vec.vec.iter().enumerate() {
            let closure_name = closure_name.as_str();

            match self.flow_state.namespace.get(closure_name) {
                None => match self.closure_index.get(closure_name) {
                    None => {
                        let _ = self.cmd_push_nil();
                    }

                    Some(my_closure_index) => {
                        let _ = self.cmd_push_closure(*my_closure_index);
                    }
                },

                Some(name_depth) => {
                    let dropped = closure_drops
                        .filter(|drops| drops.contains(closure_name))
                        .is_some();

                    let _ = match dropped {
                        true => self.cmd_lift(*name_depth),
                        false => self.cmd_dup(*name_depth),
                    };
                }
            }

            let _ = self.cmd_bind(index + 1);
        }

        Ok(())
    }

    fn assemble_struct(&mut self, keyword: &TokenRef, body: &NodeRef) -> AnalysisResult<()> {
        let origin = ScriptOrigin::from(keyword);

        let _ = self.cmd_push_struct(origin);

        let struct_index = self.stack_top();

        if let Some(ScriptNode::StructBody { entries, .. }) = body.deref(self.doc) {
            for entry_ref in entries {
                let Some(ScriptNode::StructEntry { key, value, .. }) = entry_ref.deref(self.doc)
                else {
                    continue;
                };

                let Some(ScriptNode::StructEntryKey { token, .. }) = key.deref(self.doc) else {
                    self.assemble_clause(value)?;
                    continue;
                };

                let Some(key_string) = token.string(self.doc) else {
                    self.assemble_clause(value)?;
                    continue;
                };

                let field = self.store_string(key_string);
                let field_origin = ScriptOrigin::from(token);

                let value_origin = value.script_origin(self.doc, SpanBounds::Cover);

                self.assemble_expr(value)?;

                let _ = self.cmd_dup(struct_index);
                let _ = self.cmd_field(origin, field_origin, field);

                let _ =
                    self.cmd_op_assignment(field_origin, value_origin, field_origin, OpCmd::Assign);
            }
        }

        Ok(())
    }

    fn assemble_array(&mut self, array_ref: &NodeRef, items: &[NodeRef]) -> AnalysisResult<()> {
        if items.is_empty() {
            self.cmd_push_nil();

            return Ok(());
        }

        let len = items.len();

        let mut origins = Vec::with_capacity(len + 1);

        let array_origin = array_ref.script_origin(self.doc, SpanBounds::Cover);

        for item in items {
            origins.push(item.script_origin(self.doc, SpanBounds::Cover));

            self.assemble_expr(item)?;
        }

        origins.push(array_origin);

        let _ = self.cmd_concat(len, origins);

        Ok(())
    }

    fn assemble_string(&mut self, start: &TokenRef, end: &TokenRef) -> AnalysisResult<()> {
        let mut origin = ScriptOrigin::from(start);

        origin.union(&ScriptOrigin::from(end));

        let Some(start) = start.site(self.doc) else {
            let _ = self.cmd_push_nil();
            return Ok(());
        };

        let Some(end) = end.site(self.doc) else {
            let _ = self.cmd_push_nil();
            return Ok(());
        };

        let span = (start + 1)..end;

        if !span.is_valid_span(self.doc) {
            let _ = self.cmd_push_nil();
            return Ok(());
        }

        let string = self.doc.substring(span).into_owned();

        let index = self.store_string(string);

        let _ = self.cmd_push_string(origin, index);

        Ok(())
    }

    fn assemble_crate(&mut self, token: &TokenRef) -> AnalysisResult<()> {
        let origin = ScriptOrigin::from(token);
        let _ = self.cmd_push_package(origin, self.package);

        Ok(())
    }

    fn assemble_this(&mut self) -> AnalysisResult<()> {
        let _ = self.cmd_push_closure(0);

        Ok(())
    }

    fn assemble_ident(&mut self, ident_ref: &NodeRef, token: &TokenRef) -> AnalysisResult<()> {
        match self.ident_desc_map.get(ident_ref) {
            Some(IdentDesc::LocalRead) => {
                let Some(name) = token.string(self.doc) else {
                    let _ = self.cmd_push_nil();
                    return Ok(());
                };

                let Some(depth) = self.flow_state.namespace.get(name) else {
                    let _ = self.cmd_push_nil();
                    return Ok(());
                };

                let _ = match self.ident_drops.contains(ident_ref) {
                    true => self.cmd_lift(*depth),
                    false => self.cmd_dup(*depth),
                };
            }

            Some(IdentDesc::Closure) => {
                let Some(name) = token.string(self.doc) else {
                    let _ = self.cmd_push_nil();
                    return Ok(());
                };

                let Some(index) = self.closure_index.get(name) else {
                    let _ = self.cmd_push_nil();
                    return Ok(());
                };

                let _ = self.cmd_push_closure(*index);
            }

            Some(IdentDesc::Import(package)) => {
                let Some(name) = token.string(self.doc) else {
                    let _ = self.cmd_push_nil();
                    return Ok(());
                };

                let field_origin = ScriptOrigin::from(token);
                let field = self.store_string(name);

                let _ = self.cmd_push_package(field_origin, *package);
                let _ = self.cmd_field(field_origin, field_origin, field);
            }

            _ => {
                let _ = self.cmd_push_nil();
            }
        }

        Ok(())
    }

    fn assemble_number(
        &mut self,
        token: &TokenRef,
        semantics: &Semantics<NumberSemantics>,
    ) -> AnalysisResult<()> {
        let number_semantics = semantics.get().forward()?;

        let number_value = number_semantics.number_value.read(self.context).forward()?;

        let origin = ScriptOrigin::from(token);

        let _ = match number_value.deref() {
            LocalNumberValue::Usize(Ok(value)) => self.cmd_push_usize(origin, *value),
            LocalNumberValue::Isize(Ok(value)) => self.cmd_push_isize(origin, *value),
            LocalNumberValue::Float(Ok(value)) => self.cmd_push_float(origin, *value),
            _ => self.cmd_push_nil(),
        };

        Ok(())
    }

    fn assemble_max(&mut self, token: &TokenRef) -> AnalysisResult<()> {
        let origin = ScriptOrigin::from(token);

        self.cmd_push_usize(origin, usize::MAX);

        Ok(())
    }

    fn assemble_bool(&mut self, token: &TokenRef) -> AnalysisResult<()> {
        let origin = ScriptOrigin::from(token);

        match token.deref(self.doc) {
            Some(ScriptToken::True) => {
                let _ = self.cmd_push_true(origin);
            }

            Some(ScriptToken::False) => {
                let _ = self.cmd_push_false(origin);
            }

            _ => {
                let _ = self.cmd_push_nil();
            }
        }

        Ok(())
    }

    fn assembly_unary_left(&mut self, op: &NodeRef, right: &NodeRef) -> AnalysisResult<()> {
        let Some(ScriptNode::Op { token, .. }) = op.deref(self.doc) else {
            let _ = self.cmd_push_nil();
            return Ok(());
        };

        let op_origin = ScriptOrigin::from(token);

        let Some(op) = token.deref(self.doc) else {
            let _ = self.cmd_push_nil();
            return Ok(());
        };

        let op = match op {
            ScriptToken::Mul => OpCmd::Clone,
            ScriptToken::Minus => OpCmd::Neg,
            ScriptToken::Not => OpCmd::Not,

            _ => {
                let _ = self.cmd_push_nil();
                return Ok(());
            }
        };

        let rhs_origin = right.script_origin(self.doc, SpanBounds::Cover);

        self.assemble_expr(right)?;

        let _ = self.cmd_op_unary(op_origin, rhs_origin, op);

        Ok(())
    }

    fn assembly_binary(
        &mut self,
        left: &NodeRef,
        op: &NodeRef,
        right: &NodeRef,
    ) -> AnalysisResult<()> {
        let Some(ScriptNode::Op { token, .. }) = op.deref(self.doc) else {
            let _ = self.cmd_push_nil();
            return Ok(());
        };

        let Some(op) = token.deref(self.doc) else {
            let _ = self.cmd_push_nil();
            return Ok(());
        };

        match op {
            ScriptToken::Assign => self.assemble_binary_assign(left, token, right)?,

            ScriptToken::PlusAssign => {
                self.assemble_binary_assignment(left, token, OpCmd::AddAssign, right)?
            }

            ScriptToken::MinusAssign => {
                self.assemble_binary_assignment(left, token, OpCmd::SubAssign, right)?
            }

            ScriptToken::MulAssign => {
                self.assemble_binary_assignment(left, token, OpCmd::MulAssign, right)?
            }

            ScriptToken::DivAssign => {
                self.assemble_binary_assignment(left, token, OpCmd::DivAssign, right)?
            }

            ScriptToken::BitAndAssign => {
                self.assemble_binary_assignment(left, token, OpCmd::BitAndAssign, right)?
            }

            ScriptToken::BitOrAssign => {
                self.assemble_binary_assignment(left, token, OpCmd::BitOrAssign, right)?
            }

            ScriptToken::BitXorAssign => {
                self.assemble_binary_assignment(left, token, OpCmd::BitXorAssign, right)?
            }

            ScriptToken::ShlAssign => {
                self.assemble_binary_assignment(left, token, OpCmd::ShlAssign, right)?
            }

            ScriptToken::ShrAssign => {
                self.assemble_binary_assignment(left, token, OpCmd::ShrAssign, right)?
            }

            ScriptToken::RemAssign => {
                self.assemble_binary_assignment(left, token, OpCmd::RemAssign, right)?
            }

            ScriptToken::Dot2 => self.assemble_binary_range(left, token, right)?,

            ScriptToken::Dot => self.assemble_binary_field(left, right)?,

            ScriptToken::Plus => self.assemble_binary_op(left, token, OpCmd::Add, right)?,

            ScriptToken::Minus => self.assemble_binary_op(left, token, OpCmd::Sub, right)?,

            ScriptToken::Mul => self.assemble_binary_op(left, token, OpCmd::Mul, right)?,

            ScriptToken::Div => self.assemble_binary_op(left, token, OpCmd::Div, right)?,

            ScriptToken::BitAnd => self.assemble_binary_op(left, token, OpCmd::BitAnd, right)?,

            ScriptToken::BitOr => self.assemble_binary_op(left, token, OpCmd::BitOr, right)?,

            ScriptToken::BitXor => self.assemble_binary_op(left, token, OpCmd::BitXor, right)?,

            ScriptToken::Shl => self.assemble_binary_op(left, token, OpCmd::Shl, right)?,

            ScriptToken::Shr => self.assemble_binary_op(left, token, OpCmd::Shr, right)?,

            ScriptToken::Rem => self.assemble_binary_op(left, token, OpCmd::Rem, right)?,

            ScriptToken::Lesser => self.assemble_binary_op(left, token, OpCmd::Lesser, right)?,

            ScriptToken::LesserOrEqual => {
                self.assemble_binary_op(left, token, OpCmd::LesserOrEqual, right)?
            }

            ScriptToken::Greater => self.assemble_binary_op(left, token, OpCmd::Greater, right)?,

            ScriptToken::GreaterOrEqual => {
                self.assemble_binary_op(left, token, OpCmd::GreaterOrEqual, right)?
            }

            ScriptToken::Equal => self.assemble_binary_op(left, token, OpCmd::Equal, right)?,

            ScriptToken::NotEqual => {
                self.assemble_binary_op(left, token, OpCmd::NotEqual, right)?
            }

            ScriptToken::And => self.assemble_binary_op(left, token, OpCmd::And, right)?,

            ScriptToken::Or => self.assemble_binary_op(left, token, OpCmd::Or, right)?,

            _ => {
                let _ = self.cmd_push_nil();
            }
        }

        Ok(())
    }

    fn assemble_binary_assign(
        &mut self,
        left: &NodeRef,
        op_token: &TokenRef,
        right: &NodeRef,
    ) -> AnalysisResult<()> {
        let Some(ScriptNode::Ident { token, .. }) = left.deref(self.doc) else {
            return self.assemble_binary_assignment(left, op_token, OpCmd::Assign, right);
        };

        let Some(IdentDesc::LocalWrite) = self.ident_desc_map.get(left) else {
            return self.assemble_binary_assignment(left, op_token, OpCmd::Assign, right);
        };

        let Some(token_string) = token.string(self.doc) else {
            return self.assemble_binary_assignment(left, op_token, OpCmd::Assign, right);
        };

        let Some(depth) = self.flow_state.namespace.get(token_string) else {
            return self.assemble_binary_assignment(left, op_token, OpCmd::Assign, right);
        };

        let depth = *depth;

        self.assemble_expr(right)?;

        let _ = self.cmd_swap(depth);

        Ok(())
    }

    fn assemble_binary_assignment(
        &mut self,
        left: &NodeRef,
        op_token: &TokenRef,
        op: OpCmd,
        right: &NodeRef,
    ) -> AnalysisResult<()> {
        let op_origin = ScriptOrigin::from(op_token);
        let lhs_origin = left.script_origin(self.doc, SpanBounds::Cover);
        let rhs_origin = right.script_origin(self.doc, SpanBounds::Cover);

        self.assemble_expr(right)?;
        self.assemble_expr(left)?;

        let _ = self.cmd_op_assignment(op_origin, rhs_origin, lhs_origin, op);
        let _ = self.cmd_push_nil();

        Ok(())
    }

    fn assemble_binary_range(
        &mut self,
        left: &NodeRef,
        op_token: &TokenRef,
        right: &NodeRef,
    ) -> AnalysisResult<()> {
        let op_origin = ScriptOrigin::from(op_token);
        let lhs_origin = left.script_origin(self.doc, SpanBounds::Cover);
        let rhs_origin = right.script_origin(self.doc, SpanBounds::Cover);

        self.assemble_expr(left)?;
        self.assemble_expr(right)?;

        let _ = self.cmd_range(op_origin, lhs_origin, rhs_origin);

        Ok(())
    }

    fn assemble_binary_field(&mut self, left: &NodeRef, right: &NodeRef) -> AnalysisResult<()> {
        let Some(ScriptNode::Field { token, .. }) = right.deref(self.doc) else {
            let _ = self.cmd_push_nil();
            return Ok(());
        };

        match token.deref(self.doc) {
            Some(ScriptToken::Len) => {
                let lhs_origin = left.script_origin(self.doc, SpanBounds::Cover);
                let field_origin = ScriptOrigin::from(token);

                self.assemble_expr(left)?;

                let _ = self.cmd_len(lhs_origin, field_origin);

                return Ok(());
            }

            _ => (),
        }

        let Some(field_string) = token.string(self.doc) else {
            let _ = self.cmd_push_nil();
            return Ok(());
        };

        let lhs_origin = left.script_origin(self.doc, SpanBounds::Cover);
        let field_origin = ScriptOrigin::from(token);
        let field = self.store_string(field_string);

        self.assemble_expr(left)?;

        let _ = self.cmd_field(lhs_origin, field_origin, field);

        Ok(())
    }

    fn assemble_binary_op(
        &mut self,
        left: &NodeRef,
        op_token: &TokenRef,
        op: OpCmd,
        right: &NodeRef,
    ) -> AnalysisResult<()> {
        let op_origin = ScriptOrigin::from(op_token);
        let lhs_origin = left.script_origin(self.doc, SpanBounds::Cover);
        let rhs_origin = right.script_origin(self.doc, SpanBounds::Cover);

        self.assemble_expr(left)?;
        self.assemble_expr(right)?;

        let _ = self.cmd_op_binary(op_origin, lhs_origin, rhs_origin, op);

        Ok(())
    }

    fn assemble_query(&mut self, op: &NodeRef, left: &NodeRef) -> AnalysisResult<()> {
        let op_origin = op.script_origin(self.doc, SpanBounds::Cover);

        self.assemble_expr(left)?;

        let _ = self.cmd_query(op_origin);

        Ok(())
    }

    fn assemble_call(&mut self, left: &NodeRef, args: &NodeRef) -> AnalysisResult<()> {
        let lhs_origin = left.script_origin(self.doc, SpanBounds::Cover);
        let rhs_origin = args.script_origin(self.doc, SpanBounds::Cover);

        let Some(ScriptNode::CallArgs { args, .. }) = args.deref(self.doc) else {
            self.cmd_push_nil();
            return Ok(());
        };

        let arity = args.len();

        let mut origins = Vec::with_capacity(arity + 2);

        for arg_ref in args {
            let arg_origin = arg_ref.script_origin(self.doc, SpanBounds::Cover);

            self.assemble_expr(arg_ref)?;

            origins.push(arg_origin);
        }

        origins.push(lhs_origin);
        origins.push(rhs_origin);

        self.assemble_expr(left)?;

        let _ = self.cmd_invoke(arity, origins);

        Ok(())
    }

    fn assemble_index(&mut self, left: &NodeRef, arg: &NodeRef) -> AnalysisResult<()> {
        let lhs_origin = left.script_origin(self.doc, SpanBounds::Cover);
        let rhs_origin = arg.script_origin(self.doc, SpanBounds::Cover);

        let Some(ScriptNode::IndexArg { arg, .. }) = arg.deref(self.doc) else {
            self.cmd_push_nil();
            return Ok(());
        };

        let index_origin = arg.script_origin(self.doc, SpanBounds::Cover);

        self.assemble_expr(arg)?;
        self.assemble_expr(left)?;

        let _ = self.cmd_index(index_origin, lhs_origin, rhs_origin);

        Ok(())
    }

    #[inline(always)]
    fn cmd_if_true(&mut self, condition_origin: impl Into<Origin>) -> CmdIndex {
        self.dec_stack(1);

        self.cmd_1(condition_origin, Cmd::IfTrue(IfTrueCmd { otherwise: RET }))
    }

    #[inline(always)]
    fn cmd_if_false(&mut self, condition_origin: impl Into<Origin>) -> CmdIndex {
        self.dec_stack(1);

        self.cmd_1(
            condition_origin,
            Cmd::IfFalse(IfFalseCmd { otherwise: RET }),
        )
    }

    #[inline(always)]
    fn cmd_jump_to_the_past(&mut self, past_cmd: CmdIndex) -> CmdIndex {
        self.cmd_0(Cmd::Jump(JumpCmd { command: past_cmd }))
    }

    #[inline(always)]
    fn cmd_jump_to_the_future(&mut self) -> CmdIndex {
        self.cmd_0(Cmd::Jump(JumpCmd { command: RET }))
    }

    #[inline(always)]
    fn cmd_iterate(&mut self, range_origin: impl Into<Origin>) -> CmdIndex {
        self.inc_stack(1);

        let cmd = self.cmd_1(range_origin, Cmd::Iterate(IterateCmd { finish: RET }));

        cmd
    }

    #[inline(always)]
    fn cmd_swap(&mut self, depth: StackDepth) -> CmdIndex {
        let top = self.stack_top();

        if depth == top && self.committed < self.assembly.commands.len() {
            return RET;
        }

        self.cmd_0(Cmd::Swap(SwapCmd { depth }))
    }

    #[inline(always)]
    fn cmd_lift(&mut self, depth: StackDepth) -> CmdIndex {
        let top = self.stack_top();

        if depth == top && self.committed < self.assembly.commands.len() {
            return RET;
        }

        self.inc_stack(1);

        self.cmd_0(Cmd::Lift(LiftCmd { depth }))
    }

    #[inline(always)]
    fn cmd_dup(&mut self, depth: StackDepth) -> CmdIndex {
        self.inc_stack(1);

        self.cmd_0(Cmd::Dup(DupCmd { depth }))
    }

    #[inline(always)]
    fn cmd_push_nil(&mut self) -> CmdIndex {
        self.inc_stack(1);

        self.cmd_0(Cmd::PushNil(PushNilCmd))
    }

    #[inline(always)]
    fn cmd_push_true(&mut self, origin: impl Into<Origin>) -> CmdIndex {
        self.inc_stack(1);

        self.cmd_1(origin, Cmd::PushTrue(PushTrueCmd))
    }

    #[inline(always)]
    fn cmd_push_false(&mut self, origin: impl Into<Origin>) -> CmdIndex {
        self.inc_stack(1);

        self.cmd_1(origin, Cmd::PushFalse(PushFalseCmd))
    }

    #[inline(always)]
    fn cmd_push_usize(&mut self, origin: impl Into<Origin>, value: usize) -> CmdIndex {
        self.inc_stack(1);

        self.cmd_1(origin, Cmd::PushUsize(PushUsizeCmd { value }))
    }

    #[inline(always)]
    fn cmd_push_isize(&mut self, origin: impl Into<Origin>, value: isize) -> CmdIndex {
        self.inc_stack(1);

        self.cmd_1(origin, Cmd::PushIsize(PushIsizeCmd { value }))
    }

    #[inline(always)]
    fn cmd_push_float(&mut self, origin: impl Into<Origin>, value: Float) -> CmdIndex {
        self.inc_stack(1);

        self.cmd_1(origin, Cmd::PushFloat(PushFloatCmd { value }))
    }

    #[inline(always)]
    fn cmd_push_string(
        &mut self,
        origin: impl Into<Origin>,
        string_index: StringIndex,
    ) -> CmdIndex {
        self.inc_stack(1);

        self.cmd_1(origin, Cmd::PushString(PushStringCmd { string_index }))
    }

    #[inline(always)]
    fn cmd_push_package(
        &mut self,
        origin: impl Into<Origin>,
        package: &'static PackageMeta,
    ) -> CmdIndex {
        self.inc_stack(1);

        self.cmd_1(origin, Cmd::PushPackage(PushPackageCmd { package }))
    }

    #[inline(always)]
    fn cmd_push_closure(&mut self, index: ClosureIndex) -> CmdIndex {
        self.inc_stack(1);

        self.cmd_0(Cmd::PushClosure(PushClosureCmd { index }))
    }

    #[inline(always)]
    fn cmd_push_fn(&mut self, fn_origin: impl Into<Origin>, fn_ref: &NodeRef) -> CmdIndex {
        self.inc_stack(1);

        let index = self.assembly.subroutines.len();

        self.assembly.subroutines.push(*fn_ref);

        self.cmd_1(fn_origin, Cmd::PushFn(PushFnCmd { index }))
    }

    #[inline(always)]
    fn cmd_push_struct(&mut self, struct_origin: impl Into<Origin>) -> CmdIndex {
        self.inc_stack(1);

        self.cmd_1(struct_origin, Cmd::PushStruct(PushStructCmd))
    }

    #[inline(always)]
    fn cmd_range(
        &mut self,
        range_origin: impl Into<Origin>,
        lhs_origin: impl Into<Origin>,
        rhs_origin: impl Into<Origin>,
    ) -> CmdIndex {
        self.dec_stack(1);

        self.cmd_3(range_origin, lhs_origin, rhs_origin, Cmd::Range(RangeCmd))
    }

    #[inline(always)]
    fn cmd_bind(&mut self, index: ClosureIndex) -> CmdIndex {
        self.dec_stack(1);

        self.cmd_0(Cmd::Bind(BindCmd { index }))
    }

    #[inline(always)]
    fn cmd_concat(&mut self, items: usize, origins: Vec<ScriptOrigin>) -> CmdIndex {
        self.dec_stack(items);
        self.inc_stack(1);

        self.cmd_many(origins, Cmd::Concat(ConcatCmd { items }))
    }

    #[inline(always)]
    fn cmd_field(
        &mut self,
        lhs_origin: impl Into<Origin>,
        field_origin: impl Into<Origin>,
        field_index: StringIndex,
    ) -> CmdIndex {
        self.cmd_2(
            lhs_origin,
            field_origin,
            Cmd::Field(FieldCmd { field_index }),
        )
    }

    #[inline(always)]
    fn cmd_len(
        &mut self,
        lhs_origin: impl Into<Origin>,
        field_origin: impl Into<Origin>,
    ) -> CmdIndex {
        self.cmd_2(lhs_origin, field_origin, Cmd::Len(LenCmd))
    }

    #[inline(always)]
    fn cmd_query(&mut self, op_origin: impl Into<Origin>) -> CmdIndex {
        self.cmd_1(op_origin, Cmd::Query(QueryCmd))
    }

    #[inline(always)]
    fn cmd_op_binary(
        &mut self,
        op_origin: impl Into<Origin>,
        lhs_origin: impl Into<Origin>,
        rhs_origin: impl Into<Origin>,
        op: OpCmd,
    ) -> CmdIndex {
        self.dec_stack(1);

        self.cmd_3(op_origin, lhs_origin, rhs_origin, Cmd::Op(op))
    }

    #[inline(always)]
    fn cmd_op_assignment(
        &mut self,
        op_origin: impl Into<Origin>,
        rhs_origin: impl Into<Origin>,
        lhs_origin: impl Into<Origin>,
        op: OpCmd,
    ) -> CmdIndex {
        self.dec_stack(2);

        self.cmd_3(op_origin, rhs_origin, lhs_origin, Cmd::Op(op))
    }

    #[inline(always)]
    fn cmd_op_unary(
        &mut self,
        op_origin: impl Into<Origin>,
        rhs_origin: impl Into<Origin>,
        op: OpCmd,
    ) -> CmdIndex {
        self.cmd_2(op_origin, rhs_origin, Cmd::Op(op))
    }

    #[inline(always)]
    fn cmd_invoke(&mut self, arity: usize, origins: Vec<ScriptOrigin>) -> CmdIndex {
        self.dec_stack(arity);

        self.cmd_many(origins, Cmd::Invoke(InvokeCmd { arity }))
    }

    #[inline(always)]
    fn cmd_index(
        &mut self,
        index_origin: impl Into<Origin>,
        lhs_origin: impl Into<Origin>,
        rhs_origin: impl Into<Origin>,
    ) -> CmdIndex {
        self.dec_stack(1);

        self.cmd_3(index_origin, lhs_origin, rhs_origin, Cmd::Index(IndexCmd))
    }

    #[inline(always)]
    fn set_cmd_jump_target(&mut self, cmd_index: CmdIndex, target_point: CmdIndex) {
        let target = match self.assembly.commands.get_mut(cmd_index) {
            Some(Cmd::IfTrue(IfTrueCmd { otherwise, .. })) => otherwise,
            Some(Cmd::IfFalse(IfFalseCmd { otherwise, .. })) => otherwise,
            Some(Cmd::Jump(JumpCmd { command, .. })) => command,
            Some(Cmd::Iterate(IterateCmd { finish, .. })) => finish,
            _ => system_panic!("Malformed command index."),
        };

        *target = target_point;
    }

    #[inline(always)]
    fn reserve_cmd(&mut self) -> CmdIndex {
        self.committed = self.committed.max(self.assembly.commands.len() + 1);
        self.assembly.commands.len()
    }

    #[inline(always)]
    fn cmd_0(&mut self, cmd: Cmd) -> CmdIndex {
        let index = self.assembly.commands.len();

        self.assembly.commands.push(cmd);
        self.assembly.sources.push(Vec::new().into());

        index
    }

    #[inline(always)]
    fn cmd_1(&mut self, origin_1: impl Into<Origin>, cmd: Cmd) -> CmdIndex {
        let index = self.assembly.commands.len();

        self.assembly.commands.push(cmd);

        let source = vec![self.store_origin(origin_1)];

        self.assembly.sources.push(source.into());

        index
    }

    #[inline(always)]
    fn cmd_2(
        &mut self,
        origin_1: impl Into<Origin>,
        origin_2: impl Into<Origin>,
        cmd: Cmd,
    ) -> CmdIndex {
        let index = self.assembly.commands.len();

        self.assembly.commands.push(cmd);

        let source = vec![self.store_origin(origin_1), self.store_origin(origin_2)];

        self.assembly.sources.push(source.into());

        index
    }

    #[inline(always)]
    fn cmd_3(
        &mut self,
        origin_1: impl Into<Origin>,
        origin_2: impl Into<Origin>,
        origin_3: impl Into<Origin>,
        cmd: Cmd,
    ) -> CmdIndex {
        let index = self.assembly.commands.len();

        self.assembly.commands.push(cmd);

        let source = vec![
            self.store_origin(origin_1),
            self.store_origin(origin_2),
            self.store_origin(origin_3),
        ];

        self.assembly.sources.push(source.into());

        index
    }

    #[inline(always)]
    fn cmd_many(&mut self, origins: Vec<ScriptOrigin>, cmd: Cmd) -> CmdIndex {
        let index = self.assembly.commands.len();

        self.assembly.commands.push(cmd);

        let mut source = Vec::with_capacity(origins.len());

        for origin in origins {
            source.push(self.store_origin(origin));
        }

        self.assembly.sources.push(source.into());

        index
    }

    #[inline(always)]
    fn inc_stack(&mut self, diff: StackDepth) {
        self.flow_state.stack_depth += diff;

        if self.assembly.frame < self.flow_state.stack_depth {
            self.assembly.frame = self.assembly.frame.max(self.flow_state.stack_depth);
        }
    }

    #[inline(always)]
    fn dec_stack(&mut self, diff: StackDepth) {
        if self.flow_state.stack_depth < diff {
            system_panic!("Malformed stack state.");
        }

        self.flow_state.stack_depth -= diff;
    }

    #[inline(always)]
    fn shrink_stack(&mut self, mut diff: StackDepth) {
        if self.flow_state.stack_depth < diff {
            system_panic!("Malformed stack state.");
        }

        while diff > 0 && self.committed < self.assembly.commands.len() {
            match self.assembly.commands.last_mut() {
                Some(
                    Cmd::Dup(..) | Cmd::PushNil(..) | Cmd::PushClosure(..) | Cmd::PushPackage(..),
                ) => {
                    let _ = self.assembly.sources.pop();
                    let _ = self.assembly.commands.pop();
                    self.flow_state.stack_depth -= 1;
                    diff -= 1;
                }

                Some(
                    Cmd::PushTrue(..)
                    | Cmd::PushFalse(..)
                    | Cmd::PushUsize(..)
                    | Cmd::PushIsize(..)
                    | Cmd::PushFloat(..)
                    | Cmd::PushFn(..)
                    | Cmd::PushStruct(..),
                ) => {
                    let _ = self.assembly.sources.pop();
                    let _ = self.assembly.commands.pop();
                    self.flow_state.stack_depth -= 1;
                    diff -= 1;
                }

                Some(Cmd::PushString(PushStringCmd { string_index })) => {
                    if *string_index == self.assembly.strings.len() - 1 {
                        let Entry::Occupied(rev_entry) = self.string_rev_index.entry(*string_index)
                        else {
                            system_panic!("Malformed string index.")
                        };

                        let (refs, key) = rev_entry.get();

                        if *refs == 1 {
                            let _ = self.string_index.remove(key);
                            let _ = rev_entry.remove();
                            let _ = self.assembly.strings.pop();
                        }
                    }

                    let _ = self.assembly.sources.pop();
                    let _ = self.assembly.commands.pop();
                    self.flow_state.stack_depth -= 1;
                    diff -= 1;
                }

                Some(Cmd::Shrink(ShrinkCmd { depth })) => {
                    self.flow_state.stack_depth -= diff;
                    diff = 0;

                    if *depth > self.flow_state.stack_depth {
                        *depth = self.flow_state.stack_depth;
                    }

                    break;
                }

                _ => break,
            }
        }

        if diff == 0 {
            return;
        }

        self.flow_state.stack_depth -= diff;

        let depth = self.flow_state.stack_depth;

        let _ = self.cmd_0(Cmd::Shrink(ShrinkCmd { depth }));
    }

    #[inline(always)]
    fn stack_top(&self) -> StackDepth {
        match self.flow_state.stack_depth.checked_sub(1) {
            Some(top) => top,
            None => 0,
        }
    }

    #[inline(always)]
    fn save_flow(&mut self) -> FlowState<'doc> {
        self.committed = self.committed.max(self.assembly.commands.len());
        self.flow_state.clone()
    }

    #[inline(always)]
    fn restore_flow(&mut self, state: FlowState<'doc>) -> FlowExecution {
        match self.flow_state.execution {
            FlowExecution::Normal => {
                if state.stack_depth > self.flow_state.stack_depth {
                    system_panic!("Malformed stack.");
                }

                let diff = self.flow_state.stack_depth - state.stack_depth;

                self.shrink_stack(diff);

                self.flow_state = state;

                FlowExecution::Normal
            }

            other => {
                self.flow_state = state;

                other
            }
        }
    }

    #[inline(always)]
    fn shrink_ret(&mut self) {
        while self.committed < self.assembly.commands.len() {
            let Some(Cmd::Jump(JumpCmd { command })) = self.assembly.commands.last() else {
                break;
            };

            if *command < self.assembly.commands.len() {
                break;
            }

            let _ = self.assembly.sources.pop();
            let _ = self.assembly.commands.pop();
        }
    }

    #[inline(always)]
    fn store_origin(&mut self, origin: impl Into<Origin>) -> OriginIndex {
        let origin = origin.into();

        match self.origin_index.entry(origin) {
            Entry::Occupied(entry) => *entry.get(),

            Entry::Vacant(entry) => {
                let index = self.assembly.origins.len();

                entry.insert(index);

                self.assembly.origins.push(origin);

                index
            }
        }
    }

    #[inline(always)]
    fn store_string(&mut self, string: impl Into<Cow<'doc, str>>) -> StringIndex {
        let string = string.into();

        match self.string_index.entry(string.clone()) {
            Entry::Occupied(entry) => {
                let index = entry.get();

                let Some((refs, _)) = self.string_rev_index.get_mut(index) else {
                    system_panic!("Malformed string index.")
                };

                *refs += 1;

                *index
            }

            Entry::Vacant(entry) => {
                let index = self.assembly.strings.len();

                let _ = self.string_rev_index.insert(index, (1, string.clone()));

                entry.insert(index);

                self.assembly.strings.push(CompactString::from(string));

                index
            }
        }
    }
}

#[derive(Clone)]
struct FlowState<'doc> {
    namespace: AHashMap<&'doc str, StackDepth>,
    stack_depth: StackDepth,
    execution: FlowExecution,
}

struct LoopState<'doc> {
    entry_point: CmdIndex,
    entry_state: FlowState<'doc>,
    breaks: Vec<CmdIndex>,
}

#[derive(Clone, Copy)]
enum FlowExecution {
    Normal = 1,
    Break = 2,
    Continue = 3,
    Return = 4,
    Unreachable = 5,
}
