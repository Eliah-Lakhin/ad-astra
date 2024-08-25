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
    analysis::{AnalysisResult, AttrContext, Computable, Semantics, TaskHandle},
    arena::Identifiable,
    sync::SyncBuildHasher,
    syntax::NodeRef,
};

use crate::{
    analysis::{Closeness, ModuleResultEx, ScriptIssue, StringEstimation},
    report::system_panic,
    runtime::{ops::OperatorKind, PackageMeta, Prototype, ScriptType, TypeFamily, TypeMeta},
    semantics::*,
    syntax::{ScriptDoc, ScriptNode, ScriptToken},
};

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct ResultResolution {
    pub(crate) tag: Tag,
}

impl Computable for ResultResolution {
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

        let return_points = locals.return_points.read(context).forward()?;

        let mut tag = Tag::Unset;

        for return_point in &return_points.as_ref().set {
            match return_point {
                LocalReturnPoint::Implicit | LocalReturnPoint::Explicit(_) => tag.merge(Tag::nil()),

                LocalReturnPoint::Expr(expr_ref) => {
                    let Some(expr_node) = expr_ref.deref(doc_read.deref()) else {
                        tag = Tag::dynamic();
                        break;
                    };

                    let expr_type_resolution = expr_node
                        .type_resolution()
                        .forward()?
                        .read(context)
                        .forward()?;

                    tag.merge(expr_type_resolution.tag);
                }
            }
        }

        if let Tag::Unset = &tag {
            tag = Tag::dynamic();
        }

        Ok(Self { tag })
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct TypeResolution {
    pub(crate) tag: Tag,
    pub(crate) alt: bool,
    pub(crate) issues: AHashSet<ScriptIssue>,
}

impl Computable for TypeResolution {
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

        let mut resolver = TypeResolver {
            doc: doc_read.deref(),
            node_ref,
            context,
            resolution: Self::default(),
        };

        match script_node {
            ScriptNode::InlineComment { .. } => (),
            ScriptNode::MultilineComment { .. } => (),
            ScriptNode::Root { .. } => (),
            ScriptNode::Clause { .. } => (),
            ScriptNode::Use { .. } => (),
            ScriptNode::Package { .. } => (),
            ScriptNode::If { .. } => (),
            ScriptNode::Match { .. } => (),
            ScriptNode::MatchBody { .. } => (),
            ScriptNode::MatchArm { .. } => (),
            ScriptNode::Else { .. } => (),
            ScriptNode::Let { .. } => (),
            ScriptNode::Var { semantics, .. } => resolver.resolve_var(semantics)?,
            ScriptNode::For { .. } => (),
            ScriptNode::Loop { .. } => (),
            ScriptNode::Block { .. } => (),
            ScriptNode::Break { .. } => (),
            ScriptNode::Continue { .. } => (),
            ScriptNode::Return { .. } => (),
            ScriptNode::Fn { semantics, .. } => resolver.resolve_fn(semantics)?,
            ScriptNode::FnParams { .. } => (),
            ScriptNode::Struct { .. } => resolver.resolve_struct()?,
            ScriptNode::StructBody { .. } => (),
            ScriptNode::StructEntry { .. } => (),
            ScriptNode::StructEntryKey { .. } => (),
            ScriptNode::Array { semantics, .. } => resolver.resolve_array(semantics)?,
            ScriptNode::String { .. } => resolver.resolve_string()?,
            ScriptNode::Crate { .. } => resolver.resolve_crate()?,
            ScriptNode::This { semantics, .. } => resolver.resolve_this(semantics)?,
            ScriptNode::Ident { semantics, .. } => resolver.resolve_ident(semantics)?,
            ScriptNode::Number { semantics, .. } => resolver.resolve_number(semantics)?,
            ScriptNode::Max { .. } => resolver.resolve_max()?,
            ScriptNode::Bool { .. } => resolver.resolve_bool()?,
            ScriptNode::UnaryLeft { semantics, .. } => resolver.resolve_unary_left(semantics)?,
            ScriptNode::Binary { semantics, .. } => resolver.resolve_binary(semantics)?,
            ScriptNode::Op { .. } => (),
            ScriptNode::Query { .. } => resolver.resolve_query()?,
            ScriptNode::Call { semantics, .. } => resolver.resolve_call(semantics)?,
            ScriptNode::CallArgs { .. } => (),
            ScriptNode::Index { semantics, .. } => resolver.resolve_index(semantics)?,
            ScriptNode::IndexArg { .. } => (),
            ScriptNode::Field { .. } => (),
            ScriptNode::Expr { semantics, .. } => resolver.resolve_expr(semantics)?,
        }

        if let Tag::Unset = &resolver.resolution.tag {
            resolver.resolution.tag = Tag::dynamic();
        }

        Ok(resolver.resolution)
    }
}

struct TypeResolver<'doc, 'ctx, 'ctx_param, H: TaskHandle, S: SyncBuildHasher> {
    doc: &'doc ScriptDoc,
    node_ref: &'ctx NodeRef,
    context: &'ctx mut AttrContext<'ctx_param, ScriptNode, H, S>,
    resolution: TypeResolution,
}

