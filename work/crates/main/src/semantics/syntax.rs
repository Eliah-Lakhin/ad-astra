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
    num::{ParseFloatError, ParseIntError},
    ops::Deref,
};

use ahash::{AHashMap, AHashSet};
use compact_str::CompactString;
use lady_deirdre::{
    analysis::{AnalysisResult, AttrContext, Computable, Grammar, SharedComputable, TaskHandle},
    sync::{Shared, SyncBuildHasher},
    syntax::{NodeRef, NIL_NODE_REF},
};

use crate::{
    analysis::ModuleResultEx,
    semantics::*,
    syntax::{ScriptNode, ScriptToken},
};

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalSyntax {
    pub(crate) sig: Shared<LocalSig>,
    pub(crate) ifs: Shared<LocalIfMap>,
    pub(crate) fors: Shared<LocalForMap>,
    pub(crate) matches: Shared<LocalMatchMap>,
    pub(crate) struct_entry_vecs: Shared<LocalStructEntryVecs>,
    pub(crate) struct_entry_maps: Shared<LocalStructEntryMaps>,
    pub(crate) arrays: Shared<LocalArrayMap>,
    pub(crate) fn_contexts: Shared<LocalFnContextMap>,
    pub(crate) infixes: Shared<LocalInfixMap>,
    pub(crate) calls: Shared<LocalCallMap>,
    pub(crate) indexes: Shared<LocalIndexMap>,
    pub(crate) vars: Shared<LocalVarMap>,
    pub(crate) args: Shared<LocalArgsMap>,
    pub(crate) atoms: Shared<LocalAtomMap>,
    pub(crate) exprs: Shared<LocalExprMap>,
}

impl SharedComputable for LocalSyntax {
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

