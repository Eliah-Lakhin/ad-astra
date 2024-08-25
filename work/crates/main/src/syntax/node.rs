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

use lady_deirdre::{
    analysis::{Semantics, VoidFeature},
    lexis::TokenRef,
    syntax::{Node, NodeRef},
};

use crate::{
    semantics::*,
    syntax::{classes::ScriptClassifier, expr::ExprParser, ScriptDoc, ScriptToken},
};

#[derive(Node)]
#[token(ScriptToken)]
#[classifier(ScriptClassifier)]
#[trivia($Whitespace | $Linebreak | InlineComment | MultilineComment)]
#[recovery(
    $If, $Match, $Let, $For, $Loop, $Break, $Continue, $Return, $Use,
    $BraceOpen, $BraceClose, $Semicolon,
    [$BraceOpen..$BraceClose],
    [$BracketOpen..$BracketClose],
    [$ParenOpen..$ParenClose],
)]
#[define(BlockInner = (
    | statements: If
    | statements: Match
    | statements: Let
    | statements: For
    | statements: Loop
    | statements: Block
    | statements: Break
    | statements: Continue
    | statements: Return
    | statements: Use
    | statements: Clause
    | $Semicolon
)*)]
#[define(Operand =
    | $Struct
    | $Fn
    | $True
    | $False
    | $Max
    | $ParenOpen
    | $BracketOpen
    | $Minus
    | $Mul
    | $Not
    | $DoubleQuote
    | $Crate
    | $This
    | $Ident
    | $Int
    | $Float

    // Unsupported suffixes
    | $Assign
    | $PlusAssign
    | $MinusAssign
    | $MulAssign
    | $DivAssign
    | $BitAndAssign
    | $BitOrAssign
    | $BitXorAssign
    | $ShlAssign
    | $ShrAssign
    | $RemAssign
    | $Or
    | $And
    | $Equal
    | $NotEqual
    | $Greater
    | $GreaterOrEqual
    | $Lesser
    | $LesserOrEqual
    | $Dot2
    | $BitOr
    | $BitXor
    | $BitAnd
    | $Shl
    | $Shr
    | $Plus
    | $Div
    | $Query
    | $Dot
    | $Len
)]
pub enum ScriptNode {
    #[rule(start: $InlineComment ^[$Linebreak]* end: $Linebreak?)]
    #[trivia]
    #[describe("comment", "'//...'")]
    #[denote(INLINE_COMMENT)]
    InlineComment {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(
        start: $MultilineCommentStart
        ^[$MultilineCommentEnd]*
        end: $MultilineCommentEnd
    )]
    #[trivia(MultilineComment)]
    #[describe("comment", "'/*...*/'")]
    #[denote(MULTILINE_COMMENT)]
    MultilineComment {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[root]
    #[rule(BlockInner*)]
    #[recovery]
    #[describe("Module", "Module")]
    #[scope]
    Root {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        statements: Vec<NodeRef>,
        #[semantics]
        semantics: Semantics<RootSemantics>,
    },

    #[rule(
        expr: Expr
        end: $Semicolon
    )]
    #[describe("statement", "'<expr>;'")]
    #[denote(CLAUSE)]
    Clause {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        expr: NodeRef,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(
        keyword: $Use
        (packages: Package)+{$Dot}
        end: $Semicolon
    )]
    #[describe("statement", "'use <ident>.<ident>...;'")]
    #[denote(USE)]
    Use {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        keyword: TokenRef,
        #[child]
        packages: Vec<NodeRef>,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(token: $Ident)]
    #[describe("package", "'<package name>'")]
    #[denote(PACKAGE)]
    Package {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<PackageSemantics>,
    },

    #[rule(
        keyword: $If
        condition: Expr
        body: Block
    )]
    #[describe("statement", "'if <cond> {...}'")]
    #[denote(IF)]
    If {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        keyword: TokenRef,
        #[child]
        condition: NodeRef,
        #[child]
        body: NodeRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(
        keyword: $Match
        subject: Expr?
        body: MatchBody
    )]
    #[describe("statement", "'match <subject> {<cases>}'")]
    #[denote(MATCH)]
    Match {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        keyword: TokenRef,
        #[child]
        subject: NodeRef,
        #[child]
        body: NodeRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(
        start: $BraceOpen
        (arms: MatchArm & $Comma?)*
        end: $BraceClose
    )]
    #[describe("match body", "'{<match cases>}'")]
    #[denote(MATCH_BODY)]
    MatchBody {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        arms: Vec<NodeRef>,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(
        (case: (Expr | Else))
        $Arrow
        (handler: Expr | handler: Block)
    )]
    #[recovery(
        $If, $Match, $Let, $For, $Loop, $Break, $Continue, $Return, $Use,
        $BraceOpen, $BraceClose, $Semicolon,
        [$BraceOpen..$BraceClose],
        [$BracketOpen..$BracketClose],
        [$ParenOpen..$ParenClose],
        $Comma,
    )]
    #[describe("match arm", "'<case> => <handler>'")]
    #[denote(MATCH_ARM)]
    MatchArm {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        case: NodeRef,
        #[child]
        handler: NodeRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(token: $Else)]
    #[describe("match else", "'else => ...'")]
    #[denote(ELSE)]
    Else {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(
        keyword: $Let
        name: Var
        ($Assign & value: Expr)?
        end: $Semicolon
    )]
    #[describe("statement", "'let <var> = <expr>;'")]
    #[denote(LET)]
    Let {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        keyword: TokenRef,
        #[child]
        name: NodeRef,
        #[child]
        value: NodeRef,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(token: $Ident)]
    #[describe("var name", "'<var name>'")]
    #[denote(VAR)]
    Var {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<VarSemantics>,
    },

    #[rule(
        keyword: $For
        iterator: Var
        $In
        range: Expr
        body: Block
    )]
    #[describe("statement", "'for <var> in <range> {...}'")]
    #[denote(FOR)]
    For {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        keyword: TokenRef,
        #[child]
        iterator: NodeRef,
        #[child]
        range: NodeRef,
        #[child]
        body: NodeRef,
        #[semantics]
        semantics: Semantics<ForSemantics>,
    },

    #[rule(
        keyword: $Loop
        body: Block
    )]
    #[describe("statement", "'loop {...}'")]
    #[denote(LOOP)]
    Loop {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        keyword: TokenRef,
        #[child]
        body: NodeRef,
        #[semantics]
        semantics: Semantics<LoopSemantics>,
    },

    #[rule(
        start: $BraceOpen
        BlockInner
        end: $BraceClose
    )]
    #[describe("block", "'{<code block>}'")]
    #[denote(BLOCK)]
    Block {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        statements: Vec<NodeRef>,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(
        keyword: $Break
        end: $Semicolon
    )]
    #[describe("statement", "'break;'")]
    #[denote(BREAK)]
    Break {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        keyword: TokenRef,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<BreakSemantics>,
    },

    #[rule(
        keyword: $Continue
        end: $Semicolon
    )]
    #[describe("statement", "'continue;'")]
    #[denote(CONTINUE)]
    Continue {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        keyword: TokenRef,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<ContinueSemantics>,
    },

    #[rule(
        keyword: $Return
        result: Expr?
        end: $Semicolon
    )]
    #[describe("statement", "'return <expr>;'")]
    #[denote(RETURN)]
    Return {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        keyword: TokenRef,
        #[child]
        result: NodeRef,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(
        keyword: $Fn
        params: FnParams
        body: (Expr | Block)
    )]
    #[describe("function", "'fn(...) {...}'")]
    #[denote(FN)]
    #[scope]
    Fn {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        keyword: TokenRef,
        #[child]
        params: NodeRef,
        #[child]
        body: NodeRef,
        #[semantics]
        semantics: Semantics<FnSemantics>,
    },

    #[rule(
        start: $ParenOpen
        (params: Var & ($Comma & params: Var)* & $Comma?)?
        end: $ParenClose
    )]
    #[recovery(
        $If, $Match, $Let, $For, $Loop, $Break, $Continue, $Return, $Use,
        $BraceOpen, $BraceClose, $Semicolon,
        [$BraceOpen..$BraceClose],
        [$BracketOpen..$BracketClose],
        [$ParenOpen..$ParenClose],
        $Struct, $Fn, $True, $False, $ParenOpen, $BracketOpen, $Minus,
        $Mul, $Not, $DoubleQuote, $Int, $Float,
    )]
    #[describe("fn parameters", "'(<fn params>)'")]
    #[denote(FN_PARAMETERS)]
    FnParams {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        params: Vec<NodeRef>,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(
        keyword: $Struct
        body: StructBody
    )]
    #[describe("struct", "'struct {<entries>}'")]
    #[denote(STRUCT)]
    Struct {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        keyword: TokenRef,
        #[child]
        body: NodeRef,
        #[semantics]
        semantics: Semantics<StructSemantics>,
    },

    #[rule(
        start: $BraceOpen
        (entries: StructEntry & ($Comma & entries: StructEntry)* & $Comma?)?
        end: $BraceClose
    )]
    #[describe("struct body", "'{<entry>: <value>, ...}'")]
    #[denote(STRUCT_BODY)]
    StructBody {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        entries: Vec<NodeRef>,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(
        key: StructEntryKey
        $Colon
        value: Expr
    )]
    #[recovery(
        $If, $Match, $Let, $For, $Loop, $Break, $Continue, $Return, $Use,
        $BraceOpen, $BraceClose, $Semicolon,
        [$BraceOpen..$BraceClose],
        [$BracketOpen..$BracketClose],
        [$ParenOpen..$ParenClose],
        $Comma,
    )]
    #[describe("struct entry", "'<entry>: <value>'")]
    #[denote(STRUCT_ENTRY)]
    StructEntry {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        key: NodeRef,
        #[child]
        value: NodeRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(token: ($Ident | $Int))]
    #[describe("struct entry key", "'<entry key>: ...'")]
    #[denote(STRUCT_ENTRY_KEY)]
    StructEntryKey {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(
        start: $BracketOpen
        (items: Expr & ($Comma & items: Expr)* & $Comma?)?
        end: $BracketClose
    )]
    #[describe("array", "'[<array>]'")]
    #[denote(ARRAY)]
    Array {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        items: Vec<NodeRef>,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<ArraySemantics>,
    },

    #[rule(
        start: $DoubleQuote
        ^[$DoubleQuote | $Linebreak]*
        end: $DoubleQuote
    )]
    #[trivia]
    #[recovery($Linebreak)]
    #[describe("literal", "'<string>'")]
    #[denote(STRING)]
    String {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<StringSemantics>,
    },

    #[describe("ident", "'<ident>'")]
    #[denote(CRATE)]
    Crate {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<CrateSemantics>,
    },

    #[describe("self", "'<self>'")]
    #[denote(THIS)]
    This {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<ThisSemantics>,
    },

    #[describe("ident", "'<ident>'")]
    #[denote(IDENT)]
    Ident {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<IdentSemantics>,
    },

    #[describe("literal", "'<number>'")]
    #[denote(NUMBER)]
    Number {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<NumberSemantics>,
    },

    #[describe("literal", "'<number>'")]
    #[denote(MAX)]
    Max {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<MaxSemantics>,
    },

    #[describe("literal", "'<bool>'")]
    #[denote(BOOL)]
    Bool {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<BoolSemantics>,
    },

    #[describe("operator")]
    #[denote(UNARY_LEFT)]
    UnaryLeft {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        op: NodeRef,
        #[child]
        right: NodeRef,
        #[semantics]
        semantics: Semantics<UnaryLeftSemantics>,
    },

    #[describe("operator")]
    #[denote(BINARY)]
    Binary {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        left: NodeRef,
        #[child]
        op: NodeRef,
        #[child]
        right: NodeRef,
        #[semantics]
        semantics: Semantics<BinarySemantics>,
    },

    #[describe("operator")]
    #[denote(OP)]
    Op {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[describe("operator")]
    #[denote(QUERY)]
    Query {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        left: NodeRef,
        #[child]
        op: NodeRef,
        #[semantics]
        semantics: Semantics<QuerySemantics>,
    },

    #[describe("operator")]
    #[denote(CALL)]
    Call {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        left: NodeRef,
        #[child]
        args: NodeRef,
        #[semantics]
        semantics: Semantics<CallSemantics>,
    },

    #[rule(
        start: $ParenOpen
        (args: Expr & ($Comma & args: Expr)* & $Comma?)?
        end: $ParenClose
    )]
    #[recovery(
        $If, $Match, $Let, $For, $Loop, $Break, $Continue, $Return, $Use,
        $BraceOpen, $BraceClose, $Semicolon,
        [$BraceOpen..$BraceClose],
        [$BracketOpen..$BracketClose],
        [$ParenOpen..$ParenClose],
    )]
    #[describe("call arguments", "'(<arg>, <arg>, ...)'")]
    #[denote(CALL_ARGS)]
    CallArgs {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        args: Vec<NodeRef>,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[describe("operator")]
    #[denote(INDEX)]
    Index {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        left: NodeRef,
        #[child]
        arg: NodeRef,
        #[semantics]
        semantics: Semantics<IndexSemantics>,
    },

    #[rule(
        start: $BracketOpen
        arg: Expr
        $Comma?
        end: $BracketClose
    )]
    #[recovery(
        $If, $Match, $Let, $For, $Loop, $Break, $Continue, $Return, $Use,
        $BraceOpen, $BraceClose, $Semicolon,
        [$BraceOpen..$BraceClose],
        [$BracketOpen..$BracketClose],
        [$ParenOpen..$ParenClose],
        $BracketClose,
    )]
    #[describe("index", "'[<index arg>]'")]
    #[denote(INDEX_ARG)]
    IndexArg {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        arg: NodeRef,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ScriptNode>>,
    },

    #[rule(token: ($Ident | $Int | $Len))]
    #[describe("operator")]
    #[denote(FIELD)]
    Field {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<FieldSemantics>,
    },

    #[rule(Operand)]
    #[parser(ExprParser::parse(session))]
    #[describe("expression", "'<expr>'")]
    #[denote(EXPR)]
    #[secondary]
    Expr {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        inner: NodeRef,
        #[child]
        end: TokenRef,
        #[semantics]
        semantics: Semantics<ExprSemantics>,
    },
}

