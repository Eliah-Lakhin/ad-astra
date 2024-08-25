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

use lady_deirdre::lexis::Token;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Token)]
#[repr(u8)]
#[non_exhaustive]
#[lookback(2)]
pub enum ScriptToken {
    EOI = 0,

    Unknown = 1,

    #[rule("fn")]
    #[priority(1)]
    #[describe("keyword")]
    Fn,

    #[rule("let")]
    #[priority(1)]
    #[describe("keyword")]
    Let,

    #[rule("struct")]
    #[priority(1)]
    #[describe("keyword")]
    Struct,

    #[rule("use")]
    #[priority(1)]
    #[describe("keyword")]
    Use,

    #[rule("for")]
    #[priority(1)]
    #[describe("keyword")]
    For,

    #[rule("in")]
    #[priority(1)]
    #[describe("keyword")]
    In,

    #[rule("loop")]
    #[priority(1)]
    #[describe("keyword")]
    Loop,

    #[rule("break")]
    #[priority(1)]
    #[describe("keyword")]
    Break,

    #[rule("continue")]
    #[priority(1)]
    #[describe("keyword")]
    Continue,

    #[rule("return")]
    #[priority(1)]
    #[describe("keyword")]
    Return,

    #[rule("if")]
    #[priority(1)]
    #[describe("keyword")]
    If,

    #[rule("else")]
    #[priority(1)]
    #[describe("keyword")]
    Else,

    #[rule("match")]
    #[priority(1)]
    #[describe("keyword")]
    Match,

    #[rule("true")]
    #[priority(1)]
    #[describe("bool")]
    True,

    #[rule("false")]
    #[priority(1)]
    #[describe("bool")]
    False,

    #[rule("max")]
    #[priority(1)]
    #[describe("keyword")]
    Max,

    #[rule("len")]
    #[priority(1)]
    #[describe("keyword")]
    Len,

    #[rule("//")]
    #[describe("comment")]
    InlineComment,

    #[rule("/*")]
    #[describe("comment")]
    MultilineCommentStart,

    #[rule("*/")]
    #[describe("comment")]
    MultilineCommentEnd,

    #[rule("(")]
    ParenOpen,

    #[rule(")")]
    ParenClose,

    #[rule("{")]
    BraceOpen,

    #[rule("}")]
    BraceClose,

    #[rule("[")]
    BracketOpen,

    #[rule("]")]
    BracketClose,

    #[rule("?")]
    #[describe("operator")]
    Query,

    #[rule(",")]
    Comma,

    #[rule(":")]
    Colon,

    #[rule(";")]
    Semicolon,

    #[rule(".")]
    #[describe("operator")]
    Dot,

    #[rule("..")]
    #[describe("operator")]
    Dot2,

    #[rule("=")]
    Assign,

    #[rule("=>")]
    Arrow,

    #[rule("+")]
    #[describe("operator")]
    Plus,

    #[rule("+=")]
    #[describe("operator")]
    PlusAssign,

    #[rule("-")]
    #[describe("operator")]
    Minus,

    #[rule("-=")]
    #[describe("operator")]
    MinusAssign,

    #[rule("*")]
    #[describe("operator")]
    Mul,

    #[rule("*=")]
    #[describe("operator")]
    MulAssign,

    #[rule("/")]
    #[describe("operator")]
    Div,

    #[rule("/=")]
    #[describe("operator")]
    DivAssign,

    #[rule("&&")]
    #[describe("operator")]
    And,

    #[rule("||")]
    #[describe("operator")]
    Or,

    #[rule("<<")]
    #[describe("operator")]
    Shl,

    #[rule("<<=")]
    #[describe("operator")]
    ShlAssign,

    #[rule(">>")]
    #[describe("operator")]
    Shr,

    #[rule(">>=")]
    #[describe("operator")]
    ShrAssign,

    #[rule("!")]
    #[describe("operator")]
    Not,

    #[rule("&")]
    #[describe("operator")]
    BitAnd,

    #[rule("&=")]
    #[describe("operator")]
    BitAndAssign,

    #[rule("|")]
    #[describe("operator")]
    BitOr,

    #[rule("|=")]
    #[describe("operator")]
    BitOrAssign,

    #[rule("^")]
    #[describe("operator")]
    BitXor,

    #[rule("^=")]
    #[describe("operator")]
    BitXorAssign,

    #[rule("%")]
    #[describe("operator")]
    Rem,

    #[rule("%=")]
    #[describe("operator")]
    RemAssign,

    #[rule("<")]
    #[describe("operator")]
    Lesser,

    #[rule("<=")]
    #[describe("operator")]
    LesserOrEqual,

    #[rule(">")]
    #[describe("operator")]
    Greater,

    #[rule(">=")]
    #[describe("operator")]
    GreaterOrEqual,

    #[rule("==")]
    #[describe("operator")]
    Equal,

    #[rule("!=")]
    #[describe("operator")]
    NotEqual,

    #[rule('"')]
    DoubleQuote,

    #[rule('\\' .)]
    Escaped,

    #[rule("crate")]
    #[priority(1)]
    #[describe("crate")]
    Crate,

    #[rule("self")]
    #[priority(1)]
    #[describe("self")]
    This,

    #[rule(['a'..'z', 'A'..'Z', '_'] ['a'..'z', 'A'..'Z', '0'..'9', '_']*)]
    Ident,

    #[rule(
        | '0'
        | "-0"
        | "+0"
        | ['-', '+']? ['1'..'9'] ['0'..'9']*
    )]
    Int,

    #[rule(
        ('0' | "-0" | "+0" | ['-', '+']? ['1'..'9'] ['0'..'9']*)
        (
            | '.' ['0'..'9']+ ('e' ['-', '+']? ['1'..'9'] ['0'..'9']* )?
            | 'e' ['-', '+']? ['1'..'9'] ['0'..'9']*
        )
    )]
    #[priority(1)]
    Float,

    #[rule([' ', '\t', '\x0c']+)]
    #[describe("blank")]
    Whitespace,

    #[rule("\n" | "\r\n")]
    #[describe("blank")]
    Linebreak,
}

impl Default for ScriptToken {
    #[inline(always)]
    fn default() -> Self {
        Self::Unknown
    }
}

#[cfg(test)]
mod tests {
    use lady_deirdre::lexis::Token;

    use crate::syntax::token::ScriptToken;

    #[test]
    fn test_token_descriptions() {
        assert_eq!(
            "Greater",
            <ScriptToken as Token>::rule_name(ScriptToken::Greater as u8).unwrap()
        );

        assert_eq!(
            "operator",
            <ScriptToken as Token>::rule_description(ScriptToken::Greater as u8, false).unwrap()
        );

        assert_eq!(
            ">",
            <ScriptToken as Token>::rule_description(ScriptToken::Greater as u8, true).unwrap()
        );
    }
}
