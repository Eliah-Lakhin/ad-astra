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

use std::{borrow::Cow, collections::VecDeque};

use ahash::AHashMap;
use proc_macro2::{Ident, Span};
use syn::{
    punctuated::Punctuated,
    spanned::Spanned,
    AngleBracketedGenericArguments,
    Error,
    Expr,
    ExprLit,
    FnArg,
    GenericArgument,
    GenericParam,
    Generics,
    ItemImpl,
    Pat,
    Path,
    PathArguments,
    PathSegment,
    Result,
    ReturnType,
    Token,
    Type,
    TypePath,
    TypeTuple,
};

use crate::utils::{
    resolve::{ResolveConstraints, Resolver},
    seed_hash_map_with_capacity,
    Exportable,
    CONST,
    TYPE,
};

pub struct EmptyPolymorphism;

impl PolymorphicScope for EmptyPolymorphism {
    #[inline(always)]
    fn specialize_expr(&self, _expr: &mut Expr) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn specialize_type(&self, _ty: &mut Type) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn get_self_type(&self) -> Result<Option<&Type>> {
        Ok(None)
    }

    #[inline(always)]
    fn get_trait_type(&self) -> Result<Option<&Type>> {
        Ok(None)
    }

    #[inline(always)]
    fn get_arg_type(&self, _arg: &Ident) -> Result<Option<Type>> {
        Ok(None)
    }

    #[inline(always)]
    fn get_return_type(&self) -> Result<Option<Type>> {
        Ok(None)
    }
}

pub struct TypePolymorphism<'a> {
    ident: &'a Ident,
    generics: GenericsPolymorphism,
}

impl<'a> PolymorphicScope for TypePolymorphism<'a> {
    #[inline(always)]
    fn specialize_expr(&self, expr: &mut Expr) -> Result<()> {
        self.generics.resolve_expr(
            expr,
            ResolveConstraints {
                references: true,
                impls: true,
            },
        )
    }

    #[inline(always)]
    fn specialize_type(&self, ty: &mut Type) -> Result<()> {
        self.generics.resolve_type(
            ty,
            ResolveConstraints {
                references: true,
                impls: true,
            },
        )
    }

    #[inline(always)]
    fn get_self_type(&self) -> Result<Option<&Type>> {
        Ok(None)
    }

    #[inline(always)]
    fn get_trait_type(&self) -> Result<Option<&Type>> {
        Ok(None)
    }

    #[inline(always)]
    fn get_arg_type(&self, _arg: &Ident) -> Result<Option<Type>> {
        Ok(None)
    }

    #[inline(always)]
    fn get_return_type(&self) -> Result<Option<Type>> {
        Ok(None)
    }
}

impl<'a> TypePolymorphism<'a> {
    pub fn new(ident: &'a Ident, generics: &mut Generics) -> Result<Self> {
        let generics = GenericsPolymorphism::try_from(generics)?;

        Ok(Self { ident, generics })
    }

    #[inline(always)]
    pub fn span(&self) -> Span {
        self.ident.span()
    }

    pub fn make_type(&self) -> Type {
        let arguments = self.generics.make_path_arguments(self.ident.span());

        let mut segments = Punctuated::new();

        segments.push(PathSegment {
            ident: self.ident.clone(),
            arguments,
        });

        Type::Path(TypePath {
            qself: None,
            path: Path {
                leading_colon: None,
                segments,
            },
        })
    }

    #[inline(always)]
    pub fn rotate(&mut self) -> bool {
        self.generics.rotate()
    }
}

pub struct ImplPolymorphism {
    generics: GenericsPolymorphism,
    polymorphic_trait_type: Option<Type>,
    polymorphic_self_type: Type,
    monomorphic_trait_type: Option<Type>,
    monomorphic_self_type: Type,
}

impl Resolver for ImplPolymorphism {
    #[inline(always)]
    fn get_const(&self, ident: &Ident) -> Option<Expr> {
        self.generics.get_const(ident)
    }

    #[inline(always)]
    fn get_type(&self, ident: &Ident) -> Option<Type> {
        if ident.to_string() == "Self" {
            return Some(self.monomorphic_self_type.clone());
        }

        self.generics.get_type(ident)
    }
}

