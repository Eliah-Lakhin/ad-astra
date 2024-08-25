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

use std::borrow::Cow;

use ahash::AHashSet;
use proc_macro2::Ident;
use syn::{
    punctuated::Punctuated,
    spanned::Spanned,
    AngleBracketedGenericArguments,
    Block,
    BoundLifetimes,
    Error,
    Expr,
    GenericArgument,
    GenericParam,
    Item,
    Lifetime,
    ParenthesizedGenericArguments,
    Pat,
    Path,
    PathArguments,
    Result,
    ReturnType,
    Stmt,
    Type,
    TypeParamBound,
    __private::bool,
};

use crate::utils::Exportable;

pub(super) trait Resolver: Sized {
    fn get_const(&self, ident: &Ident) -> Option<Expr>;

    fn get_type(&self, ident: &Ident) -> Option<Type>;

    #[inline(always)]
    fn resolve_type(&self, ty: &mut Type, constraints: ResolveConstraints) -> Result<()> {
        resolve_type(
            self,
            ty,
            &Context {
                constraints,
                shadow: Default::default(),
            },
        )
    }

    #[inline(always)]
    fn resolve_expr(&self, expr: &mut Expr, constraints: ResolveConstraints) -> Result<()> {
        resolve_expr(
            self,
            expr,
            &Context {
                constraints,
                shadow: Default::default(),
            },
        )
    }
}

#[derive(Default, Clone, Copy)]
pub(super) struct ResolveConstraints {
    pub references: bool,
    pub impls: bool,
}

impl ResolveConstraints {
    #[inline(always)]
    fn unlimited() -> Self {
        Self {
            references: true,
            impls: true,
        }
    }
}

fn resolve_type(this: &impl Resolver, ty: &mut Type, context: &Context) -> Result<()> {
    match ty {
        Type::Array(ty) => {
            resolve_type(this, &mut ty.elem, context)?;
            resolve_expr(this, &mut ty.len, context)?;
        }

        Type::BareFn(ty) => match &mut ty.lifetimes {
            None => {
                for input in &mut ty.inputs {
                    resolve_type(this, &mut input.ty, context)?;
                }

                if let ReturnType::Type(_, output) = &mut ty.output {
                    resolve_type(this, output.as_mut(), context)?;
                }
            }

            Some(lifetimes) => {
                let local_context = resolve_bound_lifetimes(this, lifetimes, context)?;

                for input in &mut ty.inputs {
                    resolve_type(this, &mut input.ty, &local_context)?;
                }

                if let ReturnType::Type(_, output) = &mut ty.output {
                    resolve_type(this, output.as_mut(), &local_context)?;
                }
            }
        },

        Type::Group(ty) => resolve_type(this, ty.elem.as_mut(), context)?,

        Type::ImplTrait(ty) => {
            if !context.constraints.impls {
                return Err(Error::new(
                    ty.impl_token.span,
                    "Impl trait type in this position is not supported by the \
                    introspection system.\n\nConsider using \
                    #[export(type <type_1>, <type_2>, ...)] to specify type specializations.",
                ));
            }

            resolve_param_bounds(this, &mut ty.bounds, context)?
        }

        Type::Infer(..) => (),

        Type::Never(..) => (),

        Type::Paren(ty) => resolve_type(this, ty.elem.as_mut(), context)?,

        Type::Path(type_path) => {
            if type_path.qself.is_none()
                && type_path.path.leading_colon.is_none()
                && type_path.path.segments.len() == 1
            {
                let segment = &type_path.path.segments[0];

                if let PathArguments::None = &segment.arguments {
                    if !context.contains_ident(&segment.ident) {
                        if let Some(inline) = this.get_type(&segment.ident) {
                            *ty = inline;

                            return Ok(());
                        }
                    }
                }
            }

            if let Some(q_self) = &mut type_path.qself {
                resolve_type(this, &mut q_self.ty, context)?;
            }

            resolve_path(this, &mut type_path.path, context)?;
        }

        Type::Ptr(ty) => resolve_type(this, ty.elem.as_mut(), context)?,

        Type::Reference(ty) => {
            if !context.constraints.references {
                return Err(Error::new(
                    ty.and_token.span,
                    "Reference type in this position is not supported by the introspection system.",
                ));
            }

            if let Some(lifetime) = &ty.lifetime {
                if !context.contains_lifetime(&lifetime.ident) {
                    deny_lifetime(lifetime)?
                }
            }

            resolve_type(this, ty.elem.as_mut(), context)?;
        }

        Type::Slice(ty) => resolve_type(this, ty.elem.as_mut(), context)?,

        Type::TraitObject(ty) => resolve_param_bounds(this, &mut ty.bounds, context)?,

        Type::Tuple(ty) => {
            for elem in &mut ty.elems {
                resolve_type(this, elem, context)?
            }
        }

        _ => {
            return Err(Error::new(
                ty.span(),
                "This syntax is not supported by the introspection system.",
            ))
        }
    }

    Ok(())
}

