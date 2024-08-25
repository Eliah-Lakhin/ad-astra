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

use std::ops::Deref;

use compact_str::CompactString;
use lady_deirdre::{
    analysis::{AbstractTask, TaskHandle},
    arena::{Id, Identifiable},
    lexis::{SiteSpan, SourceCode, TokenRef},
    syntax::{AbstractNode, NodeRef, PolyRef, SyntaxTree, Visitor},
    units::CompilationUnit,
};

use crate::{
    analysis::{Description, ModuleRead, ModuleResult, ModuleResultEx},
    runtime::{PackageMeta, ScriptIdent, ScriptOrigin},
    semantics::{IdentCrossResolution, LocalReturnPoint, Tag},
    syntax::{PolyRefOrigin, ScriptClass, ScriptDoc, ScriptNode, ScriptToken, SpanBounds},
};

/// A variant type that enumerates all language constructions currently
/// available for inspection.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ModuleSymbol {
    /// A special enum variant that indicates the absence of a language
    /// construction.
    ///
    /// Several API functions may return this value to indicate that the
    /// construction does not have an associated symbol or that the analyzer
    /// was unable to infer it. For example, [VarSymbol::let_value] may return
    /// `ModuleSymbol::Nil` if the `let x;` syntax does not have an
    /// initialization expression.
    ///
    /// This is the [Default] value of the ModuleSymbol.
    Nil,

    /// An import statement: `use foo.bar;`.
    Use(UseSymbol),

    /// A package identifier: `use <package_ident>;`.
    Package(PackageSymbol),

    /// A variable introduction: `let <var>;`, `fn(<var>)`, or
    /// `for <var> in range {}`.
    Var(VarSymbol),

    /// A loop statement: `loop {}` or `for x in range {}`.
    Loop(LoopSymbol),

    /// A loop-breaking statement: `break;` or `continue;`.
    Break(BreakSymbol),

    /// A function constructor: `fn() {}` or `fn(x, y) x + y`.
    Fn(FnSymbol),

    /// A return statement: `return 100;` or `return;`.
    Return(ReturnSymbol),

    /// A structure constructor: `struct { foo: 10, bar: 20 }`.
    Struct(StructSymbol),

    /// An array constructor: `[10, 20, 30]`.
    Array(ArraySymbol),

    /// An entry of the struct declaration: `struct { <entry>: 10 }`.
    Entry(EntrySymbol),

    /// An identifier in the expression: `<ident> + 10`.
    Ident(IdentSymbol),

    /// A field access operator: `foo.<field>`.
    Field(FieldSymbol),

    /// Any literal in the expression: `100`, or `true`, or `"string"`.
    Literal(LiteralSymbol),

    /// Any binary or unary operator in the expression: `10 + 20` or `!false`.
    Operator(OperatorSymbol),

    /// An invocation operator: `foo(a, b, c)`. The operator includes
    /// the content surrounded by the parentheses, as well as the parentheses
    /// themselves.
    Call(CallSymbol),

    /// An index operator: `foo[10]` or `foo[10..20]`. The operator includes
    /// the content surrounded by the brackets, as well as the brackets
    /// themselves.
    Index(IndexSymbol),
}

impl From<Tag> for ModuleSymbol {
    #[inline(always)]
    fn from(tag: Tag) -> Self {
        match &tag {
            Tag::Unset => ModuleSymbol::Nil,
            Tag::Type(_) => ModuleSymbol::Nil,
            Tag::Family(_) => ModuleSymbol::Nil,
            Tag::Struct(struct_ref) => ModuleSymbol::Struct(StructSymbol(*struct_ref)),
            Tag::Fn((fn_ref, _)) => ModuleSymbol::Fn(FnSymbol(*fn_ref)),
            Tag::Invocation(_) => ModuleSymbol::Nil,
        }
    }
}

impl Identifiable for ModuleSymbol {
    fn id(&self) -> Id {
        match self {
            Self::Nil => Id::nil(),
            Self::Use(symbol) => symbol.id(),
            Self::Package(symbol) => symbol.id(),
            Self::Var(symbol) => symbol.id(),
            Self::Loop(symbol) => symbol.id(),
            Self::Break(symbol) => symbol.id(),
            Self::Fn(symbol) => symbol.id(),
            Self::Return(symbol) => symbol.id(),
            Self::Struct(symbol) => symbol.id(),
            Self::Array(symbol) => symbol.id(),
            Self::Entry(symbol) => symbol.id(),
            Self::Ident(symbol) => symbol.id(),
            Self::Field(symbol) => symbol.id(),
            Self::Literal(symbol) => symbol.id(),
            Self::Operator(symbol) => symbol.id(),
            Self::Call(symbol) => symbol.id(),
            Self::Index(symbol) => symbol.id(),
        }
    }
}

impl Default for ModuleSymbol {
    #[inline(always)]
    fn default() -> Self {
        Self::Nil
    }
}

impl ModuleSymbol {
    #[inline(always)]
    fn new(kind: SymbolKind, node_ref: &NodeRef) -> Self {
        match kind {
            SymbolKind::Nil => Self::Nil,
            SymbolKind::Use => Self::Use(UseSymbol(*node_ref)),
            SymbolKind::Package => Self::Package(PackageSymbol(*node_ref)),
            SymbolKind::Var => Self::Var(VarSymbol(*node_ref)),
            SymbolKind::Loop => Self::Loop(LoopSymbol(*node_ref)),
            SymbolKind::Break => Self::Break(BreakSymbol(*node_ref)),
            SymbolKind::Fn => Self::Fn(FnSymbol(*node_ref)),
            SymbolKind::Return => Self::Return(ReturnSymbol(*node_ref)),
            SymbolKind::Struct => Self::Struct(StructSymbol(*node_ref)),
            SymbolKind::Array => Self::Array(ArraySymbol(*node_ref)),
            SymbolKind::Entry => Self::Entry(EntrySymbol(*node_ref)),
            SymbolKind::Ident => Self::Ident(IdentSymbol(*node_ref)),
            SymbolKind::Field => Self::Field(FieldSymbol(*node_ref)),
            SymbolKind::Literal => Self::Literal(LiteralSymbol(*node_ref)),
            SymbolKind::Operator => Self::Operator(OperatorSymbol(*node_ref)),
            SymbolKind::Call => Self::Call(CallSymbol(*node_ref)),
            SymbolKind::Index => Self::Index(IndexSymbol(*node_ref)),
        }
    }

    #[inline(always)]
    fn from_expr_node(script_node: &ScriptNode) -> Self {
        match script_node {
            ScriptNode::InlineComment { .. } => Self::Nil,
            ScriptNode::MultilineComment { .. } => Self::Nil,
            ScriptNode::Root { .. } => Self::Nil,
            ScriptNode::Clause { .. } => Self::Nil,
            ScriptNode::Use { .. } => Self::Nil,
            ScriptNode::Package { .. } => Self::Nil,
            ScriptNode::If { .. } => Self::Nil,
            ScriptNode::Match { .. } => Self::Nil,
            ScriptNode::MatchBody { .. } => Self::Nil,
            ScriptNode::MatchArm { .. } => Self::Nil,
            ScriptNode::Else { .. } => Self::Nil,
            ScriptNode::Let { .. } => Self::Nil,
            ScriptNode::Var { .. } => Self::Nil,
            ScriptNode::For { .. } => Self::Nil,
            ScriptNode::Loop { .. } => Self::Nil,
            ScriptNode::Block { .. } => Self::Nil,
            ScriptNode::Break { .. } => Self::Nil,
            ScriptNode::Continue { .. } => Self::Nil,
            ScriptNode::Return { .. } => Self::Nil,
            ScriptNode::Fn { node, .. } => Self::Fn(FnSymbol(*node)),
            ScriptNode::FnParams { .. } => Self::Nil,
            ScriptNode::Struct { node, .. } => Self::Struct(StructSymbol(*node)),
            ScriptNode::StructBody { .. } => Self::Nil,
            ScriptNode::StructEntry { .. } => Self::Nil,
            ScriptNode::StructEntryKey { .. } => Self::Nil,
            ScriptNode::Array { node, .. } => Self::Array(ArraySymbol(*node)),
            ScriptNode::String { node, .. } => Self::Literal(LiteralSymbol(*node)),
            ScriptNode::Crate { node, .. } => Self::Ident(IdentSymbol(*node)),
            ScriptNode::This { node, .. } => Self::Ident(IdentSymbol(*node)),
            ScriptNode::Ident { node, .. } => Self::Ident(IdentSymbol(*node)),
            ScriptNode::Number { node, .. } => Self::Literal(LiteralSymbol(*node)),
            ScriptNode::Max { node, .. } => Self::Literal(LiteralSymbol(*node)),
            ScriptNode::Bool { node, .. } => Self::Literal(LiteralSymbol(*node)),
            ScriptNode::UnaryLeft { op, .. } => Self::Operator(OperatorSymbol(*op)),
            ScriptNode::Binary { op, .. } => Self::Operator(OperatorSymbol(*op)),
            ScriptNode::Op { .. } => Self::Nil,
            ScriptNode::Query { op, .. } => Self::Operator(OperatorSymbol(*op)),
            ScriptNode::Call { node, .. } => Self::Call(CallSymbol(*node)),
            ScriptNode::CallArgs { .. } => Self::Nil,
            ScriptNode::Index { node, .. } => Self::Call(CallSymbol(*node)),
            ScriptNode::IndexArg { .. } => Self::Nil,
            ScriptNode::Field { node, .. } => Self::Field(FieldSymbol(*node)),
            ScriptNode::Expr { .. } => Self::Nil,
        }
    }

    /// Returns true if the ModuleSymbol's variant is [Nil](ModuleSymbol::Nil).
    #[inline(always)]
    pub fn is_nil(&self) -> bool {
        match self {
            Self::Nil => true,
            _ => false,
        }
    }

    /// Returns a descriptor object of the enum variant.
    ///
    /// The names of the [SymbolKind] variants match the names of this enum's
    /// variants exactly, except that the SymbolKind does not own the actual
    /// symbol object.
    #[inline(always)]
    pub fn kind(&self) -> SymbolKind {
        match self {
            Self::Nil => SymbolKind::Nil,
            Self::Use(_) => SymbolKind::Use,
            Self::Package(_) => SymbolKind::Package,
            Self::Var(_) => SymbolKind::Var,
            Self::Loop(_) => SymbolKind::Loop,
            Self::Break(_) => SymbolKind::Break,
            Self::Fn(_) => SymbolKind::Fn,
            Self::Return(_) => SymbolKind::Return,
            Self::Struct(_) => SymbolKind::Struct,
            Self::Array(_) => SymbolKind::Array,
            Self::Entry(_) => SymbolKind::Entry,
            Self::Ident(_) => SymbolKind::Ident,
            Self::Field(_) => SymbolKind::Field,
            Self::Literal(_) => SymbolKind::Literal,
            Self::Operator(_) => SymbolKind::Operator,
            Self::Call(_) => SymbolKind::Call,
            Self::Index(_) => SymbolKind::Index,
        }
    }

    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// Symbol objects you have obtained may become obsolete over time,
    /// for example, if the module's source code has been edited and the change
    /// affects the corresponding source code construction. This function checks
    /// the symbol's validity.
    ///
    /// The `read` argument is any content access guard object
    /// ([ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// Note that for [ModuleSymbol::Nil], this function always returns false.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        match self {
            Self::Nil => false,
            Self::Use(symbol) => symbol.is_valid(read),
            Self::Package(symbol) => symbol.is_valid(read),
            Self::Var(symbol) => symbol.is_valid(read),
            Self::Loop(symbol) => symbol.is_valid(read),
            Self::Break(symbol) => symbol.is_valid(read),
            Self::Fn(symbol) => symbol.is_valid(read),
            Self::Return(symbol) => symbol.is_valid(read),
            Self::Struct(symbol) => symbol.is_valid(read),
            Self::Array(symbol) => symbol.is_valid(read),
            Self::Entry(symbol) => symbol.is_valid(read),
            Self::Ident(symbol) => symbol.is_valid(read),
            Self::Field(symbol) => symbol.is_valid(read),
            Self::Literal(symbol) => symbol.is_valid(read),
            Self::Operator(symbol) => symbol.is_valid(read),
            Self::Call(symbol) => symbol.is_valid(read),
            Self::Index(symbol) => symbol.is_valid(read),
        }
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// The returned object typically covers just the "tagging" part of the
    /// symbol's syntax, such as the symbol's keyword.
    ///
    /// For example, for the `use foo.bar;` statement, the symbol's origin is
    /// the source code span that covers only the "use" keyword. For the `a + b`
    /// expression, the origin would cover only the "+" character.
    ///
    /// The intended use of this range is for annotating the most meaningful
    /// part of the syntax in the source code. For instance, if you want to
    /// highlight a hint in a code editor's user interface, it is more useful to
    /// annotate just the "fn" keyword of a function declaration rather than the
    /// entire function syntax, including its body.
    ///
    /// However, if the symbol is an [expression](SymbolKind::is_expr), and you
    /// need to fetch the entire expression range for code refactoring purposes,
    /// consider using the [expr_outer_origin](Self::expr_outer_origin) function
    /// instead.
    ///
    /// The `read` argument is any content access guard object
    /// ([ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// Note that if the symbol is not [valid](Self::is_valid), this function
    /// returns [ScriptOrigin::nil].
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        match self {
            Self::Nil => ScriptOrigin::nil(),
            Self::Use(symbol) => symbol.origin(read),
            Self::Package(symbol) => symbol.origin(read),
            Self::Var(symbol) => symbol.origin(read),
            Self::Loop(symbol) => symbol.origin(read),
            Self::Break(symbol) => symbol.origin(read),
            Self::Fn(symbol) => symbol.origin(read),
            Self::Return(symbol) => symbol.origin(read),
            Self::Struct(symbol) => symbol.origin(read),
            Self::Array(symbol) => symbol.origin(read),
            Self::Entry(symbol) => symbol.origin(read),
            Self::Ident(symbol) => symbol.origin(read),
            Self::Field(symbol) => symbol.origin(read),
            Self::Literal(symbol) => symbol.origin(read),
            Self::Operator(symbol) => symbol.origin(read),
            Self::Call(symbol) => symbol.origin(read),
            Self::Index(symbol) => symbol.origin(read),
        }
    }

    /// Returns the full source code range of the underlying symbol if this
    /// symbol is an [expression](SymbolKind::is_expr). Otherwise, returns
    /// a [ScriptOrigin::nil] range.
    ///
    /// Unlike the [origin](Self::origin) function, this range covers
    /// all parts of the symbol's construction. For example, for the
    /// `a + b` binary operator, this range includes both the left- and
    /// right-hand operands of the operator.
    ///
    /// Additionally, if the expression is wrapped in parentheses, the range
    /// covers the outermost parentheses: the expr_outer_origin
    /// range of the `((a + b))` expression includes the outermost "(" and ")"
    /// characters.
    ///
    /// The `read` argument is any content access guard object
    /// ([ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// Note that if the symbol is not [valid](Self::is_valid), this function
    /// also returns `ScriptOrigin::nil`.
    pub fn expr_outer_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        match self {
            Self::Nil => ScriptOrigin::nil(),
            Self::Use(_) => ScriptOrigin::nil(),
            Self::Package(_) => ScriptOrigin::nil(),
            Self::Var(_) => ScriptOrigin::nil(),
            Self::Loop(_) => ScriptOrigin::nil(),
            Self::Break(_) => ScriptOrigin::nil(),
            Self::Fn(symbol) => symbol.outer_origin(read),
            Self::Return(_) => ScriptOrigin::nil(),
            Self::Struct(symbol) => symbol.outer_origin(read),
            Self::Array(symbol) => symbol.outer_origin(read),
            Self::Entry(_) => ScriptOrigin::nil(),
            Self::Ident(symbol) => symbol.outer_origin(read),
            Self::Field(symbol) => symbol.outer_origin(read),
            Self::Literal(symbol) => symbol.outer_origin(read),
            Self::Operator(symbol) => symbol.outer_origin(read),
            Self::Call(symbol) => symbol.outer_origin(read),
            Self::Index(symbol) => symbol.outer_origin(read),
        }
    }

    /// Returns the inferred type of the expression if this symbol is an
    /// [expression](SymbolKind::is_expr). Otherwise, returns a
    /// [dynamic](crate::runtime::TypeHint::dynamic) type description.
    ///
    /// The `read` argument is any content access guard object
    /// ([ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// type inference requires deep source code analysis, and the analysis
    /// procedure is interrupted by the revocation of the module content access
    /// guard (see [ScriptModule](crate::analysis::ScriptModule) documentation
    /// for details).
    ///
    /// Note that if the symbol is not [valid](Self::is_valid), this function
    /// will also return a description of the dynamic type.
    pub fn expr_ty<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleResult<Description> {
        match self {
            Self::Nil => Ok(Description::dynamic()),
            Self::Use(_) => Ok(Description::dynamic()),
            Self::Package(_) => Ok(Description::dynamic()),
            Self::Var(_) => Ok(Description::dynamic()),
            Self::Loop(_) => Ok(Description::dynamic()),
            Self::Break(_) => Ok(Description::dynamic()),
            Self::Fn(symbol) => Ok(symbol.ty(read)),
            Self::Return(_) => Ok(Description::dynamic()),
            Self::Struct(symbol) => Ok(symbol.ty(read).unwrap_or_else(|| Description::dynamic())),
            Self::Array(symbol) => symbol.ty(read),
            Self::Entry(_) => Ok(Description::dynamic()),
            Self::Ident(symbol) => symbol.ty(read),
            Self::Field(symbol) => symbol.ty(read),
            Self::Literal(symbol) => Ok(symbol.ty(read)),
            Self::Operator(symbol) => symbol.ty(read),
            Self::Call(symbol) => symbol.ty(read),
            Self::Index(symbol) => symbol.ty(read),
        }
    }
}

