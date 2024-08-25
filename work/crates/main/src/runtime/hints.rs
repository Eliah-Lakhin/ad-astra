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

use std::{
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    hash::Hash,
};

use crate::runtime::{InvocationMeta, PackageMeta, RustIdent, TypeFamily, TypeMeta};

/// An extended type metadata that can be a [TypeMeta], [TypeFamily], or
/// [InvocationMeta].
///
/// The purpose of this object is to provide as precise type metadata as
/// possible for preliminary static semantic analysis of the Script source code.
///
/// For example, if the analyzed object is a function, [InvocationMeta] is more
/// precise than [TypeMeta] because it provides additional information about the
/// function's parameter signature and return type.
///
/// When the object may potentially represent a set of distinct types within the
/// same family (e.g., `usize` and `f64` are both numeric types), the TypeHint
/// would be a [TypeFamily].
///
/// If the type cannot be statically determined, the TypeHint is
/// [dynamic](TypeHint::dynamic).
///
/// The [Display] implementation of this object attempts to print the type's
/// signature in a way that is more user-friendly.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TypeHint {
    Type(&'static TypeMeta),
    Family(&'static TypeFamily),
    Invocation(&'static InvocationMeta),
}

impl Display for TypeHint {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Type(meta) => {
                if formatter.alternate() || meta.is_fn() {
                    return Display::fmt(meta, formatter);
                }

                Display::fmt(meta.family(), formatter)
            }

            Self::Family(meta) => Display::fmt(meta, formatter),

            Self::Invocation(meta) => Display::fmt(meta, formatter),
        }
    }
}

impl From<&'static TypeMeta> for TypeHint {
    #[inline(always)]
    fn from(value: &'static TypeMeta) -> Self {
        Self::Type(value)
    }
}

impl From<&'static TypeFamily> for TypeHint {
    #[inline(always)]
    fn from(value: &'static TypeFamily) -> Self {
        Self::Family(value)
    }
}

impl From<&'static InvocationMeta> for TypeHint {
    #[inline(always)]
    fn from(value: &'static InvocationMeta) -> Self {
        Self::Invocation(value)
    }
}

impl TypeHint {
    /// Returns a [TypeHint] for the [unit] `()` type.
    #[inline(always)]
    pub fn nil() -> Self {
        Self::Type(TypeMeta::nil())
    }

    /// Returns a [TypeHint] for a type that cannot be determined statically.
    #[inline(always)]
    pub fn dynamic() -> Self {
        Self::Type(TypeMeta::dynamic())
    }

    /// Returns true if the underlying type is the [unit] `()` type.
    #[inline(always)]
    pub fn is_nil(&self) -> bool {
        self.type_family().is_nil()
    }

    /// Returns true if the underlying type cannot be determined statically.
    #[inline(always)]
    pub fn is_dynamic(&self) -> bool {
        self.type_family().is_dynamic()
    }

    /// Returns true if the underlying type is a function, belongs to a family
    /// of functions, or is an object that supports
    /// [invocation operator](crate::runtime::Prototype::implements_invocation).
    #[inline(always)]
    pub fn is_fn(&self) -> bool {
        match self {
            Self::Type(meta) => meta.is_fn(),
            Self::Family(meta) => meta.is_fn(),
            Self::Invocation(_) => true,
        }
    }

    /// Returns true if the underlying type represents a Script package
    /// (i.e., a Rust struct that has been exported with `#[export(package)]`).
    #[inline(always)]
    pub fn is_package(&self) -> bool {
        match self {
            Self::Type(meta) => meta.family().is_package(),
            Self::Family(meta) => meta.is_package(),
            Self::Invocation(_) => false,
        }
    }

    /// Returns true if the underlying type represents a number (e.g., `u8`,
    /// `isize`, `f32`, etc.) or belongs to a family of numeric types.
    #[inline(always)]
    pub fn is_number(&self) -> bool {
        match self {
            Self::Type(meta) => meta.family().is_number(),
            Self::Family(meta) => meta.is_number(),
            Self::Invocation(_) => false,
        }
    }

    /// Attempts to provide a [TypeMeta] for the underlying type. The function
    /// returns None if a single type cannot be determined, for instance,
    /// if the TypeInfo describes a family of types.
    #[inline(always)]
    pub fn type_meta(&self) -> Option<&'static TypeMeta> {
        match self {
            Self::Type(meta) => Some(*meta),
            Self::Family(_) => None,
            Self::Invocation(meta) => {
                let arity = meta.arity()?;

                TypeMeta::script_fn(arity)
            }
        }
    }

    /// Returns the family of types to which this type belongs.
    ///
    /// This function is infallible because every Script type belongs to some
    /// family (even if the family consists of just one type).
    #[inline(always)]
    pub fn type_family(&self) -> &'static TypeFamily {
        match self {
            Self::Type(meta) => meta.family(),
            Self::Family(meta) => *meta,
            Self::Invocation(_) => TypeFamily::fn_family(),
        }
    }

    /// If the underlying type supports the
    /// [invocation operator](crate::runtime::Prototype::implements_invocation),
    /// returns metadata for this invocation.
    #[inline(always)]
    pub fn invocation(&self) -> Option<&'static InvocationMeta> {
        match self {
            Self::Type(meta) => meta.prototype().hint_invocation(),
            Self::Family(_) => None,
            Self::Invocation(meta) => Some(meta),
        }
    }

    /// Returns the package metadata of the crate from which this type was
    /// exported.
    ///
    /// Formally, exported Rust types do not belong to Script packages. The type
    /// could be exported from a crate that does not have a Script package at
    /// all. Therefore, the result of this function is a best-effort estimation.
    #[inline(always)]
    pub fn package(&self) -> Option<&'static PackageMeta> {
        match self {
            Self::Type(meta) => meta.origin().package(),
            Self::Family(_) => None,
            Self::Invocation(meta) => meta.origin.package(),
        }
    }

    /// Attempts to return the relevant RustDoc documentation for the referred
    /// type.
    #[inline(always)]
    pub fn doc(&self) -> Option<&'static str> {
        match self {
            Self::Type(meta) => {
                let family = meta.family();

                if family.len() > 1 && !family.is_package() {
                    return family.doc();
                }

                meta.doc()
            }

            Self::Family(meta) => meta.doc(),

            Self::Invocation(meta) => meta.doc,
        }
    }
}

/// A description of a statically known field of the type: `object.field`.
///
/// The field could be an object of a particular type or an object's method
/// available for invocation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ComponentHint {
    /// The name of the field.
    pub name: &'static RustIdent,

    /// The type of the field.
    ///
    /// In the case of methods, the TypeHint is usually [TypeHint::Invocation],
    /// which describes the invocation signature of the method.
    pub ty: TypeHint,

    /// The relevant RustDoc documentation for the field.
    ///
    /// This documentation pertains to the field itself and is (usually)
    /// different from the documentation found in the [TypeHint::doc] of the
    /// field's type description.
    ///
    /// ```no_run
    /// use ad_astra::export;
    ///
    /// #[export]
    /// struct Foo {
    ///     /// A ComponentHint's documentation.
    ///     pub field: usize,
    /// }
    ///
    /// #[export]
    /// impl Foo {
    ///     /// A ComponentHint's documentation.
    ///     pub fn method(&self) {}
    /// }
    /// ```
    pub doc: Option<&'static str>,
}

impl Display for ComponentHint {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name.as_ref())?;

        if !self.ty.is_dynamic() {
            formatter.write_str(": ")?;
            Display::fmt(&self.ty, formatter)?;
        }

        Ok(())
    }
}

impl PartialOrd for ComponentHint {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ComponentHint {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}
