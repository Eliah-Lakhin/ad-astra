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

use std::fmt::{Arguments, Display, Formatter};

use compact_str::CompactString;
use lady_deirdre::syntax::NodeRef;

use crate::{
    interpret::{ScriptFn, StackDepth},
    report::system_panic,
    runtime::{Origin, PackageMeta},
    semantics::Float,
};

pub(crate) type ClosureIndex = usize;
pub(crate) type SubroutineIndex = usize;
pub(crate) type StringIndex = usize;
pub(crate) type OriginIndex = usize;
pub(crate) type CmdIndex = usize;

pub(crate) const RET: CmdIndex = CmdIndex::MAX;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Assembly {
    pub(crate) arity: StackDepth,
    pub(crate) frame: StackDepth,
    pub(crate) closures: usize,
    pub(crate) subroutines: Subroutines,
    pub(crate) strings: Vec<CompactString>,
    pub(crate) origins: Vec<Origin>,
    pub(crate) commands: Vec<Cmd>,
    pub(crate) sources: Vec<Source>,
}

impl Default for Assembly {
    #[inline(always)]
    fn default() -> Self {
        let mut assembly = Self::new::<false>(0, 0, 0, Origin::nil());

        assembly.commands.push(Cmd::PushNil(PushNilCmd));

        assembly
    }
}

impl Assembly {
    #[inline(always)]
    pub(crate) fn new<const BUILDER: bool>(
        arity: StackDepth,
        closures: usize,
        subroutines: usize,
        origin: impl Into<Origin>,
    ) -> Self {
        Self {
            arity,
            frame: arity,
            closures: closures + 1,
            subroutines: match BUILDER {
                true => Subroutines::Refs(Vec::with_capacity(subroutines)),
                false => Subroutines::Len(subroutines),
            },
            strings: Vec::new(),
            origins: vec![origin.into()],
            commands: Vec::new(),
            sources: Vec::new(),
        }
    }

    #[inline(always)]
    pub(super) fn decl_origin(&self) -> Origin {
        let Some(origin) = self.origins.get(0) else {
            return Origin::nil();
        };

        *origin
    }

    #[inline(always)]
    pub(super) fn cmd_1_source(&self, cmd: CmdIndex) -> Origin {
        let Some(Source { origins }) = self.sources.get(cmd) else {
            return self.decl_origin();
        };

        let index_1 = origins.get(0).copied().unwrap_or(0);

        let origin_1 = match self.origins.get(index_1) {
            Some(origin) => *origin,
            None => Origin::nil(),
        };

        origin_1
    }

    #[inline(always)]
    pub(super) fn cmd_2_source(&self, cmd: CmdIndex) -> (Origin, Origin) {
        let Some(Source { origins }) = self.sources.get(cmd) else {
            let decl_origin = self.decl_origin();

            return (decl_origin, decl_origin);
        };

        let index_1 = origins.get(0).copied().unwrap_or(0);
        let index_2 = origins.get(1).copied().unwrap_or(0);

        let origin_1 = match self.origins.get(index_1) {
            Some(origin) => *origin,
            None => Origin::nil(),
        };

        let origin_2 = match self.origins.get(index_2) {
            Some(origin) => *origin,
            None => Origin::nil(),
        };

        (origin_1, origin_2)
    }

    #[inline(always)]
    pub(super) fn cmd_3_source(&self, cmd: CmdIndex) -> (Origin, Origin, Origin) {
        let Some(Source { origins }) = self.sources.get(cmd) else {
            let decl_origin = self.decl_origin();

            return (decl_origin, decl_origin, decl_origin);
        };

        let index_1 = origins.get(0).copied().unwrap_or(0);
        let index_2 = origins.get(1).copied().unwrap_or(0);
        let index_3 = origins.get(2).copied().unwrap_or(0);

        let origin_1 = match self.origins.get(index_1) {
            Some(origin) => *origin,
            None => Origin::nil(),
        };

        let origin_2 = match self.origins.get(index_2) {
            Some(origin) => *origin,
            None => Origin::nil(),
        };

        let origin_3 = match self.origins.get(index_3) {
            Some(origin) => *origin,
            None => Origin::nil(),
        };

        (origin_1, origin_2, origin_3)
    }