/// A descriptor of the [ModuleSymbol] without the actual symbol data.
///
/// The names of this enum's variants match the ModuleSymbol enum variant names
/// exactly, except that SymbolKind does not own the actual symbol object.
///
/// This enum is `#[repr(u32)]`, with each variant corresponding to a dedicated
/// u32 bit. You can build a bit mask of symbols using the `|` bit operator:
/// `(SymbolKind::Fn as u32) | (SymbolKind::Use as u32)` refers to
/// function constructors (`fn() {}`) and import statements (`use foo;`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(u32)]
#[non_exhaustive]
pub enum SymbolKind {
    /// A special enum variant that indicates the absence of a language
    /// construction.
    Nil = 0,

    /// An import statement: `use foo.bar;`.
    Use = 1u32 << 1,

    /// A package identifier: `use <package_ident>;`.
    Package = 1u32 << 2,

    /// A variable introduction: `let <var>;`, `fn(<var>)`, or
    /// `for <var> in range {}`.
    Var = 1u32 << 3,

    /// A loop statement: `loop {}` or `for x in range {}`.
    Loop = 1u32 << 4,

    /// A loop-breaking statement: `break;` or `continue;`.
    Break = 1u32 << 5,

    /// A function constructor: `fn() {}` or `fn(x, y) x + y`.
    Fn = 1u32 << 6,

    /// A return statement: `return 100;` or `return;`.
    Return = 1u32 << 7,

    /// A structure constructor: `struct { foo: 10, bar: 20 }`.
    Struct = 1u32 << 8,

    /// An array constructor: `[10, 20, 30]`.
    Array = 1u32 << 9,

    /// An entry of the struct declaration: `struct { <entry>: 10 }`.
    Entry = 1u32 << 10,

    /// An identifier in the expression: `<ident> + 10`.
    Ident = 1u32 << 11,

    /// A field access operator: `foo.<field>`.
    Field = 1u32 << 12,

    /// Any literal in the expression: `100`, or `true`, or `"string"`.
    Literal = 1u32 << 13,

    /// Any binary or unary operator in the expression: `10 + 20` or `!false`.
    Operator = 1u32 << 14,

    /// An invocation operator: `foo(a, b, c)`. The operator includes
    /// the content surrounded by the parentheses, as well as the parentheses
    /// themselves.
    Call = 1u32 << 15,

    /// An index operator: `foo[10]` or `foo[10..20]`. The operator includes
    /// the content surrounded by the brackets, as well as the brackets
    /// themselves.
    Index = 1u32 << 16,
}

impl SymbolKind {
    /// Returns true if the SymbolKind's variant is [Nil](SymbolKind::Nil).
    #[inline(always)]
    pub fn is_nil(&self) -> bool {
        match self {
            Self::Nil => true,
            _ => false,
        }
    }

    /// Returns true if this symbol is part of an expression.
    ///
    /// For example, `Operator` and `Fn` are considered "expressions", but
    /// `Var` (variable introduction) is not an expression.
    #[inline(always)]
    pub fn is_expr(&self) -> bool {
        match self {
            Self::Nil => false,
            Self::Use => false,
            Self::Package => false,
            Self::Var => false,
            Self::Loop => false,
            Self::Break => false,
            Self::Fn => true,
            Self::Return => false,
            Self::Struct => true,
            Self::Array => true,
            Self::Entry => false,
            Self::Ident => true,
            Self::Field => true,
            Self::Literal => true,
            Self::Operator => true,
            Self::Call => true,
            Self::Index => true,
        }
    }
}

/// An import statement: `use foo.bar;`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UseSymbol(NodeRef);

impl From<UseSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: UseSymbol) -> Self {
        Self::Use(symbol)
    }
}

impl Identifiable for UseSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl UseSymbol {
    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Use { keyword, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        ScriptOrigin::from(keyword)
    }

    /// Returns all package components of the import statement:
    /// `use <package1>.<package2>.<package3>;`.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard).
    pub fn packages<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Vec<PackageSymbol> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Use { packages, .. }) = self.0.deref(doc_read.deref()) else {
            return Vec::new();
        };

        packages
            .iter()
            .map(|node_ref| PackageSymbol(*node_ref))
            .collect()
    }

    /// Returns the last component of the import statement:
    /// `use foo.bar.<last_package>;`.
    ///
    /// This represents the object that is actually imported into the module's
    /// namespace.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard).
    pub fn last_package<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Option<PackageSymbol> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Use { packages, .. }) = self.0.deref(doc_read.deref()) else {
            return None;
        };

        packages.last().map(|node_ref| PackageSymbol(*node_ref))
    }

    /// Returns the resolution of the last component of the import statement:
    /// `use foo.bar.<last_package>;`.
    ///
    /// This represents the package that is actually imported into the module's
    /// namespace.
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// package metadata inference requires deep source code analysis, and the
    /// analysis procedure is interrupted by the revocation of the module
    /// content access guard (see [ScriptModule](crate::analysis::ScriptModule)
    /// documentation for details).
    ///
    /// The function returns None if the analyzer fails to resolve the import
    /// statement (e.g., if the statement construction has syntax or semantic
    /// errors).
    pub fn resolution<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Option<&'static PackageMeta>> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Use { packages, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(None);
        };

        let Some(package_ref) = packages.last() else {
            return Ok(None);
        };

        let Some(ScriptNode::Package { semantics, .. }) = package_ref.deref(doc_read.deref())
        else {
            return Ok(None);
        };

        let id = doc_read.id();

        let package_semantics = semantics.get().into_module_result(id)?;

        let (_, package_resolution) = package_semantics
            .package_resolution
            .snapshot(read.task())
            .into_module_result(id)?;

        Ok(package_resolution.package)
    }

    /// Returns all identifiers within the module that refer to the semantics
    /// imported by this import statement into the module's namespace.
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// package metadata inference requires deep source code analysis, and the
    /// analysis procedure is interrupted by the revocation of the module
    /// content access guard (see [ScriptModule](crate::analysis::ScriptModule)
    /// documentation for details).
    ///
    /// The function returns an empty vector if the analyzer fails to resolve
    /// the import statement (e.g., if the statement construction has syntax or
    /// semantic errors).
    pub fn all_references<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Vec<IdentSymbol>> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Use { packages, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let Some(package_ref) = packages.last() else {
            return Ok(Vec::new());
        };

        let id = doc_read.id();

        let all_ident_refs = read
            .task()
            .snapshot_class(id, &ScriptClass::AllIdents)
            .into_module_result(id)?;

        Self::collect_refs(
            read,
            doc_read.deref(),
            package_ref,
            all_ident_refs.as_ref().into_iter(),
        )
    }

    /// Similar to [all_references](Self::references_by_name), but filters the
    /// references by the specified `name`.
    ///
    /// If you need to find all identifiers with a specific name that belong to
    /// the imported package, this function will perform notably faster than the
    /// all_references function.
    pub fn references_by_name<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
        name: &str,
    ) -> ModuleResult<Vec<IdentSymbol>> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Use { packages, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let Some(package_ref) = packages.last() else {
            return Ok(Vec::new());
        };

        let id = doc_read.id();

        let ident_refs = read
            .task()
            .snapshot_class(id, &ScriptClass::Ident(CompactString::from(name)))
            .into_module_result(id)?;

        Self::collect_refs(
            read,
            doc_read.deref(),
            package_ref,
            ident_refs.as_ref().into_iter(),
        )
    }

    #[inline(always)]
    fn collect_refs<'a, H: TaskHandle>(
        read: &impl ModuleRead<H>,
        doc: &ScriptDoc,
        package_ref: &NodeRef,
        candidates: impl Iterator<Item = &'a NodeRef>,
    ) -> ModuleResult<Vec<IdentSymbol>> {
        let id = doc.id();

        let task = read.task();

        let mut result = Vec::new();

        for ident_ref in candidates {
            let Some(ScriptNode::Ident { semantics, .. }) = ident_ref.deref(doc) else {
                continue;
            };

            let ident_semantics = semantics.get().into_module_result(id)?;

            task.proceed().into_module_result(id)?;

            let (_, ident_resolution) = ident_semantics
                .cross_resolution
                .snapshot(task)
                .into_module_result(id)?;

            let IdentCrossResolution::Read { name } = &ident_resolution else {
                continue;
            };

            if &name.as_ref().decl != package_ref {
                continue;
            }

            result.push(IdentSymbol(*ident_ref));
        }

        Ok(result)
    }
}

/// A package identifier within the import statement: `use <package_ident>;`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackageSymbol(NodeRef);

impl From<PackageSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: PackageSymbol) -> Self {
        Self::Package(symbol)
    }
}

impl Identifiable for PackageSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl PackageSymbol {
    #[inline(always)]
    pub(super) fn from_package_ref(package_ref: &NodeRef) -> ModuleSymbol {
        ModuleSymbol::Package(Self(*package_ref))
    }

    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Package { token, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        ScriptOrigin::from(token)
    }

    /// Returns true if this package is the last component of the import
    /// statement: `use foo.bar.<last_package>;`.
    ///
    /// The last package is the package that is actually imported into the
    /// module's namespace.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns false if the analyzer fails to resolve the import
    /// statement (e.g., if the statement construction has syntax or semantic
    /// errors).
    pub fn is_last<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Package { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return false;
        };

        let Some(ScriptNode::Use { packages, .. }) = parent.deref(doc_read.deref()) else {
            return false;
        };

        let Some(last_ref) = packages.last() else {
            return false;
        };

        last_ref == &self.0
    }

    /// Returns the symbol of the import statement to which this package symbol
    /// belongs.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns None if the analyzer fails to find the
    /// corresponding import statement (e.g., if the PackageSymbol is
    /// not [valid](Self::is_valid)).
    pub fn use_symbol<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Option<UseSymbol> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Package { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return None;
        };

        Some(UseSymbol(*parent))
    }

    /// Returns the [description](Description) of the package type.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// package metadata inference requires deep source code analysis, and the
    /// analysis procedure is interrupted by the revocation of the module
    /// content access guard (see [ScriptModule](crate::analysis::ScriptModule)
    /// documentation for details).
    ///
    /// The function returns a [dynamic](crate::runtime::TypeHint::dynamic) type
    /// description if the analyzer fails to find the corresponding import
    /// statement (e.g., if the statement construction has syntax or semantic
    /// errors, or if the package symbol is not [valid](Self::is_valid)).
    pub fn ty<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleResult<Description> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Package { semantics, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let id = doc_read.id();

        let package_semantics = semantics.get().into_module_result(id)?;

        let (_, package_resolution) = package_semantics
            .package_resolution
            .snapshot(read.task())
            .into_module_result(id)?;

        let Some(package) = package_resolution.package else {
            return Ok(Description::dynamic());
        };

        Ok(Description::from_package(package))
    }
}

