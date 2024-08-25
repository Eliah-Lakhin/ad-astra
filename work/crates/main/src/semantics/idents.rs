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

use std::{collections::BTreeSet, ops::Deref};

use ahash::AHashMap;
use compact_str::CompactString;
use lady_deirdre::{
    analysis::{AnalysisResult, AttrContext, Computable, TaskHandle},
    arena::Identifiable,
    sync::{Shared, SyncBuildHasher},
    syntax::NodeRef,
};

use crate::{
    analysis::{Closeness, ModuleResultEx},
    report::system_panic,
    runtime::PackageMeta,
    semantics::*,
    syntax::ScriptNode,
};

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum IdentLocalResolution {
    Closure,
    Read { name: Shared<Name> },
    Write { decl: NodeRef },
}

impl Default for IdentLocalResolution {
    #[inline(always)]
    fn default() -> Self {
        Self::Closure
    }
}

impl Computable for IdentLocalResolution {
    type Node = ScriptNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(ScriptNode::Ident { semantics, .. }) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let scope_attr = semantics.scope_attr().forward()?.read(context).forward()?;

        let Some(scope_node) = scope_attr.scope_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let locals = scope_node.locals().forward()?;

        let idents_attr = locals.idents.read(context).forward()?;

        let Some(result) = idents_attr.as_ref().map.get(node_ref) else {
            return Ok(Default::default());
        };

        Ok(result.clone())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum IdentCrossResolution {
    Unresolved,
    BestMatch { estimation: NameEstimation },
    Read { name: Shared<Name> },
    Write { decl: NodeRef },
}

impl Default for IdentCrossResolution {
    #[inline(always)]
    fn default() -> Self {
        Self::Unresolved
    }
}

impl Computable for IdentCrossResolution {
    type Node = ScriptNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(ScriptNode::Ident { semantics, .. }) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let ident_semantics = semantics.get().forward()?;

        let local_resolution = ident_semantics.local_resolution.read(context).forward()?;

        match local_resolution.deref() {
            IdentLocalResolution::Closure => (),
            IdentLocalResolution::Read { name } => return Ok(Self::Read { name: name.clone() }),
            IdentLocalResolution::Write { decl } => return Ok(Self::Write { decl: *decl }),
        }

        let scope_attr = semantics.scope_attr().forward()?.read(context).forward()?;

        let Some(mut scope_node) = scope_attr.scope_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let atom_syntax = ident_semantics.atom_syntax.read(context).forward()?;

        let atom_string = &atom_syntax.0;

        let mut best_match = None;

        let mut namespace = ident_semantics.namespace.read(context).forward()?;

        namespace
            .as_ref()
            .estimate_one(atom_string, &mut best_match);

        loop {
            let ScriptNode::Fn { semantics, .. } = scope_node else {
                break;
            };

            let fn_semantics = semantics.get().forward()?;

            namespace = fn_semantics.namespace.read(context).forward()?;

            if let Some(local_name) = namespace.as_ref().map.get(atom_string) {
                return Ok(Self::Read {
                    name: local_name.clone(),
                });
            }

            namespace
                .as_ref()
                .estimate_one(atom_string, &mut best_match);

            let parent_scope = semantics.scope_attr().forward()?.read(context).forward()?;

            let Some(parent_node) = parent_scope.scope_ref.deref(doc_read.deref()) else {
                break;
            };

            scope_node = parent_node;
        }