impl PolymorphicScope for ImplPolymorphism {
    #[inline(always)]
    fn specialize_expr(&self, expr: &mut Expr) -> Result<()> {
        self.resolve_expr(
            expr,
            ResolveConstraints {
                references: true,
                impls: true,
            },
        )
    }

    #[inline(always)]
    fn specialize_type(&self, ty: &mut Type) -> Result<()> {
        self.resolve_type(
            ty,
            ResolveConstraints {
                references: true,
                impls: true,
            },
        )
    }

    #[inline(always)]
    fn get_self_type(&self) -> Result<Option<&Type>> {
        Ok(Some(&self.monomorphic_self_type))
    }

    #[inline(always)]
    fn get_trait_type(&self) -> Result<Option<&Type>> {
        Ok(self.monomorphic_trait_type.as_ref())
    }

    #[inline(always)]
    fn get_arg_type(&self, _arg: &Ident) -> Result<Option<Type>> {
        Ok(None)
    }

    #[inline(always)]
    fn get_return_type(&self) -> Result<Option<Type>> {
        Ok(None)
    }
}

impl ImplPolymorphism {
    pub fn new(item: &mut ItemImpl) -> Result<Self> {
        let generics = GenericsPolymorphism::try_from(&mut item.generics)?;

        let polymorphic_trait_type;
        let monomorphic_trait_type;

        match &item.trait_ {
            None => {
                polymorphic_trait_type = None;
                monomorphic_trait_type = None;
            }

            Some((_, path, _)) => {
                let polymorphic = Type::Path(TypePath {
                    qself: None,
                    path: path.clone(),
                });
                let mut monomorphic = polymorphic.clone();

                generics.resolve_type(
                    &mut monomorphic,
                    ResolveConstraints {
                        references: true,
                        impls: true,
                    },
                )?;

                polymorphic_trait_type = Some(polymorphic);
                monomorphic_trait_type = Some(monomorphic);
            }
        }

        let polymorphic_self_type = item.self_ty.as_ref().clone();
        let mut monomorphic_self_type = polymorphic_self_type.clone();

        generics.resolve_type(
            &mut monomorphic_self_type,
            ResolveConstraints {
                references: false,
                impls: false,
            },
        )?;

        Ok(Self {
            generics,
            polymorphic_trait_type,
            polymorphic_self_type,
            monomorphic_trait_type,
            monomorphic_self_type,
        })
    }

    #[inline(always)]
    pub fn rotate(&mut self) -> Result<bool> {
        let result = self.generics.rotate();

        if let Some(polymorphic) = &self.polymorphic_trait_type {
            let mut monomorphic = polymorphic.clone();

            self.generics.resolve_type(
                &mut monomorphic,
                ResolveConstraints {
                    references: true,
                    impls: true,
                },
            )?;

            self.monomorphic_trait_type = Some(monomorphic);
        }

        let mut monomorphic = self.polymorphic_self_type.clone();

        self.generics.resolve_type(
            &mut monomorphic,
            ResolveConstraints {
                references: false,
                impls: false,
            },
        )?;

        self.monomorphic_self_type = monomorphic;

        Ok(result)
    }
}

pub struct TraitPolymorphism<'a> {
    trait_type: TypePolymorphism<'a>,
    self_types: VecDeque<&'a Type>,
    self_rotation: usize,
    monomorphic_trait_type: Type,
    monomorphic_self_type: Type,
}

impl<'a> Resolver for TraitPolymorphism<'a> {
    #[inline(always)]
    fn get_const(&self, ident: &Ident) -> Option<Expr> {
        self.trait_type.generics.get_const(ident)
    }

    #[inline(always)]
    fn get_type(&self, ident: &Ident) -> Option<Type> {
        if ident.to_string() == "Self" {
            return Some(self.monomorphic_self_type.clone());
        }

        self.trait_type.generics.get_type(ident)
    }
}

impl<'a> PolymorphicScope for TraitPolymorphism<'a> {
    #[inline(always)]
    fn specialize_expr(&self, expr: &mut Expr) -> Result<()> {
        self.resolve_expr(
            expr,
            ResolveConstraints {
                references: true,
                impls: true,
            },
        )
    }

    #[inline(always)]
    fn specialize_type(&self, ty: &mut Type) -> Result<()> {
        self.resolve_type(
            ty,
            ResolveConstraints {
                references: true,
                impls: true,
            },
        )
    }