/// A variable introduction: `let <var>;`, `fn(<var>)`, or
/// `for <var> in range {}`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VarSymbol(NodeRef);

impl From<VarSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: VarSymbol) -> Self {
        Self::Var(symbol)
    }
}

impl Identifiable for VarSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl VarSymbol {
    #[inline(always)]
    pub(super) fn from_var_ref(var_ref: &NodeRef) -> ModuleSymbol {
        ModuleSymbol::Var(Self(*var_ref))
    }

    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns a description of the variable introduction context, such as
    /// whether it is a Let Statement, a Script Function Parameter, etc.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [VarKind::Invalid] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the VarSymbol is not [valid](Self::is_valid)).
    pub fn kind<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> VarKind {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Var { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return VarKind::Invalid;
        };

        let Some(parent_node) = parent.deref(doc_read.deref()) else {
            return VarKind::Invalid;
        };

        match parent_node {
            ScriptNode::Let { .. } => VarKind::LetVar,
            ScriptNode::FnParams { .. } => VarKind::FnParam,
            ScriptNode::For { .. } => VarKind::ForIterator,
            _ => VarKind::Invalid,
        }
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Var { token, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        ScriptOrigin::from(token)
    }

    /// Returns a source code range object for the initialization value of a
    /// let-statement: `let x = <init_expr>;`.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ScriptOrigin::nil] if the let-statement does not
    /// have an initialization value (`let x;`), if this VarSymbol is not a
    /// let-statement (which can be checked via the [kind](Self::kind)
    /// function), or if this symbol is not [valid](Self::is_valid)).
    pub fn value_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Var { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        let Some(ScriptNode::Let { value, .. }) = parent.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        value.script_origin(doc_read.deref(), SpanBounds::Cover)
    }

    /// Returns the identifier object of the introduced variable.
    ///
    /// This object can be converted into a string using the Display
    /// implementation: `var_symbol.var_name(module_read).unwrap().to_string()`.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns None if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the symbol is not [valid](Self::is_valid)).
    pub fn var_name<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Option<ScriptIdent> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Var { token, .. }) = self.0.deref(doc_read.deref()) else {
            return None;
        };

        let Some(token_string) = token.string(doc_read.deref()) else {
            return None;
        };

        Some(ScriptIdent::from_string(*token, token_string))
    }

    /// Infers the type of the introduced variable.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// type inference requires deep source code analysis, and the analysis
    /// procedure is interrupted by the revocation of the module content access
    /// guard (see [ScriptModule](crate::analysis::ScriptModule) documentation
    /// for details).
    ///
    /// The function returns a [dynamic](crate::runtime::TypeHint::dynamic) type
    /// description if the analyzer fails to infer the type (e.g., if the
    /// construction has syntax errors, or if the symbol is not
    /// [valid](Self::is_valid)).
    pub fn var_type<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleResult<Description> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Var { semantics, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let id = doc_read.id();

        let var_semantics = semantics.get().into_module_result(id)?;

        let (_, type_resolution) = var_semantics
            .type_resolution
            .snapshot(read.task())
            .into_module_result(id)?;

        Ok(Description::from_tag(type_resolution.tag))
    }

    /// Returns the symbol of the let-statement's initialization value:
    /// `let x = <init_expr>;`.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::nil] if the let-statement does not
    /// have an initialization value (`let x;`), if this VarSymbol is not a
    /// let-statement (which can be checked via the [kind](Self::kind)
    /// function), or if this symbol is not [valid](Self::is_valid)).
    pub fn let_value<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Var { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(ScriptNode::Let { value, .. }) = parent.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(value_node) = descend_expr(doc_read.deref(), value) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(value_node)
    }

    /// Returns all identifiers across the module's code that semantically
    /// refer to this variable.
    ///
    /// The resulting vector consists of [VarRef] enums with two variants:
    ///  - [VarRef::Access]: An identifier that refers to this variable
    ///    when the variable is already initialized through the control flow.
    ///  - [VarRef::Definition]: The variable initialization point
    ///    (`a = 10;`).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// symbols inference requires deep source code analysis, and the analysis
    /// procedure is interrupted by the revocation of the module content access
    /// guard (see [ScriptModule](crate::analysis::ScriptModule) documentation
    /// for details).
    ///
    /// The function returns an empty vector if this VarSymbol is not
    /// [valid](Self::is_valid), or if the analyzer fails to find any reference
    /// that clearly belongs to this variable.
    pub fn references<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Vec<VarRef>> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Var { parent, token, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let Some(var_string) = token.string(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let id = doc_read.id();

        let class = ScriptClass::Ident(CompactString::from(var_string));

        let ident_refs = read
            .task()
            .snapshot_class(id, &class)
            .into_module_result(id)?;

        let mut result = Vec::new();

        for ident_ref in ident_refs.as_ref() {
            let Some(ScriptNode::Ident { semantics, .. }) = ident_ref.deref(doc_read.deref())
            else {
                continue;
            };

            let ident_semantics = semantics.get().into_module_result(id)?;

            let (_, ident_resolution) = ident_semantics
                .cross_resolution
                .snapshot(read.task())
                .into_module_result(id)?;

            match &ident_resolution {
                IdentCrossResolution::Read { name } if &name.as_ref().decl == parent => {
                    result.push(VarRef::Access(IdentSymbol(*ident_ref)))
                }

                IdentCrossResolution::Write { decl } if decl == parent => {
                    result.push(VarRef::Definition(IdentSymbol(*ident_ref)))
                }

                _ => continue,
            }
        }

        Ok(result)
    }
}

/// A type of the [VarSymbol] construction.
///
/// Returned by the [VarSymbol::kind] function.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum VarKind {
    /// Indicates that the analyzer failed to infer the variable type (see
    /// [VarSymbol::kind] for details).
    Invalid,

    /// The VarSymbol represents a let-statement: `let <var>;`.
    LetVar,

    /// The VarSymbol represents a script function's parameter: `fn(<var>) {}`.
    FnParam,

    /// The VarSymbol represents a for-loop iterator: `for <var> in range {}`.
    ForIterator,
}

/// An identifier that refers to the variable.
///
/// Returned by the [VarSymbol::references] function.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VarRef {
    /// An identifier that refers to a variable when the variable is already
    /// initialized through the control flow.
    Access(IdentSymbol),

    /// The point where a variable is initialized (`x = 10;`).
    Definition(IdentSymbol),
}

/// A loop statement: `loop {}` or `for x in range {}`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LoopSymbol(NodeRef);

impl From<LoopSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: LoopSymbol) -> Self {
        Self::Loop(symbol)
    }
}

impl Identifiable for LoopSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl LoopSymbol {
    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns a description of the loop, indicating whether it is a
    /// `for x in range {}` loop or a standard `loop {}`.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [LoopKind::Invalid] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the LoopSymbol is not [valid](Self::is_valid)).
    pub fn kind<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> LoopKind {
        let doc_read = read.read_doc();

        match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::For { .. }) => LoopKind::For,
            Some(ScriptNode::Loop { .. }) => LoopKind::Loop,
            _ => LoopKind::Invalid,
        }
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::For { keyword, .. }) => ScriptOrigin::from(keyword),
            Some(ScriptNode::Loop { keyword, .. }) => ScriptOrigin::from(keyword),
            _ => ScriptOrigin::nil(),
        }
    }

    /// Returns the iterator variable of the for-loop: `for <var> in range {}`.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns None if this LoopSymbol is not a for-loop, or if
    /// the symbol is not [valid](Self::is_valid).
    pub fn iterator<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Option<VarSymbol> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::For { iterator, .. }) = self.0.deref(doc_read.deref()) else {
            return None;
        };

        Some(VarSymbol(*iterator))
    }

    /// Returns the range expression symbol of the for-loop:
    /// `for x in <range> {}`.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if this LoopSymbol is not a
    /// for-loop, or if the symbol is not [valid](Self::is_valid).
    pub fn range<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::For { range, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(range_node) = descend_expr(doc_read.deref(), range) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(range_node)
    }

    /// Returns all `break` and `continue` statement symbols that belong to this
    /// loop.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an [Interrupted](crate::analysis::ModuleError::Interrupted)
    /// error if the symbols inference requires deep source code analysis, and the analysis
    /// procedure is interrupted by the revocation of the module content access guard
    /// (see [ScriptModule](crate::analysis::ScriptModule) documentation for details).
    ///
    /// The function returns an empty vector if this LoopSymbol is not
    /// [valid](Self::is_valid).
    pub fn breaks<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Vec<BreakSymbol>> {
        let doc_read = read.read_doc();

        let id = doc_read.id();

        let (_, break_set) = match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::For { semantics, .. }) => {
                let for_semantics = semantics.get().into_module_result(id)?;

                for_semantics
                    .break_set
                    .snapshot(read.task())
                    .into_module_result(id)?
            }

            Some(ScriptNode::Loop { semantics, .. }) => {
                let loop_semantics = semantics.get().into_module_result(id)?;

                loop_semantics
                    .break_set
                    .snapshot(read.task())
                    .into_module_result(id)?
            }

            _ => return Ok(Vec::new()),
        };

        Ok(break_set
            .as_ref()
            .set
            .iter()
            .map(|break_ref| BreakSymbol(*break_ref))
            .collect())
    }
}

/// A type of the [LoopSymbol] construction.
///
/// Returned by the [LoopSymbol::kind] function.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[non_exhaustive]
pub enum LoopKind {
    /// Indicates that the analyzer failed to infer the loop type
    /// (see [LoopSymbol::kind] for details).
    Invalid,

    /// The LoopSymbol represents a for-statement: `for x in range {}`.
    For,

    /// The LoopSymbol represents a loop-statement: `loop {}`.
    Loop,
}

/// A loop-breaking statement: `break;` or `continue;`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BreakSymbol(NodeRef);

impl From<BreakSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: BreakSymbol) -> Self {
        Self::Break(symbol)
    }
}

impl Identifiable for BreakSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl BreakSymbol {
    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns a description of the break type: whether it is a `break;` or a
    /// `continue;`.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [BreakKind::Invalid] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the BreakSymbol is not [valid](Self::is_valid)).
    pub fn kind<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> BreakKind {
        let doc_read = read.read_doc();

        match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::Break { .. }) => BreakKind::Break,
            Some(ScriptNode::Continue { .. }) => BreakKind::Continue,
            _ => BreakKind::Invalid,
        }
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::Break { keyword, .. }) => ScriptOrigin::from(keyword),
            Some(ScriptNode::Continue { keyword, .. }) => ScriptOrigin::from(keyword),
            _ => ScriptOrigin::nil(),
        }
    }

    /// Infers the loop statement to which this break/continue statement
    /// belongs.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// symbols inference requires deep source code analysis, and the analysis
    /// procedure is interrupted by the revocation of the module content access
    /// guard (see [ScriptModule](crate::analysis::ScriptModule) documentation
    /// for details).
    ///
    /// The function returns an empty vector if this LoopSymbol is not
    /// [valid](Self::is_valid).
    pub fn loop_symbol<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Option<LoopSymbol>> {
        let doc_read = read.read_doc();

        let id = doc_read.id();

        let (_, loop_context) = match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::Break { semantics, .. }) => {
                let break_semantics = semantics.get().into_module_result(id)?;

                break_semantics
                    .loop_context
                    .snapshot(read.task())
                    .into_module_result(id)?
            }

            Some(ScriptNode::Continue { semantics, .. }) => {
                let continue_semantics = semantics.get().into_module_result(id)?;

                continue_semantics
                    .loop_context
                    .snapshot(read.task())
                    .into_module_result(id)?
            }

            _ => return Ok(None),
        };

        if loop_context.loop_ref.is_nil() {
            return Ok(None);
        }

        Ok(Some(LoopSymbol(loop_context.loop_ref)))
    }
}

/// A type of the [BreakSymbol] construction.
///
/// Returned by the [BreakSymbol::kind] function.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum BreakKind {
    /// Indicates that the analyzer failed to infer the symbol type
    /// (see [BreakSymbol::kind] for details).
    Invalid,

    /// The BreakSymbol represents a normal break: `break;`.
    Break,

    /// The BreakSymbol represents a continuation: `continue;`.
    Continue,
}

/// A function constructor, such as `fn() {}` or `fn(x, y) x + y`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FnSymbol(NodeRef);

impl From<FnSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: FnSymbol) -> Self {
        Self::Fn(symbol)
    }
}

impl Identifiable for FnSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl FnSymbol {
    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns a description of the function, indicating whether it is a
    /// multiline `fn(x) {}` or a one-line function `fn(a, b) a + b`.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [FnKind::Invalid] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the FnSymbol is not [valid](Self::is_valid)).
    pub fn kind<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> FnKind {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Fn { body, .. }) = self.0.deref(doc_read.deref()) else {
            return FnKind::Invalid;
        };

        match body.deref(doc_read.deref()) {
            Some(ScriptNode::Expr { .. }) => FnKind::Inline,
            _ => FnKind::Multiline,
        }
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Fn { keyword, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        ScriptOrigin::from(keyword)
    }

