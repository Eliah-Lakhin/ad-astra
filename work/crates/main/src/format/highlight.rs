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

use lady_deirdre::format::{Highlighter, Style};

use crate::syntax::ScriptToken;

pub(crate) struct ScriptHighlighter {
    mode: Mode,
}

impl Highlighter<ScriptToken> for ScriptHighlighter {
    fn token_style(&mut self, dim: bool, token: ScriptToken) -> Option<Style> {
        use ScriptToken::*;

        match &mut self.mode {
            Mode::Normal => {
                let class = match token {
                    Fn | Let | Struct | Use | For | In | Loop | Break | Continue | Return | If
                    | Else | Match | Crate | This => Class::Keyword,

                    True | False | Int | Float => Class::Literal,

                    InlineComment => Class::Inline,

                    MultilineCommentStart => Class::Multiline,

                    DoubleQuote => Class::String,

                    _ => Class::Other,
                };

                match &class {
                    Class::String => self.mode = Mode::String,
                    Class::Inline => self.mode = Mode::Inline,
                    Class::Multiline => self.mode = Mode::Multiline(0),
                    _ => (),
                }

                class.style(dim)
            }

            Mode::String => {
                match token {
                    DoubleQuote | Linebreak => self.mode = Mode::Normal,
                    _ => (),
                }

                Class::String.style(dim)
            }

            Mode::Inline => {
                match token {
                    Linebreak => self.mode = Mode::Normal,
                    _ => (),
                }

                Class::Inline.style(dim)
            }

            Mode::Multiline(depth) => {
                match token {
                    MultilineCommentStart => *depth += 1,

                    MultilineCommentEnd if *depth > 0 => {
                        *depth -= 1;
                    }

                    MultilineCommentEnd => {
                        self.mode = Mode::Normal;
                    }

                    _ => (),
                }

                Class::Multiline.style(dim)
            }
        }
    }
}

impl ScriptHighlighter {
    #[inline(always)]
    pub(crate) const fn new() -> Self {
        Self { mode: Mode::Normal }
    }
}

enum Mode {
    Normal,
    String,
    Inline,
    Multiline(usize),
}

enum Class {
    Keyword,
    Literal,
    Inline,
    Multiline,
    String,
    Other,
}

impl Class {
    #[inline(always)]
    fn style(&self, dim: bool) -> Option<Style> {
        static KEYWORD: Option<Style> = Some(Style::new().bold().blue());
        static KEYWORD_DIM: Option<Style> = Some(Style::new().bold().bright_black());

        static LITERAL: Option<Style> = Some(Style::new().green());
        static LITERAL_DIM: Option<Style> = Some(Style::new().bright_black());

        static COMMENT: Option<Style> = Some(Style::new().bright_black());
        static COMMENT_DIM: Option<Style> = Some(Style::new().bright_black());

        static OTHER: Option<Style> = None;
        static OTHER_DIM: Option<Style> = None;

        match (dim, self) {
            (false, Self::Keyword) => KEYWORD,
            (true, Self::Keyword) => KEYWORD_DIM,

            (false, Self::Literal | Self::String) => LITERAL,
            (true, Self::Literal | Self::String) => LITERAL_DIM,

            (false, Self::Inline | Self::Multiline) => COMMENT,
            (true, Self::Inline | Self::Multiline) => COMMENT_DIM,

            (false, Self::Other) => OTHER,
            (true, Self::Other) => OTHER_DIM,
        }
    }
}
