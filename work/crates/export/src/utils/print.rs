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

use std::fmt::{Display, Formatter, Result};

use paste::paste;
use quote::ToTokens;
use syn::{spanned::Spanned, *};

pub trait Printer: Spanned {
    fn to_display_string(&self) -> String;

    #[inline(always)]
    fn to_display_literal(&self) -> LitStr {
        let span = self.span();

        LitStr::new(self.to_display_string().as_str(), span)
    }
}

macro_rules! printer {
    ($ty: ty, $formatter: ident, $self: ident, $body: block) => {
        paste! {
            #[repr(transparent)]
            pub struct [<$ty Printer>]<'a>(pub &'a $ty);

            impl<'a> Display for [<$ty Printer>]<'a> {
                fn fmt(&self, $formatter: &mut Formatter<'_>) -> Result {
                    let $self = self;

                    $body
                }
            }

            impl<'a> Printer for $ty {
                #[inline(always)]
                fn to_display_string(&self) -> String {
                    [<$ty Printer>](self).to_string()
                }
            }
        }
    };
}

printer!(Type, formatter, this, {
    match this.0 {
        Type::Array(ty) => Display::fmt(&TypeArrayPrinter(ty), formatter),

        Type::BareFn(ty) => Display::fmt(&TypeBareFnPrinter(ty), formatter),

        Type::Group(ty) => Display::fmt(&TypeGroupPrinter(ty), formatter),

        Type::ImplTrait(ty) => Display::fmt(&TypeImplTraitPrinter(ty), formatter),

        Type::Infer(_) => formatter.write_str("_"),

        Type::Macro(ty) => Display::fmt(&MacroPrinter(&ty.mac), formatter),

        Type::Never(_) => formatter.write_str("!"),

        Type::Paren(ty) => {
            formatter.write_str("(")?;
            Display::fmt(&TypePrinter(&ty.elem), formatter)?;
            formatter.write_str(")")
        }

        Type::Path(ty) => Display::fmt(&TypePathPrinter(ty), formatter),

        Type::Ptr(ty) => Display::fmt(&TypePtrPrinter(ty), formatter),

        Type::Reference(ty) => Display::fmt(&TypeReferencePrinter(ty), formatter),

        Type::Slice(ty) => {
            formatter.write_str("[")?;
            Display::fmt(&TypePrinter(&ty.elem), formatter)?;
            formatter.write_str("]")
        }

        Type::TraitObject(ty) => Display::fmt(&TypeTraitObjectPrinter(ty), formatter),

        Type::Tuple(ty) => Display::fmt(&TypeTuplePrinter(ty), formatter),

        _ => Display::fmt(&this.0.to_token_stream(), formatter),
    }
});

printer!(TypeArray, formatter, this, {
    formatter.write_str("[")?;

    Display::fmt(&TypePrinter(this.0.elem.as_ref()), formatter)?;

    formatter.write_str("; ")?;

    Display::fmt(&ExprPrinter(&this.0.len), formatter)?;

    formatter.write_str("]")?;

    Ok(())
});

printer!(TypeBareFn, formatter, this, {
    if let Some(bound_lifetimes) = &this.0.lifetimes {
        Display::fmt(&BoundLifetimesPrinter(bound_lifetimes), formatter)?;

        formatter.write_str(" ")?;
    }

    if this.0.unsafety.is_some() {
        formatter.write_str("unsafe ")?;
    }

    if let Some(abi) = &this.0.abi {
        formatter.write_str("extern")?;

        if let Some(name) = &abi.name {
            formatter.write_str(&format!("{:?} ", name.value()))?;
        }

        formatter.write_str(" ")?;
    }

    formatter.write_str("fn(")?;

    let mut first = true;

    for argument in &this.0.inputs {
        match first {
            true => first = false,
            false => formatter.write_str(", ")?,
        }

        if let Some((name, _)) = &argument.name {
            formatter.write_str(&format!("{}: ", name.to_string()))?;
        }

        Display::fmt(&TypePrinter(&argument.ty), formatter)?;
    }

    if this.0.variadic.is_some() {
        if !first {
            formatter.write_str(", ")?;
        }

        formatter.write_str("...")?;
    }

    formatter.write_str(")")?;

    Display::fmt(&ReturnTypePrinter(&this.0.output), formatter)?;

    Ok(())
});

printer!(ReturnType, formatter, this, {
    if let ReturnType::Type(_, ty) = &this.0 {
        formatter.write_str(" -> ")?;
        Display::fmt(&TypePrinter(ty.as_ref()), formatter)?;
    }

    Ok(())
});

printer!(BoundLifetimes, formatter, this, {
    formatter.write_str("for<")?;

    let mut first = true;

    for param in &this.0.lifetimes {
        match first {
            true => first = false,
            false => formatter.write_str(", ")?,
        }

        Display::fmt(&GenericParamPrinter(param), formatter)?;
    }

    formatter.write_str(">")?;

    Ok(())
});

