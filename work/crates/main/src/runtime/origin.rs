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
    ops::{Bound, Range, RangeFrom, RangeInclusive},
};

use compact_str::CompactString;
use lady_deirdre::{
    arena::{Entry, Id, Identifiable},
    lexis::{
        Column,
        Line,
        Position,
        PositionSpan,
        SiteRef,
        SiteRefSpan,
        SiteSpan,
        SourceCode,
        ToSpan,
        TokenRef,
    },
    syntax::PolyRef,
};

use crate::runtime::{Ident, PackageMeta, ScriptIdent};

/// A representation of a Rust or Script source code range.
///
/// The primary purpose of this object is to track data flow points (both
/// Rust and Script points) during script evaluation.
///
/// Typically, you don't need to create this object manually. You receive
/// instances of Origin in runtime trait functions and pass them to the
/// corresponding runtime API functions.
///
/// This Origin object helps the Script Engine generate descriptive
/// [runtime errors](crate::runtime::RuntimeError) if an error occurs during
/// script evaluation. Additionally, its [ScriptOrigin] variant is widely used
/// in the static script code analysis API to represent script source code
/// spans.
///
/// For debugging purposes, you can instantiate the Origin object as
/// [Origin::nil], which intentionally does not point to any source code.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Origin {
    /// A range representing a portion of Rust source code.
    Rust(&'static RustOrigin),

    /// A range representing a portion of Script source code.
    Script(ScriptOrigin),
}

impl Default for Origin {
    #[inline(always)]
    fn default() -> Self {
        Self::nil()
    }
}

impl Debug for Origin {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust(origin) => Debug::fmt(origin, formatter),
            Self::Script(origin) => Debug::fmt(origin, formatter),
        }
    }
}

impl From<&'static RustOrigin> for Origin {
    #[inline(always)]
    fn from(value: &'static RustOrigin) -> Self {
        Self::Rust(value)
    }
}

impl From<ScriptOrigin> for Origin {
    #[inline(always)]
    fn from(value: ScriptOrigin) -> Self {
        Self::Script(value)
    }
}

