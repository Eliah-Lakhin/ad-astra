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
    borrow::Cow,
    fmt::{Display, Formatter},
    ops::Range,
};

use compact_str::CompactString;
use lady_deirdre::{
    format::AnnotationPriority,
    syntax::{ErrorRef, NodeRef, NodeRule, RecoveryResult},
};

use crate::{
    analysis::DiagnosticsDepth,
    runtime::{ops::OperatorKind, ScriptOrigin, ScriptType, TypeFamily, TypeHint, TypeMeta},
    syntax::{PolyRefOrigin, ScriptDoc, ScriptNode, SpanBounds},
};

/// A classification of module diagnostic issues.
///
/// Each [ModuleIssue](crate::analysis::ModuleIssue) object belongs to a specific
/// class, as described by this enum type.
///
/// From an IssueCode, you can obtain additional metadata about the diagnostic
/// issue, such as a short description (via the Display implementation of
/// IssueCode) or the severity of the issue.
///
/// Additionally, you can convert an IssueCode into a numeric representation,
/// which can be used to categorize diagnostics by their numeric codes in a
/// code editor.
///
/// The numeric representation is in the XYY decimal digits format, where X
/// represents the issue's [diagnostics depth](DiagnosticsDepth), and YY
/// represents the issue's "sub-code" within that depth level.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u16)]
#[non_exhaustive]
pub enum IssueCode {
    /// Syntax Error.
    ///
    /// This error indicates that the source code is not well-formed from a
    /// syntactical point of view. These types of errors must be addressed
    /// first.
    Parse = 101,

    /// Semantics Error.
    ///
    /// The specified package in the `use <package>;` import statement is
    /// unknown to the script engine.
    UnresolvedPackage = 201,
    /// Semantics Error.
    ///
    /// The component in the `use foo.<component>;` import statement is not a
    /// valid package.
    NotAPackage = 202,
    /// Semantics Error.
    ///
    /// The `break` or `continue` statement is used outside of a loop.
    OrphanedBreak = 203,
    /// Semantics Error.
    ///
    /// The function already has a parameter with the same name. Function
    /// parameter names must be unique.
    DuplicateParam = 204,
    /// Semantics Error.
    ///
    /// An attempt to use a variable that is not initialized at this point in
    /// the control flow.
    ReadUninit = 205,
    /// Semantics Error.
    ///
    /// An attempt to use an identifier that does not correspond to any known
    /// variable within this scope or a package symbol.
    UnresolvedIdent = 206,
    /// Semantics Error.
    ///
    /// Invalid integer literal format.
    IntParse = 207,
    /// Semantics Error.
    ///
    /// Invalid floating-point literal format.
    FloatParse = 208,
    /// Semantics Warning.
    ///
    /// This statement is not reachable during the execution of the source code.
    /// It is considered dead code. Consider removing this statement or
    /// commenting it out.
    UnreachableStatement = 209,
    /// Semantics Warning.
    ///
    /// This match arm is not reachable during the execution of the source code.
    /// It is considered dead code. Consider removing this match arm or
    /// commenting it out.
    UnreachableArm = 210,
    /// Semantics Warning.
    ///
    /// The `struct` declaration already contains a field with the same name.
    DuplicateEntry = 211,
    /// Semantics Warning.
    ///
    /// An attempt to assign to an orphaned literal. This assignment is
    /// semantically meaningless.
    LiteralAssignment = 212,

    /// Semantics Warning.
    ///
    /// The provided type does not meet the formal requirements of the
    /// specification.
    TypeMismatch = 301,
    /// Semantics Warning.
    ///
    /// An attempt to index into a nil object.
    NilIndex = 302,
    /// Semantics Warning.
    ///
    /// An attempt to index by an expression that is neither a number nor a
    /// range of integers.
    IndexTypeMismatch = 303,
    /// Semantics Warning.
    ///
    /// The object does not implement the specified operator.
    UndefinedOperator = 304,
    /// Semantics Warning.
    ///
    /// The object does not implement the Display operator.
    UndefinedDisplay = 305,
    /// Semantics Warning.
    ///
    /// The function requires either more or fewer arguments.
    CallArityMismatch = 306,
    /// Semantics Warning.
    ///
    /// Expected a function with a different number of parameters.
    FnArityMismatch = 307,
    /// Semantics Warning.
    ///
    /// The function's result type is different from the expected type.
    ResultMismatch = 308,
    /// Semantics Warning.
    ///
    /// The object does not have the specified field.
    UnknownComponent = 309,
    /// Semantics Warning.
    ///
    /// The function has inconsistent return points. Some branches return
    /// non-nil values, while others return nil values. This issue likely
    /// indicates that a trailing `return <expr>;` statement is missing.
    InconsistentReturns = 310,
}

