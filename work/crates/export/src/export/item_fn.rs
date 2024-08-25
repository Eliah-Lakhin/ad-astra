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
use syn::{ItemFn, Result};

use crate::{
    export::ExportConfig,
    utils::{
        Component,
        Context,
        EmptyPolymorphism,
        Exportable,
        Facade,
        FunctionPolymorphism,
        Group,
        Invocation,
        Prototype,
        Shallow,
        SignaturePolymorphism,
        DUMP,
        EXCLUDED,
        INCLUDED,
        RENAME,
        SHALLOW,
    },
};

pub fn export_item_fn(item: &mut ItemFn) -> Result<ExportConfig> {
    let attrs = item.drain_attrs()?;

    attrs.check(DUMP | INCLUDED | EXCLUDED | SHALLOW | RENAME)?;

    Shallow.init(attrs.shallow());

    let span = item.sig.ident.span();

    let core = span.face_core();

    let mut group = Group::default();

    let mut package_prototype = Prototype::for_package(span);

    let mut signature_polymorphism = SignaturePolymorphism::new(
        &item.sig.ident,
        &mut item.sig.generics,
        &mut item.sig.inputs,
        &item.sig.output,
    )?;

    let invocation = Invocation::new(&item.sig)?;

    loop {
        let function_polymorphism = FunctionPolymorphism {
            scope: &EmptyPolymorphism,
            signature: &signature_polymorphism,
        };

        let name = attrs
            .rename_checked(&function_polymorphism)?
            .unwrap_or_else(|| item.sig.ident.to_string());

        let name_ref = Context.make_unique_identifier(name.as_str(), span);

        let function_type = invocation.make_function_type(
            &mut group,
            &function_polymorphism,
            name.as_str(),
            &name_ref,
            item.rust_doc(),
        )?;

        let constructor = quote_spanned!(span=> {
            fn component(
                origin: #core::runtime::Origin,
                _lhs: #core::runtime::Arg,
            ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                #core::runtime::Cell::give(origin, #function_type)
            }

            component as fn(
                #core::runtime::Origin,
                #core::runtime::Arg,
            ) -> #core::runtime::RuntimeResult::<#core::runtime::Cell>
        });

        package_prototype.component(Component {
            name_ref: Cow::Owned(name_ref),
            constructor,
            hint: Cow::Owned(function_type),
            doc: item.rust_doc(),
        });

        if !signature_polymorphism.rotate() {
            break;
        }
    }

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
