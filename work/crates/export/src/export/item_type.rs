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

use quote::ToTokens;
use syn::{ItemType, LitStr, Result};

use crate::{
    export::ExportConfig,
    utils::{
        Exportable,
        Group,
        PolymorphicScope,
        Printer,
        Shallow,
        TypeMeta,
        TypePolymorphism,
        TypeUtils,
        DUMP,
        EXCLUDED,
        FAMILY,
        INCLUDED,
        RENAME,
        SHALLOW,
    },
};

pub fn export_item_type(item: &mut ItemType) -> Result<ExportConfig> {
    let attrs = item.drain_attrs()?;

    attrs.check(DUMP | INCLUDED | EXCLUDED | SHALLOW | RENAME | FAMILY)?;

    Shallow.init(attrs.shallow());

    let span = item.ident.span();
    let doc = item.rust_doc();
    let family = attrs.family();

    let mut polymorphism = TypePolymorphism::new(&item.ident, &mut item.generics)?;

    let mut group = Group::default();

    loop {
        let ty = polymorphism.make_type();

        let name = match attrs.rename_unchecked(&polymorphism)? {
            Some(name) => LitStr::new(name.as_str(), span),

            None => {
                let mut target_type = item.ty.as_ref().clone();

                polymorphism.specialize_type(&mut target_type)?;

                target_type.to_display_literal()
            }
        };

        group.type_meta(TypeMeta {
            name: &name,
            doc: doc.as_ref(),
            ty: &ty,
            family,
        });

        group.custom(ty.impl_registered_type());

        Shallow.impl_registered_type(&ty);

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