    #[inline(always)]
    pub(super) fn cmd_many_source(&self, cmd: CmdIndex) -> Vec<Origin> {
        let Some(Source { origins }) = self.sources.get(cmd) else {
            return Vec::new();
        };

        let mut result = Vec::with_capacity(origins.len());

        for index in origins {
            let origin = match self.origins.get(*index) {
                Some(origin) => *origin,
                None => Origin::nil(),
            };

            result.push(origin)
        }

        result
    }

    pub(super) fn debug(
        &self,
        formatter: &mut Formatter<'_>,
        mut indent: usize,
        subroutines: &[ScriptFn],
    ) -> std::fmt::Result {
        formatter.write_str("ScriptFn {\n")?;

        indent += 1;

        println(formatter, indent, format_args!("arity: {}", self.arity))?;

        println(formatter, indent, format_args!("frame: {}", self.frame))?;

        println(
            formatter,
            indent,
            format_args!("closures: {}", self.closures),
        )?;

        if !self.commands.is_empty() {
            println(formatter, indent, format_args!("commands:"))?;

            indent += 1;

            for (index, cmd) in self.commands.iter().enumerate() {
                cmd.debug(formatter, indent, index, self.commands.len(), &self.strings)?
            }

            indent -= 1;
        }

        if !self.strings.is_empty() {
            println(formatter, indent, format_args!("strings:"))?;

            indent += 1;

            for (index, string) in self.strings.iter().enumerate() {
                println(formatter, indent, format_args!("string{index}: {string:?}"))?;
            }

            indent -= 1;
        }

        if !subroutines.is_empty() {
            println(formatter, indent, format_args!("fns:"))?;

            indent += 1;

            for (index, routine) in subroutines.iter().enumerate() {
                formatter.write_str(&"    ".repeat(indent))?;
                formatter.write_fmt(format_args!("fn{index}: "))?;

                routine
                    .assembly
                    .as_ref()
                    .debug(formatter, indent, routine.subroutines.as_ref())?;
            }

            indent -= 1;
        }

        indent -= 1;

        println(formatter, indent, format_args!("}}"))?;

        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Cmd {
    IfTrue(IfTrueCmd),
    IfFalse(IfFalseCmd),
    Jump(JumpCmd),
    Iterate(IterateCmd),
    Lift(LiftCmd),
    Swap(SwapCmd),
    Dup(DupCmd),
    Shrink(ShrinkCmd),
    PushNil(PushNilCmd),
    PushTrue(PushTrueCmd),
    PushFalse(PushFalseCmd),
    PushUsize(PushUsizeCmd),
    PushIsize(PushIsizeCmd),
    PushFloat(PushFloatCmd),
    PushString(PushStringCmd),
    PushPackage(PushPackageCmd),
    PushClosure(PushClosureCmd),
    PushFn(PushFnCmd),
    PushStruct(PushStructCmd),
    Range(RangeCmd),
    Bind(BindCmd),
    Concat(ConcatCmd),
    Field(FieldCmd),
    Len(LenCmd),
    Query(QueryCmd),
    Op(OpCmd),
    Invoke(InvokeCmd),
    Index(IndexCmd),
}

impl Cmd {
    fn debug(
        &self,
        formatter: &mut Formatter<'_>,
        indent: usize,
        cmd: usize,
        len: usize,
        strings: &[CompactString],
    ) -> std::fmt::Result {
        match self {
            Self::IfTrue(IfTrueCmd { otherwise }) => match *otherwise < len {
                true => println(
                    formatter,
                    indent,
                    format_args!("{cmd}: if-true {otherwise}"),
                ),

                false => println(formatter, indent, format_args!("{cmd}: if-true ret")),
            },

            Self::IfFalse(IfFalseCmd { otherwise }) => match *otherwise < len {
                true => println(
                    formatter,
                    indent,
                    format_args!("{cmd}: if-false {otherwise}"),
                ),

                false => println(formatter, indent, format_args!("{cmd}: if-false ret")),
            },

            Self::Jump(JumpCmd { command }) => match *command < len {
                true => println(formatter, indent, format_args!("{cmd}: jump {command}")),
                false => println(formatter, indent, format_args!("{cmd}: jump ret")),
            },

            Self::Iterate(IterateCmd { finish }) => match *finish < len {
                true => println(formatter, indent, format_args!("{cmd}: iter {finish}")),
                false => println(formatter, indent, format_args!("{cmd}: iter ret")),
            },

            Self::Lift(LiftCmd { depth }) => {
                println(formatter, indent, format_args!("{cmd}: lift s{depth}"))
            }

            Self::Swap(SwapCmd { depth }) => {
                println(formatter, indent, format_args!("{cmd}: swap s{depth}"))
            }

            Self::Dup(DupCmd { depth }) => {
                println(formatter, indent, format_args!("{cmd}: dup s{depth}"))
            }

            Self::Shrink(ShrinkCmd { depth }) => {
                println(formatter, indent, format_args!("{cmd}: shrink {depth}"))
            }

            Self::PushNil(..) => println(formatter, indent, format_args!("{cmd}: push nil")),

            Self::PushTrue(..) => println(formatter, indent, format_args!("{cmd}: push true")),

            Self::PushFalse(..) => println(formatter, indent, format_args!("{cmd}: push false")),

            Self::PushUsize(PushUsizeCmd { value }) => {
                println(formatter, indent, format_args!("{cmd}: push {value}usize"))
            }

            Self::PushIsize(PushIsizeCmd { value }) => {
                println(formatter, indent, format_args!("{cmd}: push {value}isize"))
            }

            Self::PushFloat(PushFloatCmd { value }) => {
                println(formatter, indent, format_args!("{cmd}: push {value}float"))
            }

            Self::PushString(PushStringCmd { string_index }) => match strings.get(*string_index) {
                Some(string) => println(
                    formatter,
                    indent,
                    format_args!("{cmd}: push string{string_index}({string:?})"),
                ),

                None => println(
                    formatter,
                    indent,
                    format_args!("{cmd}: push string{string_index}(?)"),
                ),
            },

            Self::PushPackage(PushPackageCmd { package }) => {
                println(formatter, indent, format_args!("{cmd}: push ‹{package}›"))
            }

            Self::PushClosure(PushClosureCmd { index }) => println(
                formatter,
                indent,
                format_args!("{cmd}: push closure{index}"),
            ),

            Self::PushFn(PushFnCmd { index }) => {
                println(formatter, indent, format_args!("{cmd}: push fn{index}"))
            }

            Self::PushStruct(..) => println(formatter, indent, format_args!("{cmd}: push struct")),

            Self::Range(..) => println(formatter, indent, format_args!("{cmd}: range")),

            Self::Bind(BindCmd { index }) => {
                println(formatter, indent, format_args!("{cmd}: bind {index}"))
            }

            Self::Concat(ConcatCmd { items }) => {
                println(formatter, indent, format_args!("{cmd}: concat {items}"))
            }

            Self::Field(FieldCmd { field_index }) => match strings.get(*field_index) {
                Some(string) => println(
                    formatter,
                    indent,
                    format_args!("{cmd}: field string{field_index}({string:?})"),
                ),

                None => println(
                    formatter,
                    indent,
                    format_args!("{cmd}: field string{field_index}(?)"),
                ),
            },

            Self::Len(..) => println(formatter, indent, format_args!("{cmd}: len")),

            Self::Query(..) => println(formatter, indent, format_args!("{cmd}: query")),

            Self::Op(op) => println(formatter, indent, format_args!("{cmd}: {op}")),

            Self::Invoke(InvokeCmd { arity }) => {
                println(formatter, indent, format_args!("{cmd}: invoke {arity}"))
            }

            Self::Index(..) => println(formatter, indent, format_args!("{cmd}: index")),
        }
    }
}

// Stack: (condition) -> ()
// Origins: (condition)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct IfTrueCmd {
    pub(crate) otherwise: CmdIndex,
}

// Stack: (condition) -> ()
// Origins: (condition)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct IfFalseCmd {
    pub(crate) otherwise: CmdIndex,
}

