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

use lsp_types::{
    CompletionItem,
    CompletionItemKind,
    Documentation,
    InsertTextFormat,
    MarkupContent,
    MarkupKind,
};

use crate::server::LspServerConfig;

pub(super) trait Snippet {
    const LABEL: &'static str;
    const DOCUMENTATION: &'static str;
    const SNIPPET: &'static str;

    fn item(config: &LspServerConfig) -> CompletionItem {
        let documentation = match config.capabilities.completion_markdown {
            true => Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: String::from(
                    Self::DOCUMENTATION
                        .replace("```<adastra>", &format!("```{}", config.language_id)),
                ),
            })),

            false => Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::PlainText,
                value: {
                    let mut documentation = String::new();

                    let mut is_first = true;

                    for line in Self::DOCUMENTATION.lines() {
                        if line.starts_with("```") {
                            continue;
                        }

                        match is_first {
                            true => is_first = false,
                            false => documentation.push('\n'),
                        }

                        documentation.push_str(line);
                    }

                    documentation
                },
            })),
        };

        let mut text = String::new();

        let mut is_first = true;

        for mut line in Self::SNIPPET.lines() {
            match is_first {
                true => is_first = false,
                false => text.push('\n'),
            }

            if line.starts_with(' ') {
                line = &line[1..];
            }

            text.push_str(line);
        }

        CompletionItem {
            label: String::from(Self::LABEL),
            kind: Some(CompletionItemKind::SNIPPET),
            documentation,
            insert_text: Some(text),
            insert_text_format: Some(InsertTextFormat::SNIPPET),

            ..Default::default()
        }
    }
}

macro_rules! markup {
    (
        $(#[doc = $doc:expr])*
    ) => { concat!($($doc, "\n"),*) };
}

pub(super) struct SnippetFn;

impl Snippet for SnippetFn {
    const LABEL: &'static str = "fn";

    const DOCUMENTATION: &'static str = markup! {
        /// Script function.
        ///
        /// ```<adastra>
        /// fn(arg1, arg2, arg3) {
        ///
        /// }
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// fn($1) {
        ///     $2
        /// }
    };
}

pub(super) struct SnippetStruct;

impl Snippet for SnippetStruct {
    const LABEL: &'static str = "struct";

    const DOCUMENTATION: &'static str = markup! {
        /// Script structure.
        ///
        /// ```<adastra>
        /// struct {
        ///     field_1: expr,
        ///     field_2: expr,
        /// }
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// struct {
        ///     $1
        /// }
    };
}

pub(super) struct SnippetSelf;

impl Snippet for SnippetSelf {
    const LABEL: &'static str = "self";

    const DOCUMENTATION: &'static str = markup! {
        /// A special variable that points to the function's context.
        ///
        /// ```<adastra>
        /// struct {
        ///     field: 123,
        ///
        ///     set_field: fn(x) {
        ///         self.field = x;
        ///     },
        /// };
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// self
    };
}

pub(super) struct SnippetCrate;

impl Snippet for SnippetCrate {
    const LABEL: &'static str = "crate";

    const DOCUMENTATION: &'static str = markup! {
        /// A special variable that points to the root package.
        ///
        /// ```<adastra>
        /// crate.bar();
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// crate
    };
}

pub(super) struct SnippetTrue;

impl Snippet for SnippetTrue {
    const LABEL: &'static str = "true";

    const DOCUMENTATION: &'static str = markup! {
        /// True boolean value.
        ///
        /// ```<adastra>
        /// if true {
        ///
        /// }
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// true
    };
}

pub(super) struct SnippetFalse;

impl Snippet for SnippetFalse {
    const LABEL: &'static str = "false";

    const DOCUMENTATION: &'static str = markup! {
        /// False boolean value.
        ///
        /// ```<adastra>
        /// if false {
        ///
        /// }
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// false
    };
}

pub(super) struct SnippetMax;

impl Snippet for SnippetMax {
    const LABEL: &'static str = "max";

    const DOCUMENTATION: &'static str = markup! {
        /// The maximum value of an unsigned integer.
        ///
        /// ```<adastra>
        /// 0..max
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// max
    };
}

pub(super) struct SnippetLen;

impl Snippet for SnippetLen {
    const LABEL: &'static str = "len";

    const DOCUMENTATION: &'static str = markup! {
        /// Array or string length.
        ///
        /// ```<adastra>
        /// [].len == 0;
        /// [10, 20, 30].len == 3;
        /// "Буква Щ".len == 7;
        /// 123.len == 1;
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// len
    };
}