        Ok(analysis.syntax.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalSig {
    pub(crate) params: Vec<(bool, NodeRef)>,
}

impl SharedComputable for LocalSig {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().sig.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalIfMap {
    pub(crate) map: AHashMap<NodeRef, LocalIfSyntax>,
}

impl SharedComputable for LocalIfMap {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().ifs.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalForMap {
    pub(crate) map: AHashMap<NodeRef, LocalForSyntax>,
}

impl SharedComputable for LocalForMap {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().fors.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalMatchMap {
    pub(crate) map: AHashMap<NodeRef, Shared<LocalMatchSyntax>>,
}

impl SharedComputable for LocalMatchMap {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().matches.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalStructEntryVecs {
    pub(crate) map: AHashMap<NodeRef, Shared<LocalStructEntriesVecSyntax>>,
}

impl SharedComputable for LocalStructEntryVecs {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().struct_entry_vecs.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalStructEntryMaps {
    pub(crate) map: AHashMap<NodeRef, Shared<LocalStructEntiesMapSyntax>>,
}

impl SharedComputable for LocalStructEntryMaps {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().struct_entry_maps.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalArrayMap {
    pub(crate) map: AHashMap<NodeRef, Shared<LocalArraySyntax>>,
}

impl SharedComputable for LocalArrayMap {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().arrays.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalFnContextMap {
    pub(crate) map: AHashMap<NodeRef, LocalFnContextSyntax>,
}

impl SharedComputable for LocalFnContextMap {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().fn_contexts.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalInfixMap {
    pub(crate) map: AHashMap<NodeRef, LocalInfixSyntax>,
}

impl SharedComputable for LocalInfixMap {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().infixes.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalCallMap {
    pub(crate) map: AHashMap<NodeRef, Shared<LocalCallSyntax>>,
}

impl SharedComputable for LocalCallMap {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().calls.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalIndexMap {
    pub(crate) map: AHashMap<NodeRef, LocalIndexSyntax>,
}

impl SharedComputable for LocalIndexMap {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().indexes.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalVarMap {
    pub(crate) map: AHashMap<NodeRef, LocalVarSyntax>,
}

impl SharedComputable for LocalVarMap {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().vars.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalArgsMap {
    pub(crate) map: AHashMap<NodeRef, LocalArgSyntax>,
}

impl SharedComputable for LocalArgsMap {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().args.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalAtomMap {
    pub(crate) map: AHashMap<NodeRef, LocalAtomSyntax>,
}

impl SharedComputable for LocalAtomMap {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().atoms.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalExprMap {
    pub(crate) map: AHashMap<NodeRef, LocalExprSyntax>,
}

impl SharedComputable for LocalExprMap {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax.as_ref().exprs.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalIfSyntax {
    pub(crate) condition: NodeRef,
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalForSyntax {
    pub(crate) iterator: NodeRef,
    pub(crate) range: NodeRef,
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalMatchSyntax {
    pub(crate) subject: NodeRef,
    pub(crate) cases: AHashSet<NodeRef>,
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalStructEntriesVecSyntax {
    pub(crate) vec: Vec<(CompactString, NodeRef, NodeRef)>,
}

impl SharedComputable for LocalStructEntriesVecSyntax {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax
            .as_ref()
            .struct_entry_vecs
            .as_ref()
            .map
            .get(node_ref)
            .cloned()
            .unwrap_or_default())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalStructEntiesMapSyntax {
    pub(crate) map: AHashMap<CompactString, (NodeRef, NodeRef)>,
}

impl SharedComputable for LocalStructEntiesMapSyntax {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax
            .as_ref()
            .struct_entry_maps
            .as_ref()
            .map
            .get(node_ref)
            .cloned()
            .unwrap_or_default())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalArraySyntax {
    pub(crate) items: Vec<NodeRef>,
}

impl SharedComputable for LocalArraySyntax {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax
            .as_ref()
            .arrays
            .as_ref()
            .map
            .get(node_ref)
            .cloned()
            .unwrap_or_default())
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalFnContextSyntax {
    pub(crate) struct_ref: NodeRef,
}

impl Computable for LocalFnContextSyntax {
    type Node = ScriptNode;

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

        let fn_contexts = locals.fn_contexts.read(context).forward()?;

        Ok(fn_contexts
            .as_ref()
            .map
            .get(node_ref)
            .copied()
            .unwrap_or_default())
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalInfixSyntax {
    pub(crate) left: NodeRef,
    pub(crate) op_ref: NodeRef,
    pub(crate) op: ScriptToken,
    pub(crate) right: NodeRef,
}

impl Computable for LocalInfixSyntax {
    type Node = ScriptNode;

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

        let infixes = locals.infixes.read(context).forward()?;

        Ok(infixes
            .as_ref()
            .map
            .get(node_ref)
            .copied()
            .unwrap_or_default())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalCallSyntax {
    pub(crate) left: NodeRef,
    pub(crate) call_args_ref: NodeRef,
    pub(crate) args: Vec<NodeRef>,
}

impl SharedComputable for LocalCallSyntax {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax
            .as_ref()
            .calls
            .as_ref()
            .map
            .get(node_ref)
            .cloned()
            .unwrap_or_default())
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalIndexSyntax {
    pub(crate) left: NodeRef,
    pub(crate) index_arg_ref: NodeRef,
    pub(crate) index: NodeRef,
}

impl Computable for LocalIndexSyntax {
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

        let syntax = locals.syntax.read(context).forward()?;

        Ok(syntax
            .as_ref()
            .indexes
            .as_ref()
            .map
            .get(node_ref)
            .copied()
            .unwrap_or_default())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalAtomSyntax(pub(crate) CompactString);

impl Computable for LocalAtomSyntax {
    type Node = ScriptNode;

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

        let atoms = locals.atoms.read(context).forward()?;

        Ok(atoms
            .as_ref()
            .map
            .get(node_ref)
            .cloned()
            .unwrap_or_default())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum LocalNumberValue {
    Usize(Result<usize, ParseIntError>),
    Isize(Result<isize, ParseIntError>),
    Float(Result<Float, ParseFloatError>),
}

impl Default for LocalNumberValue {
    #[inline(always)]
    fn default() -> Self {
        Self::Usize(Ok(0))
    }
}

impl Computable for LocalNumberValue {
    type Node = ScriptNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(ScriptNode::Number { semantics, .. }) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let atom_syntax = semantics
            .get()
            .forward()?
            .atom_syntax
            .read(context)
            .forward()?;

        if atom_syntax.0.contains(&['.', 'e']) {
            return Ok(Self::Float(atom_syntax.0.parse()));
        }

        if atom_syntax.0.starts_with(&['-', '+']) {
            return Ok(Self::Isize(atom_syntax.0.parse()));
        }

        Ok(Self::Usize(atom_syntax.0.parse()))
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalBoolValue(pub(crate) bool);

impl Computable for LocalBoolValue {
    type Node = ScriptNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(ScriptNode::Number { semantics, .. }) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let atom_syntax = semantics
            .get()
            .forward()?
            .atom_syntax
            .read(context)
            .forward()?;

        if atom_syntax.0 == "true" {
            return Ok(Self(true));
        }

        Ok(Self(false))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalExprSyntax {
    Unknown,
    Struct(NodeRef),
    Array(NodeRef),
    Fn(NodeRef),
    Infix(NodeRef),
    Call(NodeRef),
    Index(NodeRef),
    Number(NodeRef),
    Max(NodeRef),
    String(NodeRef),
    Bool(NodeRef),
    Crate(NodeRef),
    This(NodeRef),
    Ident(NodeRef),
}

impl Default for LocalExprSyntax {
    #[inline(always)]
    fn default() -> Self {
        Self::Unknown
    }
}

impl Computable for LocalExprSyntax {
    type Node = ScriptNode;

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

        let exprs = locals.exprs.read(context).forward()?;

        Ok(exprs
            .as_ref()
            .map
            .get(node_ref)
            .copied()
            .unwrap_or_default())
    }
}

impl LocalExprSyntax {
    #[inline(always)]
    pub(crate) fn node_ref(&self) -> &NodeRef {
        match self {
            Self::Unknown => &NIL_NODE_REF,
            Self::Struct(node_ref) => node_ref,
            Self::Array(node_ref) => node_ref,
            Self::Fn(node_ref) => node_ref,
            Self::Infix(node_ref) => node_ref,
            Self::Call(node_ref) => node_ref,
            Self::Index(node_ref) => node_ref,
            Self::Number(node_ref) => node_ref,
            Self::Max(node_ref) => node_ref,
            Self::String(node_ref) => node_ref,
            Self::Bool(node_ref) => node_ref,
            Self::Crate(node_ref) => node_ref,
            Self::This(node_ref) => node_ref,
            Self::Ident(node_ref) => node_ref,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalVarSyntax {
    Unknown,
    Let(NodeRef),
    Param(NodeRef, usize),
    For(NodeRef),
}

impl Default for LocalVarSyntax {
    #[inline(always)]
    fn default() -> Self {
        Self::Unknown
    }
}

impl Computable for LocalVarSyntax {
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

        let vars = locals.vars.read(context).forward()?;

        Ok(vars.as_ref().map.get(node_ref).copied().unwrap_or_default())
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalArgSyntax {
    pub(crate) call_left_ref: NodeRef,
    pub(crate) arg_index: usize,
}

impl Computable for LocalArgSyntax {
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

        let args = locals.args.read(context).forward()?;

        Ok(args.as_ref().map.get(node_ref).copied().unwrap_or_default())
    }
}