// Stack: () -> ()
// Origins: ()
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct JumpCmd {
    pub(crate) command: CmdIndex,
}

// Stack: (range) -> (range, iteration)
// Origins: (range)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct IterateCmd {
    pub(crate) finish: CmdIndex,
}

// Stack: (depth, ...) -> (nil, ..., depth) /* pushes one */
// Origins: ()
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct LiftCmd {
    pub(crate) depth: StackDepth,
}

// Stack: (depth, ..., top) -> (top, ..., depth) /* no size change */
// Origins: ()
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SwapCmd {
    pub(crate) depth: StackDepth,
}

// Stack: (depth, ...) -> (depth, ..., depth) /* pushes one */
// Origins: ()
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct DupCmd {
    pub(crate) depth: StackDepth,
}

// Stack: (..., depth, ...) -> (..., depth) /* resizes down to `depth` */
// Origins: ()
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ShrinkCmd {
    pub(crate) depth: StackDepth,
}

// Stack: () -> (nil)
// Origins: ()
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PushNilCmd;

// Stack: () -> (bool)
// Origins: (bool)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PushTrueCmd;

// Stack: () -> (bool)
// Origins: (bool)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PushFalseCmd;

// Stack: () -> (usize)
// Origins: (usize)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PushUsizeCmd {
    pub(crate) value: usize,
}