    #[inline(always)]
    fn get_self_type(&self) -> Result<Option<&Type>> {
        Ok(Some(&self.monomorphic_self_type))
    }

    #[inline(always)]
    fn get_trait_type(&self) -> Result<Option<&Type>> {
        Ok(Some(&self.monomorphic_trait_type))
    }

    #[inline(always)]
    fn get_arg_type(&self, _arg: &Ident) -> Result<Option<Type>> {
        Ok(None)
    }

    #[inline(always)]
    fn get_return_type(&self) -> Result<Option<Type>> {
        Ok(None)
    }
}

impl<'a> TraitPolymorphism<'a> {
    pub fn new(ident: &'a Ident, generics: &mut Generics, self_types: &'a [Type]) -> Result<Self> {
        let trait_type = TypePolymorphism::new(ident, generics)?;

        let monomorphic_trait_type = trait_type.make_type();

        let mut monomorphic_self_type = self_types
            .first()
            .expect("Internal error. Missing first self type.")
            .clone();

        trait_type.generics.resolve_type(
            &mut monomorphic_self_type,
            ResolveConstraints {
                references: false,
                impls: false,
            },
        )?;

        Ok(Self {
            trait_type,
            self_types: self_types.iter().collect(),
            self_rotation: 0,
            monomorphic_trait_type,
            monomorphic_self_type,
        })
    }

    pub fn rotate(&mut self) -> Result<bool> {
        self.self_rotation += 1;
        self.self_types.rotate_left(1);

        if self.self_rotation < self.self_types.len() {
            self.specialize_self_type()?;
            return Ok(true);
        }

        let result = self.trait_type.rotate();

        self.specialize_self_type()?;
        self.monomorphic_trait_type = self.trait_type.make_type();

        Ok(result)
    }

    #[inline]
    fn specialize_self_type(&mut self) -> Result<()> {
        let polymorphic_self_type = self
            .self_types
            .front()
            .expect("Internal error. Missing first self type.");

        self.monomorphic_self_type = (*polymorphic_self_type).clone();

        self.trait_type.generics.resolve_type(
            &mut self.monomorphic_self_type,
            ResolveConstraints {
                references: false,
                impls: false,
            },
        )?;

        Ok(())
    }
}

pub struct SignaturePolymorphism<'a> {
    ident: &'a Ident,
    generics: GenericsPolymorphism,
    inputs: ArgumentsPolymorphism,
    output: Option<&'a Type>,
}

impl<'a> SignaturePolymorphism<'a> {
    pub fn new(
        ident: &'a Ident,
        generics: &mut Generics,
        inputs: &mut Punctuated<FnArg, Token![,]>,
        output: &'a ReturnType,
    ) -> Result<Self> {
        let generics = GenericsPolymorphism::try_from(generics)?;
        let inputs = ArgumentsPolymorphism::try_from(inputs)?;

        let output = match output {
            ReturnType::Type(_, ty) => Some(ty.as_ref()),

            ReturnType::Default => None,
        };

        Ok(Self {
            ident,
            generics,
            inputs,
            output,
        })
    }

    #[inline(always)]
    pub fn make_generics_path(&self) -> PathArguments {
        self.generics.make_path_arguments(self.ident.span())
    }

    #[inline]
    pub fn rotate(&mut self) -> bool {
        if self.inputs.rotate() {
            return true;
        }

        if self.generics.rotate() {
            return true;
        }

        false
    }
}

pub struct FunctionPolymorphism<'a, S: PolymorphicScope> {
    pub scope: &'a S,
    pub signature: &'a SignaturePolymorphism<'a>,
}

impl<'a, S: PolymorphicScope> PolymorphicScope for FunctionPolymorphism<'a, S> {
    #[inline]
    fn specialize_expr(&self, expr: &mut Expr) -> Result<()> {
        self.signature.generics.resolve_expr(
            expr,
            ResolveConstraints {
                references: true,
                impls: true,
            },
        )?;
        self.scope.specialize_expr(expr)?;

        Ok(())
    }

    #[inline]
    fn specialize_type(&self, ty: &mut Type) -> Result<()> {
        self.signature.generics.resolve_type(
            ty,
            ResolveConstraints {
                references: true,
                impls: true,
            },
        )?;
        self.scope.specialize_type(ty)?;

        Ok(())
    }