        match best_match {
            Some(estimation) if estimation.closeness > Closeness::half() => {
                Ok(Self::BestMatch { estimation })
            }

            _ => Ok(Self::Unresolved),
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct IdentsDescMap {
    pub(crate) map: AHashMap<NodeRef, IdentDesc>,
}

impl Computable for IdentsDescMap {
    type Node = ScriptNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(package) = PackageMeta::by_id(doc_read.id()) else {
            system_panic!("Missing package.");
        };

        let Some(script_node) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let locals = script_node.locals().forward()?;

        let idents = locals.idents.read(context).forward()?;

        let mut map = AHashMap::with_capacity(idents.as_ref().map.len());

        for (ident_ref, local_resolution) in &idents.as_ref().map {
            let Some(ScriptNode::Ident { semantics, .. }) = ident_ref.deref(doc_read.deref())
            else {
                continue;
            };

            let resolution = match local_resolution {
                IdentLocalResolution::Closure => {
                    let ident_semantics = semantics.get().forward()?;

                    let cross_resolution =
                        ident_semantics.cross_resolution.read(context).forward()?;

                    match cross_resolution.deref() {
                        IdentCrossResolution::Read { name, .. } => {
                            match name.as_ref().decl.deref(doc_read.deref()) {
                                Some(ScriptNode::Root { .. }) => IdentDesc::Import(package),

                                Some(ScriptNode::Use { .. }) => {
                                    let Some(package_ref) = name.as_ref().defs.iter().next() else {
                                        continue;
                                    };

                                    let Some(ScriptNode::Package { semantics, .. }) =
                                        package_ref.deref(doc_read.deref())
                                    else {
                                        continue;
                                    };

                                    let package_semantics = semantics.get().forward()?;

                                    let package_resolution = package_semantics
                                        .package_resolution
                                        .read(context)
                                        .forward()?;

                                    let Some(package) = package_resolution.package else {
                                        continue;
                                    };

                                    IdentDesc::Import(package)
                                }

                                _ => IdentDesc::Closure,
                            }
                        }

                        _ => IdentDesc::Closure,
                    }
                }

                IdentLocalResolution::Read { name } => {
                    match name.as_ref().decl.deref(doc_read.deref()) {
                        Some(ScriptNode::Root { .. }) => IdentDesc::Import(package),

                        Some(ScriptNode::Use { .. }) => {
                            let Some(package_ref) = name.as_ref().defs.iter().next() else {
                                continue;
                            };

                            let Some(ScriptNode::Package { semantics, .. }) =
                                package_ref.deref(doc_read.deref())
                            else {
                                continue;
                            };

                            let package_semantics = semantics.get().forward()?;

                            let package_resolution = package_semantics
                                .package_resolution
                                .read(context)
                                .forward()?;

                            let Some(package) = package_resolution.package else {
                                continue;
                            };

                            IdentDesc::Import(package)
                        }

                        _ => IdentDesc::LocalRead,
                    }
                }

                IdentLocalResolution::Write { .. } => IdentDesc::LocalWrite,
            };

            let _ = map.insert(*ident_ref, resolution);
        }

        Ok(Self { map })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum IdentDesc {
    LocalRead,
    LocalWrite,
    Closure,
    Import(&'static PackageMeta),
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct ClosureVec {
    pub(crate) vec: Vec<CompactString>,
}

impl Computable for ClosureVec {
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

        let locals = script_node.locals().forward()?;
        let compilation = script_node.compilation().forward()?;

        let atoms = locals.atoms.read(context).forward()?;
        let exprs = locals.exprs.read(context).forward()?;
        let ident_desc_map = compilation.ident_desc_map.read(context).forward()?;

        let mut set = BTreeSet::new();

        for (ident_ref, ident_desc) in &ident_desc_map.map {
            let IdentDesc::Closure = ident_desc else {
                continue;
            };

            let Some(LocalAtomSyntax(name)) = atoms.as_ref().map.get(ident_ref) else {
                continue;
            };

            let _ = set.insert(name.clone());
        }

        for (_, local_expr_syntax) in &exprs.as_ref().map {
            let LocalExprSyntax::Fn(fn_ref) = local_expr_syntax else {
                continue;
            };

            let Some(ScriptNode::Fn { semantics, .. }) = fn_ref.deref(doc_read.deref()) else {
                continue;
            };

            let fn_semantics = semantics.get()?;

            let fn_closure_vec = fn_semantics
                .compilation
                .closure_vec
                .read(context)
                .forward()?;

            for name in &fn_closure_vec.vec {
                let _ = set.insert(name.clone());
            }
        }

        Ok(Self {
            vec: set.into_iter().collect(),
        })
    }
}
