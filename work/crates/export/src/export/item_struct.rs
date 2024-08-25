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

use std::{borrow::Cow, path::PathBuf};

use quote::{quote_spanned, ToTokens};
use syn::{
    spanned::Spanned,
    Error,
    Fields,
    Index,
    ItemStruct,
    LitStr,
    Member,
    Result,
    Type,
    Visibility,
};

use crate::{
    export::ExportConfig,
    utils::{
        Attrs,
        Coercion,
        Component,
        Context,
        EmptyPolymorphism,
        Exportable,
        Facade,
        Group,
        IdRef,
        ManifestMeta,
        Operator,
        OperatorOrigin,
        Package,
        PolymorphicScope,
        Printer,
        Prototype,
        ScriptAssign,
        ScriptClone,
        ScriptConcat,
        ScriptDebug,
        ScriptDefault,
        ScriptHash,
        ScriptOrd,
        ScriptPartialEq,
        ScriptPartialOrd,
        Shallow,
        TypeFamily,
        TypeMeta,
        TypePolymorphism,
        TypeUtils,
        ACCESS,
        DUMP,
        EXCLUDED,
        FAMILY,
        INCLUDED,
        PACKAGE,
        RENAME,
        SHALLOW,
        UNSPECIFIED,
    },
};

pub fn export_item_struct(item: &mut ItemStruct) -> Result<ExportConfig> {
    let attrs = item.drain_attrs()?;

    attrs.check(DUMP | INCLUDED | EXCLUDED | SHALLOW | PACKAGE | RENAME | FAMILY)?;

    Shallow.init(attrs.shallow());

    let span = item.ident.span();

    let name;
    let doc = item.rust_doc();
    let family;
    let manifest;

    match attrs.package() {
        None => {
            name = None;
            family = attrs.family();
            manifest = None;
        }

        Some(path) => {
            if !item.generics.params.is_empty() {
                return Err(Error::new(
                    item.generics.span(),
                    "A package type cannot have generics.",
                ));
            }

            let span = path.span();

            let manifest_meta = ManifestMeta::new(span, PathBuf::from(path.value()).as_path())?;

            name = Some(LitStr::new(
                &format!("‹{}›", manifest_meta.name().value()),
                manifest_meta.name().span(),
            ));

            family = TypeFamily::Package;

            manifest = Some(manifest_meta);
        }
    };

    let mut polymorphism = TypePolymorphism::new(&item.ident, &mut item.generics)?;

    let mut group = Group::default();

    let field_set = FieldSet::new(&mut item.fields)?;

    let clone = match attrs.derive().impls_clone() {
        None => None,
        Some(span) => Some(Context.make_origin("derive_clone", span)),
    };

    let debug = match attrs.derive().impls_debug() {
        None => None,
        Some(span) => Some(Context.make_origin("derive_debug", span)),
    };

    let partial_eq = match attrs.derive().impls_partial_eq() {
        None => None,
        Some(span) => Some(Context.make_origin("derive_partial_eq", span)),
    };

    let default = match attrs.derive().impls_default() {
        None => None,
        Some(span) => Some(Context.make_origin("derive_default", span)),
    };

    let partial_ord = match attrs.derive().impls_partial_ord() {
        None => None,
        Some(span) => Some(Context.make_origin("derive_partial_ord", span)),
    };

    let ord = match attrs.derive().impls_ord() {
        None => None,
        Some(span) => Some(Context.make_origin("derive_ord", span)),
    };

    let hash = match attrs.derive().impls_hash() {
        None => None,
        Some(span) => Some(Context.make_origin("derive_hash", span)),
    };

    loop {
        let ty = polymorphism.make_type();

        let name = match &name {
            Some(name) => Cow::Borrowed(name),

            None => {
                let name = attrs
                    .rename_unchecked(&polymorphism)?
                    .map(|name| LitStr::new(name.as_str(), span))
                    .unwrap_or_else(|| ty.to_display_literal());

                Cow::Owned(name)
            }
        };

        group.type_meta(TypeMeta {
            name: name.as_ref(),
            doc: doc.as_ref(),
            ty: &ty,
            family,
        });

        let coercion = Coercion {
            downcast_own: true,
            downcast_ref: true,
            downcast_mut: true,
            upcast_own: true,
            upcast_ref: true,
            upcast_mut: true,
        };

        group.custom(ty.impl_registered_type());
        group.custom(ty.impl_coercion(coercion));

        Shallow.impl_registered_type(&ty);
        Shallow.impl_coercion(&ty, coercion);

        let mut prototype = Prototype::for_type(&ty);

        prototype.operator(OperatorOrigin::Primary, Operator::Assign);
        group.custom(ScriptAssign { span, ty: &ty });

        Shallow.impl_operator(&ty, None, Operator::Assign, span);

        prototype.operator(OperatorOrigin::Primary, Operator::Concat);
        group.custom(ScriptConcat { span, ty: &ty });

        Shallow.impl_operator(&ty, None, Operator::Concat, span);

        if let Some(origin) = &clone {
            let span = origin.span();

            prototype.operator(OperatorOrigin::Origin(origin), Operator::Clone);
            group.custom(ScriptClone { span, lhs: &ty });

            Shallow.impl_operator(&ty, None, Operator::Clone, span);
        }

        if let Some(origin) = &debug {
            let span = origin.span();

            prototype.operator(OperatorOrigin::Origin(origin), Operator::Debug);
            group.custom(ScriptDebug { span, lhs: &ty });

            Shallow.impl_operator(&ty, None, Operator::Debug, span);
        }

        if let Some(origin) = &partial_eq {
            let span = origin.span();

            prototype.operator(OperatorOrigin::Origin(origin), Operator::PartialEq);
            group.custom(ScriptPartialEq {
                span,
                lhs: &ty,
                rhs: &ty,
            });

            Shallow.impl_operator(&ty, None, Operator::PartialEq, span);
        }

        if let Some(origin) = &default {
            let span = origin.span();

            prototype.operator(OperatorOrigin::Origin(origin), Operator::Default);
            group.custom(ScriptDefault { span, lhs: &ty });

            Shallow.impl_operator(&ty, None, Operator::Default, span);
        }

        if let Some(origin) = &partial_ord {
            let span = origin.span();

            prototype.operator(OperatorOrigin::Origin(origin), Operator::PartialOrd);
            group.custom(ScriptPartialOrd {
                span,
                lhs: &ty,
                rhs: &ty,
            });

            Shallow.impl_operator(&ty, None, Operator::PartialOrd, span);
        }

        if let Some(origin) = &ord {
            let span = origin.span();

            prototype.operator(OperatorOrigin::Origin(origin), Operator::Ord);
            group.custom(ScriptOrd { span, lhs: &ty });

            Shallow.impl_operator(&ty, None, Operator::Ord, span);
        }

        if let Some(origin) = &hash {
            let span = origin.span();

            prototype.operator(OperatorOrigin::Origin(origin), Operator::Hash);
            group.custom(ScriptHash { span, lhs: &ty });

            Shallow.impl_operator(&ty, None, Operator::Hash, span);
        }

        if let Some(manifest) = &manifest {
            group.package(Package { ty: &ty, manifest });

            prototype.manifest(&manifest);

            Shallow.impl_package(&ty, Context.span());
        }

        for field in &field_set.fields {
            field.export(&ty, &polymorphism, &mut prototype)?;
        }

        group.prototype(prototype);

        if !polymorphism.rotate() {
            break;
        }
    }

    Ok(ExportConfig {
        dump: attrs.dump(),
        stream: match attrs.disabled() {
            true => None,
            false => match attrs.shallow() {
                true => Some(Shallow.to_token_stream()),
                false => Some(group.to_token_stream()),
            },
        },
    })
}