impl<'doc, 'ctx, 'ctx_param, H: TaskHandle, S: SyncBuildHasher>
    TypeResolver<'doc, 'ctx, 'ctx_param, H, S>
{
    fn resolve_var(&mut self, semantics: &Semantics<VarSemantics>) -> AnalysisResult<()> {
        let var_semantics = semantics.get().forward()?;

        match var_semantics
            .var_syntax
            .read(self.context)
            .forward()?
            .deref()
        {
            LocalVarSyntax::Unknown => Ok(()),
            LocalVarSyntax::Let(_) => self.resolve_var_let(var_semantics),
            LocalVarSyntax::Param(fn_ref, param_index) => {
                self.resolve_var_param(fn_ref, *param_index)
            }
            LocalVarSyntax::For(_) => self.resolve_var_for(),
        }
    }

    fn resolve_var_let(&mut self, var_semantics: &VarSemantics) -> AnalysisResult<()> {
        let let_inits = var_semantics.let_inits.read(self.context).forward()?;

        for expr in &let_inits.as_ref().set {
            let Some(expr_node) = expr.deref(self.doc) else {
                continue;
            };

            let expr_resolution = expr_node
                .type_resolution()
                .forward()?
                .read(self.context)
                .forward()?;

            self.resolution.tag.merge(expr_resolution.tag);
        }

        Ok(())
    }

    fn resolve_var_param(&mut self, fn_ref: &NodeRef, param_index: usize) -> AnalysisResult<()> {
        let Some(ScriptNode::Fn { semantics, .. }) = fn_ref.deref(self.doc) else {
            return Ok(());
        };

        let fn_semantics = semantics.get().forward()?;

        let arg_syntax = fn_semantics.arg_syntax.read(self.context).forward()?;

        let Some(left_node) = arg_syntax.call_left_ref.deref(self.doc) else {
            return Ok(());
        };

        let left_resolution = left_node
            .type_resolution()
            .forward()?
            .read(self.context)
            .forward()?;

        let Tag::Invocation(meta) = left_resolution.tag else {
            return Ok(());
        };

        let Some(inputs) = &meta.inputs else {
            return Ok(());
        };

        let Some(input) = inputs.get(arg_syntax.arg_index) else {
            return Ok(());
        };

        let Some(meta) = input.hint.invocation() else {
            return Ok(());
        };

        let Some(inputs) = &meta.inputs else {
            return Ok(());
        };

        let Some(input) = inputs.get(param_index) else {
            return Ok(());
        };

        self.resolution.tag = Tag::from(input.hint);

        Ok(())
    }

    fn resolve_var_for(&mut self) -> AnalysisResult<()> {
        self.resolution.tag = Tag::Type(<usize>::type_meta());

        Ok(())
    }

    fn resolve_fn(&mut self, semantics: &Semantics<FnSemantics>) -> AnalysisResult<()> {
        let fn_semantics = semantics.get().forward()?;

        let sig = fn_semantics.locals.sig.read(self.context).forward()?;

        let arity = sig.as_ref().params.len();

        self.resolution.tag = Tag::Fn((*self.node_ref, arity));

        Ok(())
    }

    fn resolve_struct(&mut self) -> AnalysisResult<()> {
        self.resolution.tag = Tag::Struct(*self.node_ref);

        Ok(())
    }

    fn resolve_array(&mut self, semantics: &Semantics<ArraySemantics>) -> AnalysisResult<()> {
        enum TailMode {
            Unknown,
            CheckDisplay,
            CheckFamily(&'static TypeFamily),
            MergeEach(&'static TypeFamily),
        }

        let array_semantics = semantics.get().forward()?;

        let array_syntax = array_semantics.array_syntax.read(self.context).forward()?;

        let items = &array_syntax.as_ref().items;

        if items.is_empty() {
            self.resolution.tag = Tag::nil();
            return Ok(());
        };

        let mut items_iter = items.iter();

        let mut mode = TailMode::Unknown;

        let str_type = <str>::type_meta();
        let str_family = str_type.family();

        while let Some(expr) = items_iter.next() {
            let Some(expr_node) = expr.deref(self.doc) else {
                continue;
            };

            let expr_resolution = expr_node
                .type_resolution()
                .forward()?
                .read(self.context)
                .forward()?;

            let tag = &expr_resolution.tag;

            let Some(ty) = tag.type_meta() else {
                match tag.type_family() == str_family {
                    true => {
                        mode = TailMode::CheckDisplay;
                        self.resolution.tag = Tag::Type(str_type);
                    }

                    false => {
                        mode = TailMode::MergeEach(tag.type_family());
                        self.resolution.tag = *tag;
                    }
                }

                break;
            };

            if ty.is_nil() {
                continue;
            }

            if ty.is_dynamic() {
                return Ok(());
            }

            if ty.family() == str_family {
                mode = TailMode::CheckDisplay;
                self.resolution.tag = Tag::Type(str_type);
                break;
            }

            let Some(result) = ty.prototype().hint_concat_result() else {
                let _ = self
                    .resolution
                    .issues
                    .insert(ScriptIssue::UndefinedOperator {
                        op_ref: *expr,
                        op: OperatorKind::Concat,
                        receiver: ty,
                    });

                return Ok(());
            };

            mode = TailMode::CheckFamily(ty.family());
            self.resolution.tag = Tag::Type(result);
            break;
        }

        match mode {
            TailMode::Unknown => (),

            TailMode::CheckDisplay => {
                while let Some(expr) = items_iter.next() {
                    let Some(expr_node) = expr.deref(self.doc) else {
                        continue;
                    };

                    let expr_resolution = expr_node
                        .type_resolution()
                        .forward()?
                        .read(self.context)
                        .forward()?;

                    let tag = &expr_resolution.tag;

                    let Some(ty) = tag.type_meta() else {
                        continue;
                    };

                    if ty.is_dynamic() || ty.is_nil() {
                        continue;
                    }

                    if !ty.prototype().implements_display() {
                        let _ = self
                            .resolution
                            .issues
                            .insert(ScriptIssue::UndefinedOperator {
                                op_ref: *expr,
                                op: OperatorKind::Display,
                                receiver: ty,
                            });
                    }
                }
            }

            TailMode::CheckFamily(expected) => {
                while let Some(expr) = items_iter.next() {
                    let Some(expr_node) = expr.deref(self.doc) else {
                        continue;
                    };

                    let expr_resolution = expr_node
                        .type_resolution()
                        .forward()?
                        .read(self.context)
                        .forward()?;

                    let tag = &expr_resolution.tag;

                    if let Some(ty) = tag.type_meta() {
                        if ty.is_nil() || ty.is_dynamic() {
                            continue;
                        }
                    }

                    let provided = tag.type_family();

                    if provided != expected {
                        let _ = self.resolution.issues.insert(ScriptIssue::TypeMismatch {
                            expr_ref: *expr,
                            expected,
                            provided,
                        });
                    }
                }
            }

            TailMode::MergeEach(expected) => {
                while let Some(expr) = items_iter.next() {
                    let Some(expr_node) = expr.deref(self.doc) else {
                        continue;
                    };

                    let expr_resolution = expr_node
                        .type_resolution()
                        .forward()?
                        .read(self.context)
                        .forward()?;

                    let tag = &expr_resolution.tag;

                    if let Some(ty) = tag.type_meta() {
                        if ty.is_nil() || ty.is_dynamic() {
                            continue;
                        }
                    }

                    let provided = tag.type_family();

                    if provided != expected {
                        let _ = self.resolution.issues.insert(ScriptIssue::TypeMismatch {
                            expr_ref: *expr,
                            expected,
                            provided,
                        });
                    }

                    self.resolution.tag.merge(*tag);
                }
            }
        }

        Ok(())
    }

    fn resolve_string(&mut self) -> AnalysisResult<()> {
        self.resolution.tag = Tag::Type(<str>::type_meta());

        Ok(())
    }

    fn resolve_crate(&mut self) -> AnalysisResult<()> {
        let Some(package) = PackageMeta::by_id(self.doc.id()) else {
            system_panic!("Missing package.");
        };

        self.resolution.tag = Tag::Type(package.ty());

        Ok(())
    }

    fn resolve_this(&mut self, semantics: &Semantics<ThisSemantics>) -> AnalysisResult<()> {
        let Some(scope_node) = semantics
            .scope_attr()
            .forward()?
            .read(self.context)
            .forward()?
            .scope_ref
            .deref(self.doc)
        else {
            return Ok(());
        };

        match scope_node {
            ScriptNode::Root { .. } => self.resolve_this_root(),
            ScriptNode::Fn { semantics, .. } => self.resolve_this_fn(semantics),
            _ => Ok(()),
        }
    }

    fn resolve_this_root(&mut self) -> AnalysisResult<()> {
        self.resolution.tag = Tag::dynamic();

        Ok(())
    }

    fn resolve_this_fn(&mut self, semantics: &Semantics<FnSemantics>) -> AnalysisResult<()> {
        let Some(fn_context) = semantics
            .get()
            .forward()?
            .fn_context_syntax
            .read(self.context)
            .forward()?
            .struct_ref
            .deref(self.doc)
        else {
            return Ok(());
        };

        let context_type = fn_context
            .type_resolution()
            .forward()?
            .read(self.context)
            .forward()?;

        let Tag::Struct(_) = &context_type.tag else {
            return Ok(());
        };

        self.resolution.tag = context_type.tag;

        Ok(())
    }

    fn resolve_ident(&mut self, semantics: &Semantics<IdentSemantics>) -> AnalysisResult<()> {
        let ident_semantics = semantics.get().forward()?;

        let ident_cross_resolution = ident_semantics
            .cross_resolution
            .read(self.context)
            .forward()?;

        match ident_cross_resolution.deref() {
            IdentCrossResolution::Unresolved => Ok(()),
            IdentCrossResolution::BestMatch { .. } => Ok(()),
            IdentCrossResolution::Read { name } => {
                self.resolve_ident_read(ident_semantics, name.as_ref())
            }
            IdentCrossResolution::Write { decl } => self.resolve_ident_write(decl),
        }
    }

    fn resolve_ident_read(
        &mut self,
        ident_semantics: &IdentSemantics,
        name: &Name,
    ) -> AnalysisResult<()> {
        let Some(decl_node) = name.decl.deref(self.doc) else {
            return Ok(());
        };

        match decl_node {
            ScriptNode::Root { .. } => {
                let Some(package) = PackageMeta::by_id(self.doc.id()) else {
                    system_panic!("Missing package.");
                };

                let atom = ident_semantics.atom_syntax.read(self.context).forward()?;

                let Some(component) = package.ty().prototype().hint_component(atom.0.as_str())
                else {
                    return Ok(());
                };

                self.resolution.tag = Tag::from(component.ty);
            }

            ScriptNode::Use { .. } => {
                let Some(package_ref) = name.defs.iter().next() else {
                    return Ok(());
                };

                let Some(ScriptNode::Package { semantics, .. }) = package_ref.deref(self.doc)
                else {
                    return Ok(());
                };

                let package_semantics = semantics.get().forward()?;

                let package_resolution = package_semantics
                    .package_resolution
                    .read(self.context)
                    .forward()?;

                let Some(package) = package_resolution.package else {
                    return Ok(());
                };

                let atom = ident_semantics.atom_syntax.read(self.context).forward()?;

                let Some(component) = package.ty().prototype().hint_component(atom.0.as_str())
                else {
                    return Ok(());
                };

                self.resolution.tag = Tag::from(component.ty);
            }

            _ => {
                for def_ref in &name.defs {
                    let Some(def_node) = def_ref.deref(self.doc) else {
                        continue;
                    };

                    let def_type_resolution = def_node
                        .type_resolution()
                        .forward()?
                        .read(self.context)
                        .forward()?;

                    self.resolution.tag.merge(def_type_resolution.tag);
                }
            }
        }

        Ok(())
    }

    fn resolve_ident_write(&mut self, decl: &NodeRef) -> AnalysisResult<()> {
        let Some(ScriptNode::Var { semantics, .. }) = decl.deref(self.doc) else {
            return Ok(());
        };

        self.resolution.tag = semantics
            .get()
            .forward()?
            .type_resolution
            .read(self.context)
            .forward()?
            .tag;

        Ok(())
    }

    fn resolve_number(&mut self, semantics: &Semantics<NumberSemantics>) -> AnalysisResult<()> {
        let number_semantics = semantics.get().forward()?;

        let number_value = number_semantics.number_value.read(self.context).forward()?;

        match number_value.deref() {
            LocalNumberValue::Usize(_) => self.resolution.tag = Tag::Type(<usize>::type_meta()),
            LocalNumberValue::Isize(_) => self.resolution.tag = Tag::Type(<isize>::type_meta()),
            LocalNumberValue::Float(_) => self.resolution.tag = Tag::Type(<FloatRepr>::type_meta()),
        }

        Ok(())
    }

    fn resolve_max(&mut self) -> AnalysisResult<()> {
        self.resolution.tag = Tag::Type(<usize>::type_meta());

        Ok(())
    }

    fn resolve_bool(&mut self) -> AnalysisResult<()> {
        self.resolution.tag = Tag::Type(<bool>::type_meta());

        Ok(())
    }

    fn resolve_unary_left(
        &mut self,
        semantics: &Semantics<UnaryLeftSemantics>,
    ) -> AnalysisResult<()> {
        let unary_left_semantics = semantics.get().forward()?;

        let infix_syntax = unary_left_semantics
            .infix_syntax
            .read(self.context)
            .forward()?;

        let Some(right_node) = infix_syntax.right.deref(self.doc) else {
            return Ok(());
        };

        let right_type_resolution = right_node
            .type_resolution()
            .forward()?
            .read(self.context)
            .forward()?;

        if right_type_resolution.tag.is_dynamic() {
            return Ok(());
        }

        let Some(receiver) = right_type_resolution.tag.type_meta() else {
            return Ok(());
        };

        let receiver_prototype = receiver.prototype();

        match infix_syntax.op {
            ScriptToken::Mul => {
                if !receiver_prototype.implements_clone() {
                    let _ = self
                        .resolution
                        .issues
                        .insert(ScriptIssue::UndefinedOperator {
                            op_ref: infix_syntax.op_ref,
                            op: OperatorKind::Clone,
                            receiver,
                        });
                }

                self.resolution.tag = right_type_resolution.tag;
            }

            ScriptToken::Minus => {
                if !receiver_prototype.implements_neg() {
                    let _ = self
                        .resolution
                        .issues
                        .insert(ScriptIssue::UndefinedOperator {
                            op_ref: infix_syntax.op_ref,
                            op: OperatorKind::Neg,
                            receiver,
                        });
                }

                if let Some(result) = receiver_prototype.hint_neg_result() {
                    self.resolution.tag = Tag::Type(result);
                }
            }

            ScriptToken::Not => {
                if !receiver_prototype.implements_not() {
                    let _ = self
                        .resolution
                        .issues
                        .insert(ScriptIssue::UndefinedOperator {
                            op_ref: infix_syntax.op_ref,
                            op: OperatorKind::Not,
                            receiver,
                        });
                }

                if let Some(result) = receiver_prototype.hint_not_result() {
                    self.resolution.tag = Tag::Type(result);
                }
            }

            _ => (),
        }

        Ok(())
    }

    fn resolve_binary(&mut self, semantics: &Semantics<BinarySemantics>) -> AnalysisResult<()> {
        let binary_semantics = semantics.get().forward()?;

        let infix_syntax = binary_semantics.infix_syntax.read(self.context).forward()?;

        match infix_syntax.op {
            ScriptToken::Dot => self.resolve_binary_dot(infix_syntax.deref()),
            ScriptToken::Dot2 => self.resolve_binary_dot2(infix_syntax.deref()),
            ScriptToken::Assign => self.resolve_binary_assign(infix_syntax.deref()),
            _ => self.resolve_binary_op(infix_syntax.deref()),
        }
    }

    fn resolve_binary_assign(&mut self, infix_syntax: &LocalInfixSyntax) -> AnalysisResult<()> {
        let Some(ScriptNode::Ident { semantics, .. }) = infix_syntax.left.deref(self.doc) else {
            return self.resolve_binary_op(infix_syntax);
        };

        let ident_semantics = semantics.get().forward()?;

        let IdentLocalResolution::Write { .. } = ident_semantics
            .local_resolution
            .read(self.context)
            .forward()?
            .deref()
        else {
            return self.resolve_binary_op(infix_syntax);
        };

        self.resolution.tag = Tag::nil();

        Ok(())
    }

    fn resolve_binary_dot(&mut self, infix_syntax: &LocalInfixSyntax) -> AnalysisResult<()> {
        let Some(ScriptNode::Field { semantics, .. }) = infix_syntax.right.deref(self.doc) else {
            return Ok(());
        };

        let field_semantics = semantics.get().forward()?;

        let field_atom = field_semantics.atom_syntax.read(self.context).forward()?;

        if field_atom.0 == "len" {
            self.resolution.tag = Tag::Type(<usize>::type_meta());
            return Ok(());
        }

        let Some(left_node) = infix_syntax.left.deref(self.doc) else {
            return Ok(());
        };

        let left_type_resolution = left_node
            .type_resolution()
            .forward()?
            .read(self.context)
            .forward()?;

        if let Tag::Struct(struct_ref) = left_type_resolution.tag {
            let Some(ScriptNode::Struct { semantics, .. }) = struct_ref.deref(self.doc) else {
                return Ok(());
            };

            let struct_semantics = semantics.get().forward()?;

            let struct_entries_map_syntax = struct_semantics
                .struct_entries_map_syntax
                .read(self.context)
                .forward()?;

            let Some((_, field_value_ref)) =
                struct_entries_map_syntax.as_ref().map.get(&field_atom.0)
            else {
                return Ok(());
            };

            let Some(field_value_node) = field_value_ref.deref(self.doc) else {
                return Ok(());
            };

            self.resolution.tag = field_value_node
                .type_resolution()
                .forward()?
                .read(self.context)
                .forward()?
                .tag;

            return Ok(());
        }

        let Some(receiver) = left_type_resolution.tag.type_meta() else {
            return Ok(());
        };

        if receiver.is_dynamic() {
            return Ok(());
        }

        let receiver_prototype = receiver.prototype();

        if let Some(component) = receiver_prototype.hint_component(&field_atom.0) {
            self.resolution.tag = Tag::from(component.ty);
            return Ok(());
        }

        if let Some(ty) = receiver_prototype.hint_field() {
            self.resolution.tag = Tag::Type(ty);
            return Ok(());
        }

        let mut best_match = (Closeness::zero(), "");

        for component in receiver_prototype.hint_all_components() {
            let estimation = component.name.string.estimate(&field_atom.0);

            if estimation <= best_match.0 {
                continue;
            }

            best_match = (estimation, component.name.string);
        }

        let _ = self
            .resolution
            .issues
            .insert(ScriptIssue::UnknownComponent {
                field_ref: infix_syntax.right,
                receiver,
                quickfix: CompactString::from(best_match.1),
            });

        Ok(())
    }

    fn resolve_binary_dot2(&mut self, infix_syntax: &LocalInfixSyntax) -> AnalysisResult<()> {
        self.resolution.tag = Tag::Type(<Range<usize>>::type_meta());

        if let Some(left_node) = infix_syntax.left.deref(self.doc) {
            let left_type_resolution = left_node
                .type_resolution()
                .forward()?
                .read(self.context)
                .forward()?;

            let left_family = left_type_resolution.tag.type_family();

            if !left_family.is_dynamic() && !left_family.is_number() {
                let _ = self.resolution.issues.insert(ScriptIssue::TypeMismatch {
                    expr_ref: infix_syntax.left,
                    expected: <usize>::type_meta().family(),
                    provided: left_family,
                });
            }
        }

        if let Some(right_node) = infix_syntax.right.deref(self.doc) {
            let right_type_resolution = right_node
                .type_resolution()
                .forward()?
                .read(self.context)
                .forward()?;

            let right_family = right_type_resolution.tag.type_family();

            if !right_family.is_dynamic() && !right_family.is_number() {
                let _ = self.resolution.issues.insert(ScriptIssue::TypeMismatch {
                    expr_ref: infix_syntax.right,
                    expected: <usize>::type_meta().family(),
                    provided: right_family,
                });
            }
        }

        Ok(())
    }

    fn resolve_binary_op(&mut self, infix_syntax: &LocalInfixSyntax) -> AnalysisResult<()> {
        let Some(left_node) = infix_syntax.left.deref(self.doc) else {
            return Ok(());
        };

        let left_type_resolution = left_node
            .type_resolution()
            .forward()?
            .read(self.context)
            .forward()?;

        let Some(lhs_provided) = left_type_resolution.tag.type_meta() else {
            return Ok(());
        };

        if lhs_provided.is_dynamic() {
            return Ok(());
        }

        let receiver = lhs_provided.prototype();

        let Some(op_description) = infix_syntax.op.describe_binary() else {
            return Ok(());
        };

        let mut rhs_provided = TypeMeta::dynamic();

        if let Some(right_node) = infix_syntax.right.deref(self.doc) {
            let right_type_resolution = right_node.type_resolution().forward()?;

            if let Some(ty) = right_type_resolution
                .read(self.context)
                .forward()?
                .tag
                .type_meta()
            {
                rhs_provided = ty;
            }
        }

        if op_description.primary.applicable_to(receiver) {
            self.resolution.tag = op_description.primary.hint_result(receiver, lhs_provided);

            if rhs_provided.is_dynamic() {
                return Ok(());
            }

            let Some(expected) = op_description.primary.hint_rhs(receiver, lhs_provided) else {
                return Ok(());
            };

            if expected.is_dynamic() {
                return Ok(());
            }

            let expected_family = expected.family();
            let provided_family = rhs_provided.family();

            if expected_family != provided_family {
                let _ = self.resolution.issues.insert(ScriptIssue::TypeMismatch {
                    expr_ref: infix_syntax.right,
                    expected: expected_family,
                    provided: provided_family,
                });
            }

            return Ok(());
        }

        let Some(secondary) = op_description.secondary else {
            if op_description.assignment {
                self.resolution.tag = Tag::nil();
            }

            let _ = self
                .resolution
                .issues
                .insert(ScriptIssue::UndefinedOperator {
                    op_ref: infix_syntax.op_ref,
                    op: op_description.primary,
                    receiver: lhs_provided,
                });

            return Ok(());
        };

        if !secondary.applicable_to(receiver) {
            if op_description.assignment {
                self.resolution.tag = Tag::nil();
            }

            let _ = self
                .resolution
                .issues
                .insert(ScriptIssue::UndefinedOperator {
                    op_ref: infix_syntax.op_ref,
                    op: op_description.primary,
                    receiver: lhs_provided,
                });

            return Ok(());
        }

        match op_description.assignment {
            true => {
                if !receiver.implements_assign() {
                    let _ = self
                        .resolution
                        .issues
                        .insert(ScriptIssue::UndefinedOperator {
                            op_ref: infix_syntax.op_ref,
                            op: OperatorKind::Assign,
                            receiver: lhs_provided,
                        });
                }

                self.resolution.alt = true;
                self.resolution.tag = Tag::nil()
            }

            false => self.resolution.tag = secondary.hint_result(receiver, lhs_provided),
        }

        if rhs_provided.is_dynamic() {
            return Ok(());
        }

        let Some(expected) = secondary.hint_rhs(receiver, lhs_provided) else {
            return Ok(());
        };

        if expected.is_dynamic() {
            return Ok(());
        }

        let expected_family = expected.family();

        let provided_family = rhs_provided.family();

        if expected_family != provided_family {
            let _ = self.resolution.issues.insert(ScriptIssue::TypeMismatch {
                expr_ref: infix_syntax.right,
                expected: expected_family,
                provided: provided_family,
            });
        }

        Ok(())
    }

    fn resolve_query(&mut self) -> AnalysisResult<()> {
        self.resolution.tag = Tag::Type(<bool>::type_meta());

        Ok(())
    }

    fn resolve_call(&mut self, semantics: &Semantics<CallSemantics>) -> AnalysisResult<()> {
        let call_semantics = semantics.get().forward()?;

        let call_syntax = call_semantics.call_syntax.read(self.context).forward()?;

        let Some(left_node) = call_syntax.as_ref().left.deref(self.doc) else {
            return Ok(());
        };

        let left_type_resolution = left_node
            .type_resolution()
            .forward()?
            .read(self.context)
            .forward()?;

        if left_type_resolution.tag.is_dynamic() {
            return Ok(());
        }

        let Some(meta) = left_type_resolution.tag.invocation_meta() else {
            if let Some(receiver) = left_type_resolution.tag.type_meta() {
                let _ = self
                    .resolution
                    .issues
                    .insert(ScriptIssue::UndefinedOperator {
                        op_ref: call_syntax.as_ref().call_args_ref,
                        op: OperatorKind::Invocation,
                        receiver,
                    });
            }

            return Ok(());
        };

        self.resolution.tag = Tag::from(meta.output);

        let Some(inputs) = meta.inputs.as_ref() else {
            return Ok(());
        };

        let expected_args = inputs.len();
        let provided_args = call_syntax.as_ref().args.len();

        if expected_args != provided_args {
            let _ = self
                .resolution
                .issues
                .insert(ScriptIssue::CallArityMismatch {
                    args_ref: call_syntax.as_ref().call_args_ref,
                    expected: expected_args,
                    provided: provided_args,
                });
        }

        let zip = inputs.iter().zip(call_syntax.as_ref().args.iter());

        for (param, arg_ref) in zip {
            if param.hint.is_dynamic() {
                continue;
            }

            let Some(arg_node) = arg_ref.deref(self.doc) else {
                continue;
            };

            let arg_type_resolution = arg_node
                .type_resolution()
                .forward()?
                .read(self.context)
                .forward()?;

            if arg_type_resolution.tag.is_dynamic() {
                continue;
            }

            let expected_family = param.hint.type_family();
            let provided_family = arg_type_resolution.tag.type_family();

            if expected_family != provided_family {
                let _ = self.resolution.issues.insert(ScriptIssue::TypeMismatch {
                    expr_ref: *arg_ref,
                    expected: expected_family,
                    provided: provided_family,
                });

                continue;
            }

            if let Tag::Fn((fn_ref, arg_arity)) = &arg_type_resolution.tag {
                let Some(param_invocation) = param.hint.invocation() else {
                    continue;
                };

                let Some(param_arity) = param_invocation.arity() else {
                    continue;
                };

                if param_arity != *arg_arity {
                    let _ = self.resolution.issues.insert(ScriptIssue::FnArityMismatch {
                        arg_ref: *arg_ref,
                        expected: param_arity,
                        provided: *arg_arity,
                    });
                }

                let param_output_family = param_invocation.output.type_family();

                if param_output_family.is_dynamic() {
                    continue;
                }

                let Some(ScriptNode::Fn { semantics, .. }) = fn_ref.deref(self.doc) else {
                    continue;
                };

                let fn_semantics = semantics.get().forward()?;

                // todo May lead to recursion?
                let arg_output_family = fn_semantics
                    .result_resolution
                    .read(self.context)
                    .forward()?
                    .tag
                    .type_family();

                if arg_output_family.is_dynamic() {
                    continue;
                }

                if param_output_family != arg_output_family {
                    let _ = self.resolution.issues.insert(ScriptIssue::ResultMismatch {
                        arg_ref: *arg_ref,
                        expected: param_output_family,
                        provided: arg_output_family,
                    });
                }

                continue;
            }

            let Some(param_invocation) = param.hint.invocation() else {
                continue;
            };

            let Some(arg_invocation) = arg_type_resolution.tag.invocation_meta() else {
                continue;
            };

            if let (Some(param_arity), Some(arg_arity)) =
                (param_invocation.arity(), arg_invocation.arity())
            {
                if param_arity != arg_arity {
                    let _ = self.resolution.issues.insert(ScriptIssue::FnArityMismatch {
                        arg_ref: *arg_ref,
                        expected: param_arity,
                        provided: arg_arity,
                    });
                }
            }

            let param_output = &param_invocation.output;

            if param_output.is_dynamic() {
                continue;
            }

            let arg_output = &arg_invocation.output;

            if arg_output.is_dynamic() {
                continue;
            }

            let param_family = param_output.type_family();
            let arg_family = arg_output.type_family();

            if param_family != arg_family {
                let _ = self.resolution.issues.insert(ScriptIssue::ResultMismatch {
                    arg_ref: *arg_ref,
                    expected: param_family,
                    provided: arg_family,
                });
            }
        }

        Ok(())
    }

    fn resolve_index(&mut self, semantics: &Semantics<IndexSemantics>) -> AnalysisResult<()> {
        let index_semantics = semantics.get().forward()?;

        let index_syntax = index_semantics.index_syntax.read(self.context).forward()?;

        if let Some(left_node) = index_syntax.left.deref(self.doc) {
            let left_type_resolution = left_node
                .type_resolution()
                .forward()?
                .read(self.context)
                .forward()?;

            if let Tag::Type(receiver) = left_type_resolution.tag {
                if receiver.is_nil() {
                    let _ = self.resolution.issues.insert(ScriptIssue::NilIndex {
                        op_ref: index_syntax.index_arg_ref,
                    });
                }
            }

            self.resolution.tag = left_type_resolution.tag;
        }

        if let Some(index_node) = index_syntax.index.deref(self.doc) {
            let index_type_resolution = index_node
                .type_resolution()
                .forward()?
                .read(self.context)
                .forward()?;

            if index_type_resolution.tag.is_dynamic() {
                return Ok(());
            }

            let index_family = index_type_resolution.tag.type_family();

            if index_family.is_number() {
                return Ok(());
            }

            if index_family == <Range<usize>>::type_meta().family() {
                return Ok(());
            }

            let _ = self
                .resolution
                .issues
                .insert(ScriptIssue::IndexTypeMismatch {
                    arg_ref: index_syntax.index,
                    provided: index_family,
                });
        }

        Ok(())
    }

    fn resolve_expr(&mut self, semantics: &Semantics<ExprSemantics>) -> AnalysisResult<()> {
        let expr_semantics = semantics.get().forward()?;

        let expr_syntax = expr_semantics.expr_syntax.read(self.context).forward()?;

        let Some(inner_node) = expr_syntax.node_ref().deref(self.doc) else {
            return Ok(());
        };

        let inner_type_resolution = inner_node
            .type_resolution()
            .forward()?
            .read(self.context)
            .forward()?;

        self.resolution.tag = inner_type_resolution.tag;

        Ok(())
    }
}

impl ScriptToken {
    #[inline(always)]
    fn describe_binary(self) -> Option<BinaryOpDescription> {
        match self {
            Self::Assign => Some(BinaryOpDescription {
                primary: OperatorKind::Assign,
                secondary: None,
                assignment: true,
            }),

            Self::Equal => Some(BinaryOpDescription {
                primary: OperatorKind::PartialEq,
                secondary: None,
                assignment: false,
            }),

            Self::Lesser | Self::LesserOrEqual | Self::Greater | Self::GreaterOrEqual => {
                Some(BinaryOpDescription {
                    primary: OperatorKind::Ord,
                    secondary: Some(OperatorKind::PartialOrd),
                    assignment: false,
                })
            }

            Self::Plus => Some(BinaryOpDescription {
                primary: OperatorKind::Add,
                secondary: None,
                assignment: false,
            }),

            Self::PlusAssign => Some(BinaryOpDescription {
                primary: OperatorKind::AddAssign,
                secondary: Some(OperatorKind::Add),
                assignment: true,
            }),

            Self::Minus => Some(BinaryOpDescription {
                primary: OperatorKind::Sub,
                secondary: None,
                assignment: false,
            }),

            Self::MinusAssign => Some(BinaryOpDescription {
                primary: OperatorKind::SubAssign,
                secondary: Some(OperatorKind::Sub),
                assignment: true,
            }),

            Self::Mul => Some(BinaryOpDescription {
                primary: OperatorKind::Mul,
                secondary: None,
                assignment: false,
            }),

            Self::MulAssign => Some(BinaryOpDescription {
                primary: OperatorKind::MulAssign,
                secondary: Some(OperatorKind::Mul),
                assignment: true,
            }),

            Self::Div => Some(BinaryOpDescription {
                primary: OperatorKind::Div,
                secondary: None,
                assignment: false,
            }),

            Self::DivAssign => Some(BinaryOpDescription {
                primary: OperatorKind::DivAssign,
                secondary: Some(OperatorKind::Div),
                assignment: true,
            }),

            Self::Or => Some(BinaryOpDescription {
                primary: OperatorKind::Or,
                secondary: None,
                assignment: false,
            }),

            Self::And => Some(BinaryOpDescription {
                primary: OperatorKind::And,
                secondary: None,
                assignment: false,
            }),

            Self::BitAnd => Some(BinaryOpDescription {
                primary: OperatorKind::BitAnd,
                secondary: None,
                assignment: false,
            }),

            Self::BitAndAssign => Some(BinaryOpDescription {
                primary: OperatorKind::BitAndAssign,
                secondary: Some(OperatorKind::BitAnd),
                assignment: true,
            }),

            Self::BitOr => Some(BinaryOpDescription {
                primary: OperatorKind::BitOr,
                secondary: None,
                assignment: false,
            }),

            Self::BitOrAssign => Some(BinaryOpDescription {
                primary: OperatorKind::BitOrAssign,
                secondary: Some(OperatorKind::BitOr),
                assignment: true,
            }),

            Self::BitXor => Some(BinaryOpDescription {
                primary: OperatorKind::BitXor,
                secondary: None,
                assignment: false,
            }),

            Self::BitXorAssign => Some(BinaryOpDescription {
                primary: OperatorKind::BitXorAssign,
                secondary: Some(OperatorKind::BitXor),
                assignment: true,
            }),

            Self::Shl => Some(BinaryOpDescription {
                primary: OperatorKind::Shl,
                secondary: None,
                assignment: false,
            }),

            Self::ShlAssign => Some(BinaryOpDescription {
                primary: OperatorKind::ShlAssign,
                secondary: Some(OperatorKind::Shl),
                assignment: true,
            }),

            Self::Shr => Some(BinaryOpDescription {
                primary: OperatorKind::Shr,
                secondary: None,
                assignment: false,
            }),

            Self::ShrAssign => Some(BinaryOpDescription {
                primary: OperatorKind::ShrAssign,
                secondary: Some(OperatorKind::Shr),
                assignment: true,
            }),

            Self::Rem => Some(BinaryOpDescription {
                primary: OperatorKind::Rem,
                secondary: None,
                assignment: false,
            }),

            Self::RemAssign => Some(BinaryOpDescription {
                primary: OperatorKind::RemAssign,
                secondary: Some(OperatorKind::Rem),
                assignment: true,
            }),

            _ => return None,
        }
    }
}

struct BinaryOpDescription {
    primary: OperatorKind,
    secondary: Option<OperatorKind>,
    assignment: bool,
}

impl OperatorKind {
    fn applicable_to(self, receiver: &Prototype) -> bool {
        match self {
            Self::Assign => receiver.implements_assign(),
            Self::Concat => receiver.implements_concat(),
            Self::Field => receiver.implements_field(),
            Self::Clone => receiver.implements_clone(),
            Self::Debug => receiver.implements_debug(),
            Self::Display => receiver.implements_display(),
            Self::PartialEq => receiver.implements_partial_eq(),
            Self::Default => receiver.implements_default(),
            Self::PartialOrd => receiver.implements_partial_ord(),
            Self::Ord => receiver.implements_ord(),
            Self::Hash => receiver.implements_hash(),
            Self::Invocation => receiver.implements_invocation(),
            Self::Binding => receiver.implements_binding(),
            Self::Add => receiver.implements_add(),
            Self::AddAssign => receiver.implements_add_assign(),
            Self::Sub => receiver.implements_sub(),
            Self::SubAssign => receiver.implements_sub_assign(),
            Self::Mul => receiver.implements_mul(),
            Self::MulAssign => receiver.implements_mul_assign(),
            Self::Div => receiver.implements_div(),
            Self::DivAssign => receiver.implements_div_assign(),
            Self::And => receiver.implements_and(),
            Self::Or => receiver.implements_or(),
            Self::Not => receiver.implements_not(),
            Self::Neg => receiver.implements_neg(),
            Self::BitAnd => receiver.implements_bit_and(),
            Self::BitAndAssign => receiver.implements_bit_and_assign(),
            Self::BitOr => receiver.implements_bit_or(),
            Self::BitOrAssign => receiver.implements_bit_or_assign(),
            Self::BitXor => receiver.implements_bit_xor(),
            Self::BitXorAssign => receiver.implements_bit_xor_assign(),
            Self::Shl => receiver.implements_shl(),
            Self::ShlAssign => receiver.implements_shl_assign(),
            Self::Shr => receiver.implements_shr(),
            Self::ShrAssign => receiver.implements_shr_assign(),
            Self::Rem => receiver.implements_rem(),
            Self::RemAssign => receiver.implements_rem_assign(),
        }
    }

    fn hint_rhs(self, receiver: &Prototype, lhs: &'static TypeMeta) -> Option<&'static TypeMeta> {
        match self {
            Self::Assign => receiver.hint_assign_rhs(),
            Self::Concat => None,
            Self::Field => None,
            Self::Clone => None,
            Self::Debug => None,
            Self::Display => None,
            Self::PartialEq => receiver.hint_partial_eq_rhs(),
            Self::Default => None,
            Self::PartialOrd => receiver.hint_partial_ord_rhs(),
            Self::Ord => Some(lhs),
            Self::Hash => None,
            Self::Invocation => None,
            Self::Binding => None,
            Self::Add => receiver.hint_add_rhs(),
            Self::AddAssign => receiver.hint_add_assign_rhs(),
            Self::Sub => receiver.hint_sub_rhs(),
            Self::SubAssign => receiver.hint_sub_assign_rhs(),
            Self::Mul => receiver.hint_mul_rhs(),
            Self::MulAssign => receiver.hint_mul_assign_rhs(),
            Self::Div => receiver.hint_div_rhs(),
            Self::DivAssign => receiver.hint_div_assign_rhs(),
            Self::And => receiver.hint_and_rhs(),
            Self::Or => receiver.hint_or_rhs(),
            Self::Not => None,
            Self::Neg => None,
            Self::BitAnd => receiver.hint_bit_and_rhs(),
            Self::BitAndAssign => receiver.hint_bit_and_assign_rhs(),
            Self::BitOr => receiver.hint_bit_or_rhs(),
            Self::BitOrAssign => receiver.hint_bit_or_assign_rhs(),
            Self::BitXor => receiver.hint_bit_xor_rhs(),
            Self::BitXorAssign => receiver.hint_bit_xor_assign_rhs(),
            Self::Shl => receiver.hint_shr_rhs(),
            Self::ShlAssign => receiver.hint_shl_assign_rhs(),
            Self::Shr => receiver.hint_shr_rhs(),
            Self::ShrAssign => receiver.hint_shr_assign_rhs(),
            Self::Rem => receiver.hint_rem_rhs(),
            Self::RemAssign => receiver.hint_rem_assign_rhs(),
        }
    }

    fn hint_result(self, receiver: &Prototype, lhs: &'static TypeMeta) -> Tag {
        match self {
            Self::Assign => Tag::nil(),

            Self::Concat => receiver
                .hint_concat_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::Field => receiver
                .hint_field()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::Clone => Tag::Type(lhs),

            Self::Debug => Tag::Type(<str>::type_meta()),

            Self::Display => Tag::Type(<str>::type_meta()),

            Self::PartialEq => Tag::Type(<bool>::type_meta()),

            Self::Default => Tag::Type(lhs),

            Self::PartialOrd => Tag::Type(<bool>::type_meta()),

            Self::Ord => Tag::Type(<bool>::type_meta()),

            Self::Hash => Tag::Type(<u64>::type_meta()),

            Self::Invocation => receiver
                .hint_invocation()
                .map(Tag::Invocation)
                .unwrap_or(Tag::dynamic()),

            Self::Binding => Tag::dynamic(),

            Self::Add => receiver
                .hint_add_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::AddAssign => Tag::nil(),

            Self::Sub => receiver
                .hint_sub_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::SubAssign => Tag::nil(),

            Self::Mul => receiver
                .hint_mul_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::MulAssign => Tag::nil(),

            Self::Div => receiver
                .hint_div_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::DivAssign => Tag::nil(),

            Self::And => receiver
                .hint_and_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::Or => receiver
                .hint_or_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::Not => receiver
                .hint_not_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::Neg => receiver
                .hint_neg_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::BitAnd => receiver
                .hint_bit_and_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::BitAndAssign => Tag::nil(),

            Self::BitOr => receiver
                .hint_bit_or_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::BitOrAssign => Tag::nil(),

            Self::BitXor => receiver
                .hint_bit_xor_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::BitXorAssign => Tag::nil(),

            Self::Shl => receiver
                .hint_shl_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::ShlAssign => Tag::nil(),

            Self::Shr => receiver
                .hint_shr_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::ShrAssign => Tag::nil(),

            Self::Rem => receiver
                .hint_rem_result()
                .map(Tag::Type)
                .unwrap_or(Tag::dynamic()),

            Self::RemAssign => Tag::nil(),
        }
    }
}