fn resolve_param_bounds<P>(
    this: &impl Resolver,
    bounds: &mut Punctuated<TypeParamBound, P>,
    context: &Context,
) -> Result<()> {
    for bound in bounds {
        match bound {
            TypeParamBound::Trait(bound) => match &mut bound.lifetimes {
                None => {
                    resolve_path(this, &mut bound.path, context)?;
                }

                Some(lifetimes) => {
                    let local_context = resolve_bound_lifetimes(this, lifetimes, context)?;

                    resolve_path(this, &mut bound.path, &local_context)?;
                }
            },

            TypeParamBound::Lifetime(bound) => {
                if context.contains_lifetime(&bound.ident) {
                    continue;
                }

                deny_lifetime(bound)?;
            }

            other => {
                return Err(Error::new(
                    other.span(),
                    "This syntax is not supported by the introspection system.",
                ));
            }
        }
    }

    Ok(())
}

fn resolve_expr(this: &impl Resolver, expr: &mut Expr, context: &Context) -> Result<()> {
    match expr {
        Expr::Array(expr) => {
            for item in expr.elems.iter_mut() {
                resolve_expr(this, item, context)?;
            }
        }

        Expr::Assign(expr) => {
            resolve_expr(this, &mut expr.right, context)?;
        }

        Expr::Async(expr) => resolve_block(this, &mut expr.block, context)?,

        Expr::Await(expr) => {
            resolve_expr(this, &mut expr.base, context)?;
        }

        Expr::Binary(expr) => {
            resolve_expr(this, &mut expr.left, context)?;
            resolve_expr(this, &mut expr.right, context)?;
        }

        Expr::Block(expr) => resolve_block(this, &mut expr.block, context)?,

        Expr::Break(expr) => {
            if let Some(expr) = &mut expr.expr {
                resolve_expr(this, expr, context)?;
            }
        }

        Expr::Call(expr) => {
            resolve_expr(this, &mut expr.func, context)?;

            for arg in &mut expr.args {
                resolve_expr(this, arg, context)?;
            }
        }

        Expr::Cast(expr) => {
            resolve_expr(this, &mut expr.expr, context)?;
            resolve_type(this, &mut expr.ty, context)?;
        }

        Expr::Closure(expr) => {
            let mut body_context = context.clone();

            body_context.constraints = ResolveConstraints::unlimited();

            for input in &mut expr.inputs {
                match input {
                    Pat::Ident(pat) => {
                        body_context.add_ident(pat.ident.clone());
                    }

                    Pat::Type(pat) => {
                        if let Pat::Ident(variable) = pat.pat.as_ref() {
                            body_context.add_ident(variable.ident.clone());
                        }

                        resolve_type(this, &mut pat.ty, context)?
                    }

                    _ => (),
                }
            }

            resolve_expr(this, &mut expr.body, &body_context)?
        }

        Expr::Const(expr) => resolve_block(this, &mut expr.block, context)?,

        Expr::Continue(_) => (),

        Expr::Field(expr) => resolve_expr(this, expr.base.as_mut(), context)?,

        Expr::ForLoop(expr) => {
            let mut body_context = context.clone();

            body_context.constraints = ResolveConstraints::unlimited();

            resolve_pat(this, &mut expr.pat, context, &mut body_context)?;

            resolve_expr(this, expr.expr.as_mut(), context)?;

            if let Some(label) = &expr.label {
                body_context.add_lifetime(label.name.ident.clone());
            }

            resolve_block(this, &mut expr.body, &body_context)?;
        }

        Expr::Group(expr) => resolve_expr(this, expr.expr.as_mut(), context)?,

        Expr::If(expr) => {
            match expr.cond.as_mut() {
                Expr::Let(if_let) => {
                    let mut then_context = context.clone();

                    then_context.constraints = ResolveConstraints::unlimited();

                    resolve_expr(this, if_let.expr.as_mut(), context)?;
                    resolve_pat(this, &mut if_let.pat, context, &mut then_context)?;

                    resolve_block(this, &mut expr.then_branch, &then_context)?;
                }

                _ => {
                    resolve_expr(this, expr.cond.as_mut(), context)?;
                    resolve_block(this, &mut expr.then_branch, context)?;
                }
            }

            if let Some((_, else_branch)) = &mut expr.else_branch {
                resolve_expr(this, else_branch.as_mut(), context)?;
            }
        }

        Expr::Index(expr) => {
            resolve_expr(this, expr.expr.as_mut(), context)?;
            resolve_expr(this, expr.index.as_mut(), context)?;
        }

        Expr::Infer(..) => (),

        Expr::Let(expr) => {
            let mut product = context.clone();

            product.constraints = ResolveConstraints::unlimited();

            resolve_pat(this, expr.pat.as_mut(), context, &mut product)?;
        }

        Expr::Lit(..) => (),

        Expr::Loop(expr) => match &expr.label {
            None => resolve_block(this, &mut expr.body, context)?,

            Some(label) => {
                let mut body_context = context.clone();

                body_context.constraints = ResolveConstraints::unlimited();

                body_context.add_lifetime(label.name.ident.clone());

                resolve_block(this, &mut expr.body, &body_context)?
            }
        },

        Expr::Match(expr) => {
            resolve_expr(this, expr.expr.as_mut(), context)?;

            for arm in &mut expr.arms {
                let mut arm_context = context.clone();

                arm_context.constraints = ResolveConstraints::unlimited();

                resolve_pat(this, &mut arm.pat, context, &mut arm_context)?;

                if let Some((_, guard)) = &mut arm.guard {
                    resolve_expr(this, guard.as_mut(), &arm_context)?;
                }

                resolve_expr(this, arm.body.as_mut(), &arm_context)?;
            }
        }

        Expr::MethodCall(expr) => {
            resolve_expr(this, expr.receiver.as_mut(), context)?;

            if let Some(turbofish) = &mut expr.turbofish {
                resolve_angle_generics(this, turbofish, context)?;
            }

            for arg in &mut expr.args {
                resolve_expr(this, arg, context)?;
            }
        }

        Expr::Paren(expr) => {
            resolve_expr(this, expr.expr.as_mut(), context)?;
        }

        Expr::Path(expr_path) => {
            if expr_path.qself.is_none()
                && expr_path.path.leading_colon.is_none()
                && expr_path.path.segments.len() == 1
            {
                let segment = &expr_path.path.segments[0];

                if let PathArguments::None = &segment.arguments {
                    if !context.contains_ident(&segment.ident) {
                        if let Some(inline) = this.get_const(&segment.ident) {
                            *expr = inline;

                            return Ok(());
                        }
                    }
                }
            }

            if let Some(q_self) = &mut expr_path.qself {
                resolve_type(this, &mut q_self.ty, context)?;
            }

            resolve_path(this, &mut expr_path.path, context)?;
        }

        Expr::Range(expr) => {
            if let Some(start) = &mut expr.start {
                resolve_expr(this, start.as_mut(), context)?;
            }

            if let Some(end) = &mut expr.end {
                resolve_expr(this, end.as_mut(), context)?;
            }
        }

        Expr::Reference(expr) => resolve_expr(this, expr.expr.as_mut(), context)?,

        Expr::Repeat(expr) => {
            resolve_expr(this, expr.expr.as_mut(), context)?;
            resolve_expr(this, expr.len.as_mut(), context)?;
        }

        Expr::Return(expr) => {
            if let Some(expr) = &mut expr.expr {
                resolve_expr(this, expr.as_mut(), context)?;
            }
        }

        Expr::Struct(expr) => {
            for field in &mut expr.fields {
                resolve_expr(this, &mut field.expr, context)?;
            }

            if let Some(expr) = &mut expr.rest {
                resolve_expr(this, expr.as_mut(), context)?;
            }
        }

        Expr::Try(expr) => resolve_expr(this, expr.expr.as_mut(), context)?,

        Expr::TryBlock(expr) => resolve_block(this, &mut expr.block, context)?,

        Expr::Tuple(expr) => {
            for elem in &mut expr.elems {
                resolve_expr(this, elem, context)?;
            }
        }

        Expr::Unary(expr) => resolve_expr(this, expr.expr.as_mut(), context)?,

        Expr::Unsafe(expr) => resolve_block(this, &mut expr.block, context)?,

        Expr::While(expr) => {
            resolve_expr(this, &mut expr.cond, context)?;

            match &expr.label {
                None => resolve_block(this, &mut expr.body, context)?,

                Some(label) => {
                    let mut body_context = context.clone();

                    body_context.constraints = ResolveConstraints::unlimited();

                    body_context.add_lifetime(label.name.ident.clone());

                    resolve_block(this, &mut expr.body, &body_context)?
                }
            }
        }

        Expr::Yield(expr) => {
            if let Some(expr) = &mut expr.expr {
                resolve_expr(this, expr, context)?
            }
        }

        _ => {
            return Err(Error::new(
                expr.span(),
                "This syntax is not supported by the introspection system.",
            ));
        }
    }

    Ok(())
}

