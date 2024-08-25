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

use proc_macro2::{Ident, TokenStream};
use quote::{quote_spanned, ToTokens};
use syn::{
    punctuated::Punctuated,
    spanned::Spanned,
    Path,
    PathArguments,
    PathSegment,
    Type,
    TypeParamBound,
    TypePath,
};

use crate::utils::{Facade, FnMeta, PathUtils, Shallow};

pub trait TypeUtils<'a> {
    fn impl_registered_type(self) -> TokenStream;
    fn type_hint(self) -> TokenStream;
    fn downcast_hint(self) -> TokenStream;
    fn upcast_hint(self) -> TokenStream;
    fn impl_coercion(self, coercion: Coercion) -> TokenStream;
    fn impl_shallow_coercion(self, coercion: Coercion) -> TokenStream;
}

impl<'a, T: ToTokens> TypeUtils<'a> for &'a T {
    fn impl_registered_type(self) -> TokenStream {
        let ty = self;
        let span = ty.span();

        let intrinsics = span.face_intrinsics();

        quote_spanned!(span=>
            #[allow(non_local_definitions)]
            impl #intrinsics::RegisteredType for #ty {}
        )
    }

    fn type_hint(self) -> TokenStream {
        let ty = self;
        let span = ty.span();
        let core = span.face_core();

        quote_spanned!(span=> <#ty as #core::runtime::ScriptType>::type_meta())
    }

    fn downcast_hint(self) -> TokenStream {
        let ty = self;
        let span = ty.span();

        let core = span.face_core();

        quote_spanned!(span=> <#ty as #core::runtime::Downcast>::hint())
    }

    fn upcast_hint(self) -> TokenStream {
        let ty = self;
        let span = ty.span();

        let core = span.face_core();

        quote_spanned!(span=> <#ty as #core::runtime::Upcast>::hint())
    }

    fn impl_coercion(self, coercion: Coercion) -> TokenStream {
        let ty = self;
        let span = ty.span();

        let core = span.face_core();

        let boxed = span.face_box();

        let hint = self.type_hint();

        let upcast_own = match coercion.upcast_own {
            false => None,

            true => Some(quote_spanned!(span=>
                #[allow(non_local_definitions)]
                impl<'a> #core::runtime::Upcast<'a> for #ty {
                    type Output = #boxed<#ty>;

                    #[inline(always)]
                    fn upcast(
                        _origin: #core::runtime::Origin,
                        this: Self,
                    ) -> #core::runtime::RuntimeResult<Self::Output> {
                        #core::runtime::RuntimeResult::Ok(#boxed::new(this))
                    }

                    #[inline(always)]
                    fn hint() -> #core::runtime::TypeHint {
                        #core::runtime::TypeHint::Type(#hint)
                    }
                }
            )),
        };

        let upcast_ref = match coercion.upcast_ref {
            false => None,

            true => Some(quote_spanned!(span=>
                #[allow(non_local_definitions)]
                impl<'a> #core::runtime::Upcast<'a> for &'a #ty {
                    type Output = &'a #ty;

                    #[inline(always)]
                    fn upcast(
                        _origin: #core::runtime::Origin,
                        this: Self,
                    ) -> #core::runtime::RuntimeResult<Self::Output> {
                        #core::runtime::RuntimeResult::Ok(this)
                    }

                    #[inline(always)]
                    fn hint() -> #core::runtime::TypeHint {
                        #core::runtime::TypeHint::Type(#hint)
                    }
                }
            )),
        };

        let upcast_mut = match coercion.upcast_mut {
            false => None,

            true => Some(quote_spanned!(span=>
                #[allow(non_local_definitions)]
                impl<'a> #core::runtime::Upcast<'a> for &'a mut #ty {
                    type Output = &'a mut #ty;

                    #[inline(always)]
                    fn upcast(
                        _origin: #core::runtime::Origin,
                        this: Self,
                    ) -> #core::runtime::RuntimeResult<Self::Output> {
                        #core::runtime::RuntimeResult::Ok(this)
                    }

                    #[inline(always)]
                    fn hint() -> #core::runtime::TypeHint {
                        #core::runtime::TypeHint::Type(#hint)
                    }
                }
            )),
        };

        let downcast_own = match coercion.downcast_own {
            false => None,

            true => Some(quote_spanned!(span=>
                #[allow(non_local_definitions)]
                impl<'a> #core::runtime::Downcast<'a> for #ty {
                    #[inline(always)]
                    fn downcast(
                        origin: #core::runtime::Origin,
                        provider: #core::runtime::Provider<'a>,
                    ) -> #core::runtime::RuntimeResult<Self> {
                        let cell = #core::runtime::Provider::to_owned(provider);

                        #core::runtime::Cell::take::<#ty>(cell, origin)
                    }

                    #[inline(always)]
                    fn hint() -> #core::runtime::TypeHint {
                        #core::runtime::TypeHint::Type(#hint)
                    }
                }
            )),
        };

        let downcast_ref = match coercion.downcast_ref {
            false => None,

            true => Some(quote_spanned!(span=>
                #[allow(non_local_definitions)]
                impl<'a> #core::runtime::Downcast<'a> for &'a #ty {
                    #[inline(always)]
                    fn downcast(
                        origin: #core::runtime::Origin,
                        provider: #core::runtime::Provider<'a>,
                    ) -> #core::runtime::RuntimeResult<Self> {
                        let cell = #core::runtime::Provider::to_borrowed(
                            provider,
                            &origin,
                        )?;

                        #core::runtime::Cell::borrow_ref::<#ty>(cell, origin)
                    }

                    #[inline(always)]
                    fn hint() -> #core::runtime::TypeHint {
                        #core::runtime::TypeHint::Type(#hint)
                    }
                }
            )),
        };

        let downcast_mut = match coercion.downcast_mut {
            false => None,

            true => Some(quote_spanned!(span=>
                #[allow(non_local_definitions)]
                impl<'a> #core::runtime::Downcast<'a> for &'a mut #ty {
                    #[inline(always)]
                    fn downcast(
                        origin: #core::runtime::Origin,
                        provider: #core::runtime::Provider<'a>,
                    ) -> #core::runtime::RuntimeResult<Self> {
                        let cell = #core::runtime::Provider::to_borrowed(
                            provider,
                            &origin,
                        )?;

                        #core::runtime::Cell::borrow_mut::<#ty>(cell, origin)
                    }

                    #[inline(always)]
                    fn hint() -> #core::runtime::TypeHint {
                        #core::runtime::TypeHint::Type(#hint)
                    }
                }
            )),
        };

        quote_spanned!(span=>
            #downcast_own
            #downcast_ref
            #downcast_mut
            #upcast_own
            #upcast_ref
            #upcast_mut
        )
    }

    fn impl_shallow_coercion(self, coercion: Coercion) -> TokenStream {
        let ty = self;
        let span = ty.span();

        let core = span.face_core();

        let boxed = span.face_box();
        let panic = span.face_panic();
        let type_name = span.face_type_name();

        let upcast_own = match coercion.upcast_own {
            false => None,

            true => Some(quote_spanned!(span=>
                #[allow(non_local_definitions)]
                impl<'a> #core::runtime::Upcast<'a> for #ty {
                    type Output = #boxed<#ty>;

                    #[inline(always)]
                    fn upcast(
                        _origin: #core::runtime::Origin,
                        _this: Self,
                    ) -> #core::runtime::RuntimeResult<Self::Output> {
                        let name = #type_name::<Self>();
                        #panic("{name} type was not registered. Probably because export has been \
                        disabled for this type.");
                    }

                    #[inline(always)]
                    fn hint() -> #core::runtime::TypeHint {
                        let name = #type_name::<Self>();
                        #panic("{name} type was not registered. Probably because export has been \
                        disabled for this type.");
                    }
                }
            )),
        };

        let upcast_ref = match coercion.upcast_ref {
            false => None,

            true => Some(quote_spanned!(span=>
                #[allow(non_local_definitions)]
                impl<'a> #core::runtime::Upcast<'a> for &'a #ty {
                    type Output = &'a #ty;

                    #[inline(always)]
                    fn upcast(
                        _origin: #core::runtime::Origin,
                        _this: Self,
                    ) -> #core::runtime::RuntimeResult<Self::Output> {
                        let name = #type_name::<Self>();
                        #panic("{name} type was not registered. Probably because export has been \
                        disabled for this type.");
                    }

                    #[inline(always)]
                    fn hint() -> #core::runtime::TypeHint {
                        let name = #type_name::<Self>();
                        #panic("{name} type was not registered. Probably because export has been \
                        disabled for this type.");
                    }
                }
            )),
        };

        let upcast_mut = match coercion.upcast_mut {
            false => None,

            true => Some(quote_spanned!(span=>
                #[allow(non_local_definitions)]
                impl<'a> #core::runtime::Upcast<'a> for &'a mut #ty {
                    type Output = &'a mut #ty;

                    #[inline(always)]
                    fn upcast(
                        _origin: #core::runtime::Origin,
                        _this: Self,
                    ) -> #core::runtime::RuntimeResult<Self::Output> {
                        let name = #type_name::<Self>();
                        #panic("{name} type was not registered. Probably because export has been \
                        disabled for this type.");
                    }

                    #[inline(always)]
                    fn hint() -> #core::runtime::TypeHint {
                        let name = #type_name::<Self>();
                        #panic("{name} type was not registered. Probably because export has been \
                        disabled for this type.");
                    }
                }
            )),
        };

        let downcast_own = match coercion.downcast_own {
            false => None,

            true => Some(quote_spanned!(span=>
                #[allow(non_local_definitions)]
                impl<'a> #core::runtime::Downcast<'a> for #ty {
                    #[inline(always)]
                    fn downcast(
                        _origin: #core::runtime::Origin,
                        _provider: #core::runtime::Provider<'a>,
                    ) -> #core::runtime::RuntimeResult<Self> {
                        let name = #type_name::<Self>();
                        #panic("{name} type was not registered. Probably because export has been \
                        disabled for this type.");
                    }

                    #[inline(always)]
                    fn hint() -> #core::runtime::TypeHint {
                        let name = #type_name::<Self>();
                        #panic("{name} type was not registered. Probably because export has been \
                        disabled for this type.");
                    }
                }
            )),
        };

        let downcast_ref = match coercion.downcast_ref {
            false => None,

            true => Some(quote_spanned!(span=>
                #[allow(non_local_definitions)]
                impl<'a> #core::runtime::Downcast<'a> for &'a #ty {
                    #[inline(always)]
                    fn downcast(
                        _origin: #core::runtime::Origin,
                        _provider: #core::runtime::Provider<'a>,
                    ) -> #core::runtime::RuntimeResult<Self> {
                        let name = #type_name::<Self>();
                        #panic("{name} type was not registered. Probably because export has been \
                        disabled for this type.")
                    }

                    #[inline(always)]
                    fn hint() -> #core::runtime::TypeHint {
                        let name = #type_name::<Self>();
                        #panic("{name} type was not registered. Probably because export has been \
                        disabled for this type.");
                    }
                }
            )),
        };

        let downcast_mut = match coercion.downcast_mut {
            false => None,

            true => Some(quote_spanned!(span=>
                #[allow(non_local_definitions)]
                impl<'a> #core::runtime::Downcast<'a> for &'a mut #ty {
                    #[inline(always)]
                    fn downcast(
                        _origin: #core::runtime::Origin,
                        _provider: #core::runtime::Provider<'a>,
                    ) -> #core::runtime::RuntimeResult<Self> {
                        let name = #type_name::<Self>();
                        #panic("{name} type was not registered. Probably because export has been disabled for this type.")
                    }

                    #[inline(always)]
                    fn hint() -> #core::runtime::TypeHint {
                        let name = #type_name::<Self>();
                        #panic("{name} type was not registered. Probably because export has been disabled for this type.")
                    }
                }
            )),
        };

        quote_spanned!(span=>
            #downcast_own
            #downcast_ref
            #downcast_mut
            #upcast_own
            #upcast_ref
            #upcast_mut
        )
    }
}

