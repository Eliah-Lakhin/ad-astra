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

use std::fmt::{Display, Formatter};

use lady_deirdre::{
    arena::{Id, Identifiable},
    format::{AnnotationPriority, SnippetConfig, SnippetFormatter, Style, TerminalString},
    lexis::{SiteSpan, ToSpan, TokenBuffer},
};

use crate::{
    format::highlight::ScriptHighlighter,
    runtime::PackageMeta,
    syntax::{ScriptDoc, ScriptToken},
};

/// A configuration of options for drawing the [ScriptSnippet] object.
///
/// The [Default] implementation of this object provides canonical configuration
/// options.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub struct ScriptSnippetConfig {
    /// Whether the boxed frame should surround the code content from all sides.
    ///
    /// The default value is `true`.
    pub show_outer_frame: bool,

    /// Whether line numbers should be shown on the left of the code content.
    ///
    /// The default value is `true`.
    pub show_line_numbers: bool,

    /// Whether the full canonical script module path should be shown in the
    /// caption of the printed snippet.
    ///
    /// If set to true, the snippet caption will look like:
    /// `‹package name›.‹module name› [<custom caption>]`.
    ///
    /// Otherwise, the printer will only use the
    /// [custom caption](ScriptSnippet::set_caption) if specified.
    ///
    /// The default value is `true`.
    pub show_module_path: bool,

    /// If set to true, syntax highlighting will be applied to the source code.
    /// Otherwise, the source code will be monochrome.
    ///
    /// The default value is `true`.
    pub highlight_code: bool,

    /// Whether the snippet printer should use Unicode
    /// [box drawing characters](https://en.wikipedia.org/wiki/Box-drawing_characters#Box_Drawing)
    /// for decorative elements. Otherwise, the printer uses only ASCII box-drawing
    /// characters.
    ///
    /// The default value is `true`.
    pub unicode_drawing: bool,
}

impl Default for ScriptSnippetConfig {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl From<ScriptSnippetConfig> for SnippetConfig {
    #[inline(always)]
    fn from(value: ScriptSnippetConfig) -> Self {
        let mut config = Self::verbose();

        config.draw_frame = value.show_outer_frame;
        config.show_numbers = value.show_line_numbers;
        config.ascii_drawing = !value.unicode_drawing;

        if !value.highlight_code {
            config.dim_code = false;
            config.style = false;
        }

        config
    }
}

impl ScriptSnippetConfig {
    /// The default constructor for the configuration.
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            show_outer_frame: true,
            show_line_numbers: true,
            show_module_path: true,
            highlight_code: true,
            unicode_drawing: true,
        }
    }

    /// The constructor for the configuration object that returns a
    /// configuration with all advanced drawing options disabled.
    #[inline(always)]
    pub const fn minimal() -> Self {
        Self {
            show_outer_frame: false,
            show_line_numbers: false,
            show_module_path: false,
            highlight_code: false,
            unicode_drawing: false,
        }
    }
}

/// A drawing object that renders the source code text of a
/// [ScriptModule](crate::analysis::ScriptModule) with syntax highlighting and
/// annotated source code ranges.
///
/// The intended use of this object is printing script source code to the
/// terminal.
///
/// ```text
///    ╭──╢ ‹doctest›.‹my_module.adastra› ╟────────────────────────────────────────╮
///  1 │                                                                           │
///  2 │     let foo = 10;                                                         │
///    │         ╰╴ Annotation text.                                               │
///  3 │     let bar = foo + 20;                                                   │
///  4 │                                                                           │
///    ╰───────────────────────────────────────────────────────────────────────────╯
/// ```
///
/// There are several crate API functions that create this object, such as
/// [ModuleText::snippet](crate::analysis::ModuleText::snippet) and
/// [ModuleDiagnostics::highlight](crate::analysis::ModuleDiagnostics::highlight).
///
/// The [Display] implementation of this object performs the actual snippet
/// rendering. For example, you can print the snippet to the terminal using the
/// `println` macro: `println!("{my_snippet}")`
pub struct ScriptSnippet<'a> {
    code: SnippetCode<'a>,
    config: ScriptSnippetConfig,
    caption: Option<String>,
    annotations: Vec<(SiteSpan, AnnotationPriority, String)>,
    summary: Option<String>,
}

impl<'a> Display for ScriptSnippet<'a> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let mut caption = String::with_capacity(512);

        match self.config.show_module_path {
            true => {
                if let Some(prefix) = &self.caption {
                    caption.push_str(&prefix.apply(Style::new().bold()));
                    caption.push_str(" [");
                }

                let id = match &self.code {
                    SnippetCode::Borrowed(code) => code.id(),
                    SnippetCode::Owned(code) => code.id(),
                };

                caption.push_str(
                    format_script_path(id, PackageMeta::by_id(id))
                        .apply(Style::new().bright_cyan())
                        .as_str(),
                );

                if self.caption.is_some() {
                    caption.push(']');
                }
            }

            false => {
                if let Some(prefix) = &self.caption {
                    caption.push_str(prefix)
                }
            }
        }

        match &self.code {
            SnippetCode::Borrowed(code) => {
                let config = self.config.into();

                let mut snippet = formatter.snippet(*code);

                snippet.set_config(&config).set_caption(caption);

                if self.config.highlight_code {
                    snippet.set_highlighter(ScriptHighlighter::new());
                }

                if let Some(summary) = &self.summary {
                    snippet.set_summary(summary.as_str());
                }

                for (span, priority, message) in &self.annotations {
                    snippet.annotate(span, *priority, message.as_str());
                }

                snippet.finish()?;
            }

            SnippetCode::Owned(code) => {
                let config = self.config.into();

                let mut snippet = formatter.snippet(code);

                snippet.set_config(&config).set_caption(caption);

                if self.config.highlight_code {
                    snippet.set_highlighter(ScriptHighlighter::new());
                }

                if let Some(summary) = &self.summary {
                    snippet.set_summary(summary.as_str());
                }

                for (span, priority, message) in &self.annotations {
                    snippet.annotate(span, *priority, message.as_str());
                }

                snippet.finish()?;
            }
        }

        Ok(())
    }
}