    #[inline]
    fn get_self_type(&self) -> Result<Option<&Type>> {
        self.scope.get_self_type()
    }

    #[inline]
    fn get_trait_type(&self) -> Result<Option<&Type>> {
        self.scope.get_trait_type()
    }

    #[inline]
    fn get_arg_type(&self, arg: &Ident) -> Result<Option<Type>> {
        let mut ty = match self.signature.inputs.get_arg(arg) {
            Some(ty) => ty,
            None => return Ok(None),
        };

        self.signature.generics.resolve_type(
            &mut ty,
            ResolveConstraints {
                references: true,
                impls: false,
            },
        )?;
        self.scope.specialize_type(&mut ty)?;

        Ok(Some(ty))
    }

    #[inline]
    fn get_return_type(&self) -> Result<Option<Type>> {
        let mut ty = match self.signature.output {
            Some(ty) => ty.clone(),
            None => Type::Tuple(TypeTuple {
                paren_token: Default::default(),
                elems: Default::default(),
            }),
        };

        self.signature.generics.resolve_type(
            &mut ty,
            ResolveConstraints {
                references: self.signature.inputs.elided_receiver,
                impls: false,
            },
        )?;
        self.scope.specialize_type(&mut ty)?;

        Ok(Some(ty))
    }
}

pub trait PolymorphicScope {
    fn specialize_expr(&self, expr: &mut Expr) -> Result<()>;

    fn specialize_type(&self, ty: &mut Type) -> Result<()>;

    fn get_self_type(&self) -> Result<Option<&Type>>;

    fn get_trait_type(&self) -> Result<Option<&Type>>;

    fn get_arg_type(&self, arg: &Ident) -> Result<Option<Type>>;

    fn get_return_type(&self) -> Result<Option<Type>>;
}

struct GenericsPolymorphism {
    variants: Vec<GenericVariant>,
    index: AHashMap<GenericParameter<'static>, usize>,
    length: usize,
    rotation: usize,
}

impl<'a> TryFrom<&'a mut Generics> for GenericsPolymorphism {
    type Error = Error;

    fn try_from(generics: &'a mut Generics) -> Result<Self> {
        let mut variants = Vec::with_capacity(generics.params.len());
        let mut index = seed_hash_map_with_capacity(generics.params.len());
        let mut length = 1;

        for generic in &mut generics.params {
            match generic {
                GenericParam::Lifetime(param) => {
                    return Err(Error::new(
                        param.span(),
                        "Explicit lifetimes not supported by the introspection system.",
                    ));
                }

                GenericParam::Const(param) => {
                    let attrs = param.drain_attrs()?;

                    attrs.check(CONST)?;

                    let constants = attrs.constants()?;

                    length *= constants.len();

                    let _ = index.insert(
                        GenericParameter::Const(Cow::Owned(param.ident.clone())),
                        variants.len(),
                    );

                    variants.push(GenericVariant {
                        specializations: constants
                            .into_iter()
                            .map(|literal| {
                                GenericArgument::Const(Expr::Lit(ExprLit {
                                    attrs: vec![],
                                    lit: literal,
                                }))
                            })
                            .collect(),
                        rotation: 0,
                    });
                }

                GenericParam::Type(param) => {
                    let attrs = param.drain_attrs()?;

                    attrs.check(TYPE)?;

                    let types = attrs.types()?;

                    length *= types.len();

                    let _ = index.insert(
                        GenericParameter::Type(Cow::Owned(param.ident.clone())),
                        variants.len(),
                    );

                    variants.push(GenericVariant {
                        specializations: types
                            .into_iter()
                            .cloned()
                            .map(|ty| GenericArgument::Type(ty))
                            .collect(),
                        rotation: 0,
                    });
                }
            }
        }

        Ok(Self {
            variants,
            index,
            length,
            rotation: 0,
        })
    }
}

impl Resolver for GenericsPolymorphism {
    #[inline]
    fn get_const(&self, ident: &Ident) -> Option<Expr> {
        let index = self
            .index
            .get(&GenericParameter::Const(Cow::Borrowed(ident)))
            .copied()?;

        if let GenericArgument::Const(result) = self.variants[index].specializations.front()? {
            return Some(result.clone());
        }

        None
    }

