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

use ahash::{AHashMap, AHashSet};
use lady_deirdre::{
    analysis::{AnalysisResult, AttrContext, Computable, Grammar, SharedComputable, TaskHandle},
    sync::{Shared, SyncBuildHasher},
    syntax::NodeRef,
};

use crate::{analysis::ModuleResultEx, semantics::*, syntax::ScriptNode};

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalFlow {
    pub(crate) return_points: Shared<ReturnPointsSet>,
    pub(crate) loop_to_break: Shared<LoopToBreakMap>,
    pub(crate) break_to_loop: Shared<BreakToLoopMap>,
    pub(crate) unreachable_statements: Shared<UnreachableStatementsSet>,
    pub(crate) unreachable_arms: Shared<UnreachableArmsSet>,
}

impl SharedComputable for LocalFlow {
    type Node = ScriptNode;

    #[inline(always)]
    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(script_node) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let locals = script_node.locals().forward()?;

        let analysis = locals.analysis.read(context).forward()?;

        Ok(analysis.flow.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct ReturnPointsSet {
    pub(crate) set: AHashSet<LocalReturnPoint>,
}

impl SharedComputable for ReturnPointsSet {
    type Node = ScriptNode;

    #[inline(always)]
    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(script_node) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let locals = script_node.locals().forward()?;

        let flow = locals.flow.read(context).forward()?;

        Ok(flow.as_ref().return_points.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LoopToBreakMap {
    pub(crate) map: AHashMap<NodeRef, Shared<BreaksSet>>,
}

impl SharedComputable for LoopToBreakMap {
    type Node = ScriptNode;

    #[inline(always)]
    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(script_node) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let locals = script_node.locals().forward()?;

        let flow = locals.flow.read(context).forward()?;

        Ok(flow.as_ref().loop_to_break.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct BreakToLoopMap {
    pub(crate) map: AHashMap<NodeRef, NodeRef>,
}

impl SharedComputable for BreakToLoopMap {
    type Node = ScriptNode;

    #[inline(always)]
    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(script_node) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let locals = script_node.locals().forward()?;

        let flow = locals.flow.read(context).forward()?;

        Ok(flow.as_ref().break_to_loop.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LoopContext {
    pub(crate) loop_ref: NodeRef,
}

impl Computable for LoopContext {
    type Node = ScriptNode;

    #[inline(always)]
    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(script_node) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let scope_attr = script_node
            .scope_attr()
            .forward()?
            .read(context)
            .forward()?;

        let Some(scope_node) = scope_attr.scope_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let locals = scope_node.locals().forward()?;

        let break_to_loop = locals.break_to_loop.read(context).forward()?;

        Ok(Self {
            loop_ref: break_to_loop
                .as_ref()
                .map
                .get(node_ref)
                .copied()
                .unwrap_or_default(),
        })
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct UnreachableStatementsSet {
    pub(crate) set: AHashSet<NodeRef>,
}

impl SharedComputable for UnreachableStatementsSet {
    type Node = ScriptNode;

    #[inline(always)]
    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(script_node) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let locals = script_node.locals().forward()?;

        let flow = locals.flow.read(context).forward()?;

        Ok(flow.as_ref().unreachable_statements.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct UnreachableArmsSet {
    pub(crate) set: AHashSet<NodeRef>,
}

impl SharedComputable for UnreachableArmsSet {
    type Node = ScriptNode;

    #[inline(always)]
    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(script_node) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let locals = script_node.locals().forward()?;

        let flow = locals.flow.read(context).forward()?;

        Ok(flow.as_ref().unreachable_arms.clone())
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum LocalReturnPoint {
    Implicit,
    Explicit(NodeRef),
    Expr(NodeRef),
}

impl Default for LocalReturnPoint {
    #[inline(always)]
    fn default() -> Self {
        Self::Implicit
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct BreaksSet {
    pub(crate) set: AHashSet<NodeRef>,
}

impl SharedComputable for BreaksSet {
    type Node = ScriptNode;

    #[inline(always)]
    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(script_node) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let scope_attr = script_node
            .scope_attr()
            .forward()?
            .read(context)
            .forward()?;

        let Some(scope_node) = scope_attr.scope_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let locals = scope_node.locals().forward()?;

        let loop_to_break = locals.loop_to_break.read(context).forward()?;

        Ok(loop_to_break
            .as_ref()
            .map
            .get(node_ref)
            .cloned()
            .unwrap_or_default())
    }
}
