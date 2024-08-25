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
    hash::{Hash, Hasher},
};

use compact_str::CompactString;
use lady_deirdre::lexis::{TokenRef, NIL_TOKEN_REF};

use crate::runtime::{Origin, RustOrigin};

/// An identifier within the Rust or Script code: `let identifier = 10;`.
///
/// You can retrieve the actual string of the identifier using the [Display],
/// [Debug], and [AsRef<str>](AsRef) implementations of this type.
///
/// Unlike [Origin] and [ScriptOrigin](crate::runtime::ScriptOrigin), the
/// identifier's string does not become obsolete or change even if the
/// underlying Script source code has been modified. This object holds a copy of
/// the string as it was at the time of the identifier's creation.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ident {
    /// An identifier declared in the Rust source code.
    Rust(&'static RustIdent),

    /// An identifier declared in the Script source code.
    Script(ScriptIdent),
}

impl Debug for Ident {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Ident::Rust(ident) => Debug::fmt(*ident, formatter),
            Ident::Script(ident) => Debug::fmt(ident, formatter),
        }
    }
}

impl Display for Ident {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Ident::Rust(ident) => Display::fmt(*ident, formatter),
            Ident::Script(ident) => Display::fmt(ident, formatter),
        }
    }
}

impl From<&'static RustIdent> for Ident {
    #[inline(always)]
    fn from(value: &'static RustIdent) -> Self {
        Self::Rust(value)
    }
}

impl From<ScriptIdent> for Ident {
    #[inline(always)]
    fn from(value: ScriptIdent) -> Self {
        Self::Script(value)
    }
}

impl AsRef<str> for Ident {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        match self {
            Self::Rust(ident) => ident.as_ref(),
            Self::Script(ident) => ident.as_ref(),
        }
    }
}

impl Ident {
    #[inline(always)]
    pub(crate) fn from_string(string: impl Into<CompactString>) -> Self {
        Self::Script(ScriptIdent {
            origin: NIL_TOKEN_REF,
            string: string.into(),
        })
    }

    /// Returns the range in the Rust or Script source code that spans the
    /// underlying identifier.
    #[inline(always)]
    pub fn origin(&self) -> Origin {
        match self {
            Self::Rust(ident) => Origin::from(ident.origin),
            Self::Script(ident) => Origin::from(ident.origin),
        }
    }
}

/// An identifier within the Rust code: `let identifier = 10;`.
///
/// This object is typically created in static memory
/// (`static IDENT: RustIdent = RustIdent { ... }`) using the
/// [export](crate::export) macro. Generally, you don't need to create it
/// manually.
#[derive(Clone, Copy)]
pub struct RustIdent {
    /// The range in the Rust source code that points to the underlying
    /// identifier.
    pub origin: &'static RustOrigin,

    /// The string representation of the identifier.
    pub string: &'static str,
}

impl Debug for RustIdent {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.string, formatter)
    }
}

impl Display for RustIdent {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.string, formatter)
    }
}

impl AsRef<str> for RustIdent {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self.string
    }
}

impl PartialEq for RustIdent {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.string.eq(other.string)
    }
}

impl Eq for RustIdent {}

impl Hash for RustIdent {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.string.hash(state)
    }
}

impl PartialOrd for RustIdent {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RustIdent {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.string.cmp(&other.string)
    }
}

/// An identifier within the Script code: `let identifier = 10;`.
///
/// You cannot instantiate this object manually, but certain API functions
/// return instances of it (e.g.,
/// [VarSymbol::var_name](crate::analysis::symbols::VarSymbol::var_name)).
///
/// You can retrieve the actual string of the identifier using the [Display],
/// [Debug], and [AsRef<str>](AsRef) implementations of this type.
///
/// The `ScriptIdent` holds a copy of the identifier's string from the time
/// of its creation. Therefore, the string does not become obsolete or change
/// even if the source code of the Script is modified.
#[derive(Clone)]
pub struct ScriptIdent {
    origin: TokenRef,
    string: CompactString,
}

impl Debug for ScriptIdent {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.string, formatter)
    }
}

impl Display for ScriptIdent {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.string, formatter)
    }
}

impl AsRef<str> for ScriptIdent {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self.string.as_ref()
    }
}

impl PartialEq for ScriptIdent {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.string.eq(&other.string)
    }
}

impl Eq for ScriptIdent {}

impl Hash for ScriptIdent {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.string.hash(state)
    }
}

impl PartialOrd for ScriptIdent {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScriptIdent {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.string.cmp(&other.string)
    }
}

impl ScriptIdent {
    #[inline(always)]
    pub(crate) fn from_string(token_ref: TokenRef, string: impl Into<CompactString>) -> Self {
        Self {
            origin: token_ref,
            string: string.into(),
        }
    }
}
