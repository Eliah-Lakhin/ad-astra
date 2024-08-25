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
    collections::hash_map::Entry,
    mem::{replace, take},
    ops::Deref,
};

use ahash::{AHashMap, AHashSet};
use compact_str::CompactString;
use lady_deirdre::{
    analysis::{AnalysisResult, AttrContext, Computable, Semantics, TaskHandle},
    lexis::TokenRef,
    sync::SyncBuildHasher,
    syntax::NodeRef,
};

use crate::{
    analysis::ModuleResultEx,
    report::system_panic,
    semantics::{setup::log_attr, FnSemantics, IdentLocalResolution, LocalIdentMap},
    syntax::{ScriptDoc, ScriptNode, ScriptToken},
};

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct Lifetimes {
    pub(crate) ident_drops: AHashSet<NodeRef>,
    pub(crate) closure_drops: AHashMap<NodeRef, AHashSet<CompactString>>,
    pub(crate) unused_vars: AHashSet<NodeRef>,
}

impl Computable for Lifetimes {
    type Node = ScriptNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
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

                let idents = root_semantics.locals.idents.read(context).forward()?;

                let mut analyzer = LifetimeAnalyzer {
                    doc,
                    context,
                    idents: idents.deref().as_ref(),
                    gen: AHashMap::new(),
                    ident_drops: AHashSet::new(),
                    closure_drops: AHashMap::new(),
                    unused_vars: AHashSet::new(),
                };

                analyzer.analyze_statements(statements)?;

                Ok(Self {
                    ident_drops: analyzer.ident_drops,
                    closure_drops: analyzer.closure_drops,
                    unused_vars: analyzer.unused_vars,
                })
            }

            Some(ScriptNode::Fn {
                params,
                body,
                semantics,
                ..
            }) => {
                let fn_semantics = semantics.get().forward()?;

                let idents = fn_semantics.locals.idents.read(context).forward()?;

                let mut analyzer = LifetimeAnalyzer {
                    doc,
                    context,
                    idents: idents.deref().as_ref(),
                    gen: AHashMap::new(),
                    ident_drops: AHashSet::new(),
                    closure_drops: AHashMap::new(),
                    unused_vars: AHashSet::new(),
                };

                match body.deref(doc) {
                    Some(ScriptNode::Block { statements, .. }) => {
                        analyzer.analyze_statements(statements)?;
                    }

                    Some(ScriptNode::Expr { inner, .. }) => {
                        analyzer.analyze_expr(inner)?;
                    }

                    _ => return Ok(Self::default()),
                }

                let Some(ScriptNode::FnParams { params, .. }) = params.deref(doc) else {
                    return Ok(Self::default());
                };

                for param_ref in params.iter().rev() {
                    analyzer.analyze_var_intro(param_ref)?;
                }

                Ok(Self {
                    ident_drops: analyzer.ident_drops,
                    closure_drops: analyzer.closure_drops,
                    unused_vars: analyzer.unused_vars,
                })
            }

            _ => Ok(Self::default()),
        }
    }
}

struct LifetimeAnalyzer<'doc, 'ctx, 'ctx_param, H: TaskHandle, S: SyncBuildHasher> {
    doc: &'doc ScriptDoc,
    context: &'ctx mut AttrContext<'ctx_param, ScriptNode, H, S>,
    idents: &'doc LocalIdentMap,
    gen: AHashMap<CompactString, Vec<GenDesc>>,
    ident_drops: AHashSet<NodeRef>,
    closure_drops: AHashMap<NodeRef, AHashSet<CompactString>>,
    unused_vars: AHashSet<NodeRef>,
}