printer!(GenericParam, formatter, this, {
    match &this.0 {
        GenericParam::Lifetime(param) => {
            Display::fmt(&LifetimePrinter(&param.lifetime), formatter)?;

            if param.colon_token.is_some() {
                formatter.write_str(": ")?;
            }

            for bound in &param.bounds {
                Display::fmt(&LifetimePrinter(&bound), formatter)?;
            }
        }

        GenericParam::Type(param) => {
            formatter.write_str(param.ident.to_string().as_str())?;

            if param.colon_token.is_some() {
                formatter.write_str(": ")?;
            }

            for bound in &param.bounds {
                Display::fmt(&TypeParamBoundPrinter(bound), formatter)?
            }

            if param.eq_token.is_some() {
                formatter.write_str(" = ")?;
            }

            if let Some(default) = &param.default {
                Display::fmt(&TypePrinter(default), formatter)?
            }
        }

        GenericParam::Const(param) => {
            formatter.write_str("const ")?;
            formatter.write_str(param.ident.to_string().as_str())?;
            formatter.write_str(": ")?;
            Display::fmt(&TypePrinter(&param.ty), formatter)?;

            if param.eq_token.is_some() {
                formatter.write_str(" = ")?;
            }

            if let Some(default) = &param.default {
                Display::fmt(&ExprPrinter(default), formatter)?
            }
        }
    }

    Ok(())
});

printer!(Lifetime, formatter, this, {
    formatter.write_str(&format!("{}", this.0))
});

printer!(TypeGroup, formatter, this, {
    formatter.write_str("(")?;
    Display::fmt(&TypePrinter(&this.0.elem), formatter)?;
    formatter.write_str(")")?;

    Ok(())
});

printer!(TypeImplTrait, formatter, this, {
    formatter.write_str("impl ")?;

    let mut first = true;

    for bound in &this.0.bounds {
        match first {
            true => first = false,
            false => formatter.write_str(" + ")?,
        }

        Display::fmt(&TypeParamBoundPrinter(bound), formatter)?;
    }

    Ok(())
});

printer!(TypeParamBound, formatter, this, {
    match this.0 {
        TypeParamBound::Trait(bound) => Display::fmt(&TraitBoundPrinter(bound), formatter),
        TypeParamBound::Lifetime(bound) => Display::fmt(&LifetimePrinter(bound), formatter),
        TypeParamBound::Verbatim(bound) => Display::fmt(bound, formatter),
        _ => Display::fmt(&this.0.to_token_stream(), formatter),
    }
});

printer!(TraitBound, formatter, this, {
    if this.0.paren_token.is_some() {
        formatter.write_str("(")?;
    }

    if let TraitBoundModifier::Maybe(_) = &this.0.modifier {
        formatter.write_str("?")?;
    }

    if let Some(bounds) = &this.0.lifetimes {
        Display::fmt(&BoundLifetimesPrinter(bounds), formatter)?;
        formatter.write_str(" ")?;
    }

    Display::fmt(&PathPrinter(&this.0.path), formatter)?;

    if this.0.paren_token.is_some() {
        formatter.write_str(")")?;
    }

    Ok(())
});

printer!(Path, formatter, this, {
    let mut first = this.0.leading_colon.is_none();

    for segment in &this.0.segments {
        match first {
            true => first = false,
            false => formatter.write_str("::")?,
        }

        Display::fmt(&PathSegmentPrinter(segment), formatter)?;
    }

    Ok(())
});

printer!(PathSegment, formatter, this, {
    formatter.write_str(&this.0.ident.to_string())?;

    match &this.0.arguments {
        PathArguments::None => (),

        PathArguments::AngleBracketed(arguments) => {
            Display::fmt(&AngleBracketedGenericArgumentsPrinter(arguments), formatter)?;
        }

        PathArguments::Parenthesized(arguments) => {
            formatter.write_str("(")?;

            let mut first = true;

            for input in &arguments.inputs {
                match first {
                    true => first = false,
                    false => formatter.write_str(", ")?,
                }

                Display::fmt(&TypePrinter(input), formatter)?;
            }

            formatter.write_str(")")?;

            Display::fmt(&ReturnTypePrinter(&arguments.output), formatter)?;
        }
    }

    Ok(())
});

printer!(AngleBracketedGenericArguments, formatter, this, {
    if this.0.colon2_token.is_some() {
        formatter.write_str("::")?;
    }

    formatter.write_str("<")?;

    let mut first = true;

    for arg in &this.0.args {
        match first {
            true => first = false,
            false => formatter.write_str(", ")?,
        }

        Display::fmt(&GenericArgumentPrinter(arg), formatter)?;
    }

    formatter.write_str(">")?;

    Ok(())
});