pub(super) struct SnippetUse;

impl Snippet for SnippetUse {
    const LABEL: &'static str = "use";

    const DOCUMENTATION: &'static str = markup! {
        /// Sub-package import.
        ///
        /// ```<adastra>
        /// use foo.bar.baz;
        ///
        /// func_from_baz();
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// use $1;
    };
}

pub(super) struct SnippetLet;

impl Snippet for SnippetLet {
    const LABEL: &'static str = "let";

    const DOCUMENTATION: &'static str = markup! {
        /// Variable declaration.
        ///
        /// ```<adastra>
        /// let var_name = expr;
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// let $1 = $2;
    };
}

pub(super) struct SnippetIf;

impl Snippet for SnippetIf {
    const LABEL: &'static str = "if";

    const DOCUMENTATION: &'static str = markup! {
        /// Conditional block.
        ///
        /// ```<adastra>
        /// if condition {
        ///
        /// }
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// if $1 {
        ///     $2
        /// }
    };
}

pub(super) struct SnippetMatch;

impl Snippet for SnippetMatch {
    const LABEL: &'static str = "match";

    const DOCUMENTATION: &'static str = markup! {
        /// Conditional branching.
        ///
        /// ```<adastra>
        /// match expr {
        ///     case_1 => {},
        ///     case_2 => {},
        ///     else => {},
        /// }
        ///
        /// match {
        ///     a == b => {},
        ///     x > y => {},
        ///     else => {},
        /// }
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// match $1 {
        ///     $2
        /// }
    };
}

pub(super) struct SnippetMatchArm;

impl Snippet for SnippetMatchArm {
    const LABEL: &'static str = "case";

    const DOCUMENTATION: &'static str = markup! {
        /// Match arm.
        ///
        /// ```<adastra>
        /// match test_expr {
        ///     case_expr_1 => {}
        ///     case_expr_2 => {}
        /// }
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// $1 => {
        ///     $2
        /// }
    };
}

pub(super) struct SnippetMatchElse;

impl Snippet for SnippetMatchElse {
    const LABEL: &'static str = "else";

    const DOCUMENTATION: &'static str = markup! {
        /// Default match arm.
        ///
        /// ```<adastra>
        /// match test_expr {
        ///     else => {}
        /// }
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// else => {
        ///     $1
        /// }
    };
}

pub(super) struct SnippetFor;

impl Snippet for SnippetFor {
    const LABEL: &'static str = "for";

    const DOCUMENTATION: &'static str = markup! {
        /// Loop in range.
        ///
        /// ```<adastra>
        /// for iterator in 20..300 {
        ///
        /// }
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// for $1 in $2 {
        ///     $3
        /// }
    };
}

pub(super) struct SnippetLoop;

impl Snippet for SnippetLoop {
    const LABEL: &'static str = "loop";

    const DOCUMENTATION: &'static str = markup! {
        /// Inbounded loop.
        ///
        /// ```<adastra>
        /// loop {
        ///     foo.bar();
        ///
        ///     if something {
        ///         break;
        ///     }
        /// }
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// loop {
        ///     $1
        /// }
    };
}

pub(super) struct SnippetBreak;

impl Snippet for SnippetBreak {
    const LABEL: &'static str = "break";

    const DOCUMENTATION: &'static str = markup! {
        /// Loop break.
        ///
        /// ```<adastra>
        /// loop {
        ///     if condition {
        ///         break;
        ///     }
        /// }
        ///
        /// for x in 1..100 {
        ///     if x > 10 {
        ///         break;
        ///     }
        /// }
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// break;
    };
}

pub(super) struct SnippetContinue;

impl Snippet for SnippetContinue {
    const LABEL: &'static str = "continue";

    const DOCUMENTATION: &'static str = markup! {
        /// Loop continuation.
        ///
        /// ```<adastra>
        /// loop {
        ///     if condition {
        ///         continue;
        ///     }
        /// }
        ///
        /// for x in 1..100 {
        ///     if x > 10 {
        ///         continue;
        ///     }
        /// }
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// continue;
    };
}

pub(super) struct SnippetReturn;

impl Snippet for SnippetReturn {
    const LABEL: &'static str = "return";

    const DOCUMENTATION: &'static str = markup! {
        /// Return from function.
        ///
        /// ```<adastra>
        /// fn() {
        ///     return 100;
        /// }
        ///
        /// fn() {
        ///     return;
        /// }
        /// ```
    };

    const SNIPPET: &'static str = markup! {
        /// return $1;
    };
}