    /// Returns the full source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::expr_outer_origin] for details.
    pub fn outer_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        outer_origin(doc_read.deref(), self.0)
    }

    /// Returns the code range of the function parameters (including the
    /// surrounding parentheses).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ScriptOrigin::nil] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the FnSymbol is not [valid](Self::is_valid)).
    pub fn params_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Fn { params, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        params.script_origin(doc_read.deref(), SpanBounds::Cover)
    }

    /// Returns a list of function parameters.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns an empty vector if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the FnSymbol is not [valid](Self::is_valid)).
    pub fn params<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Vec<VarSymbol> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Fn { params, .. }) = self.0.deref(doc_read.deref()) else {
            return Vec::new();
        };

        let Some(ScriptNode::FnParams { params, .. }) = params.deref(doc_read.deref()) else {
            return Vec::new();
        };

        params.iter().map(|var_ref| VarSymbol(*var_ref)).collect()
    }

    /// Returns a type description that formally describes this expression's
    /// type.
    ///
    /// See [ModuleSymbol::expr_ty] for details.
    pub fn ty<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Description {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Fn { params, .. }) = self.0.deref(doc_read.deref()) else {
            return Description::dynamic();
        };

        let Some(ScriptNode::FnParams { params, .. }) = params.deref(doc_read.deref()) else {
            return Description::fn_family(*self);
        };

        let arity = params.len();

        Description::fn_type(arity, *self)
    }

    /// Infers the result type of the script function.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// script function result type inference requires deep source code
    /// analysis, and the analysis procedure is interrupted by the revocation of
    /// the module content access guard (see
    /// [ScriptModule](crate::analysis::ScriptModule) documentation for
    /// details).
    ///
    /// The function returns a [dynamic](crate::runtime::TypeHint::dynamic)
    /// type description if the analyzer fails to infer the type (e.g., if the
    /// construction has syntax errors, if the FnSymbol is not
    /// [valid](Self::is_valid), or if the script function return type is too
    /// ambiguous).
    pub fn return_type<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Description> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Fn { semantics, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let id = doc_read.id();

        let fn_semantics = semantics.get().into_module_result(id)?;

        let (_, result_resolution) = fn_semantics
            .result_resolution
            .snapshot(read.task())
            .into_module_result(id)?;

        Ok(Description::from_tag(result_resolution.tag))
    }

    /// Returns a list of all explicit return statements (`return;` or
    /// `return 10;`) that belong to this script function.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// return points inference requires deep source code analysis, and the
    /// analysis procedure is interrupted by the revocation of the module
    /// content access guard (see [ScriptModule](crate::analysis::ScriptModule)
    /// documentation for details).
    ///
    /// The function returns an empty vector if the analyzer fails to infer
    /// return points (e.g., if the construction has syntax errors, or if the
    /// FnSymbol is not [valid](Self::is_valid)).
    pub fn return_symbols<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Vec<ReturnSymbol>> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Fn { semantics, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let id = doc_read.id();

        let fn_semantics = semantics.get().into_module_result(id)?;

        let (_, return_points) = fn_semantics
            .locals
            .return_points
            .snapshot(read.task())
            .into_module_result(id)?;

        let mut result = Vec::with_capacity(return_points.as_ref().set.len());

        for return_point in &return_points.as_ref().set {
            match return_point {
                LocalReturnPoint::Implicit => continue,

                LocalReturnPoint::Explicit(return_ref) => result.push(ReturnSymbol(*return_ref)),

                LocalReturnPoint::Expr(expr_ref) => {
                    let Some(ScriptNode::Expr { parent, .. }) = expr_ref.deref(doc_read.deref())
                    else {
                        continue;
                    };

                    let Some(ScriptNode::Return { .. }) = parent.deref(doc_read.deref()) else {
                        continue;
                    };

                    result.push(ReturnSymbol(*parent));
                }
            }
        }

        Ok(result)
    }

    /// Infers all expression symbols within the source code that directly
    /// or indirectly refer to this script function instance (usually, these
    /// symbols are [IdentSymbol]s).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// reference symbols inference requires deep source code analysis, and the
    /// analysis procedure is interrupted by the revocation of the module
    /// content access guard (see [ScriptModule](crate::analysis::ScriptModule)
    /// documentation for details).
    ///
    /// The function returns an empty vector if the analyzer fails to infer
    /// reference symbols (e.g., if the construction has syntax or semantic
    /// errors, or if the FnSymbol is not [valid](Self::is_valid)).
    pub fn references<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Vec<ModuleSymbol>> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Fn { .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let id = doc_read.id();

        let all_ident_refs = read
            .task()
            .snapshot_class(id, &ScriptClass::AllIdents)
            .into_module_result(id)?;

        let mut result = Vec::new();

        for ident_ref in all_ident_refs.as_ref() {
            let Some(ScriptNode::Ident { semantics, .. }) = ident_ref.deref(doc_read.deref())
            else {
                continue;
            };

            let ident_semantics = semantics.get().into_module_result(id)?;

            let (_, type_resolution) = ident_semantics
                .type_resolution
                .snapshot(read.task())
                .into_module_result(id)?;

            let Tag::Fn((fn_ref, _)) = &type_resolution.tag else {
                continue;
            };

            if fn_ref != &self.0 {
                continue;
            }

            result.push(ModuleSymbol::Ident(IdentSymbol(*ident_ref)));
        }

        let all_field_refs = read
            .task()
            .snapshot_class(id, &ScriptClass::AllFields)
            .into_module_result(id)?;

        for field_ref in all_field_refs.as_ref() {
            let Some(ScriptNode::Field { parent, .. }) = field_ref.deref(doc_read.deref()) else {
                continue;
            };

            let Some(ScriptNode::Binary { semantics, .. }) = parent.deref(doc_read.deref()) else {
                continue;
            };

            let binary_semantics = semantics.get().into_module_result(id)?;

            let (_, type_resolution) = binary_semantics
                .type_resolution
                .snapshot(read.task())
                .into_module_result(id)?;

            let Tag::Fn((fn_ref, _)) = &type_resolution.tag else {
                continue;
            };

            if fn_ref != &self.0 {
                continue;
            }

            result.push(ModuleSymbol::Ident(IdentSymbol(*field_ref)));
        }

        Ok(result)
    }

    /// Returns the parent expression of the script function if the script
    /// function is an argument of another expression. Otherwise, returns
    /// [ModuleSymbol::Nil] (e.g., if this FnSymbol is a top-level expression).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// infer the parent expression (e.g., if the construction has syntax
    /// errors, or if the FnSymbol is not [valid](Self::is_valid)).
    pub fn parent_expr<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Fn { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(parent_node) = ascend_expr(doc_read.deref(), parent) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(parent_node)
    }
}

/// A type of the [FnSymbol] construction.
///
/// Returned by the [FnSymbol::kind] function.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum FnKind {
    /// Indicates that the analyzer failed to infer the symbol type
    /// (see [FnSymbol::kind] for details).
    Invalid,

    /// The FnSymbol represents a multiline function: `fn() {}`.
    Multiline,

    /// The FnSymbol represents a one-line function: `fn(a, b) a + b`.
    Inline,
}

/// A return statement: `return 100;` or `return;`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReturnSymbol(NodeRef);

impl From<ReturnSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: ReturnSymbol) -> Self {
        Self::Return(symbol)
    }
}

impl Identifiable for ReturnSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl ReturnSymbol {
    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns the type of this return statement: whether it returns a specific
    /// expression (`return 100;`) or implicitly returns a nil value
    /// (`return;`).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ReturnKind::Invalid] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the ReturnSymbol is not [valid](Self::is_valid)).
    pub fn kind<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ReturnKind {
        let doc_read = read.read_doc();

        match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::Return { result, .. }) if result.is_nil() => ReturnKind::Nil,
            Some(ScriptNode::Return { .. }) => ReturnKind::Explicit,
            _ => ReturnKind::Invalid,
        }
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Return { keyword, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        ScriptOrigin::from(keyword)
    }

    /// Returns the source code range of the returning expression if the
    /// `return <expr>;` statement has an explicit expression. Otherwise,
    /// returns [ScriptOrigin::nil].
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ScriptOrigin::nil] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the ReturnSymbol is not [valid](Self::is_valid)).
    pub fn result_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Return { result, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        result.script_origin(doc_read.deref(), SpanBounds::Cover)
    }

    /// Infers the type of the expression returned by this statement.
    /// If the statement is an implicit return (`return;`), the function returns
    /// a [nil](crate::runtime::TypeHint::nil) type description.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// type inference requires deep source code analysis, and the analysis
    /// procedure is interrupted by the revocation of the module content access
    /// guard (see [ScriptModule](crate::analysis::ScriptModule) documentation
    /// for details).
    ///
    /// The function returns a [dynamic](crate::runtime::TypeHint::dynamic) type
    /// description if the analyzer fails to infer the type (e.g.,
    /// if the construction has syntax or semantic errors, if the ReturnSymbol
    /// is not [valid](Self::is_valid), or if the expression is too ambiguous
    /// for the type inference algorithm).
    pub fn result_type<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Description> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Return { result, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        if result.is_nil() {
            return Ok(Description::nil());
        }

        let Some(ScriptNode::Expr { semantics, .. }) = result.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let id = doc_read.id();

        let expr_semantics = semantics.get().into_module_result(id)?;

        let (_, type_resolution) = expr_semantics
            .type_resolution
            .snapshot(read.task())
            .into_module_result(id)?;

        Ok(Description::from_tag(type_resolution.tag))
    }

    /// Returns the symbol of the script function from which this return
    /// statement returns. If the statement does not belong to any explicitly
    /// defined script function (i.e., the statement returns from the script
    /// itself), the function returns None.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// script function lookup requires deep source code analysis, and the
    /// analysis procedure is interrupted by the revocation of the module
    /// content access guard (see [ScriptModule](crate::analysis::ScriptModule)
    /// documentation for details).
    ///
    /// The function returns None if the analyzer fails to find the script
    /// function (e.g., if the construction has syntax errors, or if the
    /// ReturnSymbol is not [valid](Self::is_valid)).
    pub fn fn_symbol<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Option<FnSymbol>> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Return { semantics, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(None);
        };

        let id = doc_read.id();

        let scope_attr = semantics.scope_attr().into_module_result(id)?;

        let (_, scope) = scope_attr.snapshot(read.task()).into_module_result(id)?;

        let Some(ScriptNode::Fn { .. }) = scope.scope_ref.deref(doc_read.deref()) else {
            return Ok(None);
        };

        Ok(Some(FnSymbol(scope.scope_ref)))
    }

    /// Returns all return statements (including this one) that syntactically
    /// belong to the same script function from which this return statement
    /// returns.
    ///
    /// This function is similar to calling [Self::fn_symbol] and then
    /// [FnSymbol::return_symbols], except that `Self::fn_symbol` may return
    /// None if this return statement is returning from the script itself.
    /// In contrast, the `fn_returns` function yields all related return
    /// statement symbols regardless of the context.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// statement inference requires deep source code analysis, and the analysis
    /// procedure is interrupted by the revocation of the module content access
    /// guard (see [ScriptModule](crate::analysis::ScriptModule) documentation
    /// for details).
    ///
    /// The function returns an empty vector if the analyzer fails to infer any
    /// related statements (e.g., if the construction has syntax or semantic
    /// errors, or if the ReturnSymbol is not [valid](Self::is_valid)).
    pub fn fn_returns<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleResult<Vec<Self>> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Return { semantics, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let id = doc_read.id();

        let scope_attr = semantics.scope_attr().into_module_result(id)?;

        let (_, scope) = scope_attr.snapshot(read.task()).into_module_result(id)?;

        let Some(scope_node) = scope.scope_ref.deref(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let locals = scope_node.locals().into_module_result(id)?;

        let (_, return_points) = locals
            .return_points
            .snapshot(read.task())
            .into_module_result(id)?;

        let mut result = Vec::with_capacity(return_points.as_ref().set.len());

        for return_point in &return_points.as_ref().set {
            match return_point {
                LocalReturnPoint::Implicit => continue,

                LocalReturnPoint::Explicit(return_ref) => result.push(Self(*return_ref)),

                LocalReturnPoint::Expr(expr_ref) => {
                    let Some(ScriptNode::Expr { parent, .. }) = expr_ref.deref(doc_read.deref())
                    else {
                        continue;
                    };

                    let Some(ScriptNode::Return { .. }) = parent.deref(doc_read.deref()) else {
                        continue;
                    };

                    result.push(Self(*parent));
                }
            }
        }

        Ok(result)
    }
}

/// A type of the [ReturnSymbol] construction.
///
/// Returned by the [ReturnSymbol::kind] function.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum ReturnKind {
    /// Indicates that the analyzer failed to infer the symbol type
    /// (see [ReturnSymbol::kind] for details).
    Invalid,

    /// The return statement implicitly returns a nil value: `return;`.
    Nil,

    /// The return statement returns a value explicitly: `return 100;`.
    Explicit,
}

/// A structure constructor: `struct { foo: 10, bar: 20 }`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StructSymbol(NodeRef);

impl From<StructSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: StructSymbol) -> Self {
        Self::Struct(symbol)
    }
}

impl Identifiable for StructSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl StructSymbol {
    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Struct { keyword, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        ScriptOrigin::from(keyword)
    }