struct FieldSet<'a> {
    fields: Vec<FieldMeta<'a>>,
}

impl<'a> FieldSet<'a> {
    fn new(source: &'a mut Fields) -> Result<Self> {
        let mut fields = Vec::with_capacity(source.len());

        for (index, field) in source.iter_mut().enumerate() {
            let field_attrs = field.drain_attrs()?;

            field_attrs.check(UNSPECIFIED | INCLUDED | EXCLUDED | RENAME | ACCESS)?;

            if field_attrs.excluded() {
                continue;
            }

            let included_explicitly = match &field.vis {
                Visibility::Public(..) => true,
                _ => false,
            };

            if !field_attrs.included() && !included_explicitly && !field_attrs.specified() {
                continue;
            }

            let readable = field_attrs.readable();
            let writeable = field_attrs.writeable();

            let doc = field.rust_doc();

            let member;
            let name;

            match &field.ident {
                None => {
                    member = Member::Unnamed(Index {
                        index: index as u32,
                        span: field.span(),
                    });

                    name = match field_attrs.has_rename_variables() {
                        true => None,

                        false => Some(
                            field_attrs
                                .rename_checked(&EmptyPolymorphism)?
                                .map(|name| {
                                    Context.make_unique_identifier(name.as_str(), field.span())
                                })
                                .unwrap_or_else(|| {
                                    Context.make_unique_identifier(
                                        index.to_string().as_str(),
                                        field.span(),
                                    )
                                }),
                        ),
                    }
                }

                Some(ident) => {
                    member = Member::Named(ident.clone());

                    name = match field_attrs.has_rename_variables() {
                        true => None,

                        false => Some(
                            field_attrs
                                .rename_checked(&EmptyPolymorphism)?
                                .map(|name| {
                                    Context.make_unique_identifier(name.as_str(), field.span())
                                })
                                .unwrap_or_else(|| {
                                    Context.make_unique_identifier(
                                        ident.to_string().as_str(),
                                        field.span(),
                                    )
                                }),
                        ),
                    }
                }
            };

            fields.push(FieldMeta {
                attrs: field_attrs,
                member,
                ty: &field.ty,
                doc,
                name,
                readable,
                writeable,
            })
        }

        Ok(Self { fields })
    }
}

