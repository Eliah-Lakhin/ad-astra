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
    arena::Identifiable,
    format::{AnnotationPriority, Style, TerminalString},
    lexis::{SiteRef, SourceCode, ToSpan, Token, TokenCursor},
    syntax::{AbstractNode, PolyRef, PolyVariant, RefKind},
};

use crate::{
    format::ScriptSnippet,
    runtime::ScriptOrigin,
    syntax::{ScriptDoc, ScriptNode},
};

impl<P: PolyRef + ?Sized> PolyRefOrigin for P {}

pub(crate) trait PolyRefOrigin: PolyRef {
    fn script_origin(&self, doc: &ScriptDoc, bounds: SpanBounds) -> ScriptOrigin {
        match self.kind() {
            RefKind::Token => ScriptOrigin::from(self.as_token_ref()),

            RefKind::Node => match self.as_node_ref().deref(doc) {
                Some(node) => node.script_origin(doc, bounds),
                None => ScriptOrigin::default(),
            },
        }
    }

    fn script_display<'a>(&self, doc: &'a ScriptDoc, suffix: impl AsRef<str>) -> ScriptSnippet<'a> {
        let span = self
            .script_origin(doc, SpanBounds::Cover)
            .to_site_span(doc)
            .unwrap_or(0..0);

        let mut caption = match self.as_variant() {
            PolyVariant::Token(variant) => {
                let name = variant
                    .deref(doc)
                    .map(|token| token.name())
                    .flatten()
                    .unwrap_or("?");

                let inner =
                    format!("(token: {:?})", variant.entry).apply(Style::new().bright_black());

                format!("${name}({inner})")
            }

            PolyVariant::Node(variant) => {
                let name = variant
                    .deref(doc)
                    .map(|node| node.name())
                    .flatten()
                    .unwrap_or("?");

                let inner =
                    format!("(node: {:?})", variant.entry).apply(Style::new().bright_black());

                format!("{name}({inner})")
            }
        };

        let suffix = suffix.as_ref();

        if !suffix.is_empty() {
            caption.push_str(&format!(" → {suffix}"));
        }

        let mut snippet = ScriptSnippet::from_doc(doc);

        snippet
            .set_caption(caption)
            .annotate(span, AnnotationPriority::Default, "");

        snippet
    }
}

impl ScriptNode {
    pub(crate) fn script_origin(&self, doc: &ScriptDoc, bounds: SpanBounds) -> ScriptOrigin {
        match self {
            Self::InlineComment { start, end, .. } | Self::MultilineComment { start, end, .. } => {
                let mut span = ScriptOrigin::from(start);

                if !bounds.is_header() {
                    match end.is_nil() {
                        true => span.unbound(),
                        false => span.union(&ScriptOrigin::from(end)),
                    }
                }

                span
            }

            Self::Root { .. } => {
                if let SpanBounds::Footer = bounds {
                    let eoi = SiteRef::end_of(doc.id());
                    return ScriptOrigin::from(eoi..eoi);
                }

                doc.cursor(..).site_ref(0).into()
            }

            _ => match bounds {
                SpanBounds::Header => {
                    for capture in self.children_iter() {
                        let origin = capture.script_origin(doc, SpanBounds::Header);

                        if origin.is_nil() {
                            continue;
                        }

                        return origin;
                    }

                    ScriptOrigin::nil()
                }

                SpanBounds::Footer => {
                    for capture in self.children_iter().rev() {
                        let origin = capture.script_origin(doc, SpanBounds::Footer);

                        if origin.is_nil() {
                            continue;
                        }

                        return origin;
                    }

                    ScriptOrigin::nil()
                }

                SpanBounds::Cover => {
                    let mut origin = self.script_origin(doc, SpanBounds::Header);

                    if origin.is_nil() {
                        return origin;
                    }

                    let footer = self.script_origin(doc, SpanBounds::Footer);

                    origin.union(&footer);

                    origin
                }
            },
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum SpanBounds {
    Header,
    Footer,
    Cover,
}

impl SpanBounds {
    #[inline(always)]
    fn is_header(&self) -> bool {
        match self {
            Self::Header => true,
            _ => false,
        }
    }
}
