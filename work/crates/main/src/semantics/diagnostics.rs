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

use std::ops::{Deref, Range};

use ahash::AHashSet;
use compact_str::CompactString;
use lady_deirdre::{
    analysis::{AnalysisResult, AttrContext, Computable, TaskHandle, DOC_ERRORS_EVENT},
    arena::Identifiable,
    sync::{Shared, SyncBuildHasher},
    syntax::{NodeRef, PolyRef, SyntaxTree},
};

use crate::{
    analysis::{Closeness, DiagnosticsDepth, ModuleResultEx, ScriptIssue, StringEstimation},
    report::system_panic,
    runtime::{PackageMeta, ScriptType},
    semantics::{setup::log_attr, *},
    syntax::{ScriptClass, ScriptDoc, ScriptNode, ScriptToken},
};

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct CrossDiagnostics<const DEPTH: DiagnosticsDepth> {
    pub(crate) issues: Shared<AHashSet<ScriptIssue>>,
}

impl Computable for CrossDiagnostics<1> {
    type Node = ScriptNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr!(context);

        let id = context.node_ref().id;

        let doc_read = context.read_doc(id).forward()?;

        let mut issues = AHashSet::new();

        context.subscribe(id, DOC_ERRORS_EVENT);
        let error_refs = doc_read.error_refs();

        issues.reserve(error_refs.size_hint().0);

        for error_ref in error_refs {
            let _ = issues.insert(ScriptIssue::Parse { error_ref });
        }

        Ok(Self {
            issues: Shared::new(issues),
        })
    }
}

impl Computable for CrossDiagnostics<2> {
    type Node = ScriptNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr!(context);

        let id = context.node_ref().id;

        let doc_read = context.read_doc(id).forward()?;

        let mut issues = AHashSet::new();

        let all_scopes_refs = context.read_class(id, &ScriptClass::AllScopes).forward()?;

        for scope_ref in all_scopes_refs.as_ref() {
            let Some(scope_node) = scope_ref.deref(doc_read.deref()) else {
                continue;
            };

            let locals = scope_node.locals().forward()?;

            let local_diagnostics = locals.diagnostics_local_2.read(context).forward()?;

            issues.reserve(local_diagnostics.issues.len());

            for issue in &local_diagnostics.issues {
                let _ = issues.insert(issue.clone());
            }
        }

        Ok(Self {
            issues: Shared::new(issues),
        })
    }
}

impl Computable for CrossDiagnostics<3> {
    type Node = ScriptNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr!(context);

        let id = context.node_ref().id;

        let doc_read = context.read_doc(id).forward()?;

        let mut issues = AHashSet::new();

        let all_scopes_refs = context.read_class(id, &ScriptClass::AllScopes).forward()?;

        for scope_ref in all_scopes_refs.as_ref() {
            let Some(scope_node) = scope_ref.deref(doc_read.deref()) else {
                continue;
            };

            let locals = scope_node.locals().forward()?;

            let local_diagnostics = locals.diagnostics_local_3.read(context).forward()?;

            issues.reserve(local_diagnostics.issues.len());

            for issue in &local_diagnostics.issues {
                let _ = issues.insert(issue.clone());
            }
        }

        Ok(Self {
            issues: Shared::new(issues),
        })
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalDiagnostics<const DEPTH: DiagnosticsDepth> {
    pub(crate) issues: AHashSet<ScriptIssue>,
}

impl<const DEPTH: DiagnosticsDepth> Computable for LocalDiagnostics<DEPTH> {
    type Node = ScriptNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let Some(scope_node) = node_ref.deref(doc_read.deref()) else {
            return Ok(Default::default());
        };

        let locals = scope_node.locals().forward()?;

        let local_analysis = locals.analysis.read(context).forward()?;

        let mut collection = LocalDiagnosticsCollection::<'_, '_, '_, DEPTH, H, S> {
            fn_ref: node_ref,
            doc: doc_read.deref(),
            local_analysis: local_analysis.deref(),
            context,
            issues: AHashSet::new(),
        };

