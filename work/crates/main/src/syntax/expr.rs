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

use std::{marker::PhantomData, mem::take};

use lady_deirdre::{
    analysis::{Feature, Semantics},
    lexis::{SiteRef, TokenRef, TokenSet},
    syntax::{
        NodeRef,
        NodeRule,
        NodeSet,
        PolyRef,
        Recovery,
        RecoveryResult,
        SyntaxError,
        SyntaxSession,
        EMPTY_NODE_SET,
    },
    units::CompilationUnit,
};

use crate::{
    report::debug_unreachable,
    syntax::{ScriptNode, ScriptToken},
};

const BINARY_OP: TokenSet = {
    use ScriptToken::*;

    TokenSet::inclusive(&[
        Assign as u8,
        PlusAssign as u8,
        MinusAssign as u8,
        MulAssign as u8,
        DivAssign as u8,
        BitAndAssign as u8,
        BitOrAssign as u8,
        BitXorAssign as u8,
        ShlAssign as u8,
        ShrAssign as u8,
        RemAssign as u8,
        Or as u8,
        And as u8,
        Equal as u8,
        NotEqual as u8,
        Greater as u8,
        GreaterOrEqual as u8,
        Lesser as u8,
        LesserOrEqual as u8,
        Dot2 as u8,
        BitOr as u8,
        BitXor as u8,
        BitAnd as u8,
        Shl as u8,
        Shr as u8,
        Plus as u8,
        Minus as u8,
        Mul as u8,
        Div as u8,
        Rem as u8,
        Query as u8,
        ParenOpen as u8,
        BracketOpen as u8,
        Dot as u8,
    ])
};

const OUTER_TERMINALS: TokenSet = {
    use ScriptToken::*;

    TokenSet::inclusive(&[
        If as u8,
        Match as u8,
        Let as u8,
        For as u8,
        Loop as u8,
        Break as u8,
        Continue as u8,
        Return as u8,
        Use as u8,
        Comma as u8,
        Arrow as u8,
        BraceOpen as u8,
        BraceClose as u8,
        BracketClose as u8,
        ParenClose as u8,
        Semicolon as u8,
    ])
};

static RECOVERY_LEFT: Recovery = {
    use ScriptToken::*;

    Recovery::unlimited()
        .unexpected_set(OUTER_TERMINALS)
        .group(BraceOpen as u8, BraceClose as u8)
        .group(BracketOpen as u8, BracketClose as u8)
        .group(ParenOpen as u8, ParenClose as u8)
};

