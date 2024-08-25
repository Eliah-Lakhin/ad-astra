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

use lady_deirdre::{
    analysis::{AnalysisError, AnalysisResult, Attr, Feature},
    sync::Shared,
};

use crate::{interpret::Assembly, semantics::*, syntax::ScriptNode};

#[derive(Feature)]
#[node(ScriptNode)]
pub struct Locals {
    #[scoped]
    pub(crate) analysis: Attr<LocalAnalysis>,

    pub(crate) flow: Attr<Shared<LocalFlow>>,
    pub(crate) names: Attr<Shared<LocalNames>>,
    pub(crate) syntax: Attr<Shared<LocalSyntax>>,

    pub(crate) return_points: Attr<Shared<ReturnPointsSet>>,
    pub(crate) loop_to_break: Attr<Shared<LoopToBreakMap>>,
    pub(crate) break_to_loop: Attr<Shared<BreakToLoopMap>>,
    pub(crate) unreachable_statements: Attr<Shared<UnreachableStatementsSet>>,
    pub(crate) unreachable_arms: Attr<Shared<UnreachableArmsSet>>,

    pub(crate) packages: Attr<Shared<LocalPackageMap>>,
    pub(crate) idents: Attr<Shared<LocalIdentMap>>,
    pub(crate) lets: Attr<Shared<LocalLetMap>>,
    pub(crate) namespaces: Attr<Shared<LocalNamespacesMap>>,

    pub(crate) sig: Attr<Shared<LocalSig>>,
    pub(crate) ifs: Attr<Shared<LocalIfMap>>,
    pub(crate) fors: Attr<Shared<LocalForMap>>,
    pub(crate) matches: Attr<Shared<LocalMatchMap>>,
    pub(crate) struct_keys: Attr<Shared<LocalStructEntryVecs>>,
    pub(crate) struct_values: Attr<Shared<LocalStructEntryMaps>>,
    pub(crate) arrays: Attr<Shared<LocalArrayMap>>,
    pub(crate) fn_contexts: Attr<Shared<LocalFnContextMap>>,
    pub(crate) infixes: Attr<Shared<LocalInfixMap>>,
    pub(crate) calls: Attr<Shared<LocalCallMap>>,
    pub(crate) indexes: Attr<Shared<LocalIndexMap>>,
    pub(crate) vars: Attr<Shared<LocalVarMap>>,
    pub(crate) args: Attr<Shared<LocalArgsMap>>,
    pub(crate) atoms: Attr<Shared<LocalAtomMap>>,
    pub(crate) exprs: Attr<Shared<LocalExprMap>>,