        collection.collect_issues()?;

        Ok(Self {
            issues: collection.issues,
        })
    }
}

struct LocalDiagnosticsCollection<
    'a,
    'ctx,
    'ctx_param,
    const DEPTH: DiagnosticsDepth,
    H: TaskHandle,
    S: SyncBuildHasher,
> {
    fn_ref: &'a NodeRef,
    doc: &'a ScriptDoc,
    local_analysis: &'a LocalAnalysis,
    context: &'ctx mut AttrContext<'ctx_param, ScriptNode, H, S>,
    issues: AHashSet<ScriptIssue>,
}

impl<'a, 'ctx, 'ctx_param, const DEPTH: DiagnosticsDepth, H: TaskHandle, S: SyncBuildHasher>
    LocalDiagnosticsCollection<'a, 'ctx, 'ctx_param, DEPTH, H, S>
{
    fn collect_issues(&mut self) -> AnalysisResult<()> {
        match DEPTH {
            2 => {
                self.collect_import_issues()?;
                self.collect_loop_issues()?;
                self.collect_signature_issues()?;
                self.collect_expr_issues()?;
                self.collect_reachability_issues()?;
            }

            3 => {
                self.collect_expr_issues()?;
                self.collect_return_inconsistency_issues()?;
                self.collect_st_type_issues()?;
            }

            _ => (),
        }

        #[cfg(debug_assertions)]
        for issue in &self.issues {
            let depth = issue.code().depth();

            if depth != DEPTH {
                system_panic!("Incorrect issue depth.")
            }
        }

        Ok(())
    }

    fn collect_import_issues(&mut self) -> AnalysisResult<()> {
        let atoms = self.local_analysis.syntax.as_ref().atoms.as_ref();

        let packages = self.local_analysis.names.as_ref().packages.as_ref();

        for (package_ref, resolution) in &packages.map {
            if resolution.package.is_some() {
                continue;
            }

            let Some(parent) = resolution.parent else {
                continue;
            };

            let Some(LocalAtomSyntax(package_atom)) = atoms.map.get(package_ref) else {
                continue;
            };

            let parent_ty = parent.ty();
            let parent_prototype = parent_ty.prototype();

            if let Some(component) = parent_prototype.hint_component(package_atom) {
                if !component.ty.is_package() {
                    let _ = self.issues.insert(ScriptIssue::NotAPackage {
                        ty: component.ty,
                        package_ref: *package_ref,
                    });
                    continue;
                }
            }

            let mut best_match = (Closeness::zero(), "");

            for component in parent_prototype.hint_all_components() {
                let estimation = component.name.estimate(package_atom);

                if estimation <= best_match.0 {
                    continue;
                }

                best_match = (estimation, component.name.string);
            }

            let _ = self.issues.insert(ScriptIssue::UnresolvedPackage {
                base: parent_ty,
                package_ref: *package_ref,
                quickfix: CompactString::from(best_match.1),
            });
        }

        Ok(())
    }

    fn collect_loop_issues(&mut self) -> AnalysisResult<()> {
        let break_to_loop = self.local_analysis.flow.as_ref().break_to_loop.as_ref();

        for (break_ref, loop_ref) in &break_to_loop.map {
            if break_ref.is_nil() {
                continue;
            }

            if !loop_ref.is_nil() {
                continue;
            }

            let _ = self.issues.insert(ScriptIssue::OrphanedBreak {
                break_ref: *break_ref,
            });
        }

        Ok(())
    }

    fn collect_signature_issues(&mut self) -> AnalysisResult<()> {
        let sig = self.local_analysis.syntax.as_ref().sig.as_ref();

        for (unique, var_ref) in &sig.params {
            if *unique {
                continue;
            }

            let _ = self
                .issues
                .insert(ScriptIssue::DuplicateParam { var_ref: *var_ref });
        }

        Ok(())
    }

    fn collect_expr_issues(&mut self) -> AnalysisResult<()> {
        let exprs = self.local_analysis.syntax.as_ref().exprs.as_ref();

        for (expr_ref, expr_syntax) in &exprs.map {
            if expr_ref != expr_syntax.node_ref() {
                continue;
            }

            match DEPTH {
                2 => match expr_syntax {
                    LocalExprSyntax::Infix(..) => {
                        self.collect_literal_assignment_issues(expr_ref)?
                    }
                    LocalExprSyntax::Struct(..) => self.collect_struct_issues(expr_ref)?,
                    LocalExprSyntax::Number(..) => self.collect_number_issues(expr_ref)?,
                    LocalExprSyntax::Ident(..) => self.collect_ident_issues(expr_ref)?,

                    _ => (),
                },

                3 => self.collect_expr_type_issues(expr_ref)?,

                _ => (),
            }
        }

        Ok(())
    }

    fn collect_literal_assignment_issues(&mut self, infix_ref: &NodeRef) -> AnalysisResult<()> {
        let infixes = self.local_analysis.syntax.as_ref().infixes.as_ref();

        let Some(infix_syntax) = infixes.map.get(infix_ref) else {
            return Ok(());
        };

        match infix_syntax.op {
            ScriptToken::Assign
            | ScriptToken::PlusAssign
            | ScriptToken::MinusAssign
            | ScriptToken::MulAssign
            | ScriptToken::DivAssign
            | ScriptToken::BitAndAssign
            | ScriptToken::BitOrAssign
            | ScriptToken::BitXorAssign
            | ScriptToken::ShlAssign
            | ScriptToken::ShrAssign
            | ScriptToken::RemAssign => (),

            _ => return Ok(()),
        }

        let exprs = self.local_analysis.syntax.as_ref().exprs.as_ref();

        let Some(left_expr_syntax) = exprs.map.get(&infix_syntax.left) else {
            return Ok(());
        };

        match left_expr_syntax {
            LocalExprSyntax::Number(_) | LocalExprSyntax::String(_) | LocalExprSyntax::Bool(_) => {
                ()
            }

            _ => return Ok(()),
        }

        let _ = self.issues.insert(ScriptIssue::LiteralAssignment {
            op_ref: infix_syntax.op_ref,
        });

        Ok(())
    }

    fn collect_struct_issues(&mut self, struct_ref: &NodeRef) -> AnalysisResult<()> {
        let struct_entries = self
            .local_analysis
            .syntax
            .as_ref()
            .struct_entry_vecs
            .as_ref();

        let Some(struct_entries) = struct_entries.map.get(struct_ref) else {
            return Ok(());
        };

        let mut keys = AHashSet::new();

        for (key, entry_key_ref, _) in &struct_entries.as_ref().vec {
            if keys.insert(key.as_str()) {
                continue;
            }

            let _ = self.issues.insert(ScriptIssue::DuplicateEntry {
                entry_key_ref: *entry_key_ref,
            });
        }

        Ok(())
    }

    fn collect_number_issues(&mut self, number_ref: &NodeRef) -> AnalysisResult<()> {
        let Some(ScriptNode::Number { semantics, .. }) = number_ref.deref(self.doc) else {
            return Ok(());
        };

        let number_semantics = semantics.get().forward()?;

        let number_value = number_semantics.number_value.read(self.context).forward()?;

        match number_value.deref() {
            LocalNumberValue::Usize(parse_result) if parse_result.is_err() => {
                let _ = self.issues.insert(ScriptIssue::IntParse {
                    number_ref: *number_ref,
                });
            }

            LocalNumberValue::Isize(parse_result) if parse_result.is_err() => {
                let _ = self.issues.insert(ScriptIssue::IntParse {
                    number_ref: *number_ref,
                });
            }

            LocalNumberValue::Float(parse_result) if parse_result.is_err() => {
                let _ = self.issues.insert(ScriptIssue::FloatParse {
                    number_ref: *number_ref,
                });
            }

            _ => (),
        }

        Ok(())
    }

    fn collect_ident_issues(&mut self, ident_ref: &NodeRef) -> AnalysisResult<()> {
        let atoms = self.local_analysis.syntax.as_ref().atoms.as_ref();

        let Some(ScriptNode::Ident { semantics, .. }) = ident_ref.deref(self.doc) else {
            return Ok(());
        };

        let ident_semantics = semantics.get().forward()?;

        let cross_resolution = ident_semantics
            .cross_resolution
            .read(self.context)
            .forward()?;

        match cross_resolution.deref() {
            IdentCrossResolution::Unresolved => {
                let mut import = Vec::new();

                let Some(package) = PackageMeta::by_id(self.doc.id()) else {
                    system_panic!("Missing package.");
                };

                let Some(LocalAtomSyntax(atom_string)) = atoms.map.get(ident_ref) else {
                    return Ok(());
                };

                match lookup_import(&mut import, package, atom_string) {
                    false => {
                        let _ = self.issues.insert(ScriptIssue::UnresolvedIdent {
                            ident_ref: *ident_ref,
                            quickfix: CompactString::from(""),
                            import: CompactString::from(""),
                        });
                    }

                    true => {
                        let _ = self.issues.insert(ScriptIssue::UnresolvedIdent {
                            ident_ref: *ident_ref,
                            quickfix: atom_string.clone(),
                            import: CompactString::from(import.join(".")),
                        });
                    }
                };
            }

            IdentCrossResolution::BestMatch { estimation } => {
                let _ = self.issues.insert(ScriptIssue::UnresolvedIdent {
                    ident_ref: *ident_ref,
                    quickfix: estimation.name.clone(),
                    import: CompactString::from(""),
                });
            }

            IdentCrossResolution::Read { name } if !name.as_ref().init => {
                let _ = self.issues.insert(ScriptIssue::ReadUninit {
                    ident_ref: *ident_ref,
                });
            }

            _ => (),
        }

        Ok(())
    }

    fn collect_expr_type_issues(&mut self, expr_ref: &NodeRef) -> AnalysisResult<()> {
        let Some(expr_node) = expr_ref.deref(self.doc) else {
            return Ok(());
        };

        let expr_type_resolution = expr_node
            .type_resolution()
            .forward()?
            .read(self.context)
            .forward()?;

        for issue in &expr_type_resolution.issues {
            let _ = self.issues.insert(issue.clone());
        }

        Ok(())
    }

    fn collect_reachability_issues(&mut self) -> AnalysisResult<()> {
        let unreachable_statements = self
            .local_analysis
            .flow
            .as_ref()
            .unreachable_statements
            .as_ref();

        for st_ref in &unreachable_statements.set {
            let _ = self
                .issues
                .insert(ScriptIssue::UnreachableStatement { st_ref: *st_ref });
        }

        let unreachable_arms = self.local_analysis.flow.as_ref().unreachable_arms.as_ref();

        for arm_ref in &unreachable_arms.set {
            let _ = self
                .issues
                .insert(ScriptIssue::UnreachableArm { arm_ref: *arm_ref });
        }

        Ok(())
    }

    fn collect_return_inconsistency_issues(&mut self) -> AnalysisResult<()> {
        let return_points = self.local_analysis.flow.as_ref().return_points.as_ref();

        let mut implicit = false;
        let mut expr = false;

        for return_point in &return_points.set {
            match return_point {
                LocalReturnPoint::Implicit => implicit = true,

                LocalReturnPoint::Explicit(_) => return Ok(()),

                LocalReturnPoint::Expr(expr_ref) => {
                    let Some(expr_node) = expr_ref.deref(self.doc) else {
                        return Ok(());
                    };

                    let expr_type_resolution = expr_node
                        .type_resolution()
                        .forward()?
                        .read(self.context)
                        .forward()?;

                    if expr_type_resolution.tag.is_dynamic() {
                        return Ok(());
                    }

                    if expr_type_resolution.tag.type_family().is_nil() {
                        return Ok(());
                    }

                    expr = true;
                }
            }
        }

        if !implicit || !expr {
            return Ok(());
        }

        let _ = self.issues.insert(ScriptIssue::InconsistentReturns {
            fn_ref: *self.fn_ref,
        });

        Ok(())
    }

    fn collect_st_type_issues(&mut self) -> AnalysisResult<()> {
        let bool_family = <bool>::type_meta().family();

        let ifs = self.local_analysis.syntax.as_ref().ifs.as_ref();

        for (_, syntax) in &ifs.map {
            let Some(condition_node) = syntax.condition.deref(self.doc) else {
                continue;
            };

            let condition_type_resolution = condition_node
                .type_resolution()
                .forward()?
                .read(self.context)
                .forward()?;

            if condition_type_resolution.tag.is_dynamic() {
                continue;
            }

            let provided = condition_type_resolution.tag.type_family();

            if provided != bool_family {
                let _ = self.issues.insert(ScriptIssue::TypeMismatch {
                    expr_ref: syntax.condition,
                    expected: bool_family,
                    provided,
                });
            }
        }

        let range_family = <Range<usize>>::type_meta().family();

        let fors = self.local_analysis.syntax.as_ref().fors.as_ref();

        for (_, syntax) in &fors.map {
            let Some(range_node) = syntax.range.deref(self.doc) else {
                continue;
            };

            let range_type_resolution = range_node
                .type_resolution()
                .forward()?
                .read(self.context)
                .forward()?;

            if range_type_resolution.tag.is_dynamic() {
                continue;
            }

            let provided = range_type_resolution.tag.type_family();

            if provided != range_family {
                let _ = self.issues.insert(ScriptIssue::TypeMismatch {
                    expr_ref: syntax.range,
                    expected: range_family,
                    provided,
                });
            }
        }

        let matches = self.local_analysis.syntax.as_ref().matches.as_ref();

        for (_, syntax) in &matches.map {
            let syntax = syntax.as_ref();

            let subject_family = match syntax.subject.is_nil() {
                true => bool_family,

                false => {
                    let Some(subject_node) = syntax.subject.deref(self.doc) else {
                        continue;
                    };

                    let subject_type_resolution = subject_node
                        .type_resolution()
                        .forward()?
                        .read(self.context)
                        .forward()?;

                    if subject_type_resolution.tag.is_dynamic() {
                        continue;
                    }

                    subject_type_resolution.tag.type_family()
                }
            };

            for case_ref in &syntax.cases {
                let Some(case_node) = case_ref.deref(self.doc) else {
                    continue;
                };

                let case_type_resolution = case_node
                    .type_resolution()
                    .forward()?
                    .read(self.context)
                    .forward()?;

                if case_type_resolution.tag.is_dynamic() {
                    continue;
                }

                let provided = case_type_resolution.tag.type_family();

                if provided != subject_family {
                    let _ = self.issues.insert(ScriptIssue::TypeMismatch {
                        expr_ref: *case_ref,
                        expected: subject_family,
                        provided,
                    });
                }
            }
        }

        Ok(())
    }
}

fn lookup_import(
    import: &mut Vec<&'static str>,
    package: &'static PackageMeta,
    symbol: &str,
) -> bool {
    let package_prototype = package.ty().prototype();

    for component in package_prototype.hint_all_components() {
        if component.ty.is_package() {
            continue;
        }

        if component.name.string != symbol {
            continue;
        }

        return true;
    }

    for component in package_prototype.hint_all_components() {
        if !component.ty.is_package() {
            continue;
        }

        let Some(package) = component.ty.package() else {
            continue;
        };

        if import.contains(&component.name.string) {
            continue;
        }

        import.push(component.name.string);

        if lookup_import(import, package, symbol) {
            return true;
        }

        let _ = import.pop();
    }

    false
}
