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

use std::{cmp::Ordering, ops::Deref};

use ahash::{AHashMap, AHashSet};
use compact_str::CompactString;
use lady_deirdre::{
    analysis::{AnalysisResult, AttrContext, Computable, Grammar, SharedComputable, TaskHandle},
    sync::{Shared, SyncBuildHasher},
    syntax::NodeRef,
};

use crate::{
    analysis::{Closeness, ModuleResultEx, StringEstimation},
    runtime::PackageMeta,
    semantics::*,
    syntax::ScriptNode,
};

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalNames {
    pub(crate) packages: Shared<LocalPackageMap>,
    pub(crate) idents: Shared<LocalIdentMap>,
    pub(crate) lets: Shared<LocalLetMap>,
    pub(crate) namespaces: Shared<LocalNamespacesMap>,
}

impl SharedComputable for LocalNames {
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

        Ok(analysis.names.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalPackageMap {
    pub(crate) map: AHashMap<NodeRef, LocalPackageResolution>,
}

impl SharedComputable for LocalPackageMap {
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

        let names = locals.names.read(context).forward()?;

        Ok(names.as_ref().packages.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalIdentMap {
    pub(crate) map: AHashMap<NodeRef, IdentLocalResolution>,
}

impl SharedComputable for LocalIdentMap {
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

        let names = locals.names.read(context).forward()?;

        Ok(names.as_ref().idents.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalLetMap {
    pub(crate) map: AHashMap<NodeRef, Shared<LocalLetInits>>,
}

impl SharedComputable for LocalLetMap {
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

        let names = locals.names.read(context).forward()?;

        Ok(names.as_ref().lets.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalNamespacesMap {
    pub(crate) map: AHashMap<NodeRef, Shared<LocalNamespace>>,
}

impl SharedComputable for LocalNamespacesMap {
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

        let names = locals.names.read(context).forward()?;

        Ok(names.as_ref().namespaces.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalLetInits {
    pub(crate) set: AHashSet<NodeRef>,
}

impl SharedComputable for LocalLetInits {
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

        let vars = locals.lets.read(context).forward()?;

        Ok(vars.as_ref().map.get(node_ref).cloned().unwrap_or_default())
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalPackageResolution {
    pub(crate) parent: Option<&'static PackageMeta>,
    pub(crate) package: Option<&'static PackageMeta>,
}

impl Computable for LocalPackageResolution {
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

        let vars = locals.packages.read(context).forward()?;

        Ok(vars.as_ref().map.get(node_ref).copied().unwrap_or_default())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalNamespace {
    pub(crate) map: AHashMap<CompactString, Shared<Name>>,
}

impl SharedComputable for LocalNamespace {
    type Node = ScriptNode;

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

        let namespaces_attr = locals.namespaces.read(context).forward()?;

        let Some(result) = namespaces_attr.as_ref().map.get(node_ref) else {
            return Ok(Default::default());
        };

        Ok(result.clone())
    }
}

impl LocalNamespace {
    pub(crate) fn estimate_one<'a>(&'a self, pattern: &str, result: &mut Option<NameEstimation>) {
        for (probe, desc) in &self.map {
            let closeness = probe.estimate(pattern);

            if let Some(current) = &result {
                match current.closeness.cmp(&closeness) {
                    Ordering::Less => (),

                    Ordering::Equal => {
                        if &current.name <= probe {
                            continue;
                        }
                    }

                    Ordering::Greater => continue,
                }
            }

            *result = Some(NameEstimation {
                closeness,
                name: probe.clone(),
                desc: desc.clone(),
            });
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct Name {
    pub(crate) init: bool,
    pub(crate) decl: NodeRef,
    pub(crate) defs: AHashSet<NodeRef>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct NameEstimation {
    pub(crate) closeness: Closeness,
    pub(crate) name: CompactString,
    pub(crate) desc: Shared<Name>,
}
