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

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote_spanned};
use syn::{
    spanned::Spanned,
    Error,
    FnArg,
    LitStr,
    Pat,
    PathArguments,
    Result,
    ReturnType,
    Signature,
    Token,
    Type,
};

use crate::utils::{
    morphism::FunctionPolymorphism,
    new_type,
    ty::make_param_fn_meta,
    Coercion,
    Context,
    Facade,
    Group,
    IdRef,
    Operator,
    OperatorOrigin,
    PolymorphicScope,
    Printer,
    Prototype,
    Shallow,
    TypeFamily,
    TypeMeta,
    TypeUtils,
};

pub struct Invocation<'a> {
    ident: &'a Ident,
    receiver: ReceiverMeta,
    arguments: Vec<Argument<'a>>,
    output_span: Span,
}

impl<'a> Invocation<'a> {
    pub fn new(signature: &'a Signature) -> Result<Self> {
        if let Some(abi) = &signature.abi {
            return Err(Error::new(abi.span(), "Cannot register ABI function."));
        }

        if let Some(unsafety) = &signature.unsafety {
            return Err(Error::new(
                unsafety.span(),
                "Cannot register unsafe function.",
            ));
        }

        if let Some(variadic) = &signature.variadic {
            return Err(Error::new(
                variadic.span(),
                "Cannot register variadic function.",
            ));
        }

        if let Some(asyncness) = &signature.asyncness {
            return Err(Error::new(
                asyncness.span(),
                "Cannot register async function.",
            ));
        }

        let mut receiver = ReceiverMeta::None;
        let mut arguments = Vec::with_capacity(signature.inputs.len());

        for arg in &signature.inputs {
            match arg {
                FnArg::Typed(arg) => {
                    let ident = match arg.pat.as_ref() {
                        Pat::Ident(pat) => &pat.ident,

                        _ => {
                            return Err(Error::new(
                                arg.pat.span(),
                                "Cannot introspect unnamed argument.",
                            ));
                        }
                    };

                    let name_ref =
                        Context.make_unique_identifier(ident.to_string().as_str(), ident.span());

                    arguments.push(Argument { ident, name_ref });
                }

                FnArg::Receiver(arg) => {
                    if let Some(colon) = &arg.colon_token {
                        return Err(Error::new(
                            colon.span(),
                            "The introspection system does not support complex receiver types.",
                        ));
                    }

                    if arg.mutability.is_some() {
                        receiver = ReceiverMeta::ByMut
                    } else if arg.reference.is_some() {
                        receiver = ReceiverMeta::ByRef
                    } else {
                        receiver = ReceiverMeta::Owned
                    }
                }
            }
        }

        let output_span = match &signature.output {
            ReturnType::Type(_, ty) => ty.span(),
            ReturnType::Default => signature.ident.span(),
        };

        Ok(Self {
            ident: &signature.ident,
            receiver,
            arguments,
            output_span,
        })
    }

    #[inline(always)]
    pub fn uses_receiver(&self) -> bool {
        self.receiver.is_some()
    }

    pub fn make_function_type<S: PolymorphicScope>(
        &self,
        group: &mut Group,
        polymorphism: &FunctionPolymorphism<'_, S>,
        name: &str,
        name_ref: &IdRef,
        doc: Option<LitStr>,
    ) -> Result<Type> {
        let span = self.ident.span();

        let intrinsics = span.face_intrinsics();
        let core = span.face_core();
        let clone = span.face_clone();
        let option = span.face_option();
        let deref = span.face_deref();

        let function_type = {
            let type_name = Context.make_type_name(name, span);

            let ty = new_type(type_name);

            let partial_eq = quote_spanned!(span=>
                #[allow(non_local_definitions)]
                impl #core::runtime::ops::ScriptPartialEq for #ty {
                    type RHS = #ty;

                    fn script_eq(
                        _origin: #core::runtime::Origin,
                        lhs: #core::runtime::Arg,
                        rhs: #core::runtime::Arg,
                    ) -> #core::runtime::RuntimeResult<bool> {
                        let lhs_ty = #core::runtime::Cell::ty(&lhs.data);
                        let rhs_ty = #core::runtime::Cell::ty(&rhs.data);

                        #core::runtime::RuntimeResult::<bool>::Ok(lhs_ty == rhs_ty)
                    }
                }
            );