impl ScriptNode {
    #[inline(always)]
    pub(crate) fn extract_atom_string<'a>(
        doc: &'a ScriptDoc,
        atom_ref: &NodeRef,
    ) -> Option<&'a str> {
        let script_node = atom_ref.deref(doc)?;

        let token_ref = match script_node {
            ScriptNode::Package { token, .. } => token,
            ScriptNode::Var { token, .. } => token,
            ScriptNode::Ident { token, .. } => token,
            ScriptNode::Number { token, .. } => token,
            ScriptNode::Bool { token, .. } => token,
            ScriptNode::Field { token, .. } => token,
            ScriptNode::StructEntryKey { token, .. } => token,
            _ => return None,
        };

        token_ref.string(doc)
    }

    #[inline(always)]
    pub(crate) fn extract_op(doc: &ScriptDoc, op_ref: &NodeRef) -> Option<ScriptToken> {
        let Some(ScriptNode::Op { token, .. }) = op_ref.deref(doc) else {
            return None;
        };

        token.deref(doc)
    }

    #[inline(always)]
    pub(crate) fn extract_bool(doc: &ScriptDoc, mut expr: NodeRef) -> Option<bool> {
        let token_ref = loop {
            match expr.deref(doc) {
                Some(ScriptNode::Bool { token, .. }) => break token,

                Some(ScriptNode::Expr { inner, .. }) => {
                    expr = *inner;
                    continue;
                }

                _ => return None,
            }
        };

        match token_ref.deref(doc) {
            Some(ScriptToken::True) => Some(true),
            Some(ScriptToken::False) => Some(false),
            _ => None,
        }
    }

    #[inline(always)]
    pub(crate) fn is_default_case(doc: &ScriptDoc, case: &NodeRef) -> bool {
        let Some(ScriptNode::Else { .. }) = case.deref(doc) else {
            return false;
        };

        true
    }

    #[inline(always)]
    pub(crate) fn is_match_exhaustive(doc: &ScriptDoc, arms: &[NodeRef]) -> bool {
        let mut true_arm = false;
        let mut false_arm = false;
        let mut default_arm = false;

        for arm in arms {
            let Some(ScriptNode::MatchArm { case, .. }) = arm.deref(doc) else {
                continue;
            };

            if Self::is_default_case(doc, case) {
                default_arm = true;
                break;
            }

            match Self::extract_bool(doc, *case) {
                Some(true) => true_arm = true,
                Some(false) => false_arm = true,
                _ => (),
            }

            if true_arm && false_arm {
                break;
            }
        }

        true_arm && false_arm || default_arm
    }
}
