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

use quote::{quote_spanned, ToTokens};
use syn::{spanned::Spanned, Error, ItemConst, Result};

use crate::{
    export::ExportConfig,
    utils::{
        Component,
        Context,
        EmptyPolymorphism,
        Exportable,
        Facade,
        Group,
        Prototype,
        Shallow,
        DUMP,
        EXCLUDED,
        INCLUDED,
        RENAME,
        SHALLOW,
    },
};

pub fn export_item_const(item: &mut ItemConst) -> Result<ExportConfig> {
    if !item.generics.params.is_empty() {
        return Err(Error::new(
            item.generics.span(),
            "Constants with generics are not supported by the introspection system.",
        ));
    }

    let attrs = item.drain_attrs()?;

    attrs.check(DUMP | INCLUDED | EXCLUDED | SHALLOW | RENAME)?;

    Shallow.init(attrs.shallow());

    let ident = &item.ident;
    let span = ident.span();

    let mut group = Group::default();

    let mut package_prototype = Prototype::for_package(span);

    let name = attrs
        .rename_checked(&EmptyPolymorphism)?
        .unwrap_or_else(|| item.ident.to_string());

    let name_ref = Context.make_unique_identifier(name.as_str(), span);

    let constructor = {
        let core = span.face_core();

        Shallow.assert_ref_type_impls_static_upcast(item.ty.as_ref(), item.ty.span());

        quote_spanned!(span=> {
            fn component(
                origin: #core::runtime::Origin,
                _lhs: #core::runtime::Arg,
            ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                #core::runtime::Cell::give(origin, &#ident)
            }

            component as fn(
                #core::runtime::Origin,
                #core::runtime::Arg,
            ) -> #core::runtime::RuntimeResult::<#core::runtime::Cell>
        })
    };

    package_prototype.component(Component {
        name_ref: Cow::Owned(name_ref),
        constructor,
        hint: Cow::Borrowed(item.ty.as_ref()),
        doc: item.rust_doc(),
    });

    Shallow.assert_type_impls_script_type(item.ty.as_ref(), item.ty.span());

    group.prototype(package_prototype);

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
