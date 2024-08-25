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
    format::AnnotationPriority,
    lexis::{PositionSpan, ToSpan},
};
use lsp_types::{MarkupContent, MarkupKind, Position, Range, Uri};

use crate::{
    analysis::{symbols::SymbolKind, Description, ModuleRead, ModuleText},
    format::ScriptSnippetConfig,
    runtime::{ScriptOrigin, TypeHint},
    server::rpc::LspHandle,
};

#[inline(always)]
pub(super) fn range_to_span(range: &Range) -> PositionSpan {
    lsp_position_to_ld(&range.start)..lsp_position_to_ld(&range.end)
}

#[inline(always)]
pub(super) fn span_to_range(span: &PositionSpan) -> Range {
    Range {
        start: ld_position_to_lsp(&span.start),
        end: ld_position_to_lsp(&span.end),
    }
}

#[inline(always)]
pub(super) fn lsp_position_to_ld(position: &Position) -> lady_deirdre::lexis::Position {
    lady_deirdre::lexis::Position {
        line: position.line as usize + 1,
        column: position.character as usize + 1,
    }
}

#[inline(always)]
pub(super) fn ld_position_to_lsp(position: &lady_deirdre::lexis::Position) -> Position {
    Position {
        line: position.line.checked_sub(1).unwrap_or_default() as u32,
        character: position.column.checked_sub(1).unwrap_or_default() as u32,
    }
}

pub(super) fn uri_to_name(uri: &Uri) -> Option<&str> {
    if let Some(name) = uri.path().segments().last() {
        return Some(name.as_str());
    }

    None
}

#[inline(always)]
pub(super) fn make_doc(
    read: &impl ModuleRead<LspHandle>,
    text: &ModuleText,
    markdown: bool,
    language_id: &str,
    fallback_to_type: bool,
    desc: &Description,
) -> Option<MarkupContent> {
    let impl_origin = match desc.impl_symbol.kind() {
        SymbolKind::Use | SymbolKind::Package => ScriptOrigin::nil(),
        _ => desc.impl_symbol.origin(read),
    };

    let value = match impl_origin.is_valid_span(text) {
        true => {
            let mut snippet = text.snippet();

            let mut config = ScriptSnippetConfig::minimal();

            config.show_line_numbers = true;
            config.unicode_drawing = true;

            snippet
                .set_config(config)
                .annotate(impl_origin, AnnotationPriority::Default, "");

            match markdown {
                false => snippet.to_string(),
                true => format!("```{language_id}\n{snippet}\n```"),
            }
        }

        false => match desc.doc {
            Some(doc) if !doc.is_empty() => match desc.type_hint {
                TypeHint::Invocation(meta) if fallback_to_type => {
                    format!("{meta}\n\n{doc}")
                }

                _ => String::from(doc),
            },

            _ => match fallback_to_type && !desc.type_hint.is_dynamic() {
                true => desc.type_hint.to_string(),

                _ => return None,
            },
        },
    };

    Some(MarkupContent {
        kind: match markdown {
            true => MarkupKind::Markdown,
            false => MarkupKind::PlainText,
        },

        value,
    })
}