    #[inline]
    fn get_type(&self, ident: &Ident) -> Option<Type> {
        let index = self
            .index
            .get(&GenericParameter::Type(Cow::Borrowed(ident)))
            .copied()?;

        if let GenericArgument::Type(result) = self.variants[index].specializations.front()? {
            return Some(result.clone());
        }

        None
    }
}

impl GenericsPolymorphism {
    fn rotate(&mut self) -> bool {
        for variant in self.variants.iter_mut().rev() {
            variant.rotation += 1;
            variant.specializations.rotate_left(1);

            if variant.rotation >= variant.specializations.len() {
                variant.rotation = 0;
                continue;
            }

            break;
        }

        self.rotation += 1;

        if self.rotation >= self.length {
            self.rotation = 0;
            return false;
        }

        true
    }

    fn make_path_arguments(&self, span: Span) -> PathArguments {
        if self.variants.is_empty() {
            return PathArguments::None;
        }

        let mut template = Punctuated::new();

        for variant in self.variants.iter() {
            let argument = variant
                .specializations
                .front()
                .expect("Internal error. Empty specialization set.");

            if !template.is_empty() {
                template.push_punct(Token![,](span));
            }

            template.push_value(argument.clone());
        }

        PathArguments::AngleBracketed(AngleBracketedGenericArguments {
            colon2_token: Some(Token![::](span)),
            lt_token: Token![<](span),
            args: template,
            gt_token: Token![>](span),
        })
    }
}

struct GenericVariant {
    specializations: VecDeque<GenericArgument>,
    rotation: usize,
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum GenericParameter<'a> {
    Const(Cow<'a, Ident>),
    Type(Cow<'a, Ident>),
}

struct ArgumentsPolymorphism {
    elided_receiver: bool,
    variants: Vec<ArgumentVariant>,
    index: AHashMap<Cow<'static, Ident>, usize>,
    length: usize,
    rotation: usize,
}

impl<'a> TryFrom<&'a mut Punctuated<FnArg, Token![,]>> for ArgumentsPolymorphism {
    type Error = Error;

    fn try_from(arguments: &'a mut Punctuated<FnArg, Token![,]>) -> Result<Self> {
        let mut elided_receiver = false;
        let mut variants = Vec::with_capacity(arguments.len());
        let mut index = seed_hash_map_with_capacity(arguments.len());
        let mut length = 1;

        for argument in arguments {
            match argument {
                FnArg::Receiver(argument) => {
                    if argument.reference.is_some() {
                        elided_receiver = true;
                    }
                }

                FnArg::Typed(argument) => {
                    let ident = match argument.pat.as_ref() {
                        Pat::Ident(pat) => pat.ident.clone(),

                        _ => {
                            return Err(Error::new(
                                argument.pat.span(),
                                "Cannot introspect unnamed argument.",
                            ))
                        }
                    };

                    let attrs = argument.drain_attrs()?;

                    let _ = index.insert(Cow::Owned(ident), variants.len());

                    if !attrs.has_types() {
                        variants.push(ArgumentVariant {
                            specializations: VecDeque::from([argument.ty.as_ref().clone()]),
                            rotation: 0,
                        });
                        continue;
                    }

                    let types = attrs.types()?;

                    length *= types.len();

                    variants.push(ArgumentVariant {
                        specializations: types.into_iter().cloned().collect(),
                        rotation: 0,
                    });
                }
            }
        }

        Ok(Self {
            elided_receiver,
            variants,
            index,
            length,
            rotation: 0,
        })
    }
}

impl ArgumentsPolymorphism {
    fn rotate(&mut self) -> bool {
        for variant in self.variants.iter_mut().rev() {
            variant.rotation += 1;
            variant.specializations.rotate_left(1);

            if variant.rotation >= variant.specializations.len() {
                variant.rotation = 0;
                continue;
            }

            break;
        }

        self.rotation += 1;

        if self.rotation >= self.length {
            self.rotation = 0;
            return false;
        }

        true
    }

    #[inline]
    fn get_arg(&self, arg: &Ident) -> Option<Type> {
        let index = self.index.get(&Cow::Borrowed(arg)).copied()?;

        let variant = &self.variants[index];

        Some(variant.specializations.front()?.clone())
    }
}

struct ArgumentVariant {
    specializations: VecDeque<Type>,
    rotation: usize,
}