    /// Returns the full source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::expr_outer_origin] for details.
    pub fn outer_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        outer_origin(doc_read.deref(), self.0)
    }

    /// Returns the source code range that covers only the structure body.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ScriptOrigin::nil] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the StructSymbol is not [valid](Self::is_valid)).
    pub fn body_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Struct { body, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        body.script_origin(doc_read.deref(), SpanBounds::Cover)
    }

    /// Returns a type description that formally describes this expression's type.
    ///
    /// See [ModuleSymbol::expr_ty] for details.
    pub fn ty<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Option<Description> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Struct { .. }) = self.0.deref(doc_read.deref()) else {
            return None;
        };

        Some(Description::struct_family(*self))
    }

    /// Returns the parent expression of the structure if the structure is an
    /// argument of another expression. Otherwise, returns [ModuleSymbol::Nil]
    /// (e.g., if this StructSymbol is a top-level expression).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to infer
    /// the parent expression (e.g., if the construction has syntax errors, or
    /// if the StructSymbol is not [valid](Self::is_valid)).
    pub fn parent_expr<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Struct { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(parent_node) = ascend_expr(doc_read.deref(), parent) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(parent_node)
    }

    /// Returns a list of all structure entries in the order of their declaration.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns an empty vector if the analyzer fails to
    /// recognize the structure fields (e.g., if the construction has syntax errors,
    /// or if the StructSymbol is not [valid](Self::is_valid)).
    pub fn entries<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Vec<EntrySymbol> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Struct { body, .. }) = self.0.deref(doc_read.deref()) else {
            return Vec::new();
        };

        let Some(ScriptNode::StructBody { entries, .. }) = body.deref(doc_read.deref()) else {
            return Vec::new();
        };

        let mut result = Vec::with_capacity(entries.len());

        for entry_ref in entries {
            let Some(ScriptNode::StructEntry { key, .. }) = entry_ref.deref(doc_read.deref())
            else {
                continue;
            };

            result.push(EntrySymbol(*key));
        }

        result
    }

    /// Infers all expression symbols within the source code that directly
    /// or indirectly refer to this structure instance (usually, these symbols
    /// are [IdentSymbol]s or [FieldSymbol]s).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// reference symbols inference requires deep source code analysis, and the
    /// analysis procedure is interrupted by the revocation of the module
    /// content access guard (see [ScriptModule](crate::analysis::ScriptModule)
    /// documentation for details).
    ///
    /// The function returns an empty vector if the analyzer fails to infer
    /// reference symbols (e.g., if the construction has syntax or semantic
    /// errors, or if the StructSymbol is not [valid](Self::is_valid)).
    pub fn references<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Vec<ModuleSymbol>> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Struct { .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let id = doc_read.id();

        let all_ident_refs = read
            .task()
            .snapshot_class(id, &ScriptClass::AllIdents)
            .into_module_result(id)?;

        let mut result = Vec::new();

        for ident_ref in all_ident_refs.as_ref() {
            let Some(ScriptNode::Ident { semantics, .. }) = ident_ref.deref(doc_read.deref())
            else {
                continue;
            };

            let ident_semantics = semantics.get().into_module_result(id)?;

            let (_, type_resolution) = ident_semantics
                .type_resolution
                .snapshot(read.task())
                .into_module_result(id)?;

            let Tag::Struct(struct_ref) = &type_resolution.tag else {
                continue;
            };

            if struct_ref != &self.0 {
                continue;
            }

            result.push(ModuleSymbol::Ident(IdentSymbol(*ident_ref)));
        }

        let all_these_refs = read
            .task()
            .snapshot_class(id, &ScriptClass::AllThese)
            .into_module_result(id)?;

        let mut result = Vec::new();

        for ident_ref in all_these_refs.as_ref() {
            let Some(ScriptNode::This { semantics, .. }) = ident_ref.deref(doc_read.deref()) else {
                continue;
            };

            let this_semantics = semantics.get().into_module_result(id)?;

            let (_, type_resolution) = this_semantics
                .type_resolution
                .snapshot(read.task())
                .into_module_result(id)?;

            let Tag::Struct(struct_ref) = &type_resolution.tag else {
                continue;
            };

            if struct_ref != &self.0 {
                continue;
            }

            result.push(ModuleSymbol::Ident(IdentSymbol(*ident_ref)));
        }

        let all_field_refs = read
            .task()
            .snapshot_class(id, &ScriptClass::AllFields)
            .into_module_result(id)?;

        for field_ref in all_field_refs.as_ref() {
            let Some(ScriptNode::Field { parent, .. }) = field_ref.deref(doc_read.deref()) else {
                continue;
            };

            let Some(ScriptNode::Binary { left, .. }) = parent.deref(doc_read.deref()) else {
                continue;
            };

            let Some(left_node) = left.deref(doc_read.deref()) else {
                continue;
            };

            let (_, type_resolution) = left_node
                .type_resolution()
                .into_module_result(id)?
                .snapshot(read.task())
                .into_module_result(id)?;

            let Tag::Struct(struct_ref) = &type_resolution.tag else {
                continue;
            };

            if struct_ref != &self.0 {
                continue;
            }

            let Some(left) = descend_expr(doc_read.deref(), left) else {
                continue;
            };

            result.push(ModuleSymbol::from_expr_node(left));
        }

        Ok(result)
    }
}

/// An array constructor: `[10, 20, 30]`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArraySymbol(NodeRef);

impl From<ArraySymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: ArraySymbol) -> Self {
        Self::Array(symbol)
    }
}

impl Identifiable for ArraySymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl ArraySymbol {
    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        self.0.script_origin(doc_read.deref(), SpanBounds::Cover)
    }

    /// Returns the full source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::expr_outer_origin] for details.
    pub fn outer_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        outer_origin(doc_read.deref(), self.0)
    }

    /// Returns a type description that formally describes this expression's type.
    ///
    /// See [ModuleSymbol::expr_ty] for details.
    pub fn ty<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleResult<Description> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Array { semantics, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let id = doc_read.id();

        let array_semantics = semantics.get().into_module_result(id)?;

        let (_, type_resolution) = array_semantics
            .type_resolution
            .snapshot(read.task())
            .into_module_result(id)?;

        Ok(Description::from_tag(type_resolution.tag))
    }

    /// Returns the parent expression of the array, if the array is an argument
    /// of another expression. Otherwise, returns [ModuleSymbol::Nil]
    /// (e.g., if this ArraySymbol is a top-level expression).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// infer the parent expression (e.g., if the construction has syntax
    /// errors, or if the ArraySymbol is not [valid](Self::is_valid)).
    pub fn parent_expr<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Array { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(parent_node) = ascend_expr(doc_read.deref(), parent) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(parent_node)
    }

    /// Returns a list of array items.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns an empty vector if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the ArraySymbol is not [valid](Self::is_valid)).
    pub fn items<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Vec<ModuleSymbol> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Array { items, .. }) = self.0.deref(doc_read.deref()) else {
            return Vec::new();
        };

        let mut result = Vec::with_capacity(items.len());

        for item_ref in items {
            let Some(item_node) = descend_expr(doc_read.deref(), item_ref) else {
                result.push(ModuleSymbol::Nil);
                continue;
            };

            result.push(ModuleSymbol::from_expr_node(item_node))
        }

        result
    }
}

/// An entry of the struct declaration: `struct { <entry>: 10 }`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntrySymbol(NodeRef);

impl From<EntrySymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: EntrySymbol) -> Self {
        Self::Entry(symbol)
    }
}

impl Identifiable for EntrySymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl EntrySymbol {
    #[inline(always)]
    pub(super) fn from_struct_entry_key_ref(struct_entry_key: &NodeRef) -> ModuleSymbol {
        ModuleSymbol::Entry(Self(*struct_entry_key))
    }

    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::StructEntryKey { token, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        ScriptOrigin::from(token)
    }

    /// Returns the name of this entry.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns None if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the EntrySymbol is not [valid](Self::is_valid)).
    pub fn name<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Option<String> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::StructEntryKey { token, .. }) = self.0.deref(doc_read.deref()) else {
            return None;
        };

        Some(String::from(token.string(doc_read.deref())?))
    }

    /// Returns the inferred type of the value of this field.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// type inference requires deep source code analysis, and the analysis
    /// procedure is interrupted by the revocation of the module content access
    /// guard (see [ScriptModule](crate::analysis::ScriptModule) documentation
    /// for details).
    ///
    /// The function returns a [dynamic](crate::runtime::TypeHint::dynamic) type
    /// description if the analyzer fails to infer the value type (e.g., if the
    /// construction has syntax or semantic errors, if the EntrySymbol is not
    /// [valid](Self::is_valid), or if the type is too ambiguous for the type
    /// inference algorithm).
    pub fn ty<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleResult<Description> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::StructEntryKey { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let Some(ScriptNode::StructEntry { value, .. }) = parent.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let Some(ScriptNode::Expr { semantics, .. }) = value.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let id = doc_read.id();

        let expr_semantics = semantics.get().into_module_result(id)?;

        let (_, type_resolution) = expr_semantics
            .type_resolution
            .snapshot(read.task())
            .into_module_result(id)?;

        Ok(Description::from_tag(type_resolution.tag))
    }

    /// Returns an [expression](SymbolKind::is_expr) symbol of the field's
    /// value.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to infer
    /// the field's value (e.g., if the construction has syntax errors, or if
    /// the EntrySymbol is not [valid](Self::is_valid)).
    pub fn value_symbol<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::StructEntryKey { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(ScriptNode::StructEntry { value, .. }) = parent.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(value_node) = descend_expr(doc_read.deref(), value) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(value_node)
    }

    /// Returns the symbol of the structure to which this entry belongs.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns None if the analyzer fails to infer the entry's
    /// owner (e.g., if the construction has syntax errors, or if the
    /// EntrySymbol is not [valid](Self::is_valid)).
    pub fn struct_symbol<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Option<StructSymbol> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::StructEntryKey { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return None;
        };

        let Some(ScriptNode::StructEntry { parent, .. }) = parent.deref(doc_read.deref()) else {
            return None;
        };

        let Some(ScriptNode::StructBody { parent, .. }) = parent.deref(doc_read.deref()) else {
            return None;
        };

        if !parent.is_valid_ref(doc_read.deref()) {
            return None;
        }

        Some(StructSymbol(*parent))
    }

    /// Infers all field access symbols within the source code that directly
    /// refer to this entry.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// reference symbols inference requires deep source code analysis, and the
    /// analysis procedure is interrupted by the revocation of the module
    /// content access guard (see [ScriptModule](crate::analysis::ScriptModule)
    /// documentation for details).
    ///
    /// The function returns an empty vector if the analyzer fails to
    /// infer the reference symbols (e.g., if the construction has syntax or
    /// semantic errors, or if the EntrySymbol is not [valid](Self::is_valid)).
    pub fn references<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Vec<FieldSymbol>> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::StructEntryKey { parent, token, .. }) = self.0.deref(doc_read.deref())
        else {
            return Ok(Vec::new());
        };

        let Some(ScriptNode::StructEntry { parent, .. }) = parent.deref(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let Some(token_string) = token.string(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let Some(ScriptNode::StructBody {
            parent: struct_ref, ..
        }) = parent.deref(doc_read.deref())
        else {
            return Ok(Vec::new());
        };

        let id = doc_read.id();

        let field_refs = read
            .task()
            .snapshot_class(id, &ScriptClass::Field(CompactString::from(token_string)))
            .into_module_result(id)?;

        let mut result = Vec::with_capacity(field_refs.as_ref().len());

        for field_ref in field_refs.as_ref() {
            let Some(ScriptNode::Field { parent, .. }) = field_ref.deref(doc_read.deref()) else {
                continue;
            };

            let Some(ScriptNode::Binary { left, .. }) = parent.deref(doc_read.deref()) else {
                continue;
            };

            let Some(left_node) = left.deref(doc_read.deref()) else {
                continue;
            };

            let (_, type_resolution) = left_node
                .type_resolution()
                .into_module_result(id)?
                .snapshot(read.task())
                .into_module_result(id)?;

            let Tag::Struct(left_struct_ref) = &type_resolution.tag else {
                continue;
            };

            if left_struct_ref != struct_ref {
                continue;
            }

            result.push(FieldSymbol(*field_ref));
        }

        Ok(result)
    }
}

/// An identifier in the expression: `<ident> + 10`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IdentSymbol(NodeRef);

impl From<IdentSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: IdentSymbol) -> Self {
        Self::Ident(symbol)
    }
}