fn resolve_block(this: &impl Resolver, block: &mut Block, context: &Context) -> Result<()> {
    let mut context = context.clone();

    context.constraints = ResolveConstraints::unlimited();

    for statement in &mut block.stmts {
        match statement {
            Stmt::Local(local) => {
                if let Some(init) = &mut local.init {
                    resolve_expr(this, init.expr.as_mut(), &context)?;

                    if let Some((_, diverge)) = &mut init.diverge {
                        resolve_expr(this, diverge.as_mut(), &context)?;
                    }
                }

                resolve_pat(this, &mut local.pat, &context.clone(), &mut context)?;
            }

            Stmt::Item(item) => {
                deny_item_export(item)?;
            }

            Stmt::Expr(expr, _) => {
                resolve_expr(this, expr, &context)?;
            }

            Stmt::Macro(statement) => {
                return Err(Error::new(
                    statement.span(),
                    "This syntax is not supported by the introspection system.",
                ))
            }
        }
    }

    Ok(())
}

fn resolve_pat(
    this: &impl Resolver,
    pat: &mut Pat,
    context: &Context,
    product: &mut Context,
) -> Result<()> {
    match pat {
        Pat::Const(pat) => resolve_block(this, &mut pat.block, context)?,

        Pat::Ident(pat) => {
            product.add_ident(pat.ident.clone());

            if let Some((_, sub_pat)) = &mut pat.subpat {
                resolve_pat(this, sub_pat.as_mut(), context, product)?;
            }
        }

        Pat::Lit(..) => (),

        Pat::Or(pat) => {
            for case in &mut pat.cases {
                resolve_pat(this, case, context, product)?;
            }
        }

        Pat::Paren(pat) => {
            resolve_pat(this, pat.pat.as_mut(), context, product)?;
        }

        Pat::Path(pat) => {
            if let Some(q_self) = &mut pat.qself {
                resolve_type(this, q_self.ty.as_mut(), context)?
            }

            resolve_path(this, &mut pat.path, context)?;
        }

        Pat::Range(pat) => {
            if let Some(start) = pat.start.as_mut() {
                resolve_expr(this, start, context)?;
            }
            if let Some(end) = pat.end.as_mut() {
                resolve_expr(this, end, context)?;
            }
        }

        Pat::Reference(pat) => {
            resolve_pat(this, &mut pat.pat, context, product)?;
        }

        Pat::Rest(..) => (),

        Pat::Slice(pat) => {
            for elem in &mut pat.elems {
                resolve_pat(this, elem, context, product)?;
            }
        }

        Pat::Struct(pat) => {
            resolve_path(this, &mut pat.path, context)?;

            for field in &mut pat.fields {
                resolve_pat(this, &mut field.pat, context, product)?;
            }
        }

        Pat::Tuple(pat) => {
            for elem in &mut pat.elems {
                resolve_pat(this, elem, context, product)?;
            }
        }

        Pat::TupleStruct(pat) => {
            resolve_path(this, &mut pat.path, context)?;

            for elem in &mut pat.elems {
                resolve_pat(this, elem, context, product)?;
            }
        }

        Pat::Type(pat) => {
            resolve_pat(this, pat.pat.as_mut(), context, product)?;
            resolve_type(this, pat.ty.as_mut(), context)?;
        }

        Pat::Wild(..) => (),

        _ => {
            return Err(Error::new(
                pat.span(),
                "This syntax is not supported by the introspection system.",
            ))
        }
    }

    Ok(())
}