    pub(crate) diagnostics_local_2: Attr<LocalDiagnostics<2>>,
    pub(crate) diagnostics_local_3: Attr<LocalDiagnostics<3>>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct Compilation {
    #[scoped]
    pub(crate) assembly: Attr<Shared<Assembly>>,

    pub(crate) ident_desc_map: Attr<IdentsDescMap>,
    pub(crate) closure_vec: Attr<ClosureVec>,
    pub(crate) lifetimes: Attr<Lifetimes>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct RootSemantics {
    #[scoped]
    pub(crate) locals: Locals,

    #[scoped]
    pub(crate) compilation: Compilation,

    pub(crate) result_resolution: Attr<ResultResolution>,
    pub(crate) diagnostics_cross_1: Attr<CrossDiagnostics<1>>,
    pub(crate) diagnostics_cross_2: Attr<CrossDiagnostics<2>>,
    pub(crate) diagnostics_cross_3: Attr<CrossDiagnostics<3>>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct PackageSemantics {
    pub(crate) atom_syntax: Attr<LocalAtomSyntax>,
    pub(crate) package_resolution: Attr<LocalPackageResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct VarSemantics {
    pub(crate) var_syntax: Attr<LocalVarSyntax>,
    pub(crate) let_inits: Attr<Shared<LocalLetInits>>,
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct ForSemantics {
    pub(crate) break_set: Attr<Shared<BreaksSet>>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct LoopSemantics {
    pub(crate) break_set: Attr<Shared<BreaksSet>>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct BreakSemantics {
    pub(crate) loop_context: Attr<LoopContext>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct ContinueSemantics {
    pub(crate) loop_context: Attr<LoopContext>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct FnSemantics {
    #[scoped]
    pub(crate) locals: Locals,

    #[scoped]
    pub(crate) compilation: Compilation,

    pub(crate) namespace: Attr<Shared<LocalNamespace>>,
    pub(crate) fn_context_syntax: Attr<LocalFnContextSyntax>,
    pub(crate) arg_syntax: Attr<LocalArgSyntax>,
    pub(crate) result_resolution: Attr<ResultResolution>,
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct StructSemantics {
    pub(crate) struct_entries_vec_syntax: Attr<Shared<LocalStructEntriesVecSyntax>>,
    pub(crate) struct_entries_map_syntax: Attr<Shared<LocalStructEntiesMapSyntax>>,
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct ArraySemantics {
    pub(crate) array_syntax: Attr<Shared<LocalArraySyntax>>,
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct StringSemantics {
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct CrateSemantics {
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct ThisSemantics {
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct IdentSemantics {
    pub(crate) atom_syntax: Attr<LocalAtomSyntax>,
    pub(crate) namespace: Attr<Shared<LocalNamespace>>,
    pub(crate) local_resolution: Attr<IdentLocalResolution>,
    pub(crate) cross_resolution: Attr<IdentCrossResolution>,
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct NumberSemantics {
    pub(crate) atom_syntax: Attr<LocalAtomSyntax>,
    pub(crate) number_value: Attr<LocalNumberValue>,
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct MaxSemantics {
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct BoolSemantics {
    pub(crate) atom_syntax: Attr<LocalAtomSyntax>,
    pub(crate) bool_value: Attr<LocalBoolValue>,
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct UnaryLeftSemantics {
    pub(crate) infix_syntax: Attr<LocalInfixSyntax>,
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct BinarySemantics {
    pub(crate) infix_syntax: Attr<LocalInfixSyntax>,
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct QuerySemantics {
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct CallSemantics {
    pub(crate) call_syntax: Attr<Shared<LocalCallSyntax>>,
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct IndexSemantics {
    pub(crate) index_syntax: Attr<LocalIndexSyntax>,
    pub(crate) type_resolution: Attr<TypeResolution>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct FieldSemantics {
    pub(crate) atom_syntax: Attr<LocalAtomSyntax>,
}

#[derive(Feature)]
#[node(ScriptNode)]
pub struct ExprSemantics {
    pub(crate) expr_syntax: Attr<LocalExprSyntax>,
    pub(crate) type_resolution: Attr<TypeResolution>,
}

impl ScriptNode {
    #[inline(always)]
    pub(crate) fn locals(&self) -> AnalysisResult<&Locals> {
        match self {
            Self::Root { semantics, .. } => Ok(&semantics.get()?.locals),
            Self::Fn { semantics, .. } => Ok(&semantics.get()?.locals),
            _ => Err(AnalysisError::MissingFeature),
        }
    }

    #[inline(always)]
    pub(crate) fn compilation(&self) -> AnalysisResult<&Compilation> {
        match self {
            Self::Root { semantics, .. } => Ok(&semantics.get()?.compilation),
            Self::Fn { semantics, .. } => Ok(&semantics.get()?.compilation),
            _ => Err(AnalysisError::MissingFeature),
        }
    }

    #[inline(always)]
    pub(crate) fn type_resolution(&self) -> AnalysisResult<&Attr<TypeResolution>> {
        match self {
            Self::Var { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::Fn { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::Struct { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::Array { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::String { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::Crate { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::This { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::Ident { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::Number { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::Max { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::Bool { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::UnaryLeft { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::Binary { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::Query { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::Call { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::Index { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            Self::Expr { semantics, .. } => Ok(&semantics.get()?.type_resolution),
            _ => Err(AnalysisError::MissingFeature),
        }
    }
}

macro_rules! log_attr {
    ($context:expr) => {
        #[cfg(debug_assertions)]
        {
            use std::{any::type_name, ops::Deref};

            use log::trace;

            use crate::{analysis::ModuleResultEx, syntax::PolyRefOrigin};

            let name = type_name::<Self>();
            let mut begin = 0;

            for (index, ch) in name.char_indices() {
                if ch == ':' {
                    begin = index + 1;
                }
            }

            let node_ref = $context.node_ref();
            let doc_read = $context.read_doc(node_ref.id).forward()?;
            let snippet =
                node_ref.script_display(doc_read.deref(), format!("{}::compute()", &name[begin..]));

            trace!("\n{:#}", snippet);
        }
    };
}

pub(super) use log_attr;
