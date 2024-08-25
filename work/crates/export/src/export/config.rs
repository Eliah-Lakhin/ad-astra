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

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Item, Result};

pub struct ExportConfig {
    pub dump: Option<Span>,
    pub stream: Option<TokenStream>,
}

impl ExportConfig {
    pub fn export(self, item: &Item) -> Result<TokenStream> {
        match self.dump {
            None => match self.stream {
                Some(stream) => Ok(quote!(#item #stream)),
                None => Ok(item.to_token_stream()),
            },

            Some(span) => {
                #[cfg(not(debug_assertions))]
                {
                    return Err(syn::Error::new(
                        span,
                        "Debug dump is available in development mode only.",
                    ));
                }

                #[cfg(debug_assertions)]
                {
                    let input_string = {
                        let stream = ToTokens::to_token_stream(item);

                        match syn::parse2::<syn::File>(stream.clone()) {
                            Ok(file) => prettyplease::unparse(&file),
                            Err(_) => stream.to_string(),
                        }
                    };

                    return match self.stream {
                        None => {
                            Err(syn::Error::new(
                                span,
                                format!(
                                    " -- Export System Debug Dump --\n\n{input_string}\n(excluded item)"
                                ),
                            ))
                        }

                        Some(stream) => {
                            let output_string = match syn::parse2::<syn::File>(stream.clone()) {
                                Ok(file) => prettyplease::unparse(&file),
                                Err(_) => stream.to_string(),
                            };

                            let lines = output_string.lines().count();

                            Err(syn::Error::new(span, format!(
                                " -- Export System Debug Dump ({lines} lines) --\n\n\
                                {input_string}\n{output_string}",
                            )))
                        }
                    };
                }
            }
        }
    }
}
