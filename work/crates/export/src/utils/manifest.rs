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

use std::{path::Path, str::FromStr};

use cargo_toml::{Dependency, Manifest};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote_spanned, ToTokens};
use syn::{Error, LitStr, Result};

use crate::utils::{Context, Facade};

pub struct ManifestMeta {
    pub(super) name: LitStr,
    pub(super) version: LitStr,
    pub(super) doc: Option<LitStr>,
    pub(super) dependencies: Vec<DependencyMeta>,
}

impl ToTokens for ManifestMeta {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.name.span();

        let core = span.face_core();
        let intrinsics = span.face_intrinsics();
        let vec = span.face_vec();
        let option = span.face_option();

        let origin = Context.make_origin("package", span);

        let dependencies = self.dependencies.iter().map(|dependency| {
            let use_name = &dependency.use_name;
            let name = &dependency.name;
            let package = &dependency.package;
            let version = &dependency.version;

            let unconditional = quote_spanned!(span=> {
                #[allow(unused)]
                use #use_name;

                match #core::runtime::PackageMeta::of(#package, #version) {
                    #option::<&'static #core::runtime::PackageMeta>::None => (),

                    #option::<&'static #core::runtime::PackageMeta>::Some(package_meta) => {
                        fn dependency(
                            origin: #core::runtime::Origin,
                            _lhs: #core::runtime::Arg,
                        ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                            let package_meta = match #core::runtime::PackageMeta::of(#package, #version) {
                                #option::<&'static #core::runtime::PackageMeta>::Some(package_meta) => package_meta,

                                #option::<&'static #core::runtime::PackageMeta>::None => {
                                    return #core::runtime::RuntimeResult::<#core::runtime::Cell>::Err(
                                        #core::runtime::RuntimeError::UnknownPackage {
                                            access_origin: origin,
                                            name: #package,
                                            version: #version,
                                        },
                                    );
                                },
                            };

                            #core::runtime::RuntimeResult::<#core::runtime::Cell>::Ok(
                                #core::runtime::PackageMeta::instance(package_meta),
                            )
                        }

                        let hint = #core::runtime::PackageMeta::ty(package_meta);

                        #vec::push(
                            &mut components,

                            #intrinsics::ComponentDeclaration {
                                name: {
                                    static IDENTIFIER: #core::runtime::RustIdent
                                        = #core::runtime::RustIdent {
                                            origin: &#origin,
                                            string: #name,
                                        };

                                    &IDENTIFIER
                                },

                                constructor: dependency as fn(
                                    #core::runtime::Origin,
                                    #core::runtime::Arg,
                                ) -> #core::runtime::RuntimeResult::<#core::runtime::Cell>,

                                doc: #core::runtime::TypeMeta::doc(hint),

                                hint,
                            }
                        );
                    }
                }
            });

            match dependency.preconditions.is_empty() {
                true => unconditional,

                false => {
                    let preconditions = {
                        let preconditions = &dependency.preconditions;

                        quote_spanned!(span => #( #[#preconditions] )*)
                    };

                    quote_spanned!(span=> {#preconditions #unconditional})
                }
            }
        });

        quote_spanned!(span=> #( #dependencies )*).to_tokens(tokens);
    }
}

impl ManifestMeta {
    pub fn new(span: Span, path: &Path) -> Result<Self> {
        let manifest = match Manifest::from_path(path) {
            Ok(manifest) => manifest,

            Err(error) => {
                return Err(Error::new(
                    span,
                    format!(
                        "Manifest {:?} parse error.\n{error}",
                        path.to_string_lossy(),
                    ),
                ));
            }
        };

        let name;
        let version;
        let doc;

        match manifest.package {
            None => {
                return Err(Error::new(
                    span,
                    format!(
                        "Missing [package] section in manifest {:?}.",
                        path.to_string_lossy(),
                    ),
                ));
            }

            Some(package) => match package.version.get() {
                Ok(string) => {
                    name = LitStr::new(&package.name, span);
                    version = LitStr::new(string, span);
                    doc = package.documentation().map(|doc| LitStr::new(doc, span))
                }

                Err(error) => {
                    return Err(Error::new(
                        span,
                        format!(
                            "Manifest {:?} package version parse error.\n{error}",
                            path.to_string_lossy(),
                        ),
                    ));
                }
            },
        };

        let mut dependencies = Vec::new();

        let dev = quote_spanned!(span=> cfg(test));

        for (name, dependency) in manifest.dependencies {
            dependencies.push(DependencyMeta::new(span, name, &dependency));
        }

        for (name, dependency) in manifest.dev_dependencies {
            dependencies
                .push(DependencyMeta::new(span, name, &dependency).precondition(dev.clone()));
        }

        for (precondition, target) in manifest.target {
            let feature = match TokenStream::from_str(&precondition) {
                Ok(precondition) => precondition,

                Err(error) => {
                    return Err(Error::new(
                        span,
                        format!(
                            "Target {precondition:?} in manifest {:?} parse error.\n{error}",
                            path.to_string_lossy(),
                        ),
                    ));
                }
            };

            for (name, dependency) in target.dependencies {
                dependencies.push(
                    DependencyMeta::new(span, name, &dependency).precondition(feature.clone()),
                );
            }

            for (name, dependency) in target.dev_dependencies {
                dependencies.push(
                    DependencyMeta::new(span, name, &dependency)
                        .precondition(dev.clone())
                        .precondition(feature.clone()),
                );
            }
        }

        Ok(Self {
            name,
            version,
            doc,
            dependencies,
        })
    }

    #[inline(always)]
    pub fn name(&self) -> &LitStr {
        &self.name
    }
}

pub struct DependencyMeta {
    pub(super) package: LitStr,
    pub(super) name: LitStr,
    pub(super) use_name: Ident,
    pub(super) version: LitStr,
    pub(super) preconditions: Vec<TokenStream>,
}

impl DependencyMeta {
    #[inline]
    fn new(span: Span, name: String, dependency: &Dependency) -> Self {
        let name_lit = LitStr::new(&name, span);

        let package = match dependency.package() {
            Some(package) => LitStr::new(package, span),
            None => name_lit.clone(),
        };

        let use_name = Ident::new(name.replace('-', "_").as_str(), span);
        let version = LitStr::new(dependency.req(), span);

        let mut preconditions = Vec::with_capacity(3);

        if dependency.optional() {
            preconditions.push(quote_spanned!(span=> cfg(feature = #name_lit)));
        }

        Self {
            package,
            name: name_lit,
            use_name,
            version,
            preconditions,
        }
    }

    #[inline(always)]
    fn precondition(mut self, precondition: TokenStream) -> Self {
        self.preconditions.push(precondition);

        self
    }
}