static RECOVERY_RIGHT: Recovery = {
    use ScriptToken::*;

    Recovery::unlimited()
        .unexpected_set(OUTER_TERMINALS)
        .unexpected_set(BINARY_OP)
        .group(BraceOpen as u8, BraceClose as u8)
        .group(BracketOpen as u8, BracketClose as u8)
        .group(ParenOpen as u8, ParenClose as u8)
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum Precedence {
    Outer = 0,
    Assign = 2,
    Or = 4,
    And = 6,
    Compare = 8,
    Range = 10,
    BitOr = 12,
    BitXor = 14,
    BitAnd = 16,
    Shift = 18,
    AddSub = 20,
    MulDivRem = 22,
    UnaryLeft = 24,
    UnaryRight = 26,
    Operand = 100,
}

impl Precedence {
    #[inline(always)]
    pub(crate) fn as_operand(self, assoc: Assoc) -> Operand {
        match assoc {
            Assoc::Left => Operand::Left(self),
            Assoc::Right => Operand::Right(self),
        }
    }

    #[inline(always)]
    pub(crate) fn assoc(self) -> Assoc {
        match self {
            Precedence::Assign | Precedence::UnaryLeft => Assoc::Right,
            _ => Assoc::Left,
        }
    }

    #[inline(always)]
    #[allow(unused)]
    pub(crate) fn is_binary(self) -> bool {
        match self {
            Self::Assign
            | Self::Or
            | Self::And
            | Self::Compare
            | Self::Range
            | Self::BitOr
            | Self::BitAnd
            | Self::BitXor
            | Self::Shift
            | Self::AddSub
            | Self::MulDivRem => true,

            _ => false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operand {
    Left(Precedence),
    Right(Precedence),
}

impl Operand {
    #[inline(always)]
    #[allow(unused)]
    pub(crate) fn new(assoc: Assoc, precedence: Precedence) -> Self {
        match assoc {
            Assoc::Left => Self::Left(precedence),
            Assoc::Right => Self::Right(precedence),
        }
    }

    #[inline(always)]
    pub(crate) fn of(self, operator: Precedence) -> bool {
        let op_assoc = operator.assoc();

        match self {
            Self::Left(operand) => match op_assoc {
                Assoc::Left => operand >= operator,
                Assoc::Right => operand > operator,
            },

            Self::Right(operand) => match op_assoc {
                Assoc::Left => operand > operator,
                Assoc::Right => operand >= operator,
            },
        }
    }

    #[inline(always)]
    #[allow(unused)]
    pub(crate) fn assoc(&self) -> Assoc {
        match self {
            Self::Left(..) => Assoc::Left,
            Self::Right(..) => Assoc::Right,
        }
    }

    #[inline(always)]
    #[allow(unused)]
    pub(crate) fn precedence(self) -> Precedence {
        match self {
            Self::Left(precedence) => precedence,
            Self::Right(precedence) => precedence,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Assoc {
    Left,
    Right,
}

impl ScriptNode {
    #[inline(always)]
    pub(crate) fn precedence(&self, doc: &impl CompilationUnit<Node = ScriptNode>) -> Precedence {
        match self {
            Self::Fn { .. }
            | Self::Struct { .. }
            | Self::Array { .. }
            | Self::String { .. }
            | Self::Crate { .. }
            | Self::This { .. }
            | Self::Ident { .. }
            | Self::Number { .. }
            | Self::Bool { .. }
            | Self::Expr { .. } => Precedence::Operand,

            Self::UnaryLeft { .. } => Precedence::UnaryLeft,

            Self::Binary { op, .. } => {
                let Some(Self::Op { token, .. }) = op.deref(doc) else {
                    return Precedence::Outer;
                };

                let Some(token) = token.deref(doc) else {
                    return Precedence::Outer;
                };

                token.bin_precedence()
            }

            Self::Query { .. } | Self::Call { .. } | Self::Index { .. } => Precedence::UnaryRight,

            _ => Precedence::Outer,
        }
    }
}

impl ScriptToken {
    pub(crate) fn bin_precedence(&self) -> Precedence {
        match self {
            Self::Assign
            | Self::PlusAssign
            | Self::MinusAssign
            | Self::MulAssign
            | Self::DivAssign
            | Self::BitOrAssign
            | Self::BitXorAssign
            | Self::BitAndAssign
            | Self::ShlAssign
            | Self::ShrAssign
            | Self::RemAssign => Precedence::Assign,

            Self::Or => Precedence::Or,

            Self::And => Precedence::And,

            Self::Equal
            | Self::NotEqual
            | Self::Greater
            | Self::GreaterOrEqual
            | Self::Lesser
            | Self::LesserOrEqual => Precedence::Compare,

            Self::Dot2 => Precedence::Range,

            Self::Dot => Precedence::UnaryRight,

            Self::BitOr => Precedence::BitOr,

            Self::BitXor => Precedence::BitXor,

            Self::BitAnd => Precedence::BitAnd,

            Self::Shl | Self::Shr => Precedence::Shift,

            Self::Plus | Self::Minus => Precedence::AddSub,

            Self::Mul | Self::Div | Self::Rem => Precedence::MulDivRem,

            _ => Precedence::Outer,
        }
    }
}

pub(super) struct ExprParser<'session, 'code, S: SyntaxSession<'code, Node = ScriptNode>> {
    session: &'session mut S,
    step_start_ref: SiteRef,
    _code: PhantomData<&'code ()>,
}

impl<'session, 'code, S> ExprParser<'session, 'code, S>
where
    S: SyntaxSession<'code, Node = ScriptNode>,
{
    #[inline(always)]
    pub(super) fn parse(session: &'session mut S) -> ScriptNode {
        let node = session.node_ref();
        let parent = session.parent_ref();
        let step_start_ref = session.site_ref(0);

        let mut parser = Self {
            session,
            step_start_ref,
            _code: PhantomData,
        };

        let inner = parser.parse_expr(ScriptNode::EXPR, Precedence::Outer);

        ScriptNode::Expr {
            node,
            parent,
            start: TokenRef::nil(),
            inner,
            end: TokenRef::nil(),
            semantics: Semantics::new(node),
        }
    }

    fn parse_expr(&mut self, context: NodeRule, parent_op: Precedence) -> NodeRef {
        let assoc = match parent_op == Precedence::Outer {
            true => Assoc::Left,
            false => Assoc::Right,
        };

        let mut accumulator = Some(self.parse_operand(assoc, context));

        loop {
            self.skip_trivia();

            let token = self.session.token(0);

            match token {
                ScriptToken::Query => {
                    if !self.reduce_query(&mut accumulator, parent_op) {
                        break;
                    }
                }

                ScriptToken::ParenOpen => {
                    if !self.reduce_call(&mut accumulator, parent_op) {
                        break;
                    }
                }

                ScriptToken::BracketOpen => {
                    if !self.reduce_index(&mut accumulator, parent_op) {
                        break;
                    }
                }

                ScriptToken::Dot => {
                    if !self.reduce_access(&mut accumulator, parent_op) {
                        break;
                    }
                }

                ScriptToken::EOI => break,

                _ => {
                    if OUTER_TERMINALS.contains(token as u8) {
                        break;
                    }

                    match token.bin_precedence() {
                        Precedence::Outer => {
                            if !self.recover(
                                assoc,
                                &BINARY_OP,
                                context,
                                &BINARY_OP,
                                &EMPTY_NODE_SET,
                            ) {
                                break;
                            }
                        }

                        precedence => {
                            if !self.reduce_binary(&mut accumulator, parent_op, precedence) {
                                break;
                            }
                        }
                    }
                }
            }
        }

        let Some(result) = accumulator else {
            // Safety: `accumulator` is Some except the short reducing points.
            unsafe { debug_unreachable!("Void accumulator.") }
        };

        result
    }

    fn reduce_binary(
        &mut self,
        accumulator: &mut Option<NodeRef>,
        parent_op: Precedence,
        precedence: Precedence,
    ) -> bool {
        if Operand::Left(parent_op).of(precedence) {
            return false;
        }

        let left = match take(accumulator) {
            Some(expr) => expr,

            // Safety: `accumulator` is Some except the short reducing points.
            None => unsafe { debug_unreachable!("Void accumulator.") },
        };

        let node = self.session.enter(ScriptNode::BINARY);

        if !left.is_nil() {
            self.session.lift(&left);
        }

        let parent = self.session.parent_ref();

        let op = self.parse_op();

        self.skip_trivia();

        let right = self.parse_expr(ScriptNode::BINARY, precedence);

        *accumulator = Some(self.session.leave(ScriptNode::Binary {
            node,
            parent,
            left,
            op,
            right,
            semantics: Semantics::new(node),
        }));

        true
    }

    fn reduce_query(&mut self, accumulator: &mut Option<NodeRef>, parent_op: Precedence) -> bool {
        if Operand::Left(parent_op).of(Precedence::UnaryRight) {
            return false;
        }

        let left = match take(accumulator) {
            Some(expr) => expr,

            // Safety: `accumulator` is Some except the short reducing points.
            None => unsafe { debug_unreachable!("Void accumulator.") },
        };

        let node = self.session.enter(ScriptNode::QUERY);

        if !left.is_nil() {
            self.session.lift(&left);
        }

        let parent = self.session.parent_ref();

        let op = self.parse_op();

        *accumulator = Some(self.session.leave(ScriptNode::Query {
            node,
            parent,
            left,
            op,
            semantics: Semantics::new(node),
        }));

        true
    }

    fn reduce_call(&mut self, accumulator: &mut Option<NodeRef>, parent_op: Precedence) -> bool {
        if Operand::Left(parent_op).of(Precedence::UnaryRight) {
            return false;
        }

        let left = match take(accumulator) {
            Some(expr) => expr,

            // Safety: `accumulator` is Some except the short reducing points.
            None => unsafe { debug_unreachable!("Void accumulator.") },
        };

        let node = self.session.enter(ScriptNode::CALL);

        if !left.is_nil() {
            self.session.lift(&left);
        }

        let parent = self.session.parent_ref();

        let args = self.parse_primary(ScriptNode::CALL_ARGS);

        *accumulator = Some(self.session.leave(ScriptNode::Call {
            node,
            parent,
            left,
            args,
            semantics: Semantics::new(node),
        }));

        true
    }

    fn reduce_index(&mut self, accumulator: &mut Option<NodeRef>, parent_op: Precedence) -> bool {
        if Operand::Left(parent_op).of(Precedence::UnaryRight) {
            return false;
        }

        let left = match take(accumulator) {
            Some(expr) => expr,

            // Safety: `accumulator` is Some except the short reducing points.
            None => unsafe { debug_unreachable!("Void accumulator.") },
        };

        let node = self.session.enter(ScriptNode::INDEX);

        if !left.is_nil() {
            self.session.lift(&left);
        }

        let parent = self.session.parent_ref();

        let arg = self.parse_primary(ScriptNode::INDEX_ARG);

        *accumulator = Some(self.session.leave(ScriptNode::Index {
            node,
            parent,
            left,
            arg,
            semantics: Semantics::new(node),
        }));

        true
    }

    fn reduce_access(&mut self, accumulator: &mut Option<NodeRef>, parent_op: Precedence) -> bool {
        if Operand::Left(parent_op).of(Precedence::UnaryRight) {
            return false;
        }

        let left = match take(accumulator) {
            Some(expr) => expr,

            // Safety: `accumulator` is Some except the short reducing points.
            None => unsafe { debug_unreachable!("Void accumulator.") },
        };

        let node = self.session.enter(ScriptNode::BINARY);

        if !left.is_nil() {
            self.session.lift(&left);
        }

        let parent = self.session.parent_ref();

        let op = self.parse_op();

        self.skip_trivia();

        let right = self.parse_primary(ScriptNode::FIELD);

        *accumulator = Some(self.session.leave(ScriptNode::Binary {
            node,
            parent,
            left,
            op,
            right,
            semantics: Semantics::new(node),
        }));

        true
    }

    fn parse_operand(&mut self, assoc: Assoc, context: NodeRule) -> NodeRef {
        static OPERANDS: NodeSet = NodeSet::new(&[
            ScriptNode::IDENT,
            ScriptNode::CRATE,
            ScriptNode::THIS,
            ScriptNode::NUMBER,
            ScriptNode::MAX,
            ScriptNode::BOOL,
            ScriptNode::STRING,
            ScriptNode::FN,
            ScriptNode::STRUCT,
            ScriptNode::ARRAY,
        ]);

        static EXPECTATIONS: TokenSet = TokenSet::inclusive(&[
            ScriptToken::Ident as u8,
            ScriptToken::Crate as u8,
            ScriptToken::This as u8,
            ScriptToken::Int as u8,
            ScriptToken::Float as u8,
            ScriptToken::True as u8,
            ScriptToken::False as u8,
            ScriptToken::Max as u8,
            ScriptToken::DoubleQuote as u8,
            ScriptToken::Fn as u8,
            ScriptToken::Struct as u8,
            ScriptToken::BracketOpen as u8,
            ScriptToken::ParenOpen as u8,
            ScriptToken::Mul as u8,
            ScriptToken::Not as u8,
            ScriptToken::Minus as u8,
        ]);

        loop {
            let token = self.session.token(0);

            match token {
                ScriptToken::Ident => {
                    return self.parse_ident();
                }

                ScriptToken::Crate => {
                    return self.parse_crate();
                }

                ScriptToken::This => {
                    return self.parse_this();
                }

                ScriptToken::Int | ScriptToken::Float => {
                    return self.parse_number();
                }

                ScriptToken::True | ScriptToken::False => {
                    return self.parse_bool();
                }

                ScriptToken::Max => {
                    return self.parse_max();
                }

                ScriptToken::DoubleQuote => {
                    return self.parse_primary(ScriptNode::STRING);
                }

                ScriptToken::Fn => {
                    return self.parse_primary(ScriptNode::FN);
                }

                ScriptToken::Struct => {
                    return self.parse_primary(ScriptNode::STRUCT);
                }

                ScriptToken::BracketOpen => {
                    return self.parse_primary(ScriptNode::ARRAY);
                }

                ScriptToken::ParenOpen => {
                    return self.parse_group(context);
                }

                ScriptToken::Mul | ScriptToken::Not | ScriptToken::Minus => {
                    return self.parse_unary_left();
                }

                _ => {
                    if !self.recover(assoc, &EXPECTATIONS, context, &EXPECTATIONS, &OPERANDS) {
                        return NodeRef::nil();
                    }
                }
            }
        }
    }

    fn parse_primary(&mut self, rule: NodeRule) -> NodeRef {
        let result = self.session.descend(rule);

        result
    }

    fn parse_ident(&mut self) -> NodeRef {
        let node = self.session.enter(ScriptNode::IDENT);

        let parent = self.session.parent_ref();
        let token = self.read_token();

        self.session.leave(ScriptNode::Ident {
            node,
            parent,
            token,
            semantics: Semantics::new(node),
        })
    }

    fn parse_crate(&mut self) -> NodeRef {
        let node = self.session.enter(ScriptNode::CRATE);

        let parent = self.session.parent_ref();
        let token = self.read_token();

        self.session.leave(ScriptNode::Crate {
            node,
            parent,
            token,
            semantics: Semantics::new(node),
        })
    }

    fn parse_this(&mut self) -> NodeRef {
        let node = self.session.enter(ScriptNode::THIS);

        let parent = self.session.parent_ref();
        let token = self.read_token();

        self.session.leave(ScriptNode::This {
            node,
            parent,
            token,
            semantics: Semantics::new(node),
        })
    }

    fn parse_number(&mut self) -> NodeRef {
        let node = self.session.enter(ScriptNode::NUMBER);

        let parent = self.session.parent_ref();
        let token = self.read_token();

        self.session.leave(ScriptNode::Number {
            node,
            parent,
            token,
            semantics: Semantics::new(node),
        })
    }

    fn parse_bool(&mut self) -> NodeRef {
        let node = self.session.enter(ScriptNode::BOOL);

        let parent = self.session.parent_ref();
        let token = self.read_token();

        self.session.leave(ScriptNode::Bool {
            node,
            parent,
            token,
            semantics: Semantics::new(node),
        })
    }

    fn parse_max(&mut self) -> NodeRef {
        let node = self.session.enter(ScriptNode::MAX);

        let parent = self.session.parent_ref();
        let token = self.read_token();

        self.session.leave(ScriptNode::Max {
            node,
            parent,
            token,
            semantics: Semantics::new(node),
        })
    }

    fn parse_group(&mut self, context: NodeRule) -> NodeRef {
        static EXPECTATIONS: TokenSet = TokenSet::empty().include(ScriptToken::ParenClose as u8);

        let node = self.session.enter(ScriptNode::EXPR);
        let parent = self.session.parent_ref();

        let start = self.read_token();

        self.skip_trivia();

        let inner = self.parse_expr(context, Precedence::Outer);

        let end;

        loop {
            self.skip_trivia();

            let token = self.session.token(0);

            match token {
                ScriptToken::ParenClose => {
                    end = self.read_token();
                    break;
                }

                _ => {
                    if !self.recover(
                        Assoc::Left,
                        &EXPECTATIONS,
                        context,
                        &EXPECTATIONS,
                        &EMPTY_NODE_SET,
                    ) {
                        end = TokenRef::nil();
                        break;
                    }
                }
            }
        }

        self.session.leave(ScriptNode::Expr {
            node,
            parent,
            start,
            inner,
            end,
            semantics: Semantics::new(node),
        })
    }

    fn parse_unary_left(&mut self) -> NodeRef {
        let node = self.session.enter(ScriptNode::UNARY_LEFT);

        let parent = self.session.parent_ref();
        let op = self.parse_op();

        self.skip_trivia();

        let right = self.parse_expr(ScriptNode::UNARY_LEFT, Precedence::UnaryLeft);

        self.session.leave(ScriptNode::UnaryLeft {
            node,
            parent,
            op,
            right,
            semantics: Semantics::new(node),
        })
    }

    fn parse_op(&mut self) -> NodeRef {
        let node = self.session.enter(ScriptNode::OP);

        let parent = self.session.parent_ref();
        let token = self.read_token();

        self.session.leave(ScriptNode::Op {
            node,
            parent,
            token,
            semantics: Semantics::new(node),
        })
    }

    fn read_token(&mut self) -> TokenRef {
        let token_ref = self.session.token_ref(0);

        let _ = self.session.advance();

        token_ref
    }

    fn recover(
        &mut self,
        assoc: Assoc,
        until: &TokenSet,
        context: NodeRule,
        expected_tokens: &'static TokenSet,
        expected_nodes: &'static NodeSet,
    ) -> bool {
        let step_end_ref;

        if self.session.token(0) == ScriptToken::EOI {
            step_end_ref = self.session.site_ref(0);

            let _ = self.session.failure(SyntaxError {
                span: self.step_start_ref..step_end_ref,
                context,
                recovery: RecoveryResult::UnexpectedEOI,
                expected_tokens,
                expected_nodes,
            });

            return false;
        }

        let recovery = match assoc {
            Assoc::Left => RECOVERY_LEFT.recover(self.session, until),
            Assoc::Right => RECOVERY_RIGHT.recover(self.session, until),
        };

        step_end_ref = self.session.site_ref(0);

        let _ = self.session.failure(SyntaxError {
            span: self.step_start_ref..step_end_ref,
            context,
            recovery,
            expected_tokens,
            expected_nodes,
        });

        recovery.recovered()
    }

    fn skip_trivia(&mut self) {
        self.step_start_ref = self.session.site_ref(0);

        static SKIP_TOKENS: TokenSet =
            TokenSet::inclusive(&[ScriptToken::Linebreak as u8, ScriptToken::Whitespace as u8]);

        loop {
            let token = self.session.token(0);

            if SKIP_TOKENS.contains(token as u8) {
                self.session.advance();
                continue;
            }

            if token == ScriptToken::InlineComment {
                self.session.descend(ScriptNode::INLINE_COMMENT);
                continue;
            }

            if token == ScriptToken::MultilineCommentStart {
                self.session.descend(ScriptNode::MULTILINE_COMMENT);
                continue;
            }

            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use lady_deirdre::{
        lexis::TokenRef,
        syntax::{AbstractNode, Node, NodeRef, SyntaxTree, Visitor},
    };

    use crate::syntax::{ScriptDoc, ScriptNode};

    #[test]
    fn test_expr_inheritance() {
        struct InheritanceChecker<'a> {
            doc: &'a ScriptDoc,
            context: Vec<NodeRef>,
        }

        impl<'a> InheritanceChecker<'a> {
            fn new(doc: &'a ScriptDoc) -> Self {
                Self {
                    doc,
                    context: Vec::with_capacity(10),
                }
            }

            fn parent_ref(&self) -> NodeRef {
                self.context.last().copied().unwrap_or(NodeRef::nil())
            }
        }

        impl<'a> Visitor for InheritanceChecker<'a> {
            fn visit_token(&mut self, _token_ref: &TokenRef) {}

            fn enter_node(&mut self, node_ref: &NodeRef) -> bool {
                assert_eq!(node_ref.deref(self.doc).unwrap().node_ref(), *node_ref);
                assert_eq!(node_ref.parent(self.doc), self.parent_ref());

                self.context.push(*node_ref);

                true
            }

            fn leave_node(&mut self, node_ref: &NodeRef) {
                assert!(self.context.pop().is_some());
            }
        }

        let mut doc = ScriptDoc::from("(a + b);");
        let mut checker = InheritanceChecker::new(&doc);
        doc.traverse_tree(&mut checker);

        let mut doc = ScriptDoc::from("let a = a + (1 - b.foo);");
        let mut checker = InheritanceChecker::new(&doc);
        doc.traverse_tree(&mut checker);
    }
}