pub fn new_type(ident: Ident) -> Type {
    let mut segments = Punctuated::new();

    segments.push(PathSegment {
        ident,
        arguments: PathArguments::None,
    });

    Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments,
        },
    })
}

#[derive(Default, Clone, Copy)]
pub struct Coercion {
    pub downcast_own: bool,
    pub downcast_ref: bool,
    pub downcast_mut: bool,
    pub upcast_own: bool,
    pub upcast_ref: bool,
    pub upcast_mut: bool,
}

impl Coercion {
    #[inline]
    pub fn enrich(&mut self, other: Coercion) {
        self.downcast_own = self.downcast_own || other.downcast_own;
        self.downcast_ref = self.downcast_ref || other.downcast_ref;
        self.downcast_mut = self.downcast_mut || other.downcast_mut;
        self.upcast_own = self.upcast_own || other.upcast_own;
        self.upcast_ref = self.upcast_ref || other.upcast_ref;
        self.upcast_mut = self.upcast_mut || other.upcast_mut;
    }
}

pub(super) fn is_str_type(ty: &Type) -> bool {
    if let Type::Path(ty) = ty {
        if ty.qself.is_none() {
            return ty.path.matches_bracketed(&["str"], 0..=0).is_some();
        }
    }

    false
}

pub(super) fn make_param_fn_meta(
    origin: &TokenStream,
    mut ty: &Type,
    downcast: bool,
) -> Option<TokenStream> {
    let meta;

    loop {
        match ty {
            Type::Group(inner) => {
                ty = inner.elem.as_ref();
                continue;
            }

            Type::ImplTrait(inner) => {
                let Some(TypeParamBound::Trait(inner)) = inner.bounds.first() else {
                    return None;
                };

                meta = inner.path.matches_fn()?;
                break;
            }

            Type::TraitObject(inner) => {
                let Some(TypeParamBound::Trait(inner)) = inner.bounds.first() else {
                    return None;
                };

                meta = inner.path.matches_fn()?;
                break;
            }

            Type::Path(inner) => {
                if inner.qself.is_some() {
                    return None;
                }

                if let Some(meta) = inner.path.matches_box() {
                    let Some(arg) = meta.args.first() else {
                        return None;
                    };

                    ty = *arg;
                    continue;
                }

                if let Some(mut path_meta) = inner.path.matches_rust_fn() {
                    let output = path_meta.args.pop()?;

                    meta = FnMeta {
                        span: path_meta.span,
                        inputs: path_meta.args,
                        output: Some(output),
                    };

                    break;
                }

                return None;
            }

            Type::Reference(inner) => {
                ty = inner.elem.as_ref();
                continue;
            }

            _ => return None,
        }
    }

    let span = meta.span;

    let core = span.face_core();
    let intrinsics = span.face_intrinsics();
    let vec_macro = span.face_vec_macro();
    let option = span.face_option();

    let inputs = meta.inputs.into_iter().map(|ty| {
        let span = ty.span();

        let core = span.face_core();
        let option = span.face_option();

        let signature = make_param_fn_meta(origin, ty, !downcast);

        let hint = match signature {
            Some(meta) => quote_spanned!(span=> #core::runtime::TypeHint::Invocation(#meta)),

            None => match downcast {
                true => {
                    Shallow.assert_type_impls_upcast(ty, ty.span());
                    ty.upcast_hint()
                }
                false => {
                    Shallow.assert_type_impls_downcast(ty, ty.span());
                    ty.downcast_hint()
                }
            },
        };

        quote_spanned!(span=> #core::runtime::Param {
            name: #option::None,
            hint: #hint,
        })
    });

    let output = {
        match meta.output {
            Some(ty) => match downcast {
                true => {
                    Shallow.assert_type_impls_downcast(ty, ty.span());
                    ty.downcast_hint()
                }
                false => {
                    Shallow.assert_type_impls_upcast(ty, ty.span());
                    ty.upcast_hint()
                }
            },

            None => quote_spanned!(span=> #core::runtime::TypeHint::nil()),
        }
    };

    Some(quote_spanned!(span=> {
        static META: #intrinsics::Lazy::<#core::runtime::InvocationMeta>
            = #intrinsics::Lazy::<#core::runtime::InvocationMeta>::new(|| {
            #core::runtime::InvocationMeta {
                origin: #origin,
                name: #option::None,
                doc: #option::None,
                receiver: #option::None,
                inputs: #option::Some(#vec_macro[
                    #( #inputs ),*
                ]),
                output: #output,
            }
        });

        &META
    }))
}