struct FieldMeta<'a> {
    attrs: Attrs,
    member: Member,
    ty: &'a Type,
    doc: Option<LitStr>,
    name: Option<IdRef>,
    readable: bool,
    writeable: bool,
}

impl<'a> FieldMeta<'a> {
    fn export(
        &'a self,
        receiver_type: &Type,
        polymorphism: &TypePolymorphism<'_>,
        prototype: &mut Prototype<'a>,
    ) -> Result<()> {
        let span = self.member.span();

        let core = span.face_core();
        let option = span.face_option();

        let component_type = {
            let mut ty = self.ty.clone();

            polymorphism.specialize_type(&mut ty)?;

            ty
        };

        let member = &self.member;

        let by_ref = match self.readable {
            false => {
                quote_spanned!(span=> #option::<unsafe fn(
                    *const #receiver_type
                ) -> *const #component_type>::None)
            }

            true => {
                let addr_of = span.face_addr_of();

                quote_spanned!(span=> {
                    unsafe fn by_ref(
                        from: *const #receiver_type,
                    ) -> *const #component_type {
                        #addr_of((*from).#member)
                    }

                    #option::<unsafe fn(
                        *const #receiver_type
                    ) -> *const #component_type>::Some(by_ref)
                })
            }
        };

        let by_mut = match self.writeable {
            false => {
                quote_spanned!(span=> #option::<unsafe fn(
                    *mut #receiver_type
                ) -> *mut #component_type>::None)
            }

            true => {
                let addr_of_mut = span.face_addr_of_mut();

                quote_spanned!(span=> {
                    unsafe fn by_mut(
                        from: *mut #receiver_type,
                    ) -> *mut #component_type {
                        #addr_of_mut((*from).#member)
                    }

                    #option::<unsafe fn(
                        *mut #receiver_type
                    ) -> *mut #component_type>::Some(by_mut)
                })
            }
        };

        let constructor = quote_spanned!(span=> {
            fn component(
                origin: #core::runtime::Origin,
                lhs: #core::runtime::Arg,
            ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                #core::runtime::Cell::map_ptr::<#receiver_type, #component_type>(
                    lhs.data,
                    origin,
                    #by_ref,
                    #by_mut,
                )
            }

            component as fn(
                #core::runtime::Origin,
                #core::runtime::Arg,
            ) -> #core::runtime::RuntimeResult::<#core::runtime::Cell>
        });

        Shallow.assert_type_impls_script_type(&component_type, span);

        let name_ref = match &self.name {
            Some(name) => Cow::Borrowed(name),

            None => {
                let name = self
                    .attrs
                    .rename_checked(polymorphism)?
                    .expect("Missing name.");

                let name_ref = Context.make_shared_identifier(name.as_str(), member.span());

                Cow::Owned(name_ref)
            }
        };

        prototype.component(Component {
            name_ref,
            constructor,
            hint: Cow::Owned(component_type),
            doc: self.doc.clone(),
        });

        Ok(())
    }
}