impl From<SiteRefSpan> for Origin {
    #[inline(always)]
    fn from(value: SiteRefSpan) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl<'a> From<&'a SiteRefSpan> for Origin {
    #[inline(always)]
    fn from(value: &'a SiteRefSpan) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl From<SiteRef> for Origin {
    #[inline(always)]
    fn from(value: SiteRef) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl<'a> From<&'a SiteRef> for Origin {
    #[inline(always)]
    fn from(value: &'a SiteRef) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl From<TokenRef> for Origin {
    #[inline(always)]
    fn from(value: TokenRef) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl<'a> From<&'a TokenRef> for Origin {
    fn from(value: &'a TokenRef) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl From<Range<TokenRef>> for Origin {
    #[inline(always)]
    fn from(value: Range<TokenRef>) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl<'a> From<Range<&'a TokenRef>> for Origin {
    #[inline(always)]
    fn from(value: Range<&'a TokenRef>) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl From<RangeInclusive<TokenRef>> for Origin {
    #[inline(always)]
    fn from(value: RangeInclusive<TokenRef>) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl<'a> From<RangeInclusive<&'a TokenRef>> for Origin {
    #[inline(always)]
    fn from(value: RangeInclusive<&'a TokenRef>) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl From<RangeFrom<TokenRef>> for Origin {
    #[inline(always)]
    fn from(value: RangeFrom<TokenRef>) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl<'a> From<RangeFrom<&'a TokenRef>> for Origin {
    #[inline(always)]
    fn from(value: RangeFrom<&'a TokenRef>) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl From<RangeFrom<SiteRef>> for Origin {
    #[inline(always)]
    fn from(value: RangeFrom<SiteRef>) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl<'a> From<RangeFrom<&'a SiteRef>> for Origin {
    #[inline(always)]
    fn from(value: RangeFrom<&'a SiteRef>) -> Self {
        Self::Script(ScriptOrigin::from(value))
    }
}

impl Origin {
    /// Creates an instance of Origin that intentionally does not point
    /// to any source code. This serves as the Default constructor for this
    /// object.
    ///
    /// Use this function for API debugging purposes, or when the corresponding
    /// Script runtime operation cannot be associated with any Rust or Script
    /// source code.
    #[inline(always)]
    pub fn nil() -> Self {
        Self::Rust(&RustOrigin::nil())
    }

    /// Returns true if this instance is the [Nil Origin](Origin::nil).
    #[inline(always)]
    pub fn is_nil(&self) -> bool {
        match self {
            Self::Rust(origin) => origin.is_nil(),
            Self::Script(origin) => origin.is_nil(),
        }
    }

    /// Attempts to return the [script package](PackageMeta) to which this
    /// source code belongs.
    ///
    /// This function returns `None` if the package cannot be recognized (e.g.,
    /// in the case of the [Nil Origin](Origin::nil)).
    #[inline(always)]
    pub fn package(&self) -> Option<&'static PackageMeta> {
        match self {
            Self::Rust(origin) => origin.package(),
            Self::Script(origin) => origin.package(),
        }
    }

    #[inline(always)]
    pub(crate) fn into_ident(self, string: impl Into<CompactString>) -> Ident {
        match self {
            Self::Rust(..) => Ident::Script(ScriptIdent::from_string(TokenRef::nil(), string)),

            Self::Script(origin) => {
                Ident::Script(ScriptIdent::from_string(origin.into_token_ref(), string))
            }
        }
    }
}

static NIL_RUST_ORIGIN: RustOrigin = RustOrigin {
    package: None,
    code: None,
};

/// A pointer to a specific location in the Rust source code.
///
/// This object points to a particular place in a Rust file and represents the
/// origin of an exported Rust construct or a part of it.
///
/// Typically, you don't need to create this object manually. The
/// [export](crate::export) macro generates static instances of RustOrigin
/// during the introspection of Rust items. These instances are created in
/// static memory by the export system. Therefore, you would generally use
/// `&'static RustOrigin` references wrapped in [Origin].
#[derive(Clone, Copy, PartialOrd, Ord, Hash)]
pub struct RustOrigin {
    /// The name and version of the crate to which the Rust file belongs.
    pub package: Option<(&'static str, &'static str)>,

    /// The actual reference to the Rust file within the crate.
    pub code: Option<RustCode>,
}

impl Default for RustOrigin {
    #[inline(always)]
    fn default() -> Self {
        NIL_RUST_ORIGIN
    }
}

impl PartialEq for RustOrigin {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (&self.package, &other.package) {
            (Some(this), Some(other)) => {
                if this.ne(other) {
                    return false;
                }
            }

            _ => return false,
        }

        if let (Some(this), Some(other)) = (&self.code, &other.code) {
            if this.ne(other) {
                return false;
            }
        }

        true
    }
}

impl Eq for RustOrigin {}

impl Debug for RustOrigin {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        if self.package.is_none() && self.code.is_none() {
            return formatter.write_str("RustOrigin(invalid)");
        }

        let mut debug_struct = formatter.debug_struct("RustOrigin");

        if let Some((name, version)) = &self.package {
            debug_struct.field("package", &format_args!("{name}@{version}"));
        }

        if let Some(code) = &self.code {
            debug_struct.field("code", &code);
        }

        debug_struct.finish()
    }
}

impl Display for RustOrigin {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(code) = &self.code {
            return Display::fmt(code, formatter);
        }

        if let Some((name, _)) = self.package {
            return formatter.write_str(name);
        }

        formatter.write_str("[?]")
    }
}

impl RustOrigin {
    /// Returns a RustOrigin that intentionally does not point to any Rust code.
    /// This is the [Default] value of this object.
    #[inline(always)]
    pub fn nil() -> &'static Self {
        &NIL_RUST_ORIGIN
    }

    /// Returns true if this instance is the [Nil RustOrigin](Self::nil).
    pub fn is_nil(&self) -> bool {
        self == &NIL_RUST_ORIGIN
    }

    /// Returns the [script package metadata](PackageMeta) of the crate that
    /// this RustOrigin points to.
    ///
    /// Returns None if the package is not specified or if the crate is not
    /// a script package.
    #[inline(always)]
    pub fn package(&self) -> Option<&'static PackageMeta> {
        if let Some((name, version)) = self.package {
            return PackageMeta::of(name, &format!("={}", version));
        }

        None
    }

    /// This function is guaranteed to panic with the provided `message`.
    ///
    /// Unlike a normal `panic!`, the stack trace for this panic will typically
    /// start from the Rust code that this RustOrigin points to.
    #[inline(never)]
    pub fn blame<T>(&self, message: &str) -> T {
        if let Some(code) = self.code {
            (code.blame_fn)(message);
        }

        match self.package {
            Some((name, _)) => panic!("{}: {}", name, message),
            None => panic!("{}", message),
        }
    }
}

/// A representation of a range within Script source code.
///
/// This object points to specific span within the
/// [ScriptModule](crate::analysis::ScriptModule)'s source code text, often
/// highlighting identifiers and similar syntactic constructs. Generally,
/// it can span arbitrary range of text tokens.
///
/// Unlike absolute ranges (such as `10..20` or
/// `Position::new(1, 4)..Position::new(3, 8)`), the ScriptOrigin range is
/// relative. If you edit the script's text before, after, or inside the
/// referenced range, this range will realign accordingly. However, if the
/// text edits affect the ScriptOrigin bounds directly, the object will
/// become obsolete.
///
/// In general, ScriptOrigin may represent an invalid range or become
/// invalid over time if the source code is edited. You can check the range's
/// validity using the [is_valid_span](ToSpan::is_valid_span) method, passing
/// an instance of [ModuleText](crate::analysis::ModuleText):
/// `script_origin.is_valid_span(&module_text)`.
///
/// Note that ScriptOrigin belongs to a specific ScriptModule instance
/// and is valid only for that module. The [id](Id::id) function of ScriptOrigin
/// returns a globally unique identifier for the script module to which this
/// ScriptOrigin object belongs.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ScriptOrigin {
    id: Id,
    start: Option<Entry>,
    end: Bound<Entry>,
}

impl Default for ScriptOrigin {
    #[inline(always)]
    fn default() -> Self {
        Self::nil()
    }
}

impl PartialOrd for ScriptOrigin {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScriptOrigin {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        match self.id.cmp(&other.id) {
            Ordering::Equal => match self.start.cmp(&other.start) {
                Ordering::Equal => match (&self.end, &other.end) {
                    (Bound::Included(this), Bound::Included(other)) => this.cmp(other),
                    (Bound::Included(..), Bound::Excluded(..)) => Ordering::Less,
                    (Bound::Included(..), Bound::Unbounded) => Ordering::Less,

                    (Bound::Excluded(..), Bound::Included(..)) => Ordering::Greater,
                    (Bound::Excluded(this), Bound::Excluded(other)) => this.cmp(other),
                    (Bound::Excluded(..), Bound::Unbounded) => Ordering::Less,

                    (Bound::Unbounded, Bound::Unbounded) => Ordering::Equal,
                    (Bound::Unbounded, _) => Ordering::Greater,
                },

                other => other,
            },

            other => other,
        }
    }
}

impl Identifiable for ScriptOrigin {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl From<SiteRefSpan> for ScriptOrigin {
    #[inline(always)]
    fn from(value: SiteRefSpan) -> Self {
        Self::from(&value)
    }
}

impl<'a> From<&'a SiteRefSpan> for ScriptOrigin {
    #[inline(always)]
    fn from(value: &'a SiteRefSpan) -> Self {
        let id = value.start.id();

        if id.is_nil() || id != value.end.id() {
            return Self::default();
        }

        Self {
            id,

            start: {
                let bound = value.start.token_ref();

                match bound.is_nil() {
                    true => None,
                    false => Some(bound.entry),
                }
            },

            end: {
                let bound = value.end.token_ref();

                match bound.is_nil() {
                    true => Bound::Unbounded,
                    false => Bound::Excluded(bound.entry),
                }
            },
        }
    }
}

impl From<SiteRef> for ScriptOrigin {
    #[inline(always)]
    fn from(value: SiteRef) -> Self {
        Self::from(&value)
    }
}

impl<'a> From<&'a SiteRef> for ScriptOrigin {
    #[inline(always)]
    fn from(value: &'a SiteRef) -> Self {
        let id = value.id();

        if id.is_nil() {
            return Self::default();
        }

        let bound = value.token_ref();

        match bound.id.is_nil() {
            true => Self {
                id,
                start: None,
                end: Bound::Unbounded,
            },

            false => Self {
                id,
                start: Some(bound.entry),
                end: Bound::Excluded(bound.entry),
            },
        }
    }
}

impl From<TokenRef> for ScriptOrigin {
    #[inline(always)]
    fn from(value: TokenRef) -> Self {
        Self::from(&value)
    }
}

impl<'a> From<&'a TokenRef> for ScriptOrigin {
    #[inline(always)]
    fn from(value: &'a TokenRef) -> Self {
        Self {
            id: value.id,
            start: Some(value.entry),
            end: Bound::Included(value.entry),
        }
    }
}

impl From<Range<TokenRef>> for ScriptOrigin {
    #[inline(always)]
    fn from(value: Range<TokenRef>) -> Self {
        Self::from(&value.start..&value.end)
    }
}

impl<'a> From<Range<&'a TokenRef>> for ScriptOrigin {
    #[inline(always)]
    fn from(value: Range<&'a TokenRef>) -> Self {
        let id = value.start.id;

        if id.is_nil() || id != value.end.id {
            return Self::default();
        }

        Self {
            id,
            start: Some(value.start.entry),
            end: Bound::Excluded(value.end.entry),
        }
    }
}

impl From<RangeInclusive<TokenRef>> for ScriptOrigin {
    #[inline(always)]
    fn from(value: RangeInclusive<TokenRef>) -> Self {
        Self::from(value.start()..=value.end())
    }
}

impl<'a> From<RangeInclusive<&'a TokenRef>> for ScriptOrigin {
    #[inline(always)]
    fn from(value: RangeInclusive<&'a TokenRef>) -> Self {
        let id = value.start().id;

        if id.is_nil() || id != value.end().id {
            return Self::default();
        }

        Self {
            id,
            start: Some(value.start().entry),
            end: Bound::Included(value.end().entry),
        }
    }
}

impl From<RangeFrom<TokenRef>> for ScriptOrigin {
    #[inline(always)]
    fn from(value: RangeFrom<TokenRef>) -> Self {
        Self::from(&value.start..)
    }
}

impl<'a> From<RangeFrom<&'a TokenRef>> for ScriptOrigin {
    #[inline(always)]
    fn from(value: RangeFrom<&'a TokenRef>) -> Self {
        let id = value.start.id();

        if id.is_nil() {
            return Self::default();
        }

        Self {
            id: value.start.id,
            start: Some(value.start.entry),
            end: Bound::Unbounded,
        }
    }
}

impl From<RangeFrom<SiteRef>> for ScriptOrigin {
    #[inline(always)]
    fn from(value: RangeFrom<SiteRef>) -> Self {
        Self::from(&value.start..)
    }
}

impl<'a> From<RangeFrom<&'a SiteRef>> for ScriptOrigin {
    #[inline(always)]
    fn from(value: RangeFrom<&'a SiteRef>) -> Self {
        let id = value.start.id();

        if id.is_nil() {
            return Self::default();
        }

        let bound = value.start.token_ref();

        match bound.id.is_nil() {
            true => Self {
                id,
                start: None,
                end: Bound::Unbounded,
            },

            false => Self {
                id,
                start: Some(bound.entry),
                end: Bound::Unbounded,
            },
        }
    }
}

// Safety: `is_valid_span` falls back to `to_site_span`.
unsafe impl ToSpan for ScriptOrigin {
    #[inline(always)]
    fn to_site_span(&self, code: &impl SourceCode) -> Option<SiteSpan> {
        if self.id != code.id() {
            return None;
        }

        let length = code.length();

        let start = match &self.start {
            Some(entry) => code.get_site(entry)?.min(length),
            None => length,
        };

        let end = match &self.end {
            Bound::Included(entry) => {
                let chunk_length = code.get_length(entry)?;
                let bound = (code.get_site(entry)? + chunk_length).min(length);

                if bound < start {
                    return None;
                }

                bound
            }

            Bound::Excluded(entry) => {
                let bound = code.get_site(entry)?;

                if bound < start {
                    return None;
                }

                bound
            }

            Bound::Unbounded => length,
        };

        Some(start..end)
    }

    #[inline(always)]
    fn is_valid_span(&self, code: &impl SourceCode) -> bool {
        self.to_site_span(code).is_some()
    }
}

impl ScriptOrigin {
    /// Creates an instance of ScriptOrigin that intentionally does not point
    /// to any Script code. This serves as the [Default] constructor for this
    /// object.
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: Id::nil(),
            start: None,
            end: Bound::Unbounded,
        }
    }

    #[inline(always)]
    pub(crate) fn invalid(id: Id) -> Self {
        Self {
            id,
            start: Some(Entry::nil()),
            end: Bound::Unbounded,
        }
    }

    #[inline(always)]
    pub(crate) fn eoi(id: Id) -> Self {
        Self {
            id,
            start: None,
            end: Bound::Unbounded,
        }
    }

    /// Returns the [script package metadata](PackageMeta) of the
    /// [ScriptModule](crate::analysis::ScriptModule) that this ScriptOrigin
    /// points to.
    ///
    /// Returns None if the package cannot be found, for instance, if the
    /// corresponding ScriptModule has been dropped.
    #[inline(always)]
    pub fn package(&self) -> Option<&'static PackageMeta> {
        PackageMeta::by_id(self.id)
    }

    /// Returns true if this instance is the [Nil ScriptOrigin](Self::nil).
    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        if self.id.is_nil() {
            return true;
        }

        if let Some(entry) = &self.start {
            if entry.is_nil() {
                return true;
            }
        }

        match &self.end {
            Bound::Included(entry) | Bound::Excluded(entry) => {
                if entry.is_nil() {
                    return true;
                }
            }
            _ => (),
        }

        false
    }

    #[inline(always)]
    pub(crate) fn union(&mut self, other: &Self) {
        self.end = other.end;
    }

    #[inline(always)]
    pub(crate) fn unbound(&mut self) {
        self.end = Bound::Unbounded;
    }

    fn into_token_ref(self) -> TokenRef {
        let Some(entry) = self.start else {
            return TokenRef::nil();
        };

        TokenRef { id: self.id, entry }
    }
}

/// A component of [RustOrigin] that indicates a specific location in the Rust
/// source code.
#[derive(Clone, Copy, PartialOrd, Ord)]
pub struct RustCode {
    /// The name of the Rust module.
    pub module: &'static str,

    /// A one-based line number within the module file.
    pub line: u32,

    /// A one-based column number within a line of the module file.
    pub column: u32,

    /// A function that panics at the location of the pointed Rust construct
    /// with the specified error message.
    pub blame_fn: fn(&str),
}

impl Hash for RustCode {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.module.hash(state);
        self.line.hash(state);
        self.column.hash(state);
    }
}

impl PartialEq for RustCode {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.module.ne(other.module) {
            return false;
        }

        if self.line.ne(&other.line) {
            return false;
        }

        if self.column.ne(&other.column) {
            return false;
        }

        true
    }
}

impl Eq for RustCode {}

impl Debug for RustCode {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RustCode")
            .field("module", &self.module)
            .field("position", &format_args!("{}", self.position()))
            .finish()
    }
}

impl Display for RustCode {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_fmt(format_args!("{} [{}]", self.module, self.span_string()))
    }
}

impl RustCode {
    /// Similar to [RustOrigin::blame]. Refer to its documentation for details.
    #[inline(never)]
    pub fn blame(&self, message: &str) {
        (self.blame_fn)(message);
        panic!("{}: {}", self, message);
    }

    /// A helper function that returns a [Position] representation of the line
    /// and column of this RustCode.
    #[inline(always)]
    pub fn position(&self) -> Position {
        Position::new(self.line as Line, self.column as Column)
    }

    /// A helper function that returns a zero-span [Range] of
    /// [positions](Self::position): `code.position()..code.position()`.
    #[inline(always)]
    pub fn span(&self) -> PositionSpan {
        let bound = self.position();

        bound..bound
    }

    /// A helper function that formats the line and column in the canonical
    /// end-user-facing form of `[<line>]:[<column>]`.
    pub fn span_string(&self) -> String {
        format!("{}:{}", self.line, self.column)
    }
}