impl Identifiable for IdentSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl IdentSymbol {
    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns a description of this identifier, indicating what kind of data
    /// it is: a variable access or initialization, a `crate` or `self` built-in
    /// identifier, or something else.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// description inference requires deep source code analysis, and the
    /// analysis procedure is interrupted by the revocation of the module
    /// content access guard (see [ScriptModule](crate::analysis::ScriptModule)
    /// documentation for details).
    ///
    /// The function returns [IdentKind::Invalid] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax or
    /// semantic errors, or if the IdentSymbol is not [valid](Self::is_valid)).
    pub fn kind<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleResult<IdentKind> {
        let doc_read = read.read_doc();

        let semantics = match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::Ident { semantics, .. }) => semantics,
            Some(ScriptNode::Crate { .. }) => return Ok(IdentKind::CrateIdent),
            Some(ScriptNode::This { .. }) => return Ok(IdentKind::SelfIdent),
            _ => return Ok(IdentKind::Invalid),
        };

        let id = doc_read.id();

        let ident_semantics = semantics.get().into_module_result(id)?;

        let (_, cross_resolution) = ident_semantics
            .cross_resolution
            .snapshot(read.task())
            .into_module_result(id)?;

        match cross_resolution {
            IdentCrossResolution::Read { name } => match name.as_ref().decl.deref(doc_read.deref())
            {
                Some(ScriptNode::Root { .. }) => Ok(IdentKind::CrateAccess),

                Some(ScriptNode::Use { .. }) => Ok(IdentKind::PackageAccess),

                Some(
                    ScriptNode::Let { .. } | ScriptNode::For { .. } | ScriptNode::FnParams { .. },
                ) => Ok(IdentKind::VarAccess),

                _ => Ok(IdentKind::Invalid),
            },

            IdentCrossResolution::Write { .. } => Ok(IdentKind::VarDefinition),

            _ => Ok(IdentKind::Invalid),
        }
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let token = match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::Ident { token, .. }) => token,
            Some(ScriptNode::Crate { token, .. }) => token,
            Some(ScriptNode::This { token, .. }) => token,
            _ => return ScriptOrigin::nil(),
        };

        ScriptOrigin::from(token)
    }

    /// Returns the full source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::expr_outer_origin] for details.
    pub fn outer_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        outer_origin(doc_read.deref(), self.0)
    }

    /// Returns the name of this identifier.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns None if the analyzer fails to resolve this
    /// construction (e.g., if the construction has syntax errors, or if the
    /// IdentSymbol is not [valid](Self::is_valid)).
    pub fn name<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Option<String> {
        let doc_read = read.read_doc();

        let token = match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::Ident { token, .. }) => token,
            Some(ScriptNode::Crate { token, .. }) => token,
            Some(ScriptNode::This { token, .. }) => token,
            _ => return None,
        };

        Some(String::from(token.string(doc_read.deref())?))
    }

    /// Looks up all identifiers (including this one) across the source code
    /// that are semantically similar to the current identifier.
    ///
    /// The identifier's similarity depends on its [kind](Self::kind) and the
    /// context:
    ///
    /// - If this identifier refers to a variable, the function looks up all
    ///   identifiers that refer to the same variable.
    /// - If this identifier refers to a component of an imported package,
    ///   the function looks up all identifiers that refer to the same component
    ///   usage across the source code.
    /// - If this is a `self` built-in identifier, the function looks up all
    ///   `self` identifiers that are linked to the same context.
    /// - If this is a `crate` built-in identifier, the function lists all
    ///   `crate` identifiers across the source code.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// identifier lookup requires deep source code analysis, and the analysis
    /// procedure is interrupted by the revocation of the module content access
    /// guard (see [ScriptModule](crate::analysis::ScriptModule) documentation
    /// for details).
    ///
    /// The function returns an empty vector if the analyzer fails to resolve
    /// this construction (e.g., if the construction has syntax or semantic
    /// errors, or if the IdentSymbol is not [valid](Self::is_valid)).
    pub fn similar_idents<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Vec<Self>> {
        let doc_read = read.read_doc();

        let id = read.id();

        match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::Crate { .. }) => {
                let all_crates = read
                    .task()
                    .snapshot_class(id, &ScriptClass::AllCrates)
                    .into_module_result(id)?;

                let mut result = Vec::with_capacity(all_crates.as_ref().len());

                for crate_ref in all_crates.as_ref() {
                    if !crate_ref.is_valid_ref(doc_read.deref()) {
                        continue;
                    }

                    result.push(Self(*crate_ref));
                }

                Ok(result)
            }

            Some(ScriptNode::This { semantics, .. }) => {
                let task = read.task();

                let (_, scope_ref) = semantics
                    .scope_attr()
                    .into_module_result(id)?
                    .snapshot(task)
                    .into_module_result(id)?;

                let scope_ref = scope_ref.scope_ref;

                let all_these = read
                    .task()
                    .snapshot_class(id, &ScriptClass::AllThese)
                    .into_module_result(id)?;

                let mut result = Vec::with_capacity(all_these.as_ref().len());

                for this_ref in all_these.as_ref() {
                    let Some(ScriptNode::This { semantics, .. }) = this_ref.deref(doc_read.deref())
                    else {
                        continue;
                    };

                    let (_, other_scope_ref) = semantics
                        .scope_attr()
                        .into_module_result(id)?
                        .snapshot(task)
                        .into_module_result(id)?;

                    let other_scope_ref = other_scope_ref.scope_ref;

                    if scope_ref != other_scope_ref {
                        continue;
                    }

                    result.push(Self(*this_ref));
                }

                Ok(result)
            }

            Some(ScriptNode::Ident {
                semantics, token, ..
            }) => {
                let task = read.task();

                let (_, cross_resolution) = semantics
                    .get()
                    .into_module_result(id)?
                    .cross_resolution
                    .snapshot(task)
                    .into_module_result(id)?;

                let IdentCrossResolution::Read { name } = cross_resolution else {
                    return Ok(Vec::new());
                };

                let Some(ScriptNode::Root { .. }) = name.as_ref().decl.deref(doc_read.deref())
                else {
                    return Ok(Vec::new());
                };

                let Some(token_string) = token.string(doc_read.deref()) else {
                    return Ok(Vec::new());
                };

                let mut result = Vec::new();

                let ident_refs = task
                    .snapshot_class(id, &ScriptClass::Ident(CompactString::from(token_string)))
                    .into_module_result(id)?;

                for ident_ref in ident_refs.as_ref() {
                    let Some(ScriptNode::Ident { semantics, .. }) =
                        ident_ref.deref(doc_read.deref())
                    else {
                        continue;
                    };

                    let (_, cross_resolution) = semantics
                        .get()
                        .into_module_result(id)?
                        .cross_resolution
                        .snapshot(task)
                        .into_module_result(id)?;

                    let IdentCrossResolution::Read { name } = cross_resolution else {
                        continue;
                    };

                    let Some(ScriptNode::Root { .. }) = name.as_ref().decl.deref(doc_read.deref())
                    else {
                        continue;
                    };

                    result.push(Self(*ident_ref));
                }

                Ok(result)
            }

            _ => return Ok(Vec::new()),
        }
    }

    /// Returns a type description that formally describes this expression's type.
    ///
    /// See [ModuleSymbol::expr_ty] for details.
    pub fn ty<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleResult<Description> {
        let doc_read = read.read_doc();

        match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::Ident { semantics, .. }) => {
                let id = doc_read.id();

                let ident_semantics = semantics.get().into_module_result(id)?;

                let (_, type_resolution) = ident_semantics
                    .type_resolution
                    .snapshot(read.task())
                    .into_module_result(id)?;

                Ok(Description::from_tag(type_resolution.tag))
            }

            Some(ScriptNode::This { semantics, .. }) => {
                let id = doc_read.id();

                let this_semantics = semantics.get().into_module_result(id)?;

                let (_, type_resolution) = this_semantics
                    .type_resolution
                    .snapshot(read.task())
                    .into_module_result(id)?;

                Ok(Description::from_tag(type_resolution.tag))
            }

            _ => Ok(Description::dynamic()),
        }
    }

    /// Infers the declaration symbol for the identifier.
    ///
    /// The identifier's declaration depends on its [kind](Self::kind) and the
    /// context:
    ///
    /// - If this identifier refers to a variable, the function looks up the
    ///   [VarSymbol] that declares this variable.
    /// - If this identifier refers to a component of an imported package,
    ///   the function looks up the [PackageSymbol] that injects the component
    ///   into the namespace.
    /// - If this is a `self` built-in identifier, the function attempts to
    ///   look up the [StructSymbol] related to the `self` variable.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// declaration lookup requires deep source code analysis, and the analysis
    /// procedure is interrupted by the revocation of the module content access
    /// guard (see [ScriptModule](crate::analysis::ScriptModule) documentation
    /// for details).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax or
    /// semantic errors, if the IdentSymbol is not [valid](Self::is_valid), or if
    /// the declaration site is unclear to the static source code analyzer).
    pub fn declaration<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<ModuleSymbol> {
        let doc_read = read.read_doc();

        let id = doc_read.id();

        match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::Ident { semantics, .. }) => {
                let ident_semantics = semantics.get().into_module_result(id)?;

                let (_, cross_resolution) = ident_semantics
                    .cross_resolution
                    .snapshot(read.task())
                    .into_module_result(id)?;

                match cross_resolution {
                    IdentCrossResolution::Unresolved | IdentCrossResolution::BestMatch { .. } => {
                        Ok(ModuleSymbol::Nil)
                    }

                    IdentCrossResolution::Read { name } => {
                        let Some(decl_node) = name.as_ref().decl.deref(doc_read.deref()) else {
                            return Ok(ModuleSymbol::Nil);
                        };

                        match decl_node {
                            ScriptNode::Use { packages, .. } => {
                                let Some(last) = packages.last() else {
                                    return Ok(ModuleSymbol::Nil);
                                };

                                Ok(ModuleSymbol::Package(PackageSymbol(*last)))
                            }

                            ScriptNode::For { iterator, .. } => {
                                Ok(ModuleSymbol::Var(VarSymbol(*iterator)))
                            }

                            ScriptNode::Let { name, .. } => Ok(ModuleSymbol::Var(VarSymbol(*name))),

                            ScriptNode::FnParams { .. } => {
                                let Some(param_ref) = name.as_ref().defs.iter().next() else {
                                    return Ok(ModuleSymbol::Nil);
                                };

                                Ok(ModuleSymbol::Var(VarSymbol(*param_ref)))
                            }

                            _ => Ok(ModuleSymbol::Nil),
                        }
                    }

                    IdentCrossResolution::Write { decl } => {
                        let Some(ScriptNode::Let { name, .. }) = decl.deref(doc_read.deref())
                        else {
                            return Ok(ModuleSymbol::Nil);
                        };

                        Ok(ModuleSymbol::Var(VarSymbol(*name)))
                    }
                }
            }

            Some(ScriptNode::This { semantics, .. }) => {
                let this_semantics = semantics.get().into_module_result(id)?;

                let (_, type_resolution) = this_semantics
                    .type_resolution
                    .snapshot(read.task())
                    .into_module_result(id)?;

                Ok(type_resolution.tag.into())
            }

            _ => Ok(ModuleSymbol::Nil),
        }
    }

    /// Infers symbols that initialize the value referred to by this
    /// IdentSymbol.
    ///
    /// The identifier's definitions depend on its [kind](Self::kind) and the
    /// context:
    ///
    /// - If this identifier refers to a variable, the function looks up
    ///   the [VarSymbol] or [IdentSymbol]s that initialize the variable's
    ///   value.
    /// - If this identifier refers to a component of an imported package,
    ///   the function looks up the [PackageSymbol] that injects the component
    ///   into the namespace.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// definitions lookup requires deep source code analysis, and the analysis
    /// procedure is interrupted by the revocation of the module content access
    /// guard (see [ScriptModule](crate::analysis::ScriptModule) documentation
    /// for details).
    ///
    /// The function returns an empty vector if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax or
    /// semantic errors, if the IdentSymbol is not [valid](Self::is_valid), or
    /// if the definition sites are unclear to the static source code analyzer).
    pub fn definitions<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Vec<ModuleSymbol>> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Ident { semantics, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let id = doc_read.id();

        let ident_semantics = semantics.get().into_module_result(id)?;

        let (_, cross_resolution) = ident_semantics
            .cross_resolution
            .snapshot(read.task())
            .into_module_result(id)?;

        let IdentCrossResolution::Read { name } = cross_resolution else {
            return Ok(Vec::new());
        };

        let mut result = Vec::with_capacity(name.as_ref().defs.len());

        for def_ref in &name.as_ref().defs {
            let Some(def_node) = def_ref.deref(doc_read.deref()) else {
                continue;
            };

            match def_node {
                ScriptNode::Package { .. } => {
                    result.push(ModuleSymbol::Package(PackageSymbol(*def_ref)));
                }

                ScriptNode::Var { .. } => result.push(ModuleSymbol::Var(VarSymbol(*def_ref))),

                ScriptNode::Expr { parent, .. } => {
                    let Some(parent_node) = parent.deref(doc_read.deref()) else {
                        continue;
                    };

                    match parent_node {
                        ScriptNode::Let { name, .. } => {
                            result.push(ModuleSymbol::Var(VarSymbol(*name)));
                        }

                        ScriptNode::Binary { left, .. } => {
                            let Some(ScriptNode::Ident { .. }) = left.deref(doc_read.deref())
                            else {
                                continue;
                            };

                            result.push(ModuleSymbol::Ident(Self(*left)));
                        }

                        _ => (),
                    }
                }

                _ => (),
            }
        }

        Ok(result)
    }

    /// Returns the parent expression of the identifier if the identifier is an
    /// argument of another expression. Otherwise, returns [ModuleSymbol::Nil]
    /// (e.g., if this IdentSymbol is a top-level expression).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// infer the parent expression (e.g., if the construction has syntax
    /// errors, or if the IdentSymbol is not [valid](Self::is_valid)).
    pub fn parent_expr<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Ident { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(parent_node) = ascend_expr(doc_read.deref(), parent) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(parent_node)
    }
}

/// A type of the [IdentSymbol] construction.
///
/// Returned by the [IdentSymbol::kind] function.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[non_exhaustive]
pub enum IdentKind {
    /// Indicates that the analyzer failed to infer the symbol type
    /// (see [IdentSymbol::kind] for details).
    Invalid,

    /// This identifier refers to the component from the
    /// [ScriptPackage](crate::runtime::ScriptPackage) of the current crate.
    CrateAccess,

    /// This identifier refers to the component from the
    /// [ScriptPackage](crate::runtime::ScriptPackage) imported by the
    /// `use foo;` import statement.
    PackageAccess,

    /// This identifier refers to the value of a variable.
    VarAccess,

    /// This identifier initializes a variable's value.
    VarDefinition,

    /// This identifier is the `crate` built-in keyword that points to the
    /// [ScriptPackage](crate::runtime::ScriptPackage) of the current crate.
    CrateIdent,

    /// This identifier is the `self` built-in keyword that points to the
    /// script function's invocation context.
    SelfIdent,
}

/// A field access operator: `foo.<field>`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldSymbol(NodeRef);

impl From<FieldSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: FieldSymbol) -> Self {
        Self::Field(symbol)
    }
}

impl Identifiable for FieldSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl FieldSymbol {
    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Field { token, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        ScriptOrigin::from(token)
    }

    /// Returns the full source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::expr_outer_origin] for details.
    pub fn outer_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Field { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        outer_origin(doc_read.deref(), *parent)
    }