            match self.receiver.is_some() {
                false => group.custom(quote_spanned!(span=>
                    #[allow(non_camel_case_types)]
                    struct #ty;

                    impl #clone for #ty {
                        #[inline(always)]
                        fn clone(&self) -> Self {
                            Self
                        }
                    }

                    #partial_eq
                )),

                true => group.custom(quote_spanned!(span=>
                    #[allow(non_camel_case_types)]
                    struct #ty(#core::runtime::Arg);

                    impl #clone for #ty {
                        #[inline(always)]
                        fn clone(&self) -> Self {
                            Self(<#core::runtime::Arg as #clone>::clone(&self.0))
                        }
                    }

                    #partial_eq
                )),
            };

            ty
        };

        let arguments = {
            let mut arguments = Vec::with_capacity(self.arguments.len());

            for arg in &self.arguments {
                let ty = polymorphism
                    .get_arg_type(arg.ident)?
                    .expect("Internal error. Missing polymorphic argument type.");

                arguments.push(TypedArgument {
                    ident: arg.ident,
                    name_ref: &arg.name_ref,
                    ty,
                })
            }

            arguments
        };

        let output = polymorphism
            .get_return_type()?
            .expect("Internal error. Missing polymorphic return type.");

        group.custom(function_type.impl_registered_type());
        group.custom(function_type.impl_coercion(Coercion {
            upcast_own: true,

            ..Coercion::default()
        }));

        let formatted = self.format(&arguments, &output);

        group.type_meta(TypeMeta {
            name: &LitStr::new(formatted.as_str(), span),
            doc: None,
            ty: &function_type,
            family: TypeFamily::Function,
        });

        let mut function_type_prototype = Prototype::for_type(&function_type);

        function_type_prototype.operator(OperatorOrigin::Id(name_ref), Operator::Clone);
        function_type_prototype.operator(OperatorOrigin::Id(name_ref), Operator::PartialEq);
        function_type_prototype.operator(OperatorOrigin::Id(name_ref), Operator::Invocation);

        group.prototype(function_type_prototype);

        let invocation = {
            let self_type = polymorphism.get_self_type()?;
            let trait_type = polymorphism.get_trait_type()?;

            let invoke = {
                let function_generics = polymorphism.signature.make_generics_path();

                self.make_invoke(
                    name_ref,
                    self_type,
                    trait_type,
                    &function_type,
                    function_generics,
                    &arguments,
                    &output,
                )
            };

            let meta = self.make_meta(name_ref, doc, self_type, self.receiver, &arguments, &output);

            quote_spanned!(span=>
                #[allow(non_local_definitions)]
                #[allow(unused_variables)]
                #[allow(non_snake_case)]
                impl #core::runtime::ops::ScriptInvocation for #function_type {
                    fn invoke(
                        origin: #core::runtime::Origin,
                        lhs: #core::runtime::Arg,
                        arguments: &mut [#core::runtime::Arg],
                    ) -> #core::runtime::RuntimeResult<#core::runtime::Cell> {
                        #invoke
                    }

                    fn hint() -> #option<&'static #core::runtime::InvocationMeta> {
                        static META: #intrinsics::Lazy::<#core::runtime::InvocationMeta>
                            = #intrinsics::Lazy::<#core::runtime::InvocationMeta>::new(
                            || #meta
                        );

                        #option::<&'static #core::runtime::InvocationMeta>::Some(
                            <#intrinsics::Lazy::<#core::runtime::InvocationMeta> as #deref>::deref(
                                &META,
                            ),
                        )
                    }
                }
            )
        };

        group.custom(invocation);

        Ok(function_type)
    }

    fn make_invoke(
        &self,
        name_ref: &IdRef,
        self_type: Option<&Type>,
        trait_type: Option<&Type>,
        function_type: &Type,
        function_generics: PathArguments,
        arguments: &[TypedArgument<'_>],
        output: &Type,
    ) -> TokenStream {
        let function_span = self.ident.span();

        let arguments_count = arguments.len();

        let mut arguments_downcast = Vec::with_capacity(arguments_count);
        let mut arguments_list = Vec::with_capacity(arguments_count);

        for (index, arg) in arguments.iter().enumerate() {
            let arg_span = arg.ident.span();
            let arg_type_span = arg.ty.span();

            let core = arg_span.face_core();

            let ident = arg.ident.to_string();
            let ty = &arg.ty;
            let origin = format_ident!("origin_{ident}", span = arg_span);
            let cell = format_ident!("{ident}", span = arg_type_span);
            let provider = format_ident!("provider_{ident}", span = arg_type_span);
            let data = format_ident!("data_{ident}", span = arg_type_span);

            let provider_creation = quote_spanned!(arg_type_span=>
                let #provider = #core::runtime::Provider::Borrowed(&mut #cell);

                let #data = <#ty as #core::runtime::Downcast>::downcast(#origin, #provider)?;
            );

            arguments_downcast.push(quote_spanned!(arg_span=>
                // Safety: Arity checked above.
                let (#origin, mut #cell) = #core::runtime::Arg::split(unsafe {
                    #core::runtime::Arg::take_unchecked(arguments, #index)
                });

                #provider_creation
            ));

            arguments_list.push(data);
        }

        let core = function_span.face_core();

        let arity_check = quote_spanned!(function_span=>
            let arguments_count = arguments.len();

            if arguments_count != #arguments_count {
                return #core::runtime::RuntimeResult::<#core::runtime::Cell>::Err(
                    #core::runtime::RuntimeError::ArityMismatch {
                        invocation_origin: origin,
                        function_origin: #core::runtime::Origin::Rust(#name_ref.origin),
                        parameters: #arguments_count,
                        arguments: arguments_count,
                    },
                );
            }
        );

        let output_span = self.output_span;
        let function = self.ident;
        let receiver;

        let function = match (self_type, trait_type) {
            (Some(self_type), None) => {
                receiver = self.receiver;
                quote_spanned!(function_span=> <#self_type>::#function #function_generics)
            }

            (Some(self_type), Some(trait_type)) => {
                receiver = self.receiver;
                quote_spanned!(function_span=> <#self_type as #trait_type>::#function #function_generics)
            }

            _ => {
                receiver = ReceiverMeta::None;
                quote_spanned!(function_span=> #function #function_generics)
            }
        };

        match receiver {
            ReceiverMeta::None => {
                let application = quote_spanned!(output_span=>
                    let result: #output = #function(#(
                        #arguments_list
                    ),*);
                );

                quote_spanned!(function_span=> {
                    #arity_check

                    #(
                    #arguments_downcast
                    )*

                    #application

                    #core::runtime::Cell::give(origin, result)
                })
            }

            ReceiverMeta::Owned => {
                let self_type = self_type
                    .as_ref()
                    .expect("Internal error. Missing receiver type.");

                Shallow.assert_type_impls_script_type(*self_type, function_span);

                let application = quote_spanned!(output_span=>
                    let result: #output = #function(receiver, #(
                        #arguments_list
                    ),*);
                );

                quote_spanned!(function_span=> {
                    #arity_check

                    let receiver = #core::runtime::Cell::take::<#function_type>(
                        lhs.data,
                        lhs.origin,
                    )?.0;
                    let receiver = #core::runtime::Cell::take::<#self_type>(
                        receiver.data,
                        receiver.origin,
                    )?;

                    #(
                    #arguments_downcast
                    )*

                    #application

                    #core::runtime::Cell::give(origin, result)
                })
            }

            ReceiverMeta::ByRef | ReceiverMeta::ByMut => {
                let self_type = self_type
                    .as_ref()
                    .expect("Internal error. Missing receiver type.");

                Shallow.assert_type_impls_script_type(*self_type, function_span);

                let fn_once = function_span.face_fn_once();

                let application = quote_spanned!(output_span=>
                    let result: #output = #function(receiver, #(
                        #arguments_list
                    ),*);
                );

                let map_function;
                let mut_token;

                match receiver {
                    ReceiverMeta::ByMut => {
                        map_function = quote_spanned!(function_span=> map_mut);
                        mut_token = Some(Token![mut](function_span));
                    }

                    _ => {
                        map_function = quote_spanned!(function_span=> map_ref);
                        mut_token = None;
                    }
                }

                quote_spanned!(function_span=> {
                    #arity_check

                    let receiver = #core::runtime::Cell::take::<#function_type>(
                        lhs.data,
                        lhs.origin,
                    )?.0;

                    #core::runtime::Cell:: #map_function ::<#self_type>(
                        receiver.data,
                        receiver.origin,
                        ({
                            #[inline(always)]
                            fn funnel<
                                __FUNCTION: #fn_once(
                                & #mut_token #self_type,
                            ) -> #core::runtime::RuntimeResult<#output>,
                            >(f: __FUNCTION) -> __FUNCTION {
                                f
                            }

                            funnel
                        })(move |receiver: & #mut_token #self_type| {
                            let origin = origin;
                            let arguments = arguments;

                            #(
                            #arguments_downcast
                            )*

                            #application

                            #core::runtime::RuntimeResult::<#output>::Ok(result)
                        })
                    )
                })
            }
        }
    }

    fn make_meta(
        &self,
        name_ref: &IdRef,
        doc: Option<LitStr>,
        self_type: Option<&Type>,
        receiver: ReceiverMeta,
        arguments: &[TypedArgument<'_>],
        output: &Type,
    ) -> TokenStream {
        let span = self.ident.span();

        let core = span.face_core();
        let option = span.face_option();
        let vec_macro = span.face_vec_macro();

        let receiver = match self_type {
            Some(self_type) if receiver.is_some() => {
                let hint = self_type.downcast_hint();

                Shallow.assert_type_impls_downcast(self_type, span);

                quote_spanned!(span=> #option::Some(#hint))
            }

            _ => quote_spanned!(span=> #option::None),
        };

        let inputs = arguments.iter().map(|arg| {
            let span = arg.ty.span();
            let name_ref = arg.name_ref;

            let signature = make_param_fn_meta(
                &quote_spanned!(span=> #core::runtime::Origin::Rust(#name_ref.origin)),
                &arg.ty,
                true,
            );

            let hint = match signature {
                Some(meta) => quote_spanned!(span=> #core::runtime::TypeHint::Invocation(#meta)),
                None => arg.ty.downcast_hint(),
            };

            Shallow.assert_type_impls_downcast(&arg.ty, span);

            quote_spanned!(span=> #core::runtime::Param {
                name: #option::<#core::runtime::Ident>::Some(
                    #core::runtime::Ident::Rust(&#name_ref),
                ),
                hint: #hint,
            })
        });

        Shallow.assert_type_impls_upcast(output, span);

        let output = output.upcast_hint();

        let doc = match doc {
            None => quote_spanned!(span=> #option::None),
            Some(doc) => quote_spanned!(span=> #option::Some(#doc)),
        };

        quote_spanned!(span=>
            #core::runtime::InvocationMeta {
                origin: #core::runtime::Origin::Rust(#name_ref.origin),
                name: #option::Some(#name_ref.string),
                doc: #doc,
                receiver: #receiver,
                inputs: #option::Some(#vec_macro[
                    #( #inputs ),*
                ]),
                output: #output,
            }
        )
    }

    fn format(&self, arguments: &[TypedArgument<'_>], output: &Type) -> String {
        let mut result = String::with_capacity(50);

        result.push_str("fn(");

        let mut first = true;
        for arg in arguments {
            match first {
                true => first = false,
                false => result.push_str(", "),
            }

            result.push_str(arg.ident.to_string().as_str());
            result.push_str(": ");
            result.push_str(arg.ty.to_display_string().as_str());
        }

        result.push_str(")");

        let output = output.to_display_string();

        if output != "()" {
            result.push_str(" -> ");
            result.push_str(output.as_str());
        }

        result
    }
}

#[derive(Clone, Copy)]
enum ReceiverMeta {
    None,
    Owned,
    ByRef,
    ByMut,
}

impl ReceiverMeta {
    fn is_some(&self) -> bool {
        match &self {
            Self::None => false,
            _ => true,
        }
    }
}

struct Argument<'a> {
    ident: &'a Ident,
    name_ref: IdRef,
}

struct TypedArgument<'a> {
    ident: &'a Ident,
    name_ref: &'a IdRef,
    ty: Type,
}
