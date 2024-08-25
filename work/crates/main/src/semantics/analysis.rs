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

use std::{collections::hash_map::Entry, mem::replace, ops::Deref};

use ahash::{AHashMap, AHashSet};
use compact_str::CompactString;
use lady_deirdre::{
    analysis::{AnalysisResult, AttrContext, Computable, TaskHandle},
    arena::Identifiable,
    lexis::TokenRef,
    sync::{Shared, SyncBuildHasher},
    syntax::{NodeRef, PolyRef, NIL_NODE_REF},
};

use crate::{
    analysis::ModuleResultEx,
    report::{debug_unreachable, system_panic},
    runtime::PackageMeta,
    semantics::*,
    syntax::{ScriptDoc, ScriptNode, ScriptToken},
};

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct LocalAnalysis {
    pub(crate) flow: Shared<LocalFlow>,
    pub(crate) names: Shared<LocalNames>,
    pub(crate) syntax: Shared<LocalSyntax>,
}

impl Computable for LocalAnalysis {
    type Node = ScriptNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr!(context);

        let node_ref = context.node_ref();

        let doc_read = context.read_doc(node_ref.id).forward()?;

        let mut analyzer = LocalAnalyzer::new(doc_read.deref(), context);

        analyzer.analyze(node_ref)?;

        Ok(analyzer.analysis)
    }
}

struct LocalAnalyzer<'doc, 'ctx, 'ctx_param, H: TaskHandle, S: SyncBuildHasher> {
    package: &'static PackageMeta,
    doc: &'doc ScriptDoc,
    context: &'ctx mut AttrContext<'ctx_param, ScriptNode, H, S>,
    analysis: LocalAnalysis,
    st_reachable: bool,
    loop_reachable: bool,
    loop_ref: NodeRef,
    depth: usize,
    namespace: AHashMap<CompactString, NameDesc>,
}