impl Display for IssueCode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Parse => "Parse error.",

            Self::UnresolvedPackage => "Unresolved import.",
            Self::NotAPackage => "Importing a component that is not a package.",
            Self::OrphanedBreak => "Break outside of a loop.",
            Self::DuplicateParam => "Duplicate function parameter name.",
            Self::ReadUninit => "Use of possibly uninitialized variable.",
            Self::UnresolvedIdent => "Unresolved reference.",
            Self::IntParse => "Invalid integer literal.",
            Self::FloatParse => "Invalid float literal.",
            Self::UnreachableStatement => "Unreachable statement.",
            Self::UnreachableArm => "Unreachable match arm.",
            Self::DuplicateEntry => "Duplicate struct entry.",
            Self::LiteralAssignment => "Assignment to literal is meaningless.",

            Self::TypeMismatch => "Type mismatch.",
            Self::NilIndex => "Index operator is not applicable to nil type.",
            Self::IndexTypeMismatch => "Index type must be an integer or an integer range.",
            Self::UndefinedOperator => "Unresolved operator.",
            Self::UndefinedDisplay => "Type does not implement the Display operator.",
            Self::CallArityMismatch => "Call arity mismatch.",
            Self::FnArityMismatch => "Function arity mismatch.",
            Self::ResultMismatch => "Function result type mismatch.",
            Self::UnknownComponent => "Unknown field.",
            Self::InconsistentReturns => "Missing trailing return statement.",
        };

        formatter.write_str(message)
    }
}

impl IssueCode {
    /// Returns the issue's [severity](IssueSeverity), which is either an error
    /// or a warning.
    #[inline(always)]
    pub fn severity(self) -> IssueSeverity {
        match self {
            Self::Parse => IssueSeverity::Error,

            Self::UnresolvedPackage => IssueSeverity::Error,
            Self::NotAPackage => IssueSeverity::Error,
            Self::OrphanedBreak => IssueSeverity::Error,
            Self::DuplicateParam => IssueSeverity::Error,
            Self::ReadUninit => IssueSeverity::Error,
            Self::UnresolvedIdent => IssueSeverity::Error,
            Self::IntParse => IssueSeverity::Error,
            Self::FloatParse => IssueSeverity::Error,
            Self::UnreachableStatement => IssueSeverity::Warning,
            Self::UnreachableArm => IssueSeverity::Warning,
            Self::DuplicateEntry => IssueSeverity::Warning,
            Self::LiteralAssignment => IssueSeverity::Warning,

            Self::TypeMismatch => IssueSeverity::Warning,
            Self::NilIndex => IssueSeverity::Warning,
            Self::IndexTypeMismatch => IssueSeverity::Warning,
            Self::UndefinedOperator => IssueSeverity::Warning,
            Self::UndefinedDisplay => IssueSeverity::Warning,
            Self::CallArityMismatch => IssueSeverity::Warning,
            Self::FnArityMismatch => IssueSeverity::Warning,
            Self::ResultMismatch => IssueSeverity::Warning,
            Self::UnknownComponent => IssueSeverity::Warning,
            Self::InconsistentReturns => IssueSeverity::Warning,
        }
    }

    /// Returns the level of semantic analysis depth at which this issue was inferred.
    ///
    /// See [DiagnosticsDepth] for details.
    #[inline(always)]
    pub fn depth(self) -> DiagnosticsDepth {
        (self as u16 / 100) as DiagnosticsDepth
    }
}

/// The severity level of the source code diagnostics.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u8)]
pub enum IssueSeverity {
    /// Hard errors. The analyzer is quite confident that these kinds of
    /// diagnostic issues would lead to runtime errors.
    Error = 1 << 0,