// Stack: () -> (isize)
// Origins: (isize)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PushIsizeCmd {
    pub(crate) value: isize,
}

// Stack: () -> (float)
// Origins: (float)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PushFloatCmd {
    pub(crate) value: Float,
}

// Stack: () -> (string)
// Origins: (string)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PushStringCmd {
    pub(crate) string_index: StringIndex,
}

// Stack: () -> (package)
// Origins: ()
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PushPackageCmd {
    pub(crate) package: &'static PackageMeta,
}

// Stack: () -> (closure)
// Origins: ()
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PushClosureCmd {
    pub(crate) index: ClosureIndex,
}

// Stack: () -> (fn)
// Origins: (fn)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PushFnCmd {
    pub(crate) index: SubroutineIndex,
}

// Stack: () -> (struct)
// Origins: (struct)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PushStructCmd;

// Stack: (lhs, rhs) -> (range)
// Origins: (range, lhs, rhs)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct RangeCmd;

// Stack: (fn, closure) -> (fn)
// Origins: ()
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct BindCmd {
    pub(crate) index: ClosureIndex,
}

// Stack: (items..) -> (result) /* pops items, pushes result */
// Origins: (items.., array)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ConcatCmd {
    pub(crate) items: usize,
}

// Stack: (lhs) -> (result)
// Origins: (lhs, field)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct FieldCmd {
    pub(crate) field_index: StringIndex,
}

// Stack: (lhs) -> (result)
// Origins: (lhs, field)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct LenCmd;

// Stack: (lhs) -> (bool)
// Origins: (op)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct QueryCmd;

// For binary:
//     Stack: (lhs, rhs) -> (result)
//     Origins: (op, lhs, rhs)
// For assignment:
//     Stack: (rhs, lhs) -> ()
//     Origins: (op, rhs, lhs)
// For unary:
//     Stack: (rhs) -> (result)
//     Origins: (op, rhs)
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum OpCmd {
    Clone,
    Neg,
    Not,
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
    RemAssign,
    Equal,
    NotEqual,
    Greater,
    GreaterOrEqual,
    Lesser,
    LesserOrEqual,
    And,
    Or,
    Add,
    Sub,
    Mul,
    Div,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Rem,
}