impl<'doc, 'ctx, 'ctx_param, H: TaskHandle, S: SyncBuildHasher>
    LifetimeAnalyzer<'doc, 'ctx, 'ctx_param, H, S>
{
    fn analyze_block(&mut self, isolation: Isolation, block_ref: &NodeRef) -> AnalysisResult<()> {
        let statements = match block_ref.deref(self.doc) {
            Some(ScriptNode::Block { statements, .. }) => statements,
            Some(ScriptNode::Root { statements, .. }) => statements,
            _ => return Ok(()),
        };

        match isolation {
            Isolation::Nested => {
                let gen_before = take(&mut self.gen);
                self.analyze_statements(statements)?;
                let gen_after = replace(&mut self.gen, gen_before);

                for (name, gens) in gen_after {
                    let Entry::Vacant(entry) = self.gen.entry(name) else {
                        continue;
                    };

                    let _ = entry.insert(gens);
                }
            }

            Isolation::Loop(iterator) => {
                let gen_before = take(&mut self.gen);
                self.analyze_statements(statements)?;
                let gen_after = replace(&mut self.gen, gen_before);

                let iterator_string = iterator.string(self.doc).unwrap_or("");

                for (name, _) in gen_after {
                    if name == iterator_string {
                        continue;
                    }

                    let Entry::Vacant(entry) = self.gen.entry(name) else {
                        continue;
                    };

                    let _ = entry.insert(Vec::new());
                }
            }
        }

        Ok(())
    }

    fn analyze_statements(&mut self, statements: &[NodeRef]) -> AnalysisResult<()> {
        for st in statements.iter().rev() {
            self.analyze_st(st)?;
        }

        Ok(())
    }

    fn analyze_st(&mut self, st: &NodeRef) -> AnalysisResult<()> {
        self.context.proceed()?;

        let Some(script_node) = st.deref(self.doc) else {
            return Ok(());
        };

        match script_node {
            ScriptNode::Clause { expr, .. } => self.analyze_expr(expr),

            ScriptNode::If {
                condition, body, ..
            } => self.analyze_if(condition, body),

            ScriptNode::Match { subject, body, .. } => self.analyze_match(subject, body),

            ScriptNode::Let { name, value, .. } => self.analyze_let(name, value),

            ScriptNode::For {
                iterator,
                range,
                body,
                ..
            } => self.analyze_for(iterator, range, body),

            ScriptNode::Loop { body, .. } => self.analyze_loop(body),

            ScriptNode::Block { .. } => self.analyze_block(Isolation::Nested, st),

            ScriptNode::Break { .. } => Ok(()),

            ScriptNode::Continue { .. } => Ok(()),

            ScriptNode::Return { result, .. } => self.analyze_expr(result),

            _ => Ok(()),
        }
    }

    fn analyze_if(&mut self, condition: &NodeRef, body: &NodeRef) -> AnalysisResult<()> {
        self.analyze_block(Isolation::Nested, body)?;
        self.analyze_expr(condition)?;

        Ok(())
    }

    fn analyze_match(&mut self, subject: &NodeRef, body: &NodeRef) -> AnalysisResult<()> {
        self.analyze_match_body(body)?;
        self.analyze_expr(subject)?;

        Ok(())
    }

    fn analyze_match_body(&mut self, body: &NodeRef) -> AnalysisResult<()> {
        let Some(ScriptNode::MatchBody { arms, .. }) = body.deref(self.doc) else {
            return Ok(());
        };

        let mut new_gens = AHashMap::new();

        for arm_ref in arms.iter().rev() {
            let Some(ScriptNode::MatchArm { case, handler, .. }) = arm_ref.deref(self.doc) else {
                continue;
            };

            let gen_before = take(&mut self.gen);

            match handler.deref(self.doc) {
                Some(ScriptNode::Block { statements, .. }) => {
                    self.analyze_statements(statements)?;
                }

                Some(ScriptNode::Expr { inner, .. }) => {
                    self.analyze_expr(inner)?;
                }

                _ => (),
            }

            if let Some(ScriptNode::Expr { inner, .. }) = case.deref(self.doc) {
                self.analyze_expr(inner)?;
            }

            let gen_after = replace(&mut self.gen, gen_before);

            for (name, mut gens) in gen_after {
                if self.gen.contains_key(&name) {
                    continue;
                }

                match new_gens.entry(name) {
                    Entry::Vacant(entry) => {
                        let _ = entry.insert(gens);
                    }

                    Entry::Occupied(mut entry) => {
                        entry.get_mut().append(&mut gens);
                    }
                }
            }
        }

        for (name, gens) in new_gens {
            if self.gen.insert(name, gens).is_some() {
                system_panic!("Duplicate gen entry.");
            }
        }

        Ok(())
    }

    fn analyze_let(&mut self, name: &NodeRef, value: &NodeRef) -> AnalysisResult<()> {
        self.analyze_var_intro(name)?;
        self.analyze_expr(value)?;

        Ok(())
    }

    fn analyze_var_intro(&mut self, var_ref: &NodeRef) -> AnalysisResult<()> {
        let Some(ScriptNode::Var { token, .. }) = var_ref.deref(self.doc) else {
            return Ok(());
        };

        let Some(token_string) = token.string(self.doc) else {
            return Ok(());
        };

        let Some(gens) = self.gen.remove(token_string) else {
            let _ = self.unused_vars.insert(*var_ref);

            return Ok(());
        };

        for gen in gens {
            match gen {
                GenDesc::Ident(node_ref) => {
                    let _ = self.ident_drops.insert(node_ref);
                }

                GenDesc::Fn(node_ref) => match self.closure_drops.entry(node_ref) {
                    Entry::Occupied(mut entry) => {
                        let closures = entry.get_mut();

                        let _ = closures.insert(CompactString::from(token_string));
                    }

                    Entry::Vacant(entry) => {
                        let _ = entry.insert(AHashSet::from([CompactString::from(token_string)]));
                    }
                },
            }
        }

        Ok(())
    }

    fn analyze_for(
        &mut self,
        iterator: &NodeRef,
        range: &NodeRef,
        body: &NodeRef,
    ) -> AnalysisResult<()> {
        let iterator = match iterator.deref(self.doc) {
            Some(ScriptNode::Var { token, .. }) => *token,
            _ => TokenRef::nil(),
        };

        self.analyze_block(Isolation::Loop(iterator), body)?;
        self.analyze_expr(range)?;

        Ok(())
    }

    fn analyze_loop(&mut self, body: &NodeRef) -> AnalysisResult<()> {
        self.analyze_block(Isolation::Loop(TokenRef::nil()), body)?;

        Ok(())
    }

    fn analyze_expr(&mut self, node_ref: &NodeRef) -> AnalysisResult<()> {
        let Some(script_node) = node_ref.deref(self.doc) else {
            return Ok(());
        };

        match script_node {
            ScriptNode::Fn { semantics, .. } => self.analyze_fn(node_ref, semantics),
            ScriptNode::Struct { body, .. } => self.analyze_struct(body),
            ScriptNode::Array { items, .. } => self.analyze_array(items),
            ScriptNode::Ident { token, .. } => self.analyze_ident(node_ref, token),
            ScriptNode::UnaryLeft { right, .. } => self.analyze_expr(right),
            ScriptNode::Binary {
                left, op, right, ..
            } => self.analyze_binary(left, op, right),
            ScriptNode::Query { left, .. } => self.analyze_expr(left),
            ScriptNode::Call { left, args, .. } => self.analyze_call(left, args),
            ScriptNode::Index { left, arg, .. } => self.analyze_index(left, arg),
            ScriptNode::Expr { inner, .. } => self.analyze_expr(inner),

            _ => Ok(()),
        }
    }

    fn analyze_fn(
        &mut self,
        fn_ref: &NodeRef,
        semantics: &Semantics<FnSemantics>,
    ) -> AnalysisResult<()> {
        let fn_semantics = semantics.get().forward()?;

        let closure_vec = fn_semantics
            .compilation
            .closure_vec
            .read(self.context)
            .forward()?;

        for closure in closure_vec.vec.iter().rev() {
            if self.gen.contains_key(closure) {
                continue;
            }

            let _ = self.gen.insert(closure.clone(), vec![GenDesc::Fn(*fn_ref)]);
        }

        Ok(())
    }

    fn analyze_struct(&mut self, body: &NodeRef) -> AnalysisResult<()> {
        let Some(ScriptNode::StructBody { entries, .. }) = body.deref(self.doc) else {
            return Ok(());
        };

        for entry_ref in entries.iter().rev() {
            let Some(ScriptNode::StructEntry { value, .. }) = entry_ref.deref(self.doc) else {
                continue;
            };

            self.analyze_expr(value)?;
        }

        Ok(())
    }

    fn analyze_array(&mut self, items: &[NodeRef]) -> AnalysisResult<()> {
        for item_ref in items.iter().rev() {
            self.analyze_expr(item_ref)?;
        }

        Ok(())
    }

    fn analyze_ident(&mut self, ident_ref: &NodeRef, token: &TokenRef) -> AnalysisResult<()> {
        let Some(ident_string) = token.string(self.doc) else {
            return Ok(());
        };

        if self.gen.contains_key(ident_string) {
            return Ok(());
        }

        let _ = self.gen.insert(
            CompactString::from(ident_string),
            vec![GenDesc::Ident(*ident_ref)],
        );

        Ok(())
    }

    fn analyze_binary(
        &mut self,
        left: &NodeRef,
        op: &NodeRef,
        right: &NodeRef,
    ) -> AnalysisResult<()> {
        let Some(ScriptNode::Op { token, .. }) = op.deref(self.doc) else {
            return Ok(());
        };

        match token.deref(self.doc) {
            Some(ScriptToken::Assign) => self.analyze_binary_assign(left, right),

            Some(ScriptToken::Dot) => self.analyze_expr(left),

            Some(
                ScriptToken::PlusAssign
                | ScriptToken::MinusAssign
                | ScriptToken::MulAssign
                | ScriptToken::DivAssign
                | ScriptToken::BitAndAssign
                | ScriptToken::BitOrAssign
                | ScriptToken::BitXorAssign
                | ScriptToken::ShlAssign
                | ScriptToken::ShrAssign
                | ScriptToken::RemAssign,
            ) => self.analyze_binary_right(left, right),

            _ => self.analyze_binary_left(left, right),
        }
    }

    fn analyze_binary_assign(&mut self, left: &NodeRef, right: &NodeRef) -> AnalysisResult<()> {
        let Some(IdentLocalResolution::Write { .. }) = self.idents.map.get(left) else {
            return self.analyze_binary_right(left, right);
        };

        self.analyze_expr(right)?;

        Ok(())
    }

    fn analyze_binary_left(&mut self, left: &NodeRef, right: &NodeRef) -> AnalysisResult<()> {
        self.analyze_expr(right)?;
        self.analyze_expr(left)?;

        Ok(())
    }

    fn analyze_binary_right(&mut self, left: &NodeRef, right: &NodeRef) -> AnalysisResult<()> {
        self.analyze_expr(left)?;
        self.analyze_expr(right)?;

        Ok(())
    }

    fn analyze_call(&mut self, left: &NodeRef, args: &NodeRef) -> AnalysisResult<()> {
        self.analyze_expr(left)?;

        let Some(ScriptNode::CallArgs { args, .. }) = args.deref(self.doc) else {
            return Ok(());
        };

        for arg_ref in args.iter().rev() {
            self.analyze_expr(arg_ref)?;
        }

        Ok(())
    }

    fn analyze_index(&mut self, left: &NodeRef, args: &NodeRef) -> AnalysisResult<()> {
        self.analyze_expr(left)?;

        let Some(ScriptNode::IndexArg { arg, .. }) = args.deref(self.doc) else {
            return Ok(());
        };

        self.analyze_expr(arg)?;

        Ok(())
    }
}

enum Isolation {
    Nested,
    Loop(TokenRef),
}

#[derive(Clone)]
enum GenDesc {
    Ident(NodeRef),
    Fn(NodeRef),
}