    /// These types of diagnostic issues are likely to lead to runtime problems.
    /// However, the analyzer is not confident enough to classify them as hard
    /// errors due to the dynamic nature of the script execution model.
    Warning = 1 << 1,
}

impl Display for IssueSeverity {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueSeverity::Error => formatter.write_str("error"),
            IssueSeverity::Warning => formatter.write_str("warning"),
        }
    }
}

impl IssueSeverity {
    #[inline(always)]
    pub(crate) fn priority(self) -> AnnotationPriority {
        match self {
            Self::Error => AnnotationPriority::Primary,
            Self::Warning => AnnotationPriority::Note,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum ScriptIssue {
    Parse {
        error_ref: ErrorRef,
    },

    UnresolvedPackage {
        base: &'static TypeMeta,
        package_ref: NodeRef,
        quickfix: CompactString,
    },

    NotAPackage {
        ty: TypeHint,
        package_ref: NodeRef,
    },

    OrphanedBreak {
        break_ref: NodeRef,
    },

    DuplicateParam {
        var_ref: NodeRef,
    },

    ReadUninit {
        ident_ref: NodeRef,
    },

    UnresolvedIdent {
        ident_ref: NodeRef,
        quickfix: CompactString,
        import: CompactString,
    },

    IntParse {
        number_ref: NodeRef,
    },

    FloatParse {
        number_ref: NodeRef,
    },

    UnreachableStatement {
        st_ref: NodeRef,
    },

    UnreachableArm {
        arm_ref: NodeRef,
    },

    DuplicateEntry {
        entry_key_ref: NodeRef,
    },

    LiteralAssignment {
        op_ref: NodeRef,
    },

    TypeMismatch {
        expr_ref: NodeRef,
        expected: &'static TypeFamily,
        provided: &'static TypeFamily,
    },

    NilIndex {
        op_ref: NodeRef,
    },

    IndexTypeMismatch {
        arg_ref: NodeRef,
        provided: &'static TypeFamily,
    },

    UndefinedOperator {
        op_ref: NodeRef,
        op: OperatorKind,
        receiver: &'static TypeMeta,
    },

    CallArityMismatch {
        args_ref: NodeRef,
        expected: usize,
        provided: usize,
    },

    FnArityMismatch {
        arg_ref: NodeRef,
        expected: usize,
        provided: usize,
    },

    ResultMismatch {
        arg_ref: NodeRef,
        expected: &'static TypeFamily,
        provided: &'static TypeFamily,
    },

    UnknownComponent {
        field_ref: NodeRef,
        receiver: &'static TypeMeta,
        quickfix: CompactString,
    },

    InconsistentReturns {
        fn_ref: NodeRef,
    },
}

impl ScriptIssue {
    pub(crate) fn code(&self) -> IssueCode {
        match self {
            Self::Parse { .. } => IssueCode::Parse,
            Self::UnresolvedPackage { .. } => IssueCode::UnresolvedPackage,
            Self::NotAPackage { .. } => IssueCode::NotAPackage,
            Self::OrphanedBreak { .. } => IssueCode::OrphanedBreak,
            Self::DuplicateParam { .. } => IssueCode::DuplicateParam,
            Self::ReadUninit { .. } => IssueCode::ReadUninit,
            Self::UnresolvedIdent { .. } => IssueCode::UnresolvedIdent,
            Self::IntParse { .. } => IssueCode::IntParse,
            Self::FloatParse { .. } => IssueCode::FloatParse,
            Self::UnreachableStatement { .. } => IssueCode::UnreachableStatement,
            Self::UnreachableArm { .. } => IssueCode::UnreachableArm,
            Self::DuplicateEntry { .. } => IssueCode::DuplicateEntry,
            Self::LiteralAssignment { .. } => IssueCode::LiteralAssignment,
            Self::TypeMismatch { .. } => IssueCode::TypeMismatch,
            Self::NilIndex { .. } => IssueCode::NilIndex,
            Self::IndexTypeMismatch { .. } => IssueCode::IndexTypeMismatch,
            Self::UndefinedOperator { .. } => IssueCode::UndefinedOperator,
            Self::CallArityMismatch { .. } => IssueCode::CallArityMismatch,
            Self::FnArityMismatch { .. } => IssueCode::FnArityMismatch,
            Self::ResultMismatch { .. } => IssueCode::ResultMismatch,
            Self::UnknownComponent { .. } => IssueCode::UnknownComponent,
            Self::InconsistentReturns { .. } => IssueCode::InconsistentReturns,
        }
    }

    pub(crate) fn span(&self, doc: &ScriptDoc) -> ScriptOrigin {
        match self {
            Self::Parse { error_ref, .. } => match error_ref.deref(doc) {
                Some(issue) => ScriptOrigin::from(issue.aligned_span(doc)),
                None => ScriptOrigin::invalid(error_ref.id),
            },

            Self::UnresolvedPackage { package_ref, .. } => {
                package_ref.script_origin(doc, SpanBounds::Cover)
            }

            Self::NotAPackage { package_ref, .. } => {
                package_ref.script_origin(doc, SpanBounds::Cover)
            }

            Self::OrphanedBreak { break_ref, .. } => {
                break_ref.script_origin(doc, SpanBounds::Header)
            }

            Self::DuplicateParam { var_ref, .. } => var_ref.script_origin(doc, SpanBounds::Header),

            Self::ReadUninit { ident_ref, .. } => ident_ref.script_origin(doc, SpanBounds::Cover),

            Self::UnresolvedIdent { ident_ref, .. } => {
                ident_ref.script_origin(doc, SpanBounds::Cover)
            }

            Self::IntParse { number_ref, .. } => number_ref.script_origin(doc, SpanBounds::Cover),

            Self::FloatParse { number_ref, .. } => number_ref.script_origin(doc, SpanBounds::Cover),

            Self::UnreachableStatement { st_ref, .. } => {
                st_ref.script_origin(doc, SpanBounds::Cover)
            }

            Self::UnreachableArm { arm_ref, .. } => arm_ref.script_origin(doc, SpanBounds::Cover),

            Self::DuplicateEntry { entry_key_ref, .. } => {
                entry_key_ref.script_origin(doc, SpanBounds::Cover)
            }

            Self::LiteralAssignment { op_ref, .. } => op_ref.script_origin(doc, SpanBounds::Cover),

            Self::TypeMismatch { expr_ref, .. } => expr_ref.script_origin(doc, SpanBounds::Cover),

            Self::NilIndex { op_ref, .. } => op_ref.script_origin(doc, SpanBounds::Cover),

            Self::IndexTypeMismatch { arg_ref, .. } => {
                arg_ref.script_origin(doc, SpanBounds::Cover)
            }

            Self::UndefinedOperator { op_ref, .. } => op_ref.script_origin(doc, SpanBounds::Cover),

            Self::CallArityMismatch { args_ref, .. } => {
                args_ref.script_origin(doc, SpanBounds::Cover)
            }

            Self::FnArityMismatch { arg_ref, .. } => arg_ref.script_origin(doc, SpanBounds::Header),

            Self::ResultMismatch { arg_ref, .. } => arg_ref.script_origin(doc, SpanBounds::Header),

            Self::UnknownComponent { field_ref, .. } => {
                field_ref.script_origin(doc, SpanBounds::Cover)
            }

            Self::InconsistentReturns { fn_ref, .. } => {
                fn_ref.script_origin(doc, SpanBounds::Header)
            }
        }
    }

    pub(crate) fn message(&self, doc: &ScriptDoc) -> Cow<'static, str> {
        match self {
            Self::Parse { error_ref, .. } => Self::message_parse(doc, error_ref),

            Self::UnresolvedPackage { base, quickfix, .. } => {
                match (base.is_nil(), quickfix.is_empty()) {
                    (true, true) => Cow::from("unresolved import"),
                    (false, true) => Cow::from(format!("unresolved import from {base}")),
                    (true, false) => {
                        Cow::from(format!("unresolved import. did you mean {quickfix:?}?"))
                    }
                    (false, false) => Cow::from(format!(
                        "unresolved import from {base}. did you mean {quickfix:?}?"
                    )),
                }
            }

            Self::NotAPackage { ty, .. } => Cow::from(format!("type '{ty}' is not a package")),

            Self::OrphanedBreak { break_ref, .. } => match break_ref.deref(doc) {
                Some(ScriptNode::Continue { .. }) => {
                    Cow::from("continue statement outside of a loop")
                }
                _ => Cow::from("break statement outside of a loop"),
            },

            Self::DuplicateParam { .. } => Cow::from("duplicate fn parameter name"),

            Self::ReadUninit { .. } => Cow::from("use of possibly uninitialized variable"),

            Self::UnresolvedIdent {
                quickfix, import, ..
            } => match (quickfix.is_empty(), import.is_empty()) {
                (true, true) => Cow::from("unresolved reference"),

                (false, true) => Cow::from(format!(
                    "unresolved reference. did you mean \"{quickfix}\"?"
                )),

                _ => Cow::from(format!(
                    "unresolved reference. did you mean \"{import}.{quickfix}\"?"
                )),
            },

            Self::IntParse { .. } => Cow::from("invalid integer literal"),

            Self::FloatParse { .. } => Cow::from("invalid float literal"),

            Self::UnreachableStatement { .. } => Cow::from("unreachable statement"),

            Self::UnreachableArm { .. } => Cow::from("unreachable match arm"),

            Self::DuplicateEntry { .. } => Cow::from("duplicate struct entry"),

            Self::LiteralAssignment { .. } => Cow::from("assignment to literal is meaningless"),

            Self::TypeMismatch {
                expected, provided, ..
            } => {
                let expected = TypeHint::from(*expected);
                let provided = TypeHint::from(*provided);

                Cow::from(format!(
                    "expected '{expected}' type, but '{provided}' found"
                ))
            }

            Self::NilIndex { .. } => Cow::from("nil type cannot be indexed"),

            Self::IndexTypeMismatch { provided, .. } => {
                let numeric_family = <usize>::type_meta().family();
                let range_family = <Range<usize>>::type_meta().family();

                Cow::from(format!(
                    "expected '{numeric_family}' or '{range_family}' type, but '{provided}' found",
                ))
            }

            Self::UndefinedOperator { receiver, op, .. } => {
                let receiver = TypeHint::from(*receiver);

                Cow::from(format!("'{receiver}' does not implement {op}"))
            }

            Self::CallArityMismatch {
                expected, provided, ..
            } => match *expected {
                1 => Cow::from(format!("expected 1 argument, but {provided} provided",)),

                _ => Cow::from(format!(
                    "expected {expected} arguments, but {provided} provided",
                )),
            },

            Self::FnArityMismatch {
                expected, provided, ..
            } => Cow::from(format!(
                "expected fn({expected}) function, but fn({provided}) provided",
            )),

            Self::ResultMismatch {
                expected, provided, ..
            } => {
                let expected = TypeHint::from(*expected);
                let provided = TypeHint::from(*provided);

                Cow::from(format!(
                    "expected a function with '{expected}' return type, but the function returns '{provided}'"
                ))
            }

            Self::UnknownComponent {
                receiver, quickfix, ..
            } => {
                let receiver = TypeHint::from(*receiver);

                match quickfix.is_empty() {
                    true => Cow::from(format!("unknown '{receiver}' field",)),

                    false => Cow::from(format!(
                        "unknown '{receiver}' field. did you mean {quickfix:?}?",
                    )),
                }
            }

            Self::InconsistentReturns { .. } => Cow::from("missing trailing return expression"),
        }
    }

    fn message_parse(doc: &ScriptDoc, error_ref: &ErrorRef) -> Cow<'static, str> {
        let Some(issue) = error_ref.deref(doc) else {
            return Cow::from("parse error");
        };

        match issue.context {
            ScriptNode::STRING => Cow::from("unenclosed string literal"),
            ScriptNode::MULTILINE_COMMENT => Cow::from("unenclosed comment"),
            ScriptNode::BLOCK if issue.recovery == RecoveryResult::UnexpectedEOI => {
                Cow::from("unenclosed code block")
            }
            ScriptNode::EXPR if issue.recovery == RecoveryResult::PanicRecover => {
                Cow::from("unexpected operator")
            }

            _ => {
                if Self::is_operator_rule(issue.context) {
                    return Cow::from("missing operand");
                }

                Cow::from(issue.message::<ScriptNode>(doc).to_string())
            }
        }
    }

    #[inline(always)]
    fn is_operator_rule(rule: NodeRule) -> bool {
        match rule {
            ScriptNode::UNARY_LEFT | ScriptNode::BINARY | ScriptNode::QUERY => true,
            _ => false,
        }
    }
}