impl Display for OpCmd {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clone => formatter.write_str("clone"),
            Self::Neg => formatter.write_str("neg"),
            Self::Not => formatter.write_str("not"),
            Self::Assign => formatter.write_str("assign"),
            Self::AddAssign => formatter.write_str("add-assign"),
            Self::SubAssign => formatter.write_str("sub-assign"),
            Self::MulAssign => formatter.write_str("mul-assign"),
            Self::DivAssign => formatter.write_str("div-assign"),
            Self::BitAndAssign => formatter.write_str("bit-and-assign"),
            Self::BitOrAssign => formatter.write_str("bit-or-assign"),
            Self::BitXorAssign => formatter.write_str("bit-xor-assign"),
            Self::ShlAssign => formatter.write_str("shl-assign"),
            Self::ShrAssign => formatter.write_str("shr-assign"),
            Self::RemAssign => formatter.write_str("rem-assign"),
            Self::Equal => formatter.write_str("equal"),
            Self::NotEqual => formatter.write_str("not-equal"),
            Self::Greater => formatter.write_str("greater"),
            Self::GreaterOrEqual => formatter.write_str("greater_or_equal"),
            Self::Lesser => formatter.write_str("lesser"),
            Self::LesserOrEqual => formatter.write_str("lesser-or-equal"),
            Self::And => formatter.write_str("and"),
            Self::Or => formatter.write_str("or"),
            Self::Add => formatter.write_str("add"),
            Self::Sub => formatter.write_str("sub"),
            Self::Mul => formatter.write_str("mul"),
            Self::Div => formatter.write_str("div"),
            Self::BitAnd => formatter.write_str("bit-and"),
            Self::BitOr => formatter.write_str("bit-or"),
            Self::BitXor => formatter.write_str("bit-xor"),
            Self::Shl => formatter.write_str("shl"),
            Self::Shr => formatter.write_str("shr"),
            Self::Rem => formatter.write_str("rem"),
        }
    }
}

// Stack: (args.., lhs) -> (result) /* reduces args */
// Origins: (args.., lhs, rhs)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct InvokeCmd {
    pub(crate) arity: usize,
}

// Stack: (index, lhs) -> (result)
// Origins: (index, lhs, rhs)
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct IndexCmd;

// Stack: () -> ()
// Origins: ()
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Source {
    pub(crate) origins: Vec<OriginIndex>,
}

impl From<Vec<OriginIndex>> for Source {
    #[inline(always)]
    fn from(origins: Vec<OriginIndex>) -> Self {
        Self { origins }
    }
}

#[derive(Clone)]
pub(crate) enum Subroutines {
    Refs(Vec<NodeRef>),
    Len(usize),
}

impl PartialEq for Subroutines {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Refs(this), Self::Refs(other)) => this.eq(other),
            (Self::Refs(this), Self::Len(other)) => this.len().eq(other),
            (Self::Len(this), Self::Refs(other)) => this.eq(&other.len()),
            (Self::Len(this), Self::Len(other)) => this.eq(other),
        }
    }
}

impl Eq for Subroutines {}

impl Subroutines {
    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Refs(vec) => vec.len(),
            Self::Len(len) => *len,
        }
    }

    #[inline(always)]
    pub(crate) fn push(&mut self, fn_ref: NodeRef) {
        let Self::Refs(vec) = self else {
            system_panic!("Pushing subroutine to the static assembly.");
        };

        vec.push(fn_ref);
    }
}

#[inline(always)]
fn println(formatter: &mut Formatter<'_>, indent: usize, fmt: Arguments) -> std::fmt::Result {
    formatter.write_str(&"    ".repeat(indent))?;
    formatter.write_fmt(fmt)?;
    formatter.write_str("\n")?;

    Ok(())
}
