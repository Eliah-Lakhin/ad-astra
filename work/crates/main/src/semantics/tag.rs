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

use std::mem::swap;

use lady_deirdre::syntax::NodeRef;

use crate::{
    exports::Struct,
    runtime::{ops::DynamicType, InvocationMeta, ScriptType, TypeFamily, TypeHint, TypeMeta},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tag {
    Unset,
    Type(&'static TypeMeta),
    Family(&'static TypeFamily),
    Struct(NodeRef),
    Fn((NodeRef, usize)),
    Invocation(&'static InvocationMeta),
}

impl Default for Tag {
    #[inline(always)]
    fn default() -> Self {
        Self::Unset
    }
}

impl From<TypeHint> for Tag {
    #[inline(always)]
    fn from(value: TypeHint) -> Self {
        match value {
            TypeHint::Type(meta) => {
                if let Some(meta) = meta.prototype().hint_invocation() {
                    return Self::Invocation(meta);
                }

                Self::Type(meta)
            }

            TypeHint::Family(meta) => Self::Family(meta),

            TypeHint::Invocation(meta) => Self::Invocation(meta),
        }
    }
}

impl Tag {
    #[inline(always)]
    pub(crate) fn nil() -> Self {
        Self::Type(TypeMeta::nil())
    }

    #[inline(always)]
    pub(crate) fn dynamic() -> Self {
        Self::Type(DynamicType::type_meta())
    }

    #[inline(always)]
    pub(crate) fn type_hint(&self) -> TypeHint {
        match self {
            Self::Unset => TypeHint::dynamic(),

            Self::Type(this) => TypeHint::from(*this),

            Self::Family(this) => TypeHint::from(*this),

            Self::Struct(_) => TypeHint::from(<Struct>::type_meta().family()),

            Self::Fn((_, arity)) => match TypeMeta::script_fn(*arity) {
                Some(ty) => TypeHint::from(ty),
                None => TypeHint::from(TypeFamily::fn_family()),
            },

            Self::Invocation(this) => TypeHint::from(*this),
        }
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub(crate) fn type_source(&self) -> Option<&NodeRef> {
        match self {
            Self::Unset | Self::Type(_) | Self::Family(_) | Self::Invocation(_) => None,
            Self::Struct(source) | Self::Fn((source, _)) => Some(source),
        }
    }

    pub(crate) fn type_meta(&self) -> Option<&'static TypeMeta> {
        match self {
            Self::Unset => None,

            Self::Type(this) => Some(this),

            Self::Family(_) => None,

            Self::Struct(_) => Some(<Struct>::type_meta()),

            Self::Fn((_, arity)) => TypeMeta::script_fn(*arity),

            Self::Invocation(meta) => {
                let Some(arity) = meta.arity() else {
                    return None;
                };

                TypeMeta::script_fn(arity)
            }
        }
    }

    #[inline(always)]
    pub(crate) fn is_dynamic(&self) -> bool {
        if let Self::Type(ty) = self {
            return ty.is_dynamic();
        }

        false
    }

    pub(crate) fn type_family(&self) -> &'static TypeFamily {
        match self {
            Self::Unset => TypeFamily::dynamic(),

            Self::Type(this) => this.family(),

            Self::Family(this) => this,

            Self::Struct(_) => <Struct>::type_meta().family(),

            Self::Fn(_) => TypeFamily::fn_family(),

            Self::Invocation(_) => TypeFamily::fn_family(),
        }
    }

    pub(crate) fn invocation_meta(&self) -> Option<&'static InvocationMeta> {
        match self {
            Self::Type(this) => this.prototype().hint_invocation(),

            Self::Fn((_, arity)) => TypeMeta::script_fn(*arity)
                .map(|meta| meta.prototype().hint_invocation())
                .flatten(),

            Self::Invocation(meta) => Some(*meta),

            _ => None,
        }
    }

    pub(crate) fn merge(&mut self, mut other: Self) {
        match (&self, other) {
            (Self::Unset, other) => *self = other,

            (Self::Type(this), Self::Type(other)) => {
                if this != &other {
                    let this_family = this.family();

                    *self = match this_family == other.family() {
                        true => Self::Family(this_family),
                        false => Self::dynamic(),
                    };
                }
            }

            (Self::Type(this), Self::Family(other)) => {
                let this_family = this.family();

                *self = match this_family == other {
                    true => Self::Family(this_family),
                    false => Self::dynamic(),
                }
            }

            (Self::Type(this), Self::Struct(_)) => {
                if this == &<Struct>::type_meta() {
                    return;
                }

                let this_family = this.family();

                if this_family == <Struct>::type_meta().family() {
                    *self = Self::Family(this_family);
                    return;
                }

                *self = Self::dynamic();
            }

            (Self::Type(this), Self::Fn((_, arity))) => {
                if Some(*this) == TypeMeta::script_fn(arity) {
                    return;
                }

                let this_family = this.family();

                if this_family == TypeFamily::fn_family() {
                    *self = Self::Family(this_family);
                    return;
                }

                *self = Self::dynamic();
            }

            (Self::Type(this), Self::Invocation(other)) => {
                if let Some(other_params) = &other.inputs {
                    if let Some(other) = TypeMeta::script_fn(other_params.len()) {
                        if this == &other {
                            return;
                        }

                        let this_family = this.family();
                        let other_family = other.family();

                        *self = match this_family == other_family {
                            true => Self::Family(this_family),
                            false => Self::dynamic(),
                        };

                        return;
                    };
                }

                let this_family = this.family();

                *self = match this_family == TypeFamily::fn_family() {
                    true => Self::Family(this_family),
                    false => Self::dynamic(),
                }
            }

            (Self::Family(this), Self::Family(other)) => {
                if this != &other {
                    *self = Self::dynamic();
                }
            }

            (Self::Family(this), Self::Struct(_)) => {
                if this != &<Struct>::type_meta().family() {
                    *self = Self::dynamic();
                }
            }

            (Self::Family(this), Self::Fn(_)) => {
                if !this.is_fn() {
                    *self = Self::dynamic();
                }
            }

            (Self::Family(this), Self::Invocation(_)) => {
                if !this.is_fn() {
                    *self = Self::dynamic();
                }
            }

            (Self::Struct(this), Self::Struct(other)) => {
                if this != &other {
                    *self = Self::Type(<Struct>::type_meta());
                }
            }

            (Self::Struct(_), Self::Fn(_)) => *self = Self::dynamic(),

            (Self::Struct(_), Self::Invocation(_)) => *self = Self::dynamic(),

            (Self::Fn(this), Self::Fn(other)) => {
                if this == &other {
                    return;
                }

                *self = match TypeMeta::script_fn(this.1) {
                    Some(ty) if this.1 == other.1 => Tag::Type(ty),
                    _ => Tag::Family(TypeFamily::fn_family()),
                }
            }

            (Self::Fn(this), Self::Invocation(other)) => {
                let other_arity = other.arity();

                *self = match TypeMeta::script_fn(this.1) {
                    Some(ty) if Some(this.1) == other_arity => Tag::Type(ty),
                    _ => Tag::Family(TypeFamily::fn_family()),
                }
            }

            (Self::Invocation(this), Self::Invocation(other)) => {
                if this == &other {
                    return;
                }

                if let (Some(this_arity), Some(other_arity)) = (this.arity(), other.arity()) {
                    if this_arity == other_arity {
                        if let Some(script_fn_ty) = TypeMeta::script_fn(this_arity) {
                            *self = Self::Type(script_fn_ty);
                        }
                    }
                }

                *self = Self::Family(TypeFamily::fn_family());
            }

            _ => {
                swap(self, &mut other);

                self.merge(other);
            }
        }
    }
}