printer!(GenericArgument, formatter, this, {
    match this.0 {
        GenericArgument::Lifetime(arg) => Display::fmt(&LifetimePrinter(arg), formatter),
        GenericArgument::Type(arg) => Display::fmt(&TypePrinter(arg), formatter),
        GenericArgument::Const(arg) => Display::fmt(&ExprPrinter(arg), formatter),
        GenericArgument::AssocType(arg) => Display::fmt(&AssocTypePrinter(arg), formatter),
        GenericArgument::AssocConst(arg) => Display::fmt(&AssocConstPrinter(arg), formatter),
        GenericArgument::Constraint(arg) => Display::fmt(&ConstraintPrinter(arg), formatter),
        _ => Display::fmt(&this.0.to_token_stream(), formatter),
    }
});

printer!(AssocType, formatter, this, {
    Display::fmt(&this.0.ident, formatter)?;

    if let Some(generics) = &this.0.generics {
        Display::fmt(&AngleBracketedGenericArgumentsPrinter(generics), formatter)?;
    }

    formatter.write_str(" = ")?;

    Display::fmt(&TypePrinter(&this.0.ty), formatter)?;

    Ok(())
});

printer!(AssocConst, formatter, this, {
    Display::fmt(&this.0.ident, formatter)?;

    if let Some(generics) = &this.0.generics {
        Display::fmt(&AngleBracketedGenericArgumentsPrinter(generics), formatter)?;
    }

    formatter.write_str(" = ")?;

    Display::fmt(&ExprPrinter(&this.0.value), formatter)?;

    Ok(())
});

printer!(Constraint, formatter, this, {
    Display::fmt(&this.0.ident, formatter)?;
    formatter.write_str(": ")?;

    let mut first = true;

    for bound in &this.0.bounds {
        match first {
            true => first = false,
            false => formatter.write_str(" + ")?,
        }

        Display::fmt(&TypeParamBoundPrinter(bound), formatter)?;
    }

    Ok(())
});

printer!(Macro, formatter, this, {
    Display::fmt(&PathPrinter(&this.0.path), formatter)?;

    match this.0.delimiter {
        MacroDelimiter::Paren(_) => formatter.write_str("!(")?,
        MacroDelimiter::Brace(_) => formatter.write_str("!{")?,
        MacroDelimiter::Bracket(_) => formatter.write_str("![")?,
    }

    formatter.write_str(&this.0.tokens.to_string())?;

    match this.0.delimiter {
        MacroDelimiter::Paren(_) => formatter.write_str(")")?,
        MacroDelimiter::Brace(_) => formatter.write_str("}")?,
        MacroDelimiter::Bracket(_) => formatter.write_str("]")?,
    }

    Ok(())
});

printer!(TypePath, formatter, this, {
    match &this.0.qself {
        None => Display::fmt(&PathPrinter(&this.0.path), formatter)?,

        Some(qself) => {
            formatter.write_str("<")?;
            Display::fmt(&TypePrinter(qself.ty.as_ref()), formatter)?;

            if qself.as_token.is_some() {
                formatter.write_str(" as ")?;
            }

            let mut first = this.0.path.leading_colon.is_none();

            for (index, segment) in this.0.path.segments.iter().enumerate() {
                if index == qself.position {
                    formatter.write_str(">")?;
                }

                match first {
                    true => first = false,
                    false => formatter.write_str("::")?,
                }

                Display::fmt(&PathSegmentPrinter(segment), formatter)?;
            }
        }
    }

    Ok(())
});

printer!(TypePtr, formatter, this, {
    if this.0.const_token.is_some() {
        formatter.write_str("*const ")?;
    }

    if this.0.mutability.is_some() {
        formatter.write_str("*mut ")?;
    }

    Display::fmt(&TypePrinter(this.0.elem.as_ref()), formatter)?;

    Ok(())
});

printer!(TypeReference, formatter, this, {
    formatter.write_str("&")?;

    if let Some(lifetime) = &this.0.lifetime {
        Display::fmt(&LifetimePrinter(lifetime), formatter)?;
        formatter.write_str(" ")?;
    }

    if this.0.mutability.is_some() {
        formatter.write_str("mut ")?;
    }

    Display::fmt(&TypePrinter(this.0.elem.as_ref()), formatter)?;

    Ok(())
});

printer!(TypeTraitObject, formatter, this, {
    formatter.write_str("dyn ")?;

    let mut first = true;

    for bound in &this.0.bounds {
        match first {
            true => first = false,
            false => formatter.write_str(" + ")?,
        }

        Display::fmt(&TypeParamBoundPrinter(bound), formatter)?;
    }

    Ok(())
});

printer!(TypeTuple, formatter, this, {
    formatter.write_str("(")?;

    let mut first = true;

    for elem in &this.0.elems {
        match first {
            true => first = false,
            false => formatter.write_str(", ")?,
        }

        Display::fmt(&TypePrinter(elem), formatter)?;
    }

    formatter.write_str(")")?;

    Ok(())
});

printer!(Expr, formatter, this, {
    Display::fmt(&this.0.to_token_stream(), formatter)
});