    /// Returns the name of this field.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns None if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the FieldSymbol is not [valid](Self::is_valid)).
    pub fn name<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Option<String> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Field { token, .. }) = self.0.deref(doc_read.deref()) else {
            return None;
        };

        Some(String::from(token.string(doc_read.deref())?))
    }

    /// Returns a Script identifier of this field.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns None if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the FieldSymbol is not [valid](Self::is_valid)).
    pub fn ident<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Option<ScriptIdent> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Field { token, .. }) = self.0.deref(doc_read.deref()) else {
            return None;
        };

        Some(ScriptIdent::from_string(
            *token,
            String::from(token.string(doc_read.deref())?),
        ))
    }

    /// Looks up all field access operators (including this one)
    /// across the source code that access the same object's field.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// fields lookup requires deep source code analysis, and the analysis
    /// procedure is interrupted by the revocation of the module content access
    /// guard (see [ScriptModule](crate::analysis::ScriptModule) documentation
    /// for details).
    ///
    /// The function returns an empty vector if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax or
    /// semantic errors, or if the FieldSymbol is not [valid](Self::is_valid)).
    pub fn similar_fields<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Vec<Self>> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Field { parent, token, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let Some(ScriptNode::Binary { left, .. }) = parent.deref(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let Some(left_node) = left.deref(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let id = doc_read.id();

        let task = read.task();

        let (_, type_resolution) = left_node
            .type_resolution()
            .into_module_result(id)?
            .snapshot(task)
            .into_module_result(id)?;

        let ty = type_resolution.tag.type_family();

        if ty.is_dynamic() {
            return Ok(Vec::new());
        }

        let Some(token_string) = token.string(doc_read.deref()) else {
            return Ok(Vec::new());
        };

        let field_refs = task
            .snapshot_class(id, &ScriptClass::Field(CompactString::from(token_string)))
            .into_module_result(id)?;

        let mut result = Vec::with_capacity(field_refs.as_ref().len());

        for field_ref in field_refs.as_ref() {
            let Some(ScriptNode::Field { parent, .. }) = field_ref.deref(doc_read.deref()) else {
                continue;
            };

            let Some(ScriptNode::Binary { left, .. }) = parent.deref(doc_read.deref()) else {
                continue;
            };

            let Some(left_node) = left.deref(doc_read.deref()) else {
                continue;
            };

            let (_, type_resolution) = left_node
                .type_resolution()
                .into_module_result(id)?
                .snapshot(task)
                .into_module_result(id)?;

            let other_ty = type_resolution.tag.type_family();

            if ty != other_ty {
                continue;
            }

            result.push(FieldSymbol(*field_ref));
        }

        Ok(result)
    }

    /// Returns a type description that formally describes this expression's type.
    ///
    /// See [ModuleSymbol::expr_ty] for details.
    pub fn ty<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleResult<Description> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Field { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let Some(ScriptNode::Binary { semantics, .. }) = parent.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let id = doc_read.id();

        let binary_semantics = semantics.get().into_module_result(id)?;

        let (_, type_resolution) = binary_semantics
            .type_resolution
            .snapshot(read.task())
            .into_module_result(id)?;

        Ok(Description::from_tag(type_resolution.tag))
    }

    /// Returns the script struct entry symbol where this field has been
    /// declared if the field refers to an entry of the script structure.
    /// Otherwise, returns None.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function may return an
    /// [Interrupted](crate::analysis::ModuleError::Interrupted) error if the
    /// script structure entry lookup requires deep source code analysis, and
    /// the analysis procedure is interrupted by the revocation of the module
    /// content access guard (see [ScriptModule](crate::analysis::ScriptModule)
    /// documentation for details).
    ///
    /// The function returns None if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax or
    /// semantic errors, if the FieldSymbol is not [valid](Self::is_valid), or
    /// if the FieldSymbol is not a field of the script structure).
    pub fn declaration<H: TaskHandle>(
        &self,
        read: &impl ModuleRead<H>,
    ) -> ModuleResult<Option<EntrySymbol>> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Field { parent, token, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(None);
        };

        let Some(field_string) = token.string(doc_read.deref()) else {
            return Ok(None);
        };

        let Some(ScriptNode::Binary { left, .. }) = parent.deref(doc_read.deref()) else {
            return Ok(None);
        };

        let Some(left_node) = left.deref(doc_read.deref()) else {
            return Ok(None);
        };

        let id = doc_read.id();

        let (_, type_resolution) = left_node
            .type_resolution()
            .into_module_result(id)?
            .snapshot(read.task())
            .into_module_result(id)?;

        let Tag::Struct(struct_ref) = type_resolution.tag else {
            return Ok(None);
        };

        let Some(ScriptNode::Struct { semantics, .. }) = struct_ref.deref(doc_read.deref()) else {
            return Ok(None);
        };

        let struct_semantics = semantics.get().into_module_result(id)?;

        let (_, struct_entries_map_syntax) = struct_semantics
            .struct_entries_map_syntax
            .snapshot(read.task())
            .into_module_result(id)?;

        let Some((struct_key_ref, _)) = struct_entries_map_syntax.as_ref().map.get(field_string)
        else {
            return Ok(None);
        };

        Ok(Some(EntrySymbol(*struct_key_ref)))
    }

    /// Returns the parent expression of the field access operator
    /// if the identifier is an argument of another expression. Otherwise,
    /// returns [ModuleSymbol::Nil] (e.g., if this FieldSymbol is a top-level
    /// expression).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// infer the parent expression (e.g., if the construction has syntax
    /// errors, or if the IdentSymbol is not [valid](Self::is_valid)).
    pub fn parent_expr<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Field { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(ScriptNode::Binary { parent, .. }) = parent.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(parent_node) = ascend_expr(doc_read.deref(), parent) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(parent_node)
    }
}

/// Any literal within the expression: `100`, or `true`, or `"string"`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LiteralSymbol(NodeRef);

impl From<LiteralSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: LiteralSymbol) -> Self {
        Self::Literal(symbol)
    }
}

impl Identifiable for LiteralSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl LiteralSymbol {
    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns a description of the literal: whether it is a number, string,
    /// or boolean literal.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [LiteralKind::Invalid] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the VarSymbol is not [valid](Self::is_valid)).
    pub fn kind<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> LiteralKind {
        let doc_read = read.read_doc();

        match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::Number { .. } | ScriptNode::Max { .. }) => LiteralKind::Number,
            Some(ScriptNode::Bool { .. }) => LiteralKind::Bool,
            Some(ScriptNode::String { .. }) => LiteralKind::String,
            _ => LiteralKind::Invalid,
        }
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        self.0.script_origin(doc_read.deref(), SpanBounds::Cover)
    }

    /// Returns the full source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::expr_outer_origin] for details.
    pub fn outer_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        outer_origin(doc_read.deref(), self.0)
    }

    /// Returns a type description that formally describes this expression's type.
    ///
    /// See [ModuleSymbol::expr_ty] for details.
    pub fn ty<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Description {
        let doc_read = read.read_doc();

        match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::Number { .. }) => Description::number_family(*self),
            Some(ScriptNode::Bool { .. }) => Description::bool_type(*self),
            Some(ScriptNode::String { .. }) => Description::string_family(*self),
            _ => Description::dynamic(),
        }
    }

    /// Returns the parent expression of the literal, if the literal is an
    /// argument of another expression. Otherwise, returns [ModuleSymbol::Nil]
    /// (e.g., if this LiteralSymbol is a top-level expression).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// infer the parent expression (e.g., if the construction has syntax
    /// errors, or if the LiteralSymbol is not [valid](Self::is_valid)).
    pub fn parent_expr<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let parent_ref = match self.0.deref(doc_read.deref()) {
            Some(ScriptNode::Number { parent, .. }) => parent,
            Some(ScriptNode::Bool { parent, .. }) => parent,
            Some(ScriptNode::String { parent, .. }) => parent,
            _ => return ModuleSymbol::Nil,
        };

        let Some(parent_node) = ascend_expr(doc_read.deref(), parent_ref) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(parent_node)
    }
}

/// A type of the [LiteralSymbol] construction.
///
/// Returned by the [LiteralSymbol::kind] function.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[non_exhaustive]
pub enum LiteralKind {
    /// Indicates that the analyzer failed to infer the symbol type
    /// (see [LiteralSymbol::kind] for details).
    Invalid,

    /// The LiteralSymbol represents a numeric value: `123`.
    Number,

    /// The LiteralSymbol represents a boolean value: `true` or `false`.
    Bool,

    /// The LiteralSymbol represents a string value: `"foo bar"`.
    String,
}

/// Any binary or unary operator in the expression: `10 + 20` or `!false`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperatorSymbol(NodeRef);

impl From<OperatorSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: OperatorSymbol) -> Self {
        Self::Operator(symbol)
    }
}

impl Identifiable for OperatorSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl OperatorSymbol {
    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Op { token, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        ScriptOrigin::from(token)
    }

    /// Returns the full source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::expr_outer_origin] for details.
    pub fn outer_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        outer_origin(doc_read.deref(), self.0)
    }

    /// Returns a description of this operation, indicating whether it has
    /// left- and right-hand sides.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [LiteralKind::Invalid] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the VarSymbol is not [valid](Self::is_valid)).
    pub fn operation_pattern<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> OperationPattern {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Op { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return OperationPattern::default();
        };

        match parent.deref(doc_read.deref()) {
            Some(ScriptNode::UnaryLeft { .. }) => OperationPattern {
                lhs: true,
                rhs: false,
            },

            Some(ScriptNode::Binary { .. }) => OperationPattern {
                lhs: true,
                rhs: true,
            },

            Some(ScriptNode::Query { .. }) => OperationPattern {
                lhs: false,
                rhs: true,
            },

            _ => OperationPattern::default(),
        }
    }

    /// Returns a type description that formally describes this expression's type.
    ///
    /// See [ModuleSymbol::expr_ty] for details.
    pub fn ty<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleResult<Description> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Op { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let Some(operator_node) = parent.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let id = doc_read.id();

        let (_, type_resolution) = operator_node
            .type_resolution()
            .into_module_result(id)?
            .snapshot(read.task())
            .into_module_result(id)?;

        Ok(Description::from_tag(type_resolution.tag))
    }

    /// Returns an expression symbol that points to the left-hand side (LHS) of
    /// this operation. If the operation does not have an LHS, returns
    /// [ModuleSymbol::Nil].
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the OperatorSymbol is not [valid](Self::is_valid)).
    pub fn lhs<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Op { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let operand_ref = match parent.deref(doc_read.deref()) {
            Some(ScriptNode::Binary { left, .. }) if left.is_valid_ref(doc_read.deref()) => left,
            Some(ScriptNode::Query { left, .. }) if left.is_valid_ref(doc_read.deref()) => left,
            _ => return ModuleSymbol::Nil,
        };

        let Some(operator_node) = descend_expr(doc_read.deref(), operand_ref) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(operator_node)
    }

    /// Returns an expression symbol that points to the right-hand side (RHS) of
    /// this operation. If the operation does not have an RHS, returns
    /// [ModuleSymbol::Nil].
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the OperatorSymbol is not [valid](Self::is_valid)).
    pub fn rhs<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Op { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let operand_ref = match parent.deref(doc_read.deref()) {
            Some(ScriptNode::UnaryLeft { right, .. }) if right.is_valid_ref(doc_read.deref()) => {
                right
            }

            Some(ScriptNode::Binary { right, .. }) if right.is_valid_ref(doc_read.deref()) => right,

            _ => return ModuleSymbol::Nil,
        };

        let Some(operator_node) = descend_expr(doc_read.deref(), operand_ref) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(operator_node)
    }

    /// Returns the parent expression of the operation if the operation is an
    /// argument of another expression. Otherwise, returns [ModuleSymbol::Nil]
    /// (e.g., if this OperatorSymbol is a top-level expression).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// infer the parent expression (e.g., if the construction has syntax
    /// errors, or if the OperatorSymbol is not [valid](Self::is_valid)).
    pub fn parent_expr<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Op { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(parent_node) = ascend_expr(doc_read.deref(), parent) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(parent_node)
    }
}

/// A description of the [OperatorSymbol] construction.
///
/// Returned by the [OperatorSymbol::operation_pattern] function.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperationPattern {
    /// The operation has a left-hand side: `a + b` or `a?`.
    pub lhs: bool,

    /// The operation has a right-hand side: `a + b` or `!a`.
    pub rhs: bool,
}

/// An invocation operator: `foo(a, b, c)`.
///
/// The invocation is the content surrounded by the parentheses, including the
/// parentheses themselves.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallSymbol(NodeRef);

impl From<CallSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: CallSymbol) -> Self {
        Self::Call(symbol)
    }
}

impl Identifiable for CallSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl CallSymbol {
    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Call { args, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        args.script_origin(doc_read.deref(), SpanBounds::Cover)
    }

    /// Returns the full source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::expr_outer_origin] for details.
    pub fn outer_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        outer_origin(doc_read.deref(), self.0)
    }

    /// Returns a type description that formally describes this expression's type.
    ///
    /// See [ModuleSymbol::expr_ty] for details.
    pub fn ty<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleResult<Description> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Call { semantics, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let id = doc_read.id();

        let call_semantics = semantics.get().into_module_result(id)?;

        let (_, type_resolution) = call_semantics
            .type_resolution
            .snapshot(read.task())
            .into_module_result(id)?;

        Ok(Description::from_tag(type_resolution.tag))
    }

    /// Returns the parent expression of the invocation if the invocation is an
    /// argument of another expression. Otherwise, returns [ModuleSymbol::Nil]
    /// (e.g., if this CallSymbol is a top-level expression).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// infer the parent expression (e.g., if the construction has syntax
    /// errors, or if the CallSymbol is not [valid](Self::is_valid)).
    pub fn parent_expr<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Call { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(parent_node) = ascend_expr(doc_read.deref(), parent) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(parent_node)
    }

    /// Returns the expression symbol on which this invocation is applied:
    /// `<receiver>(a, b, c)`.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the CallSymbol is not [valid](Self::is_valid)).
    pub fn receiver<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Call { left, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(left_node) = descend_expr(doc_read.deref(), left) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(left_node)
    }

    /// Returns a list of invocation arguments:
    /// `foo(<comma separated arguments>)`.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns an empty vector if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the CallSymbol is not [valid](Self::is_valid)).
    pub fn args<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> Vec<ModuleSymbol> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Call { args, .. }) = self.0.deref(doc_read.deref()) else {
            return Vec::new();
        };

        let Some(ScriptNode::CallArgs { args, .. }) = args.deref(doc_read.deref()) else {
            return Vec::new();
        };

        let mut result = Vec::with_capacity(args.len());

        for arg_ref in args {
            let Some(arg_node) = descend_expr(doc_read.deref(), arg_ref) else {
                result.push(ModuleSymbol::Nil);
                continue;
            };

            result.push(ModuleSymbol::from_expr_node(arg_node))
        }

        result
    }
}

/// An index operator: `foo[10]` or `foo[10..20]`.
///
/// The index is the content surrounded by the parentheses, including the
/// parentheses themselves.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IndexSymbol(NodeRef);

impl From<IndexSymbol> for ModuleSymbol {
    #[inline(always)]
    fn from(symbol: IndexSymbol) -> Self {
        Self::Index(symbol)
    }
}

impl Identifiable for IndexSymbol {
    #[inline(always)]
    fn id(&self) -> Id {
        self.0.id
    }
}

impl IndexSymbol {
    /// Returns true if this symbol still exists in the
    /// [ScriptModule](crate::analysis::ScriptModule).
    ///
    /// See [ModuleSymbol::is_valid] for details.
    pub fn is_valid<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> bool {
        let doc_read = read.read_doc();

        self.0.is_valid_ref(doc_read.deref())
    }