fn resolve_path(this: &impl Resolver, path: &mut Path, context: &Context) -> Result<()> {
    for segment in &mut path.segments {
        match &mut segment.arguments {
            PathArguments::None => (),

            PathArguments::AngleBracketed(arguments) => {
                resolve_angle_generics(this, arguments, context)?
            }

            PathArguments::Parenthesized(arguments) => {
                resolve_paren_generics(this, arguments, context)?;
            }
        }
    }

    Ok(())
}

fn resolve_angle_generics(
    this: &impl Resolver,
    angle_generics: &mut AngleBracketedGenericArguments,
    context: &Context,
) -> Result<()> {
    for argument in &mut angle_generics.args {
        match argument {
            GenericArgument::Lifetime(argument) => {
                if context.contains_lifetime(&argument.ident) {
                    continue;
                }

                deny_lifetime(argument)?;
            }

            GenericArgument::Type(argument) => resolve_type(this, argument, context)?,

            GenericArgument::Const(argument) => resolve_expr(this, argument, context)?,

            GenericArgument::AssocType(argument) => {
                let mut generics_context = context.clone();

                generics_context.add_ident(argument.ident.clone());

                if let Some(generics) = &mut argument.generics {
                    resolve_angle_generics(this, generics, &generics_context)?;
                }

                resolve_type(this, &mut argument.ty, &generics_context)?;
            }

            GenericArgument::AssocConst(argument) => {
                let mut generics_context = context.clone();

                generics_context.add_ident(argument.ident.clone());

                if let Some(generics) = &mut argument.generics {
                    resolve_angle_generics(this, generics, &generics_context)?;
                }

                resolve_expr(this, &mut argument.value, &generics_context)?;
            }

            GenericArgument::Constraint(argument) => {
                let mut generics_context = context.clone();

                generics_context.add_ident(argument.ident.clone());

                if let Some(generics) = &mut argument.generics {
                    resolve_angle_generics(this, generics, &generics_context)?;
                }

                resolve_param_bounds(this, &mut argument.bounds, &generics_context)?;
            }

            other => {
                return Err(Error::new(
                    other.span(),
                    "This syntax is not supported by the introspection system.",
                ))
            }
        }
    }

    Ok(())
}