impl<S: AsRef<str>> From<S> for ScriptSnippet<'static> {
    #[inline(always)]
    fn from(string: S) -> Self {
        let buffer = TokenBuffer::from(string);

        Self::new(SnippetCode::Owned(buffer))
    }
}

impl<'a> ScriptSnippet<'a> {
    #[inline(always)]
    fn new(code: SnippetCode<'a>) -> Self {
        Self {
            code,
            config: ScriptSnippetConfig::default(),
            caption: None,
            annotations: Vec::new(),
            summary: None,
        }
    }

    #[inline(always)]
    pub(crate) fn from_doc(doc: &'a ScriptDoc) -> Self {
        Self::new(SnippetCode::Borrowed(doc))
    }

    /// Sets the configuration for snippet drawing features.
    ///
    /// See [ScriptSnippetConfig] for details.
    pub fn set_config(&mut self, config: ScriptSnippetConfig) -> &mut Self {
        self.config = config;

        self
    }

    /// Sets the caption of the printed content.
    ///
    /// The `caption` string will be printed in the header of the snippet.
    /// By default, the caption is an empty string, and in this case, the
    /// renderer does not include a custom caption in the header.
    ///
    /// The `caption` parameter must be a single-line string. Any additional
    /// caption lines (separated by the `\n` character) will be ignored.
    pub fn set_caption(&mut self, caption: impl AsRef<str>) -> &mut Self {
        self.caption = caption
            .as_ref()
            .lines()
            .next()
            .map(|line| String::from(line));

        self
    }

    /// Sets the footer summary text of the printed content.
    ///
    /// The `summary` string will be printed below the source code. By default,
    /// the summary is an empty string, and in this case, the renderer does not
    /// print any footer text.
    ///
    /// Unlike the [caption](Self::set_caption) and
    /// [annotation](Self::annotate) text, the summary text can have
    /// multiple lines.
    pub fn set_summary(&mut self, summary: impl AsRef<str>) -> &mut Self {
        self.summary = Some(String::from(summary.as_ref()));

        self
    }

    /// Adds an annotation to the source code.
    ///
    /// The `span` argument specifies the source code range intended for
    /// annotation. You can use a `10..20` absolute Unicode character range, the
    /// [line-column](lady_deirdre::lexis::Position) range
    /// `Position::new(10, 3)..Position::new(12, 4)`, or the [ScriptOrigin]
    /// instance. The span argument must represent a
    /// [valid](ToSpan::is_valid_span) value (e.g., `20..10` is not a valid
    /// range because the upper bound is less than the lower bound). Otherwise,
    /// the annotation will be silently ignored.
    ///
    /// The `priority` argument specifies the annotation priority. The snippet
    /// interface supports the following priority types:
    ///
    /// - [AnnotationPriority::Default]: A default annotation. The spanned text
    ///   will be simply inverted (e.g., white text on a black background).
    /// - [AnnotationPriority::Primary]: The spanned text will be inverted with
    ///   a red background.
    /// - [AnnotationPriority::Secondary]: The spanned text will be inverted
    ///   with a blue background.
    /// - [AnnotationPriority::Note]: The spanned text will be inverted with a
    ///   yellow background.
    ///
    /// The `message` argument specifies the text that should label the spanned
    /// range. The message should be a single-line string. Any additional message
    /// lines (separated by the `\n` character) will be ignored.
    ///
    /// You can leave the message as an empty string. In this case, the renderer
    /// will not label the spanned text.
    ///
    /// Note that if the ScriptSnippet does not have any annotations, the object
    /// will render the entire source code. Otherwise, the renderer will output
    /// only the annotated lines plus a few lines of surrounding context.
    pub fn annotate(
        &mut self,
        span: impl ToSpan,
        priority: AnnotationPriority,
        message: impl AsRef<str>,
    ) -> &mut Self {
        let span = match &self.code {
            SnippetCode::Borrowed(code) => span.to_site_span(*code),
            SnippetCode::Owned(code) => span.to_site_span(code),
        };

        let Some(span) = span else {
            return self;
        };

        let message = message
            .as_ref()
            .lines()
            .next()
            .map(|line| String::from(line))
            .unwrap_or(String::new());

        self.annotations.push((span, priority, message));

        self
    }
}

#[inline(always)]
pub(crate) fn format_script_path(id: Id, package: Option<&'static PackageMeta>) -> String {
    let mut path = String::with_capacity(512);

    if let Some(package) = package {
        path.push_str(&format!("‹{}›.", package));
    }

    let name = id.name();

    match name.is_empty() {
        true => path.push_str(&format!("‹#{}›", id.into_inner())),
        false => path.push_str(&format!("‹{}›", name.escape_debug())),
    }

    path
}

enum SnippetCode<'a> {
    Borrowed(&'a ScriptDoc),
    Owned(TokenBuffer<ScriptToken>),
}