    /// Returns the source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::origin] for details.
    pub fn origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Index { arg, .. }) = self.0.deref(doc_read.deref()) else {
            return ScriptOrigin::nil();
        };

        arg.script_origin(doc_read.deref(), SpanBounds::Cover)
    }

    /// Returns the full source code range of the underlying symbol.
    ///
    /// See [ModuleSymbol::expr_outer_origin] for details.
    pub fn outer_origin<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ScriptOrigin {
        let doc_read = read.read_doc();

        outer_origin(doc_read.deref(), self.0)
    }

    /// Returns a type description that formally describes this expression's type.
    ///
    /// See [ModuleSymbol::expr_ty] for details.
    pub fn ty<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleResult<Description> {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Index { semantics, .. }) = self.0.deref(doc_read.deref()) else {
            return Ok(Description::dynamic());
        };

        let id = doc_read.id();

        let index_semantics = semantics.get().into_module_result(id)?;

        let (_, type_resolution) = index_semantics
            .type_resolution
            .snapshot(read.task())
            .into_module_result(id)?;

        Ok(Description::from_tag(type_resolution.tag))
    }

    /// Returns the parent expression of the index operation if the index
    /// operation is an argument of another expression. Otherwise, returns
    /// [ModuleSymbol::Nil] (e.g., if this IndexSymbol is a top-level
    /// expression).
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// infer the parent expression (e.g., if the construction has syntax
    /// errors, or if the IndexSymbol is not [valid](Self::is_valid)).
    pub fn parent_expr<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Index { parent, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(parent_node) = ascend_expr(doc_read.deref(), parent) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(parent_node)
    }

    /// Returns the expression symbol on which this index operation is applied:
    /// `<receiver>[idx]`.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the IndexSymbol is not [valid](Self::is_valid)).
    pub fn receiver<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Index { left, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(left_node) = descend_expr(doc_read.deref(), left) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(left_node)
    }

    /// Returns the index argument: `foo[<argument>]`.
    ///
    /// The `read` argument can be any content access guard object, such as
    /// a [ModuleReadGuard](crate::analysis::ModuleReadGuard) or
    /// a [ModuleWriteGuard](crate::analysis::ModuleWriteGuard)).
    ///
    /// The function returns [ModuleSymbol::Nil] if the analyzer fails to
    /// resolve this construction (e.g., if the construction has syntax errors,
    /// or if the IndexSymbol is not [valid](Self::is_valid)).
    pub fn arg<H: TaskHandle>(&self, read: &impl ModuleRead<H>) -> ModuleSymbol {
        let doc_read = read.read_doc();

        let Some(ScriptNode::Index { arg, .. }) = self.0.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(ScriptNode::IndexArg { arg, .. }) = arg.deref(doc_read.deref()) else {
            return ModuleSymbol::Nil;
        };

        let Some(arg_node) = descend_expr(doc_read.deref(), arg) else {
            return ModuleSymbol::Nil;
        };

        ModuleSymbol::from_expr_node(arg_node)
    }
}

/// A symbol lookup filter options.
///
/// This object is an argument of the [ModuleRead::symbols] function.
///
/// You can construct this object in the builder style:
/// `LookupOptions::new().filter(SymbolKind::Break as u32 | SymbolKind::Ident as u32).outer()`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub struct LookupOptions {
    /// A bit mask of the symbols that should be included in the result.
    ///
    /// This mask is a union of the numeric discriminants of the [SymbolKind].
    /// For example, `SymbolKind::Break | SymbolKind::Ident` means all break
    /// statements and identifier expressions.
    ///
    /// By default, all bits are set (the [Default] value is `!0`).
    pub symbols_mask: u32,

    /// If set to false, the [ModuleRead::symbols] function only looks for
    /// symbols that are fully covered by the specified source code range.
    ///
    /// Otherwise, it also includes symbols that are fully or partially
    /// intersected by the range.
    ///
    /// This flag is disabled by [Default].
    pub include_outer: bool,
}

impl Default for LookupOptions {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl LookupOptions {
    /// The default constructor of the object.
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            symbols_mask: !0,
            include_outer: false,
        }
    }

    /// Limits the set of lookup symbols to the specified bit `mask`. This
    /// function intersects the current `symbols_mask` with the `mask` argument
    /// value.
    ///
    /// By default, the `symbols_mask` includes all symbol bits.
    #[inline(always)]
    pub fn filter(mut self, mask: u32) -> Self {
        self.symbols_mask = self.symbols_mask & mask;

        self
    }

    /// Sets the `include_outer` flag to true.
    ///
    /// By default, this flag is set to false.
    #[inline(always)]
    pub fn outer(mut self) -> Self {
        self.include_outer = true;

        self
    }
}

pub(super) struct SymbolsLookup<'a> {
    doc: &'a ScriptDoc,
    span: SiteSpan,
    mask: u32,
    symbols: Vec<ModuleSymbol>,
}

impl<'a> Visitor for SymbolsLookup<'a> {
    fn visit_token(&mut self, _token_ref: &TokenRef) {}

    fn enter_node(&mut self, node_ref: &NodeRef) -> bool {
        let Some(script_node) = node_ref.deref(self.doc) else {
            return false;
        };

        let Some(node_span) = script_node.span(self.doc) else {
            return false;
        };

        if !self.touches(&node_span) {
            return false;
        }

        match script_node {
            ScriptNode::InlineComment { .. } => (),
            ScriptNode::MultilineComment { .. } => (),
            ScriptNode::Root { .. } => (),
            ScriptNode::Clause { .. } => (),
            ScriptNode::Use { keyword, .. } => self.visit(SymbolKind::Use, node_ref, keyword),
            ScriptNode::Package { token, .. } => self.visit(SymbolKind::Package, node_ref, token),
            ScriptNode::If { .. } => (),
            ScriptNode::Match { .. } => (),
            ScriptNode::MatchBody { .. } => (),
            ScriptNode::MatchArm { .. } => (),
            ScriptNode::Else { .. } => (),
            ScriptNode::Let { .. } => (),
            ScriptNode::Var { token, .. } => self.visit(SymbolKind::Var, node_ref, token),
            ScriptNode::For { keyword, .. } => self.visit(SymbolKind::Loop, node_ref, keyword),
            ScriptNode::Loop { keyword, .. } => self.visit(SymbolKind::Loop, node_ref, keyword),
            ScriptNode::Block { .. } => (),
            ScriptNode::Break { keyword, .. } => self.visit(SymbolKind::Break, node_ref, keyword),
            ScriptNode::Continue { keyword, .. } => {
                self.visit(SymbolKind::Break, node_ref, keyword)
            }
            ScriptNode::Return { keyword, .. } => self.visit(SymbolKind::Return, node_ref, keyword),
            ScriptNode::Fn { keyword, .. } => self.visit(SymbolKind::Fn, node_ref, keyword),
            ScriptNode::FnParams { .. } => (),
            ScriptNode::Struct { keyword, .. } => self.visit(SymbolKind::Struct, node_ref, keyword),
            ScriptNode::StructBody { .. } => (),
            ScriptNode::StructEntry { .. } => (),
            ScriptNode::StructEntryKey { token, .. } => {
                self.visit(SymbolKind::Entry, node_ref, token)
            }
            ScriptNode::Array { start, end, .. } => self.visit_array(node_ref, start, end),
            ScriptNode::String { start, end, .. } => self.visit_string(node_ref, start, end),
            ScriptNode::Crate { token, .. } => self.visit(SymbolKind::Ident, node_ref, token),
            ScriptNode::This { token, .. } => self.visit(SymbolKind::Ident, node_ref, token),
            ScriptNode::Ident { token, .. } => self.visit(SymbolKind::Ident, node_ref, token),
            ScriptNode::Number { token, .. } => self.visit(SymbolKind::Literal, node_ref, token),
            ScriptNode::Max { token, .. } => self.visit(SymbolKind::Literal, node_ref, token),
            ScriptNode::Bool { token, .. } => self.visit(SymbolKind::Literal, node_ref, token),
            ScriptNode::UnaryLeft { .. } => (),
            ScriptNode::Binary { .. } => (),
            ScriptNode::Op { token, .. } => self.visit_operator(node_ref, token),
            ScriptNode::Query { .. } => (),
            ScriptNode::Call { args, .. } => self.visit_call(node_ref, args),
            ScriptNode::CallArgs { .. } => (),
            ScriptNode::Index { arg, .. } => self.visit_index(node_ref, arg),
            ScriptNode::IndexArg { .. } => (),
            ScriptNode::Field { token, .. } => self.visit(SymbolKind::Field, node_ref, token),
            ScriptNode::Expr { .. } => (),
        }

        true
    }

    fn leave_node(&mut self, _node_ref: &NodeRef) {}
}

impl<'a> SymbolsLookup<'a> {
    pub(super) fn lookup(
        doc: &'a ScriptDoc,
        span: SiteSpan,
        options: LookupOptions,
    ) -> Vec<ModuleSymbol> {
        let from = match span.start > 0 {
            true => span.start - 1,
            false => span.start,
        };

        let to = match span.end < doc.length() {
            true => span.end + 1,
            false => span.end,
        };

        let subtree = match options.include_outer {
            false => doc.cover(from..to),
            true => doc.root_node_ref(),
        };

        let mut symbols_lookup = SymbolsLookup {
            doc,
            span,
            mask: options.symbols_mask,
            symbols: Vec::new(),
        };

        doc.traverse_subtree(&subtree, &mut symbols_lookup);

        symbols_lookup.symbols
    }

    fn visit(&mut self, kind: SymbolKind, node_ref: &NodeRef, token_ref: &TokenRef) {
        if self.mask & (kind as u32) == 0 {
            return;
        }

        if !self.touches_token(token_ref) {
            return;
        }

        self.symbols.push(ModuleSymbol::new(kind, node_ref));
    }

    fn visit_operator(&mut self, node_ref: &NodeRef, token_ref: &TokenRef) {
        if self.mask & (SymbolKind::Operator as u32) == 0 {
            return;
        }

        let Some(token) = token_ref.deref(self.doc) else {
            return;
        };

        if token == ScriptToken::Dot {
            return;
        }

        if !self.touches_token(token_ref) {
            return;
        }

        self.symbols
            .push(ModuleSymbol::Operator(OperatorSymbol(*node_ref)));
    }

    fn visit_call(&mut self, node_ref: &NodeRef, args: &NodeRef) {
        if self.mask & (SymbolKind::Call as u32) == 0 {
            return;
        }

        let Some(ScriptNode::CallArgs { start, end, .. }) = args.deref(self.doc) else {
            return;
        };

        if !self.touches_token_span(start, end) {
            return;
        }

        self.symbols.push(ModuleSymbol::Call(CallSymbol(*node_ref)));
    }

    fn visit_index(&mut self, node_ref: &NodeRef, arg: &NodeRef) {
        if self.mask & (SymbolKind::Index as u32) == 0 {
            return;
        }

        let Some(ScriptNode::IndexArg { start, end, .. }) = arg.deref(self.doc) else {
            return;
        };

        if !self.touches_token_span(start, end) {
            return;
        }

        self.symbols
            .push(ModuleSymbol::Index(IndexSymbol(*node_ref)));
    }

    fn visit_string(&mut self, node_ref: &NodeRef, start: &TokenRef, end: &TokenRef) {
        if self.mask & (SymbolKind::Literal as u32) == 0 {
            return;
        }

        if !self.touches_token_span(start, end) {
            return;
        }

        self.symbols
            .push(ModuleSymbol::Literal(LiteralSymbol(*node_ref)));
    }

    fn visit_array(&mut self, node_ref: &NodeRef, start: &TokenRef, end: &TokenRef) {
        if self.mask & (SymbolKind::Array as u32) == 0 {
            return;
        }

        if !self.touches_token_span(start, end) {
            return;
        }

        self.symbols
            .push(ModuleSymbol::Array(ArraySymbol(*node_ref)));
    }

    #[inline(always)]
    fn touches(&self, span: &SiteSpan) -> bool {
        if self.span.start > span.end {
            return false;
        }

        if self.span.end < span.start {
            return false;
        }

        true
    }

    #[inline(always)]
    fn touches_token(&self, token_ref: &TokenRef) -> bool {
        let Some(span) = self.token_span(token_ref) else {
            return false;
        };

        self.touches(&span)
    }

    #[inline(always)]
    fn touches_token_span(&self, start: &TokenRef, end: &TokenRef) -> bool {
        let Some(start_site) = start.site(self.doc) else {
            return false;
        };

        let Some(end_span) = self.token_span(end) else {
            return false;
        };

        self.touches(&(start_site..end_span.end))
    }

    #[inline(always)]
    fn token_span(&self, token_ref: &TokenRef) -> Option<SiteSpan> {
        let site = token_ref.site(self.doc)?;
        let length = token_ref.length(self.doc)?;

        Some(site..(site + length))
    }
}

#[inline(always)]
fn outer_origin(doc: &ScriptDoc, mut expr: NodeRef) -> ScriptOrigin {
    loop {
        let parent = expr.parent(doc);

        if parent.rule(doc) == ScriptNode::EXPR {
            expr = parent;
            continue;
        }

        return expr.script_origin(doc, SpanBounds::Cover);
    }
}

#[inline(always)]
fn ascend_expr<'a>(doc: &'a ScriptDoc, mut expr: &'a NodeRef) -> Option<&'a ScriptNode> {
    loop {
        let Some(script_node) = expr.deref(doc) else {
            return None;
        };

        match script_node {
            ScriptNode::Expr { parent, .. } => {
                expr = parent;
            }

            _ => return Some(script_node),
        }
    }
}

#[inline(always)]
fn descend_expr<'a>(doc: &'a ScriptDoc, mut expr: &'a NodeRef) -> Option<&'a ScriptNode> {
    loop {
        let Some(script_node) = expr.deref(doc) else {
            return None;
        };

        match script_node {
            ScriptNode::Expr { inner, .. } => {
                expr = inner;
            }

            _ => return Some(script_node),
        }
    }
}