impl<'doc, 'ctx, 'ctx_param, H: TaskHandle, S: SyncBuildHasher>
    LocalAnalyzer<'doc, 'ctx, 'ctx_param, H, S>
{
    #[inline(always)]
    fn new(
        doc: &'doc ScriptDoc,
        context: &'ctx mut AttrContext<'ctx_param, ScriptNode, H, S>,
    ) -> Self {
        let Some(package) = PackageMeta::by_id(doc.id()) else {
            system_panic!("Missing package.");
        };

        Self {
            package,
            doc,
            context,
            analysis: LocalAnalysis::default(),
            st_reachable: true,
            loop_reachable: true,
            loop_ref: NodeRef::nil(),
            depth: 0,
            namespace: AHashMap::new(),
        }
    }

    fn analyze(&mut self, node_ref: &NodeRef) -> AnalysisResult<()> {
        self.context.proceed()?;

        let Some(script_node) = node_ref.deref(self.doc) else {
            return Ok(());
        };

        match script_node {
            ScriptNode::InlineComment { .. } => Ok(()),

            ScriptNode::MultilineComment { .. } => Ok(()),

            ScriptNode::Root { statements, .. } => self.analyze_root(node_ref, statements),

            ScriptNode::Clause { expr, .. } => self.analyze_clause(node_ref, expr),

            ScriptNode::Use { packages, .. } => self.analyze_use(node_ref, packages),

            ScriptNode::Package { .. } => Ok(()),

            ScriptNode::If {
                condition, body, ..
            } => self.analyze_if(node_ref, condition, body),

            ScriptNode::Match { subject, body, .. } => self.analyze_match(node_ref, subject, body),

            ScriptNode::MatchBody { .. } => Ok(()),

            ScriptNode::MatchArm { .. } => Ok(()),

            ScriptNode::Else { .. } => Ok(()),

            ScriptNode::Let { name, value, .. } => self.analyze_let(node_ref, name, value),

            ScriptNode::Var { .. } => Ok(()),

            ScriptNode::For {
                iterator,
                range,
                body,
                ..
            } => self.analyze_for(node_ref, iterator, range, body),

            ScriptNode::Loop { body, .. } => self.analyze_loop(node_ref, body),

            ScriptNode::Block { statements, .. } => self.analyze_block(statements),

            ScriptNode::Break { .. } => self.analyze_break(node_ref),

            ScriptNode::Continue { .. } => self.analyze_break(node_ref),

            ScriptNode::Return { result, .. } => self.analyze_return(node_ref, result),

            ScriptNode::Fn {
                parent,
                params,
                body,
                ..
            } => self.analyze_fn(node_ref, parent, params, body),

            ScriptNode::FnParams { parent, params, .. } => {
                self.analyze_fn_params(node_ref, parent, params)
            }

            ScriptNode::Struct { body, .. } => self.analyze_struct(node_ref, body),

            ScriptNode::StructBody { .. } => Ok(()),

            ScriptNode::StructEntry { .. } => Ok(()),

            ScriptNode::StructEntryKey { .. } => Ok(()),

            ScriptNode::Array { items, .. } => self.analyze_array(node_ref, items),

            ScriptNode::String { .. } => self.analyze_string(node_ref),

            ScriptNode::Crate { .. } => self.analyze_crate(node_ref),

            ScriptNode::This { .. } => self.analyze_this(node_ref),

            ScriptNode::Ident { token, .. } => self.analyze_ident(node_ref, token),

            ScriptNode::Number { token, .. } => self.analyze_number(node_ref, token),

            ScriptNode::Max { .. } => self.analyze_max(node_ref),

            ScriptNode::Bool { token, .. } => self.analyze_bool(node_ref, token),

            ScriptNode::UnaryLeft { op, right, .. } => {
                self.analyze_infix(node_ref, &NIL_NODE_REF, op, right)
            }

            ScriptNode::Binary {
                left, op, right, ..
            } => self.analyze_infix(node_ref, left, op, right),

            ScriptNode::Op { .. } => Ok(()),

            ScriptNode::Query { left, op, .. } => {
                self.analyze_infix(node_ref, left, op, &NIL_NODE_REF)
            }

            ScriptNode::Call { left, args, .. } => self.analyse_call(node_ref, left, args),

            ScriptNode::CallArgs { .. } => Ok(()),

            ScriptNode::Index { left, arg, .. } => self.analyse_index(node_ref, left, arg),

            ScriptNode::IndexArg { .. } => Ok(()),

            ScriptNode::Field { token, .. } => self.analyze_field(node_ref, token),

            ScriptNode::Expr { inner, .. } => self.analyze_expr(node_ref, inner),
        }
    }

    fn analyze_root(&mut self, node_ref: &NodeRef, statements: &[NodeRef]) -> AnalysisResult<()> {
        self.depth += 1;

        for hint in self.package.ty().prototype().hint_all_components() {
            let name = hint.name;

            let _ = self.namespace.insert(
                CompactString::from(name.string),
                NameDesc {
                    decl: *node_ref,
                    decl_depth: self.depth,
                    defs: Vec::new(),
                    init_depth: self.depth,
                },
            );
        }

        for st in statements {
            self.analyze(st)?;
        }

        self.depth -= 1;

        self.check_implicit_return();

        Ok(())
    }

    fn analyze_clause(&mut self, node_ref: &NodeRef, expr: &NodeRef) -> AnalysisResult<()> {
        self.check_st_reachability(node_ref);
        self.analyze_top_expr(expr)
    }

    fn analyze_top_expr(&mut self, expr: &NodeRef) -> AnalysisResult<()> {
        let Some(ScriptNode::Expr { inner, .. }) = expr.deref(self.doc) else {
            return self.analyze(expr);
        };

        let Some(ScriptNode::Binary {
            left, op, right, ..
        }) = inner.deref(self.doc)
        else {
            return self.analyze(expr);
        };

        let Some(op_token) = ScriptNode::extract_op(self.doc, op) else {
            return self.analyze(expr);
        };

        if op_token != ScriptToken::Assign {
            return self.analyze(expr);
        }

        let Some(ScriptNode::Ident {
            token: ident_token, ..
        }) = left.deref(self.doc)
        else {
            return self.analyze(expr);
        };

        let Some(ident_string) = ident_token.string(self.doc) else {
            return self.analyze(expr);
        };

        let Some(name_desc) = self.namespace.get(ident_string) else {
            return self.analyze(expr);
        };

        if name_desc.init_depth <= self.depth {
            return self.analyze(expr);
        }

        let Some(ScriptNode::Let { name: let_name, .. }) = name_desc.decl.deref(self.doc) else {
            return self.analyze(expr);
        };

        let Some(names) = self.analysis.names.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(lets) = names.lets.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(inits) = lets.map.entry(*let_name).or_default().get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = inits.set.insert(*right);

        let Some(idents) = names.idents.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = idents.map.insert(
            *left,
            IdentLocalResolution::Write {
                decl: name_desc.decl,
            },
        );

        self.analyze(right)?;

        let Some(name_desc) = self.namespace.get_mut(ident_string) else {
            system_panic!("Missing name entry.");
        };

        name_desc.init_depth = name_desc.init_depth.min(self.depth);
        name_desc.defs.push(*right);

        Ok(())
    }

    fn analyze_use(&mut self, node_ref: &NodeRef, packages: &[NodeRef]) -> AnalysisResult<()> {
        self.check_st_reachability(node_ref);

        if packages.is_empty() {
            return Ok(());
        }

        let mut resolution = Some((NodeRef::nil(), self.package));

        for package_ref in packages {
            let Some((_, parent)) = resolution else {
                break;
            };

            let Some(string) = ScriptNode::extract_atom_string(self.doc, package_ref) else {
                resolution = None;
                break;
            };

            let Some(syntax) = self.analysis.syntax.get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            let Some(names) = self.analysis.names.get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            let Some(packages) = names.packages.get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            if let Some(component) = parent.ty().prototype().hint_component(string) {
                if component.ty.is_package() {
                    if let Some(next) = component.ty.package() {
                        let _ = packages.map.insert(
                            *package_ref,
                            LocalPackageResolution {
                                parent: Some(parent),
                                package: Some(next),
                            },
                        );

                        resolution = Some((*package_ref, next));

                        continue;
                    }
                }
            }

            let Some(atoms) = syntax.atoms.get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            let _ = atoms
                .map
                .insert(*package_ref, LocalAtomSyntax(CompactString::from(string)));

            let _ = packages.map.insert(
                *package_ref,
                LocalPackageResolution {
                    parent: Some(parent),
                    package: None,
                },
            );

            break;
        }

        let Some((package_ref, package_meta)) = resolution else {
            return Ok(());
        };

        for component in package_meta.ty().prototype().hint_all_components() {
            let name = component.name;

            let _ = self.namespace.insert(
                CompactString::from(name.string),
                NameDesc {
                    decl: *node_ref,
                    decl_depth: self.depth,
                    defs: vec![package_ref],
                    init_depth: self.depth,
                },
            );
        }

        Ok(())
    }

    fn analyze_if(
        &mut self,
        node_ref: &NodeRef,
        condition: &NodeRef,
        body: &NodeRef,
    ) -> AnalysisResult<()> {
        self.check_st_reachability(node_ref);

        let exhaustive = ScriptNode::extract_bool(self.doc, *condition) == Some(true);

        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(ifs) = syntax.ifs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = ifs.map.insert(
            *node_ref,
            LocalIfSyntax {
                condition: *condition,
            },
        );

        self.analyze(condition)?;

        if let Some(ScriptNode::Block { statements, .. }) = body.deref(self.doc) {
            self.depth += 1;

            let namespace_before = self.namespace.clone();

            for st in statements {
                self.analyze(st)?;
            }

            let namespace_after = replace(&mut self.namespace, namespace_before);

            match exhaustive {
                true => {
                    for (name, mut name_desc) in namespace_after {
                        if name_desc.decl_depth >= self.depth {
                            continue;
                        }

                        if name_desc.init_depth == self.depth {
                            name_desc.init_depth -= 1;

                            let _ = self.namespace.insert(name, name_desc);
                        }
                    }
                }

                false => {
                    self.st_reachable = true;
                    self.loop_reachable = true;
                }
            }

            self.depth -= 1;
        }

        Ok(())
    }

    fn analyze_match(
        &mut self,
        node_ref: &NodeRef,
        subject: &NodeRef,
        body: &NodeRef,
    ) -> AnalysisResult<()> {
        self.check_st_reachability(node_ref);

        self.analyze(subject)?;

        let mut match_cases = AHashSet::new();

        let Some(ScriptNode::MatchBody { arms, .. }) = body.deref(self.doc) else {
            return Ok(());
        };

        let exhaustive = ScriptNode::is_match_exhaustive(self.doc, arms);

        let mut st_reachable = 0;
        let mut loop_reachable = 0;
        let mut defaults = 0;

        let mut inherited: AHashMap<CompactString, (usize, NameDesc)> = match exhaustive {
            true => AHashMap::with_capacity(arms.len()),
            false => AHashMap::new(),
        };

        self.depth += 1;

        for (index, arm) in arms.iter().enumerate() {
            let Some(ScriptNode::MatchArm { case, handler, .. }) = arm.deref(self.doc) else {
                continue;
            };

            let namespace_before = self.namespace.clone();

            self.analyze(case)?;

            if let Some(handler_node) = handler.deref(self.doc) {
                match handler_node {
                    ScriptNode::Expr { .. } => {
                        self.analyze_top_expr(handler)?;
                    }

                    ScriptNode::Block { statements, .. } => {
                        for st in statements {
                            self.analyze(st)?;
                        }
                    }

                    _ => (),
                }

                let namespace_after = replace(&mut self.namespace, namespace_before);

                if exhaustive {
                    for (name, mut name_desc) in namespace_after {
                        if name_desc.decl_depth >= self.depth {
                            continue;
                        }

                        if name_desc.init_depth == self.depth {
                            name_desc.init_depth -= 1;

                            if let Some((arms, inherited)) = inherited.get_mut(&name) {
                                if *arms < index {
                                    continue;
                                }

                                *arms += 1;

                                inherited.defs.append(&mut name_desc.defs);

                                continue;
                            }

                            if index > 0 {
                                continue;
                            }

                            if inherited.insert(name, (1, name_desc)).is_some() {
                                // Safety: Existence checked above.
                                unsafe { debug_unreachable!("Duplicate entry.") };
                            }
                        }
                    }
                }
            }

            if defaults == 1 {
                let Some(flow) = self.analysis.flow.get_mut() else {
                    // Safety: `self` state is unique during analysis.
                    unsafe { debug_unreachable!("Non-unique access.") };
                };

                let Some(unreachable_arms) = flow.unreachable_arms.get_mut() else {
                    // Safety: `self` state is unique during analysis.
                    unsafe { debug_unreachable!("Non-unique access.") };
                };

                let _ = unreachable_arms.set.insert(*arm);
            }

            match ScriptNode::is_default_case(self.doc, case) {
                true => defaults += 1,
                false => {
                    let _ = match_cases.insert(*case);
                }
            }

            if replace(&mut self.st_reachable, true) {
                st_reachable += 1;
            }

            if replace(&mut self.loop_reachable, true) {
                loop_reachable += 1;
            }
        }

        self.depth -= 1;

        if exhaustive {
            if st_reachable == 0 {
                self.st_reachable = false;
            }

            if loop_reachable == 0 {
                self.loop_reachable = false;
            }

            let arms_len = arms.len();

            for (name, (arms, name_desc)) in inherited {
                if arms < arms_len {
                    continue;
                }

                let _ = self.namespace.insert(name, name_desc);
            }
        }

        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(matches) = syntax.matches.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = matches.map.insert(
            *node_ref,
            Shared::new(LocalMatchSyntax {
                subject: *subject,
                cases: match_cases,
            }),
        );

        Ok(())
    }

    fn analyze_let(
        &mut self,
        node_ref: &NodeRef,
        name: &NodeRef,
        value: &NodeRef,
    ) -> AnalysisResult<()> {
        self.check_st_reachability(node_ref);

        if !value.is_nil() {
            self.analyze(value)?;

            let Some(names) = self.analysis.names.get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            let Some(lets) = names.lets.get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            let Some(inits) = lets.map.entry(*name).or_default().get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            let _ = inits.set.insert(*value);
        }

        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(vars) = syntax.vars.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = vars.map.insert(*name, LocalVarSyntax::Let(*node_ref));

        let Some(name_string) = ScriptNode::extract_atom_string(self.doc, name) else {
            return Ok(());
        };

        let _ = self.namespace.insert(
            CompactString::from(name_string),
            NameDesc {
                decl: *node_ref,
                decl_depth: self.depth,
                defs: match value.is_nil() {
                    true => Vec::new(),
                    false => vec![*value],
                },
                init_depth: match value.is_nil() {
                    true => usize::MAX,
                    false => self.depth,
                },
            },
        );

        Ok(())
    }

    fn analyze_for(
        &mut self,
        node_ref: &NodeRef,
        iterator: &NodeRef,
        range: &NodeRef,
        body: &NodeRef,
    ) -> AnalysisResult<()> {
        self.check_st_reachability(node_ref);

        self.analyze(range)?;

        let Some(names) = self.analysis.names.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(lets) = names.lets.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(inits) = lets.map.entry(*iterator).or_default().get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = inits.set.insert(*node_ref);

        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(fors) = syntax.fors.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = fors.map.insert(
            *node_ref,
            LocalForSyntax {
                iterator: *iterator,
                range: *range,
            },
        );

        let Some(vars) = syntax.vars.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = vars.map.insert(*iterator, LocalVarSyntax::For(*node_ref));

        let Some(ScriptNode::Block { statements, .. }) = body.deref(self.doc) else {
            return Ok(());
        };

        let namespace_before = self.namespace.clone();
        let loop_ref_before = self.loop_ref;
        self.loop_ref = *node_ref;

        if let Some(name) = ScriptNode::extract_atom_string(self.doc, iterator) {
            let _ = self.namespace.insert(
                CompactString::from(name),
                NameDesc {
                    decl: *node_ref,
                    decl_depth: self.depth,
                    defs: vec![*iterator],
                    init_depth: self.depth,
                },
            );
        }

        self.depth += 1;

        for st in statements {
            self.analyze(st)?;
        }

        self.depth -= 1;

        self.loop_ref = loop_ref_before;
        self.namespace = namespace_before;
        self.st_reachable = true;
        self.loop_reachable = true;

        Ok(())
    }

    fn analyze_loop(&mut self, node_ref: &NodeRef, body: &NodeRef) -> AnalysisResult<()> {
        self.check_st_reachability(node_ref);

        let Some(ScriptNode::Block { statements, .. }) = body.deref(self.doc) else {
            return Ok(());
        };

        let loop_ref_before = self.loop_ref;
        self.loop_ref = *node_ref;

        self.depth += 1;

        let namespace_before = self.namespace.clone();

        for st in statements {
            self.analyze(st)?;
        }

        let namespace_after = replace(&mut self.namespace, namespace_before);

        for (name, mut name_desc) in namespace_after {
            if name_desc.decl_depth >= self.depth {
                continue;
            }

            if name_desc.init_depth == self.depth {
                name_desc.init_depth -= 1;

                let _ = self.namespace.insert(name, name_desc);
            }
        }

        self.depth -= 1;

        self.loop_ref = loop_ref_before;
        self.loop_reachable = true;

        Ok(())
    }

    fn analyze_block(&mut self, statements: &[NodeRef]) -> AnalysisResult<()> {
        self.depth += 1;

        let namespace_before = self.namespace.clone();

        for st in statements {
            self.analyze(st)?;
        }

        let namespace_after = replace(&mut self.namespace, namespace_before);

        for (name, mut name_desc) in namespace_after {
            if name_desc.decl_depth >= self.depth {
                continue;
            }

            if name_desc.init_depth == self.depth {
                name_desc.init_depth -= 1;

                let _ = self.namespace.insert(name, name_desc);
            }
        }

        self.depth -= 1;

        Ok(())
    }

    fn analyze_break(&mut self, node_ref: &NodeRef) -> AnalysisResult<()> {
        let Some(flow) = self.analysis.flow.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(break_to_loop) = flow.break_to_loop.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = break_to_loop.map.insert(*node_ref, self.loop_ref);

        if !self.loop_ref.is_nil() {
            let Some(loop_to_break) = flow.loop_to_break.get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            let Some(breaks) = loop_to_break
                .map
                .entry(self.loop_ref)
                .or_default()
                .get_mut()
            else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            breaks.set.insert(*node_ref);
        }

        self.check_st_reachability(node_ref);

        self.loop_reachable = false;

        Ok(())
    }

    fn analyze_return(&mut self, node_ref: &NodeRef, result: &NodeRef) -> AnalysisResult<()> {
        self.check_st_reachability(node_ref);

        self.analyze(result)?;

        let Some(flow) = self.analysis.flow.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(return_points) = flow.return_points.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = return_points.set.insert(match result.is_nil() {
            true => LocalReturnPoint::Explicit(*node_ref),
            false => LocalReturnPoint::Expr(*result),
        });

        self.st_reachable = false;

        Ok(())
    }

    fn analyze_fn(
        &mut self,
        node_ref: &NodeRef,
        parent_ref: &NodeRef,
        params: &NodeRef,
        body: &NodeRef,
    ) -> AnalysisResult<()> {
        if self.depth == 0 {
            self.analyze(params)?;

            match body.deref(self.doc) {
                Some(ScriptNode::Block { .. }) => {
                    self.analyze(body)?;
                    self.check_implicit_return();
                }

                Some(ScriptNode::Expr { .. }) => {
                    self.depth += 1;
                    self.analyze(body)?;
                    self.depth -= 1;

                    let Some(flow) = self.analysis.flow.get_mut() else {
                        // Safety: `self` state is unique during analysis.
                        unsafe { debug_unreachable!("Non-unique access.") };
                    };

                    let Some(return_points) = flow.return_points.get_mut() else {
                        // Safety: `self` collections unique during analysis.
                        unsafe { debug_unreachable!("Non-unique access.") }
                    };

                    let _ = return_points.set.insert(LocalReturnPoint::Expr(*body));
                }

                _ => (),
            }

            return Ok(());
        }

        self.snapshot_namespace(node_ref);

        let context_ref = self.infer_fn_context(*parent_ref, *node_ref);

        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = exprs.map.insert(*node_ref, LocalExprSyntax::Fn(*node_ref));

        if !context_ref.is_nil() {
            let Some(fn_contexts) = syntax.fn_contexts.get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            let _ = fn_contexts.map.insert(
                *node_ref,
                LocalFnContextSyntax {
                    struct_ref: context_ref,
                },
            );
        }

        Ok(())
    }

    fn analyze_fn_params(
        &mut self,
        node_ref: &NodeRef,
        parent: &NodeRef,
        params: &[NodeRef],
    ) -> AnalysisResult<()> {
        let mut vector = Vec::with_capacity(params.len());

        for (index, param_ref) in params.iter().enumerate() {
            let Some(syntax) = self.analysis.syntax.get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            let Some(vars) = syntax.vars.get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            let _ = vars
                .map
                .insert(*param_ref, LocalVarSyntax::Param(*parent, index));

            let Some(name) = ScriptNode::extract_atom_string(self.doc, param_ref) else {
                vector.push((true, *param_ref));
                continue;
            };

            if self.namespace.contains_key(name) {
                vector.push((false, *param_ref));
                continue;
            }

            let _ = self.namespace.insert(
                CompactString::from(name),
                NameDesc {
                    decl: *node_ref,
                    decl_depth: self.depth,
                    defs: vec![*param_ref],
                    init_depth: self.depth,
                },
            );

            vector.push((true, *param_ref));
        }

        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(sig) = syntax.sig.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        sig.params = vector;

        Ok(())
    }

    fn analyze_struct(&mut self, node_ref: &NodeRef, body: &NodeRef) -> AnalysisResult<()> {
        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = exprs
            .map
            .insert(*node_ref, LocalExprSyntax::Struct(*node_ref));

        let Some(ScriptNode::StructBody { entries, .. }) = body.deref(self.doc) else {
            return Ok(());
        };

        let mut entry_vec = Vec::with_capacity(entries.len());
        let mut value_map = AHashMap::with_capacity(entries.len());

        for entry in entries {
            let Some(ScriptNode::StructEntry { key, value, .. }) = entry.deref(self.doc) else {
                continue;
            };

            if let Some(key_string) = ScriptNode::extract_atom_string(self.doc, key) {
                let key_string = CompactString::from(key_string);

                entry_vec.push((key_string.clone(), *key, *value));

                if let Entry::Vacant(entry) = value_map.entry(key_string) {
                    entry.insert((*key, *value));
                }
            }

            self.analyze(value)?;
        }

        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(struct_entry_vecs) = syntax.struct_entry_vecs.get_mut() else {
            return Ok(());
        };

        let _ = struct_entry_vecs.map.insert(
            *node_ref,
            Shared::new(LocalStructEntriesVecSyntax { vec: entry_vec }),
        );

        let Some(struct_entry_maps) = syntax.struct_entry_maps.get_mut() else {
            return Ok(());
        };

        let _ = struct_entry_maps.map.insert(
            *node_ref,
            Shared::new(LocalStructEntiesMapSyntax { map: value_map }),
        );

        Ok(())
    }

    fn analyze_array(&mut self, node_ref: &NodeRef, items: &Vec<NodeRef>) -> AnalysisResult<()> {
        for item in items {
            self.analyze(item)?;
        }

        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = exprs
            .map
            .insert(*node_ref, LocalExprSyntax::Array(*node_ref));

        let Some(arrays) = syntax.arrays.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = arrays.map.insert(
            *node_ref,
            Shared::new(LocalArraySyntax {
                items: items.clone(),
            }),
        );

        Ok(())
    }

    fn analyze_string(&mut self, node_ref: &NodeRef) -> AnalysisResult<()> {
        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = exprs
            .map
            .insert(*node_ref, LocalExprSyntax::String(*node_ref));

        Ok(())
    }

    fn analyze_crate(&mut self, node_ref: &NodeRef) -> AnalysisResult<()> {
        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = exprs
            .map
            .insert(*node_ref, LocalExprSyntax::Crate(*node_ref));

        Ok(())
    }

    fn analyze_this(&mut self, node_ref: &NodeRef) -> AnalysisResult<()> {
        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = exprs
            .map
            .insert(*node_ref, LocalExprSyntax::This(*node_ref));

        Ok(())
    }

    fn analyze_ident(&mut self, node_ref: &NodeRef, token: &TokenRef) -> AnalysisResult<()> {
        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = exprs
            .map
            .insert(*node_ref, LocalExprSyntax::Ident(*node_ref));

        let Some(token_string) = token.string(self.doc) else {
            return Ok(());
        };

        let Some(atoms) = syntax.atoms.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        atoms.map.insert(
            *node_ref,
            LocalAtomSyntax(CompactString::from(token_string)),
        );

        let local_ident = match self.namespace.get(token_string) {
            Some(name_desc) => IdentLocalResolution::Read {
                name: name_desc.local_name(self.depth),
            },

            None => {
                self.snapshot_namespace(node_ref);

                IdentLocalResolution::Closure
            }
        };

        let Some(names) = self.analysis.names.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(idents) = names.idents.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = idents.map.insert(*node_ref, local_ident);

        Ok(())
    }

    fn analyze_number(&mut self, node_ref: &NodeRef, token: &TokenRef) -> AnalysisResult<()> {
        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = exprs
            .map
            .insert(*node_ref, LocalExprSyntax::Number(*node_ref));

        let Some(token_string) = token.string(self.doc) else {
            return Ok(());
        };

        let Some(atoms) = syntax.atoms.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = atoms.map.insert(
            *node_ref,
            LocalAtomSyntax(CompactString::from(token_string)),
        );

        Ok(())
    }

    fn analyze_max(&mut self, node_ref: &NodeRef) -> AnalysisResult<()> {
        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = exprs.map.insert(*node_ref, LocalExprSyntax::Max(*node_ref));

        Ok(())
    }

    fn analyze_bool(&mut self, node_ref: &NodeRef, token: &TokenRef) -> AnalysisResult<()> {
        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = exprs
            .map
            .insert(*node_ref, LocalExprSyntax::Bool(*node_ref));

        let Some(token_string) = token.string(self.doc) else {
            return Ok(());
        };

        let Some(atoms) = syntax.atoms.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = atoms.map.insert(
            *node_ref,
            LocalAtomSyntax(CompactString::from(token_string)),
        );

        Ok(())
    }

    fn analyze_infix(
        &mut self,
        node_ref: &NodeRef,
        left: &NodeRef,
        op: &NodeRef,
        right: &NodeRef,
    ) -> AnalysisResult<()> {
        self.analyze(left)?;
        self.analyze(right)?;

        let Some(op_token) = ScriptNode::extract_op(self.doc, op) else {
            return Ok(());
        };

        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = exprs
            .map
            .insert(*node_ref, LocalExprSyntax::Infix(*node_ref));

        let Some(infixes) = syntax.infixes.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = infixes.map.insert(
            *node_ref,
            LocalInfixSyntax {
                left: *left,
                op_ref: *op,
                op: op_token,
                right: *right,
            },
        );

        Ok(())
    }

    fn analyse_call(
        &mut self,
        node_ref: &NodeRef,
        left: &NodeRef,
        args: &NodeRef,
    ) -> AnalysisResult<()> {
        self.analyze(left)?;

        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = exprs
            .map
            .insert(*node_ref, LocalExprSyntax::Call(*node_ref));

        let Some(ScriptNode::CallArgs {
            args: arg_nodes, ..
        }) = args.deref(self.doc)
        else {
            return Ok(());
        };

        let Some(calls) = syntax.calls.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = calls.map.insert(
            *node_ref,
            Shared::new(LocalCallSyntax {
                left: *left,
                call_args_ref: *args,
                args: arg_nodes.clone(),
            }),
        );

        for (index, mut arg_node) in arg_nodes.iter().enumerate() {
            self.analyze(arg_node)?;

            loop {
                match arg_node.deref(self.doc) {
                    Some(ScriptNode::Fn { .. }) => (),
                    Some(ScriptNode::Expr { inner, .. }) => {
                        arg_node = inner;
                        continue;
                    }

                    _ => break,
                }

                let Some(syntax) = self.analysis.syntax.get_mut() else {
                    // Safety: `self` state is unique during analysis.
                    unsafe { debug_unreachable!("Non-unique access.") };
                };

                let Some(args) = syntax.args.get_mut() else {
                    // Safety: `self` state is unique during analysis.
                    unsafe { debug_unreachable!("Non-unique access.") };
                };

                args.map.insert(
                    *arg_node,
                    LocalArgSyntax {
                        call_left_ref: *left,
                        arg_index: index,
                    },
                );

                break;
            }
        }

        Ok(())
    }

    fn analyse_index(
        &mut self,
        node_ref: &NodeRef,
        left: &NodeRef,
        arg: &NodeRef,
    ) -> AnalysisResult<()> {
        self.analyze(left)?;

        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = exprs
            .map
            .insert(*node_ref, LocalExprSyntax::Index(*node_ref));

        let Some(ScriptNode::IndexArg { arg: arg_node, .. }) = arg.deref(self.doc) else {
            return Ok(());
        };

        let Some(indexes) = syntax.indexes.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = indexes.map.insert(
            *node_ref,
            LocalIndexSyntax {
                left: *left,
                index_arg_ref: *arg,
                index: *arg_node,
            },
        );

        self.analyze(arg_node)?;

        Ok(())
    }

    fn analyze_field(&mut self, node_ref: &NodeRef, token: &TokenRef) -> AnalysisResult<()> {
        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(token_string) = token.string(self.doc) else {
            return Ok(());
        };

        let Some(atoms) = syntax.atoms.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = atoms.map.insert(
            *node_ref,
            LocalAtomSyntax(CompactString::from(token_string)),
        );

        Ok(())
    }

    fn analyze_expr(&mut self, node_ref: &NodeRef, inner: &NodeRef) -> AnalysisResult<()> {
        self.analyze(inner)?;

        let Some(syntax) = self.analysis.syntax.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(exprs) = syntax.exprs.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(expr_syntax) = exprs.map.get(inner) else {
            return Ok(());
        };

        let _ = exprs.map.insert(*node_ref, *expr_syntax);

        Ok(())
    }

    fn infer_fn_context(&self, mut parent_ref: NodeRef, mut node_ref: NodeRef) -> NodeRef {
        loop {
            let Some(parent_node) = parent_ref.deref(self.doc) else {
                parent_ref = NodeRef::nil();
                break;
            };

            let (left, op, right) = match parent_node {
                ScriptNode::Expr { node, parent, .. }
                | ScriptNode::StructEntry { node, parent, .. }
                | ScriptNode::StructBody { node, parent, .. } => {
                    node_ref = *node;
                    parent_ref = *parent;
                    continue;
                }

                ScriptNode::Binary {
                    left, op, right, ..
                } => (left, op, right),

                ScriptNode::Struct { .. } => {
                    break;
                }

                _ => {
                    parent_ref = NodeRef::nil();
                    break;
                }
            };

            let Some(op) = ScriptNode::extract_op(self.doc, op) else {
                parent_ref = NodeRef::nil();
                break;
            };

            match op {
                ScriptToken::Assign if right == &node_ref => {
                    parent_ref = *left;
                    continue;
                }

                ScriptToken::Dot => parent_ref = *left,

                _ => parent_ref = NodeRef::nil(),
            }

            break;
        }

        parent_ref
    }

    fn check_st_reachability(&mut self, node_ref: &NodeRef) {
        if !self.st_reachable || !self.loop_reachable {
            let Some(flow) = self.analysis.flow.get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            let Some(unreachable_statements) = flow.unreachable_statements.get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            let _ = unreachable_statements.set.insert(*node_ref);

            self.st_reachable = true;
            self.loop_reachable = true;
        }
    }

    fn check_implicit_return(&mut self) {
        if self.st_reachable {
            let Some(flow) = self.analysis.flow.get_mut() else {
                // Safety: `self` state is unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") };
            };

            let Some(return_points) = flow.return_points.get_mut() else {
                // Safety: `self` collections unique during analysis.
                unsafe { debug_unreachable!("Non-unique access.") }
            };

            return_points.set.insert(LocalReturnPoint::Implicit);
        }
    }

    fn snapshot_namespace(&mut self, node_ref: &NodeRef) {
        let snapshot = self
            .namespace
            .iter()
            .map(|(name, name_desc)| (name.clone(), name_desc.local_name(self.depth)))
            .collect();

        let Some(local_names) = self.analysis.names.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let Some(namespaces) = local_names.namespaces.get_mut() else {
            // Safety: `self` state is unique during analysis.
            unsafe { debug_unreachable!("Non-unique access.") };
        };

        let _ = namespaces
            .map
            .insert(*node_ref, Shared::new(LocalNamespace { map: snapshot }));
    }
}

#[derive(Clone)]
struct NameDesc {
    decl: NodeRef,
    decl_depth: usize,
    defs: Vec<NodeRef>,
    init_depth: usize,
}

impl NameDesc {
    fn local_name(&self, depth: usize) -> Shared<Name> {
        Shared::new(Name {
            init: self.init_depth <= depth,
            decl: self.decl,
            defs: self.defs.iter().copied().collect(),
        })
    }
}