fn resolve_bound_lifetimes(
    this: &impl Resolver,
    bound: &mut BoundLifetimes,
    context: &Context,
) -> Result<Context> {
    let mut local_context = context.clone();

    local_context.constraints = ResolveConstraints::unlimited();

    for param in &mut bound.lifetimes {
        match param {
            GenericParam::Lifetime(param) => {
                local_context.add_lifetime(param.lifetime.ident.clone());

                for bound in &mut param.bounds {
                    if local_context.contains_lifetime(&bound.ident) {
                        continue;
                    }

                    deny_lifetime(bound)?;
                }
            }

            GenericParam::Type(param) => {
                local_context.add_ident(param.ident.clone());

                resolve_param_bounds(this, &mut param.bounds, &local_context)?;

                if let Some(default) = &mut param.default {
                    resolve_type(this, default, &local_context)?;
                }
            }

            GenericParam::Const(param) => {
                local_context.add_ident(param.ident.clone());

                resolve_type(this, &mut param.ty, &local_context)?;

                if let Some(default) = &mut param.default {
                    resolve_expr(this, default, &local_context)?;
                }
            }
        }
    }

    Ok(local_context)
}

fn resolve_paren_generics(
    this: &impl Resolver,
    parent_generics: &mut ParenthesizedGenericArguments,
    context: &Context,
) -> Result<()> {
    if let ReturnType::Type(_, output) = &mut parent_generics.output {
        resolve_type(this, output.as_mut(), context)?;
    }

    for input in &mut parent_generics.inputs {
        resolve_type(this, input, context)?;
    }

    Ok(())
}

#[inline(always)]
fn deny_lifetime(item: &Lifetime) -> Result<()> {
    return Err(Error::new(
        item.span(),
        "Non-elided lifetimes not supported by the introspection system.",
    ));
}

fn deny_item_export(item: &mut Item) -> Result<()> {
    match item {
        Item::Const(item) => item.deny_export()?,
        Item::Static(item) => item.deny_export()?,
        Item::Struct(item) => item.deny_export()?,
        Item::Trait(item) => item.deny_export()?,
        Item::Fn(item) => item.deny_export()?,
        Item::Impl(item) => item.deny_export()?,

        _ => (),
    }

    Ok(())
}

#[derive(Clone)]
struct Context {
    constraints: ResolveConstraints,
    shadow: AHashSet<ShadowItem<'static>>,
}

impl Context {
    #[inline]
    fn add_lifetime(&mut self, ident: Ident) {
        let _ = self
            .shadow
            .insert(ShadowItem::LifetimeLike(Cow::Owned(ident)));
    }

    #[inline]
    fn add_ident(&mut self, ident: Ident) {
        let _ = self.shadow.insert(ShadowItem::IdentLike(Cow::Owned(ident)));
    }

    #[inline]
    fn contains_lifetime(&self, ident: &Ident) -> bool {
        self.shadow
            .contains(&ShadowItem::LifetimeLike(Cow::Borrowed(ident)))
    }

    #[inline]
    fn contains_ident(&self, ident: &Ident) -> bool {
        self.shadow
            .contains(&ShadowItem::IdentLike(Cow::Borrowed(ident)))
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum ShadowItem<'a> {
    LifetimeLike(Cow<'a, Ident>),
    IdentLike(Cow<'a, Ident>),
}
