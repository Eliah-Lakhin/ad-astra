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
use syn::{ItemTrait, Result};

use crate::{
    export::{item_impl::ItemSet, ExportConfig},
    utils::{
        Exportable,
        Group,
        PolymorphicScope,
        Prototype,
        Shallow,
        TraitPolymorphism,
        DUMP,
        EXCLUDED,
        INCLUDED,
        SHALLOW,
        TYPE,
    },
};

pub fn export_item_trait(item: &mut ItemTrait) -> Result<ExportConfig> {
    let attrs = item.drain_attrs()?;

    attrs.check(DUMP | INCLUDED | EXCLUDED | SHALLOW | TYPE)?;

    Shallow.init(attrs.shallow());

    let mut trait_polymorphism =
        TraitPolymorphism::new(&item.ident, &mut item.generics, attrs.types()?)?;

    let mut group = Group::default();

    let mut item_set = ItemSet::from_trait_items(&mut item.items)?;

    let mut package_prototype = match item_set.has_package_items() {
        false => None,
        true => Some(Prototype::for_package(item.trait_token.span)),
    };

    loop {
        let mut self_prototype = match item_set.has_self_items() {
            false => None,

            true => {
                let ty = trait_polymorphism
                    .get_self_type()?
                    .expect("Internal error. Missing self type.");

                Some(Prototype::for_type(ty))
            }
        };

        item_set.export::<TraitPolymorphism>(
            &trait_polymorphism,
            &mut group,
            &mut self_prototype,
            &mut package_prototype,
        )?;

        if let Some(prototype) = self_prototype {
            group.prototype(prototype);
        }

        if !trait_polymorphism.rotate()? {
            break;
        }
    }

    if let Some(prototype) = package_prototype {
        group.prototype(prototype);
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
