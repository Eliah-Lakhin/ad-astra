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

use proc_macro2::TokenStream;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Error,
    Item,
    Result,
};

use crate::{
    export::{
        item_const::export_item_const,
        item_fn::export_item_fn,
        item_impl::export_item_impl,
        item_static::export_item_static,
        item_struct::export_item_struct,
        item_trait::export_item_trait,
        item_type::export_item_type,
    },
    utils::{Context, Shallow},
};

pub struct ExportItem(TokenStream);

impl Parse for ExportItem {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut item = input.parse::<Item>()?;

        Context.init(&item);

        let config = match &mut item {
            Item::Const(item) => export_item_const(item),

            Item::Enum(item) => Err(Error::new(
                item.enum_token.span,
                "Enum types cannot be exported.",
            )),

            Item::ExternCrate(item) => Err(Error::new(
                item.extern_token.span,
                "External crate declarations cannot be exported.",
            )),

            Item::Fn(item) => export_item_fn(item),

            Item::ForeignMod(item) => Err(Error::new(
                item.abi.extern_token.span,
                "Abi modules cannot be exported.",
            )),

            Item::Impl(item) => export_item_impl(item),

            Item::Macro(item) => Err(Error::new(
                item.mac.bang_token.span,
                "Macro invocation cannot be exported.",
            )),

            Item::Mod(item) => Err(Error::new(
                item.mod_token.span,
                "Modules cannot be exported.",
            )),

            Item::Static(item) => export_item_static(item),

            Item::Struct(item) => export_item_struct(item),

            Item::Trait(item) => export_item_trait(item),

            Item::TraitAlias(item) => Err(Error::new(
                item.trait_token.span,
                "Trait aliases cannot be exported.",
            )),

            Item::Type(item) => export_item_type(item),

            Item::Union(item) => Err(Error::new(
                item.union_token.span,
                "Union types cannot be exported.",
            )),

            Item::Use(item) => Err(Error::new(
                item.use_token.span,
                "Use declarations cannot be exported.",
            )),

            _ => Err(Error::new(
                item.span(),
                "This syntax is not supported by introspection system.",
            )),
        };

        Shallow.release(config.is_ok());
        Context.release();

        Ok(Self(config?.export(&item)?))
    }
}

impl From<ExportItem> for proc_macro::TokenStream {
    #[inline(always)]
    fn from(value: ExportItem) -> Self {
        value.0.into()
    }
}
