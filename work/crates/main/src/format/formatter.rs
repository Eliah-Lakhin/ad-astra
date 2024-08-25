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
    format::{PrettyPrintConfig, PrettyPrinter},
    lexis::{Line, SourceCode, TokenBuffer, TokenRule},
    syntax::{AbstractNode, ParseNode, ParseNodeChild, ParseToken, ParseTree, PolyRef, SyntaxTree},
};

use crate::syntax::{Assoc, Precedence, ScriptDoc, ScriptNode, ScriptToken};

/// Configuration options for the Ad Astra script formatting utility.
///
/// Used as an argument for the [format_script_text] and
/// [ModuleText::format](crate::analysis::ModuleText::format) functions.
///
/// The [Default] implementation of this object provides canonical configuration
/// options, though you can customize some formatting options at your
/// discretion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ScriptFormatConfig {
    /// The number of characters the formatter should keep in a line before
    /// breaking the content into multiple lines.
    ///
    /// The default value is `80`.
    pub margin: u16,

    /// If the formatter breaks a line into multiple lines, it should attempt
    /// to keep at least the `inline` number of characters in each line,
    /// relative to the current indentation.
    ///
    /// The default value is `60`.
    pub inline: u16,

    /// The number of whitespace characters for a single level of indentation.
    ///
    /// The default value is `4`.
    pub indent: u16,

    /// If set to true, the formatter preserves excessive parentheses in
    /// expressions.
    ///
    /// For example, the formatter will keep the expression `(1 + 2) + 3` as it
    /// is, even though it could otherwise be rewritten as `1 + 2 + 3`.
    ///
    /// The default value is `false`, meaning that the formatter will attempt
    /// to remove unnecessary parentheses whenever possible.
    pub preserve_expr_groups: bool,

    /// If set to true, the formatter preserves at most one blank line between
    /// script code statements. Otherwise, the formatter will eliminate
    /// excessive blank lines.
    ///
    /// The default value is `true`.
    pub preserve_blank_lines: bool,

    /// If set to true, the formatter preserves statement blocks even if they
    /// could clearly be merged into the surrounding code.
    ///
    /// For example, the formatter will keep the code `foo(); { bar(); } baz();`
    /// as it is, even though it could otherwise be rewritten as
    /// `foo(); bar(); baz();`.
    ///
    /// The default value is `false`, meaning that the formatter will attempt
    /// to merge blocks together.
    pub preserve_blocks: bool,

    /// When set to true, the formatter attempts to keep short code blocks
    /// in line.
    ///
    /// For example, the formatter will keep the code `{ foo(); }` in line
    /// instead of breaking it into three lines:
    ///
    /// ```text
    /// {
    ///     foo();
    /// }
    /// ```
    ///
    /// The default value is `false`, meaning that the formatter will typically
    /// break single-statement blocks into multiple lines.
    pub compact_blocks: bool,
}

impl Default for ScriptFormatConfig {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptFormatConfig {
    /// The default constructor for the configuration.
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            margin: 80,
            inline: 60,
            indent: 4,
            preserve_expr_groups: false,
            preserve_blank_lines: true,
            preserve_blocks: false,
            compact_blocks: false,
        }
    }
}

/// Formats source code text according to the Ad Astra formatting rules.
///
/// The function formats the specified source code `text` in accordance with the
/// rules and the `config` options, preserving code comments and other elements
/// as per the original author's intentions, whenever the original code format
/// does not contradict the canonical formatting rules.
///
/// The function returns None if the source code contains parse errors.
/// Currently, Ad Astra cannot format source code with syntax errors, but
/// it can format code regardless of its semantics.
///
/// Unlike the [ModuleText::format](crate::analysis::ModuleText::format)
/// function, this function does not require creating a dedicated
/// [ScriptModule](crate::analysis::ScriptModule) when you need to format
/// arbitrary Ad Astra text one time.
#[inline(always)]
pub fn format_script_text(config: ScriptFormatConfig, text: impl AsRef<str>) -> Option<String> {
    let buffer = TokenBuffer::from(text);
    let tree = ParseTree::new(&buffer, ..);

    ScriptFormatter::format(config, &tree)
}

#[inline(always)]
pub(crate) fn format_script_doc(config: ScriptFormatConfig, doc: &ScriptDoc) -> Option<String> {
    let tree = ParseTree::new(doc, ..);

    ScriptFormatter::format(config, &tree)
}

struct ScriptFormatter<'a, C: SourceCode<Token = ScriptToken>> {
    config: ScriptFormatConfig,
    tree: &'a ParseTree<'a, ScriptNode, C>,
    printer: PrettyPrinter,
    state: State,
}

impl<'a, C: SourceCode<Token = ScriptToken>> ScriptFormatter<'a, C> {
    fn format(
        config: ScriptFormatConfig,
        tree: &'a ParseTree<'a, ScriptNode, C>,
    ) -> Option<String> {
        if tree.errors().next().is_some() {
            return None;
        }

        let mut printer_config = PrettyPrintConfig::new();

        printer_config.margin = config.margin;
        printer_config.inline = config.inline;
        printer_config.indent = config.indent;
        printer_config.debug = false;

        let printer = PrettyPrinter::new(printer_config);

        let mut formatter = Self {
            config,
            tree,
            printer,
            state: State::Break1,
        };

        formatter.format_node(tree.parse_tree_root());

        match &formatter.state {
            State::Break1 | State::PendingBreak2 => (),
            _ => formatter.printer.hardbreak(),
        }

        Some(formatter.printer.finish())
    }

    fn format_node(&mut self, parse_node: &ParseNode) {
        if !parse_node.well_formed {
            self.print_node_as_is(parse_node);
            return;
        }

        let Some(syntax_node) = parse_node.node_ref.deref(self.tree) else {
            return;
        };

        match syntax_node {
            ScriptNode::InlineComment { .. } => self.format_inline_comment(parse_node),
            ScriptNode::MultilineComment { .. } => self.format_multiline_comment(parse_node),
            ScriptNode::Root { .. } => {
                let _ = self.format_block(parse_node, BlockUnwrap::AsIs);
            }
            ScriptNode::Clause { .. } => self.format_concat(parse_node),
            ScriptNode::Use { .. } => self.format_concat(parse_node),
            ScriptNode::Package { .. } => self.format_concat(parse_node),
            ScriptNode::If { .. } => self.format_concat(parse_node),
            ScriptNode::Match { .. } => self.format_concat(parse_node),
            ScriptNode::MatchBody { .. } => self.format_list(parse_node),
            ScriptNode::MatchArm { .. } => {
                let _ = self.format_match_arm(parse_node);
            }
            ScriptNode::Else { .. } => self.format_concat(parse_node),
            ScriptNode::Let { .. } => self.format_concat(parse_node),
            ScriptNode::Var { .. } => self.format_concat(parse_node),
            ScriptNode::For { .. } => self.format_concat(parse_node),
            ScriptNode::Loop { .. } => self.format_concat(parse_node),
            ScriptNode::Block { .. } => {
                let _ = self.format_block(parse_node, BlockUnwrap::AsIs);
            }
            ScriptNode::Break { .. } => self.format_concat(parse_node),
            ScriptNode::Continue { .. } => self.format_concat(parse_node),
            ScriptNode::Return { .. } => self.format_concat(parse_node),
            ScriptNode::Fn { .. } => self.format_fn(parse_node),
            ScriptNode::FnParams { .. } => self.format_list(parse_node),
            ScriptNode::Struct { .. } => self.format_concat(parse_node),
            ScriptNode::StructBody { .. } => self.format_list(parse_node),
            ScriptNode::StructEntry { .. } => self.format_concat(parse_node),
            ScriptNode::StructEntryKey { .. } => self.format_concat(parse_node),
            ScriptNode::Array { .. } => self.format_list(parse_node),
            ScriptNode::String { .. } => self.print_node_as_is(parse_node),
            ScriptNode::Crate { .. } => self.format_concat(parse_node),
            ScriptNode::This { .. } => self.format_concat(parse_node),
            ScriptNode::Ident { .. } => self.format_concat(parse_node),
            ScriptNode::Number { .. } => self.format_concat(parse_node),
            ScriptNode::Max { .. } => self.format_concat(parse_node),
            ScriptNode::Bool { .. } => self.format_concat(parse_node),
            ScriptNode::UnaryLeft { .. } => self.format_expr(parse_node),
            ScriptNode::Binary { .. } => self.format_expr(parse_node),
            ScriptNode::Op { .. } => self.format_concat(parse_node),
            ScriptNode::Query { .. } => self.format_expr(parse_node),
            ScriptNode::Call { .. } => self.format_expr(parse_node),
            ScriptNode::CallArgs { .. } => self.format_list(parse_node),
            ScriptNode::Index { .. } => self.format_expr(parse_node),
            ScriptNode::IndexArg { .. } => self.format_list(parse_node),
            ScriptNode::Field { .. } => self.format_concat(parse_node),
            ScriptNode::Expr { .. } => self.format_expr(parse_node),
        }
    }

    fn format_inline_comment(&mut self, parse_node: &ParseNode) {
        let text = match parse_node.breaks() > 0 {
            false => self.tree.substring(&parse_node.site_span).into_owned(),

            true => self
                .tree
                .substring(parse_node.site_span.start..(parse_node.site_span.end - 1))
                .into_owned(),
        };

        match &self.state {
            State::Break1 => (),

            State::PendingBreak2 => self.printer.hardbreak(),

            State::Word { line, .. } | State::EmbeddedEnd { line } => {
                match line == &parse_node.start_line() {
                    true => self.printer.word(" "),
                    false => self.printer.hardbreak(),
                }
            }

            State::ParenOpen {
                consistent,
                dedent_next,
                ..
            }
            | State::BracketOpen {
                consistent,
                dedent_next,
                ..
            } => {
                match *consistent {
                    true => self.printer.cbox(1),
                    false => self.printer.ibox(1),
                }

                self.printer.hardbreak();

                if *dedent_next {
                    match *consistent {
                        true => self.printer.cbox(-1),
                        false => self.printer.ibox(-1),
                    }
                }
            }

            State::BraceOpen {
                reenter,
                reenter_consistency,
                ..
            } => {
                self.printer.neverbreak();
                self.printer.cbox(1);
                self.printer.hardbreak();

                if *reenter {
                    match *reenter_consistency {
                        true => {
                            self.printer.cbox(1);
                            self.printer.cbox(-1);
                        }

                        false => {
                            self.printer.ibox(1);
                            self.printer.ibox(-1);
                        }
                    }
                }
            }

            State::PendingSep {
                sep,
                line,
                embedded_after,
                ..
            } => {
                if !sep.is_empty() {
                    self.printer.word(*sep);
                }

                for comment in embedded_after {
                    self.printer.word(" ");
                    self.printer.word(comment);
                }

                match line == &parse_node.start_line() {
                    true => self.printer.word(" "),
                    false => self.printer.hardbreak(),
                }
            }
        }

        self.printer.word(text);
        self.printer.hardbreak();

        self.state = State::Break1;
    }

    fn format_multiline_comment(&mut self, parse_node: &ParseNode) {
        let text = self.tree.substring(&parse_node.site_span).into_owned();

        let embedded = parse_node.breaks() == 0;

        match &mut self.state {
            State::Break1 => (),

            State::PendingBreak2 => self.printer.hardbreak(),

            State::Word { line, .. } | State::EmbeddedEnd { line } => {
                match embedded && line == &parse_node.start_line() {
                    true => self.printer.word(" "),
                    false => self.printer.hardbreak(),
                }
            }

            State::ParenOpen {
                line,
                consistent,
                dedent_next,
            }
            | State::BracketOpen {
                line,
                consistent,
                dedent_next,
            } => {
                match *consistent {
                    true => self.printer.cbox(1),
                    false => self.printer.ibox(0),
                }

                match embedded && line == &parse_node.start_line() {
                    true => self.printer.softbreak(),
                    false => self.printer.hardbreak(),
                }

                if *dedent_next {
                    match *consistent {
                        true => self.printer.cbox(-1),
                        false => self.printer.ibox(0),
                    }
                }
            }

            State::BraceOpen {
                line,
                reenter,
                reenter_consistency,
            } => {
                self.printer.neverbreak();
                self.printer.cbox(1);

                match embedded && self.config.compact_blocks && line == &parse_node.start_line() {
                    true => self.printer.blank(),
                    false => self.printer.hardbreak(),
                }

                if *reenter {
                    match *reenter_consistency {
                        true => {
                            self.printer.cbox(1);
                            self.printer.cbox(-1);
                        }

                        false => {
                            self.printer.ibox(1);
                            self.printer.ibox(-1);
                        }
                    }
                }
            }

            State::PendingSep {
                sep,
                line,
                is_last,
                embedded_after,
            } => match embedded && (line == &parse_node.start_line() || *is_last) {
                true => {
                    embedded_after.push(text);
                    return;
                }

                false => {
                    if !sep.is_empty() {
                        self.printer.word(*sep);
                    }

                    for comment in embedded_after {
                        self.printer.word(" ");
                        self.printer.word(comment.to_string());
                    }

                    self.printer.hardbreak();
                }
            },
        }

        let alignment = parse_node
            .position_span
            .start
            .column
            .checked_sub(1)
            .unwrap_or_default();

        let mut is_first = true;

        for mut string in text.as_str().split("\n") {
            match is_first {
                true => is_first = false,

                false => {
                    self.printer.hardbreak();
                    string = dedent_line(string, alignment);
                }
            }

            self.printer.word(string);
        }

        match embedded {
            true => {
                self.state = State::EmbeddedEnd {
                    line: parse_node.end_line(),
                };
            }

            false => {
                self.printer.hardbreak();
                self.state = State::Break1;
            }
        }
    }

    fn format_block(&mut self, parse_node: &ParseNode, mut unwrap: BlockUnwrap) -> BlockUnwrap {
        fn find_one(children: &[ParseNodeChild]) -> Option<&ParseNode> {
            let mut candidate = None;

            for child in children {
                if let ParseNodeChild::Node(child) = child {
                    if let ScriptNode::INLINE_COMMENT | ScriptNode::MULTILINE_COMMENT = child.rule {
                        return None;
                    }

                    if candidate.is_some() {
                        return None;
                    }

                    candidate = Some(child);
                }
            }

            candidate
        }

        if self.config.preserve_blocks {
            unwrap = BlockUnwrap::AsIs;
        }

        match &unwrap {
            BlockUnwrap::AsIs => (),

            BlockUnwrap::UnwrapOuter | BlockUnwrap::UnwrapOuterEmpty => {
                for child in &parse_node.children {
                    if let ParseNodeChild::Node(child) = child {
                        if let ScriptNode::INLINE_COMMENT
                        | ScriptNode::MULTILINE_COMMENT
                        | ScriptNode::LET = child.rule
                        {
                            unwrap = BlockUnwrap::AsIs;
                            break;
                        }
                    }
                }
            }

            BlockUnwrap::UnwrapClause => {
                if let Some(single) = find_one(&parse_node.children) {
                    if single.rule == ScriptNode::CLAUSE {
                        if let Some(single) = find_one(&single.children) {
                            self.format_node(single);

                            return unwrap;
                        }
                    }
                }

                unwrap = BlockUnwrap::AsIs;
            }

            BlockUnwrap::UnwrapReturn => {
                if let Some(single) = find_one(&parse_node.children) {
                    if single.rule == ScriptNode::RETURN {
                        if let Some(single) = find_one(&single.children) {
                            let mut is_fn = false;

                            if let Some(ScriptNode::Expr { inner, .. }) =
                                single.node_ref.deref(self.tree)
                            {
                                if let Some(ScriptNode::Fn { .. }) = inner.deref(self.tree) {
                                    is_fn = true;
                                }
                            }

                            if !is_fn {
                                self.format_node(single);

                                return unwrap;
                            }
                        }
                    }
                }

                unwrap = BlockUnwrap::AsIs;
            }
        }

        let mut inner_empty = true;
        let mut st_printed = false;

        for (index, child) in parse_node.children.iter().enumerate() {
            match child {
                ParseNodeChild::Blank(child) => {
                    if !self.config.preserve_blank_lines {
                        continue;
                    }

                    if inner_empty {
                        continue;
                    }

                    let next = parse_node.children.get(index + 1);

                    if let Some(ParseNodeChild::Token(token)) = next {
                        if let Some((_, open_or_close)) = Group::from_token_rule(token.rule) {
                            if let OpenOrClose::Close = open_or_close {
                                continue;
                            };
                        }
                    }

                    let breaks = child.breaks();

                    self.format_break(breaks, false);
                }

                ParseNodeChild::Token(child) => {
                    if child.rule == ScriptToken::Semicolon as u8 {
                        continue;
                    }

                    if let Some((group, open_or_close)) = Group::from_token_rule(child.rule) {
                        if !unwrap.as_is() {
                            continue;
                        }

                        match open_or_close {
                            OpenOrClose::Open => self.print_open(group, child.start_line(), true),
                            OpenOrClose::Close => self.print_close(group, child.start_line()),
                        }

                        continue;
                    }

                    self.format_token(child, false, false);
                }

                ParseNodeChild::Node(child) => match child.rule {
                    ScriptNode::BLOCK => {
                        match &self.state {
                            State::Break1 | State::PendingBreak2 => self.flush(child.start_line()),

                            _ => {
                                if st_printed {
                                    self.format_break(1, true);
                                }
                            }
                        }

                        match self.format_block(child, BlockUnwrap::UnwrapOuter) {
                            BlockUnwrap::UnwrapOuterEmpty => (),

                            _ => {
                                inner_empty = false;
                                st_printed = true;
                            }
                        }
                    }

                    ScriptNode::INLINE_COMMENT => {
                        self.format_node(child);
                        inner_empty = false;
                    }

                    ScriptNode::MULTILINE_COMMENT => {
                        self.format_node(child);
                        inner_empty = false;
                    }

                    _ => {
                        match &self.state {
                            State::Break1 | State::PendingBreak2 => self.flush(child.start_line()),

                            _ => {
                                if st_printed {
                                    self.format_break(1, true);
                                }
                            }
                        }

                        self.format_node(child);

                        inner_empty = false;
                        st_printed = true;
                    }
                },
            }
        }

        if let BlockUnwrap::UnwrapOuter = &unwrap {
            if inner_empty {
                return BlockUnwrap::UnwrapOuterEmpty;
            }
        }

        unwrap
    }

    fn format_list(&mut self, parse_node: &ParseNode) {
        enum ListState {
            Begin,
            Next,
            ItemPrinted {
                end_line: Line,
                is_last: bool,
                is_block: bool,
            },
        }

        let mut last_item = 0;
        let mut consistent = false;

        for (index, child) in parse_node.children.iter().enumerate() {
            match child {
                ParseNodeChild::Blank(_) => (),

                ParseNodeChild::Token(child) => {
                    if child.rule == ScriptToken::BraceOpen as u8 {
                        consistent = true;
                    }
                }

                ParseNodeChild::Node(child) => {
                    match child.rule {
                        ScriptNode::INLINE_COMMENT | ScriptNode::MULTILINE_COMMENT => {
                            consistent = true;

                            continue;
                        }

                        ScriptNode::EXPR => {
                            if !is_simple_expr(child) {
                                consistent = true;
                            }
                        }

                        _ => (),
                    }

                    last_item = index;
                }
            }
        }

        let mut state = ListState::Begin;

        'outer: for (index, child) in parse_node.children.iter().enumerate() {
            match child {
                ParseNodeChild::Blank(child) => {
                    if !self.config.preserve_blank_lines {
                        continue;
                    }

                    if !consistent {
                        continue;
                    }

                    match &state {
                        ListState::Begin => continue,

                        ListState::ItemPrinted { .. } => {
                            let mut lookahead = index + 1;

                            while let Some(next) = parse_node.children.get(lookahead) {
                                match next {
                                    ParseNodeChild::Blank(..) => (),
                                    ParseNodeChild::Token(..) => continue 'outer,
                                    ParseNodeChild::Node(next) => {
                                        let (ScriptNode::MULTILINE_COMMENT
                                        | ScriptNode::INLINE_COMMENT) = next.rule
                                        else {
                                            break;
                                        };
                                    }
                                }

                                lookahead += 1;
                            }
                        }

                        ListState::Next => (),
                    }

                    let next = parse_node.children.get(index + 1);

                    if let Some(ParseNodeChild::Token(token)) = next {
                        if let Some((_, open_or_close)) = Group::from_token_rule(token.rule) {
                            if let OpenOrClose::Close = open_or_close {
                                continue;
                            };
                        }
                    }

                    self.format_break(child.breaks(), false);
                }

                ParseNodeChild::Token(child) => {
                    let start_line = child.start_line();

                    if let Some((group, open_or_close)) = Group::from_token_rule(child.rule) {
                        match open_or_close {
                            OpenOrClose::Open => self.print_open(group, start_line, consistent),

                            OpenOrClose::Close => {
                                if let ListState::ItemPrinted {
                                    end_line,
                                    is_last,
                                    is_block,
                                    ..
                                } = &state
                                {
                                    self.print_sep(
                                        match *is_block {
                                            true => "",
                                            false => ",",
                                        },
                                        *end_line,
                                        *is_last,
                                    );
                                }

                                self.print_close(group, start_line);
                            }
                        }

                        continue;
                    }

                    if child.rule == ScriptToken::Comma as u8 {
                        let ListState::ItemPrinted {
                            is_last, is_block, ..
                        } = &state
                        else {
                            continue;
                        };

                        self.print_sep(
                            match *is_block {
                                true => "",
                                false => ",",
                            },
                            start_line,
                            *is_last,
                        );

                        state = ListState::Next;

                        continue;
                    }

                    self.format_token(child, !consistent, false);
                }

                ParseNodeChild::Node(child) => match child.rule {
                    ScriptNode::INLINE_COMMENT => match &state {
                        ListState::Begin | ListState::Next => {
                            self.format_node(child);
                            state = ListState::Next;
                        }

                        ListState::ItemPrinted {
                            end_line,
                            is_last,
                            is_block,
                        } => {
                            self.print_sep(
                                match *is_block {
                                    true => "",
                                    false => ",",
                                },
                                *end_line,
                                *is_last,
                            );

                            self.format_node(child);
                            state = ListState::Next;
                        }
                    },

                    ScriptNode::MULTILINE_COMMENT => match &state {
                        ListState::Begin | ListState::Next => {
                            self.format_node(child);
                            state = ListState::Next;
                        }

                        ListState::ItemPrinted {
                            end_line,
                            is_last,
                            is_block,
                        } => {
                            if child.breaks() == 0 && end_line == &child.start_line() {
                                self.format_node(child);
                                continue;
                            }

                            self.print_sep(
                                match *is_block {
                                    true => "",
                                    false => ",",
                                },
                                *end_line,
                                *is_last,
                            );

                            self.format_node(child);
                            state = ListState::Next;
                        }
                    },

                    ScriptNode::MATCH_ARM => {
                        if let ListState::ItemPrinted {
                            end_line, is_block, ..
                        } = &state
                        {
                            self.print_sep(
                                match *is_block {
                                    true => "",
                                    false => ",",
                                },
                                *end_line,
                                false,
                            );
                        }

                        self.format_break(1, true);

                        let end_line = child.end_line();

                        let format = self.format_match_arm(child);

                        state = ListState::ItemPrinted {
                            end_line,
                            is_last: last_item == index,
                            is_block: match format {
                                MatchArmFormat::EndsWithExpr => false,
                                MatchArmFormat::EndsWithBlock => true,
                            },
                        };
                    }

                    _ => {
                        self.format_node(child);

                        state = ListState::ItemPrinted {
                            end_line: child.end_line(),
                            is_last: last_item == index,
                            is_block: false,
                        };
                    }
                },
            }
        }
    }

    fn format_match_arm(&mut self, parse_node: &ParseNode) -> MatchArmFormat {
        let mut result = MatchArmFormat::EndsWithExpr;

        let mut arrow_scanned = false;

        for child in &parse_node.children {
            match child {
                ParseNodeChild::Blank(_) => (),

                ParseNodeChild::Token(child) => {
                    if child.rule == ScriptToken::Arrow as u8 {
                        arrow_scanned = true;
                    }

                    self.format_token(child, true, true);
                }

                ParseNodeChild::Node(child) => {
                    if arrow_scanned {
                        if child.rule == ScriptNode::BLOCK {
                            if let BlockUnwrap::AsIs =
                                self.format_block(child, BlockUnwrap::UnwrapClause)
                            {
                                result = MatchArmFormat::EndsWithBlock;
                            }

                            continue;
                        }
                    }

                    self.format_node(child);
                }
            }
        }

        result
    }

    fn format_fn(&mut self, parse_node: &ParseNode) {
        for child in &parse_node.children {
            match child {
                ParseNodeChild::Blank(_) => (),

                ParseNodeChild::Token(child) => {
                    self.format_token(child, true, false);
                }

                ParseNodeChild::Node(child) => {
                    if child.rule == ScriptNode::BLOCK {
                        if let Some(script_node) = parse_node.node_ref.deref(self.tree) {
                            if let Some(script_node) = script_node.parent_ref().deref(self.tree) {
                                if let ScriptNode::Expr { .. } = script_node {
                                    let _ = self.format_block(child, BlockUnwrap::UnwrapReturn);
                                    return;
                                }
                            }
                        }
                    }

                    self.format_node(child);
                }
            }
        }
    }

    fn format_expr(&mut self, parse_node: &ParseNode) {
        let Some(ScriptNode::Expr { start, .. }) = parse_node.node_ref.deref(self.tree) else {
            return;
        };

        if !start.is_nil() {
            self.format_group(parse_node);
            return;
        }

        let mut flatten = Vec::new();

        self.flat_expr(parse_node, &mut flatten);

        self.format_flatten(flatten);
    }

    fn format_group(&mut self, parse_node: &ParseNode) {
        let mut flatten = Vec::new();

        self.flat_expr(parse_node, &mut flatten);

        let mut consistent = false;

        for child in &flatten {
            if child.requires_consistency(self.tree) {
                consistent = true;
                break;
            }
        }

        self.print_open(Group::Paren, parse_node.start_line(), consistent);

        self.format_flatten(flatten);

        self.print_close(Group::Paren, parse_node.end_line());
    }

    fn format_flatten(&mut self, flatten: Vec<FlatChild>) {
        let mut first_binary_operator = None;

        for (index, child) in flatten.iter().enumerate() {
            let FlatChild::OperatorMiddle(_) = child else {
                continue;
            };

            first_binary_operator = Some(index);
            break;
        }

        let mut boxed = false;

        if first_binary_operator.is_some() {
            let mut consistent = false;

            for child in &flatten {
                if child.requires_consistency(self.tree) {
                    consistent = true;
                    break;
                }
            }

            match &mut self.state {
                State::ParenOpen { dedent_next, .. } | State::BracketOpen { dedent_next, .. } => {
                    *dedent_next = true;
                }

                State::BraceOpen {
                    reenter,
                    reenter_consistency,
                    ..
                } => {
                    *reenter = true;
                    *reenter_consistency = consistent;
                    boxed = true;
                }

                _ => {
                    match consistent {
                        true => {
                            self.printer.cbox(1);
                            self.printer.cbox(-1);
                        }

                        false => {
                            self.printer.ibox(1);
                            self.printer.ibox(-1);
                        }
                    }

                    boxed = true;
                }
            };
        }

        for (index, child) in flatten.iter().enumerate() {
            match child {
                FlatChild::OperatorLeft(child) => {
                    let mut has_comment_after = false;

                    if let Some(next_child) = flatten.get(index + 1) {
                        if let FlatChild::Comment(_) = next_child {
                            has_comment_after = true;
                        }
                    }

                    for child in &child.children {
                        let ParseNodeChild::Token(token) = child else {
                            continue;
                        };

                        let Some(text) = token.token_ref.string(self.tree) else {
                            continue;
                        };

                        let line = child.start_line();

                        self.print_word(text, line, true, !has_comment_after, true);
                    }
                }

                FlatChild::OperatorMiddle(child) => {
                    if let Some(first_binary_operator) = first_binary_operator {
                        if first_binary_operator == index {
                            self.printer.end();
                        }
                    }

                    for child in &child.children {
                        let ParseNodeChild::Token(token) = child else {
                            continue;
                        };

                        let Some(text) = token.token_ref.string(self.tree) else {
                            continue;
                        };

                        let line = child.start_line();

                        self.print_word(text, line, false, false, true);
                    }
                }

                FlatChild::OperatorRight(child) => self.format_node(child),

                FlatChild::Operand(child) => {
                    self.format_node(child);
                }

                FlatChild::Group(child) => self.format_group(child),

                FlatChild::Comment(child) => self.format_node(child),
            }
        }

        if boxed {
            self.printer.end();
        }
    }

    fn format_concat(&mut self, parse_node: &ParseNode) {
        for child in &parse_node.children {
            match child {
                ParseNodeChild::Blank(_) => (),

                ParseNodeChild::Token(child) => {
                    self.format_token(child, true, false);
                }

                ParseNodeChild::Node(child) => {
                    self.format_node(child);
                }
            }
        }
    }

    fn format_break(&mut self, breaks: usize, force: bool) {
        match &self.state {
            State::PendingBreak2
            | State::ParenOpen { .. }
            | State::BraceOpen { .. }
            | State::BracketOpen { .. } => (),

            State::Break1 => {
                if breaks >= 1 {
                    self.state = State::PendingBreak2;
                }
            }

            State::Word { .. } => {
                if breaks >= 2 || force {
                    self.printer.hardbreak();
                    self.state = State::Break1;
                }

                if breaks >= 2 {
                    self.state = State::PendingBreak2;
                }
            }

            State::EmbeddedEnd { .. } => {
                if breaks >= 1 {
                    self.printer.hardbreak();
                    self.state = State::Break1;
                }

                if breaks >= 2 {
                    self.state = State::PendingBreak2;
                }
            }

            State::PendingSep {
                sep,
                embedded_after,
                ..
            } => {
                if breaks >= 2 || force {
                    if !sep.is_empty() {
                        self.printer.word(*sep);
                    }

                    for comment in embedded_after {
                        self.printer.word(" ");
                        self.printer.word(comment);
                    }

                    self.printer.hardbreak();
                    self.state = State::Break1;
                }

                if breaks >= 2 {
                    self.state = State::PendingBreak2;
                }
            }
        }
    }

    fn format_token(&mut self, parse_token: &ParseToken, concat: bool, concat_next: bool) {
        let Some(text) = parse_token.token_ref.string(self.tree) else {
            return;
        };

        let line = parse_token.start_line();

        self.print_word(text, line, concat, false, concat_next);
    }

    fn print_node_as_is(&mut self, parse_node: &ParseNode) {
        let text = self.tree.substring(&parse_node.site_span).into_owned();

        let alignment = parse_node
            .position_span
            .start
            .column
            .checked_sub(1)
            .unwrap_or_default();

        let mut line = parse_node.start_line();

        let mut is_first = true;

        for mut string in text.as_str().split("\n") {
            match is_first {
                true => is_first = false,
                false => {
                    self.printer.hardbreak();
                    string = dedent_line(string, alignment);
                }
            }

            self.print_word(string, line, true, false, false);
            line += 1;
        }
    }

    fn print_sep(&mut self, sep: &'static str, line: Line, is_last: bool) {
        self.state = State::PendingSep {
            sep,
            line,
            is_last,
            embedded_after: Vec::new(),
        };
    }

    fn print_open(&mut self, group: Group, line: Line, consistent: bool) {
        self.print_word(group.open(), line, true, false, false);

        match group {
            Group::Paren => {
                self.state = State::ParenOpen {
                    line,
                    consistent,
                    dedent_next: false,
                }
            }
            Group::Brace => {
                self.state = State::BraceOpen {
                    line,
                    reenter: false,
                    reenter_consistency: false,
                }
            }
            Group::Bracket => {
                self.state = State::BracketOpen {
                    line,
                    consistent,
                    dedent_next: false,
                }
            }
        }
    }

    fn print_close(&mut self, group: Group, line: Line) {
        let stickiness = match &group {
            Group::Brace => 2,
            _ => 0,
        };

        match &self.state {
            State::Break1 | State::PendingBreak2 => (),

            State::Word { .. } | State::EmbeddedEnd { .. } => match group {
                Group::Brace if self.config.compact_blocks => self.printer.blank(),
                Group::Brace => self.printer.hardbreak(),
                _ => self.printer.softbreak(),
            },

            State::ParenOpen { .. } | State::BraceOpen { .. } | State::BracketOpen { .. } => {
                self.printer.word(group.close());
                self.state = State::Word {
                    stickiness,
                    concat_next: false,
                    line,
                };
                return;
            }

            State::PendingSep {
                sep,
                embedded_after,
                ..
            } => {
                let mut embedded = String::new();

                for comment in embedded_after.iter() {
                    embedded.push(' ');
                    embedded.push_str(comment);
                }

                match group {
                    Group::Brace => self.printer.blank(),
                    _ => self.printer.softbreak(),
                }

                self.printer.pre_space(&embedded);
                self.printer.pre_break(format!("{sep}{embedded}"));
            }
        }

        self.printer.indent(-1);
        self.printer.end();
        self.printer.word(group.close());

        self.state = State::Word {
            stickiness,
            concat_next: false,
            line,
        }
    }

    fn print_word(
        &mut self,
        text: &str,
        line: Line,
        mut concat: bool,
        stick_next: bool,
        concat_next: bool,
    ) {
        let stickiness_left;
        let mut stickiness_right;

        match text {
            "{" | "}" => {
                stickiness_left = 2;
                stickiness_right = 2;
            }

            "(" | ")" | "[" | "]" | "." | ".." => {
                stickiness_left = -1;
                stickiness_right = -1;
            }

            ";" | ":" | "?" => {
                concat = true;
                stickiness_left = -2;
                stickiness_right = 2;
            }

            _ => {
                let is_alphanum = text
                    .chars()
                    .any(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '"');

                match is_alphanum {
                    true => {
                        stickiness_left = 1;
                        stickiness_right = 1;
                    }
                    false => {
                        stickiness_left = 2;
                        stickiness_right = 2;
                    }
                }
            }
        };

        if stick_next {
            stickiness_right = -2;
        }

        match &self.state {
            State::Break1 => (),

            State::PendingBreak2 => {
                self.printer.hardbreak();
            }

            State::Word {
                concat_next,
                stickiness,
                ..
            } => {
                let sticky = *stickiness + stickiness_left <= 0;
                let concat = concat || *concat_next;

                match (sticky, concat) {
                    (true, true) => (),
                    (true, false) => self.printer.softbreak(),
                    (false, true) => self.printer.word(" "),
                    (false, false) => self.printer.blank(),
                }
            }

            State::ParenOpen {
                consistent,
                dedent_next,
                ..
            }
            | State::BracketOpen {
                consistent,
                dedent_next,
                ..
            } => match (*consistent, *dedent_next) {
                (true, false) => {
                    self.printer.cbox(1);
                    self.printer.softbreak();
                }
                (false, false) => {
                    self.printer.ibox(1);
                }
                (true, true) => {
                    self.printer.cbox(1);
                    self.printer.softbreak();

                    self.printer.cbox(-1);
                }
                (false, true) => {
                    self.printer.ibox(1);
                    self.printer.ibox(-1);
                }
            },

            State::BraceOpen {
                reenter,
                reenter_consistency,
                ..
            } => {
                self.printer.neverbreak();
                self.printer.cbox(1);

                match self.config.compact_blocks {
                    true => self.printer.blank(),
                    false => self.printer.hardbreak(),
                }

                if *reenter {
                    match *reenter_consistency {
                        true => {
                            self.printer.cbox(1);
                            self.printer.cbox(-1);
                        }

                        false => {
                            self.printer.ibox(1);
                            self.printer.ibox(-1);
                        }
                    }
                }
            }

            State::EmbeddedEnd {
                line: embedded_line,
            } => {
                let sticky = 2 + stickiness_left <= 0;
                let concat = concat && embedded_line == &line;

                match (sticky, concat) {
                    (true, true) => (),
                    (true, false) => self.printer.softbreak(),
                    (false, true) => self.printer.word(" "),
                    (false, false) => self.printer.blank(),
                }
            }

            State::PendingSep {
                sep,
                line: sep_line,
                is_last,
                embedded_after,
            } => match sep_line == &line {
                true => {
                    match *is_last {
                        true => {
                            self.printer.softbreak();
                            self.printer.pre_break(*sep);
                        }

                        false => {
                            self.printer.blank();
                            self.printer.pre_space(*sep);
                            self.printer.pre_break(*sep);
                        }
                    }

                    for comment in embedded_after.iter() {
                        self.printer.word(comment);
                        self.printer.word(" ");
                    }
                }

                false => {
                    let mut embedded = String::new();

                    for comment in embedded_after.iter() {
                        embedded.push(' ');
                        embedded.push_str(comment);
                    }

                    match *is_last {
                        true => {
                            self.printer.softbreak();
                            self.printer.pre_space(&embedded);
                            self.printer.pre_break(format!("{sep}{embedded}"));
                        }

                        false => {
                            embedded = format!("{sep}{embedded}");

                            self.printer.blank();
                            self.printer.pre_space(&embedded);
                            self.printer.pre_break(embedded);
                        }
                    }
                }
            },
        }

        self.printer.word(text);

        self.state = State::Word {
            stickiness: stickiness_right,
            concat_next,
            line,
        };
    }

    fn flush(&mut self, line: Line) {
        self.print_word("", line, true, true, true);
    }

    fn flat<'b>(
        &self,
        parse_node: &'b ParseNode,
        parent_op: Precedence,
        assoc: Assoc,
        unwrap: bool,
        result: &mut Vec<FlatChild<'b>>,
    ) {
        if !parse_node.well_formed {
            result.push(FlatChild::Operand(parse_node));
            return;
        }

        match parse_node.rule {
            ScriptNode::MULTILINE_COMMENT | ScriptNode::INLINE_COMMENT => {
                result.push(FlatChild::Comment(parse_node))
            }

            ScriptNode::STRING
            | ScriptNode::CRATE
            | ScriptNode::THIS
            | ScriptNode::IDENT
            | ScriptNode::NUMBER
            | ScriptNode::BOOL
            | ScriptNode::MAX
            | ScriptNode::FIELD
            | ScriptNode::FN
            | ScriptNode::STRUCT
            | ScriptNode::ARRAY => result.push(FlatChild::Operand(parse_node)),

            ScriptNode::UNARY_LEFT
            | ScriptNode::BINARY
            | ScriptNode::QUERY
            | ScriptNode::CALL
            | ScriptNode::INDEX => self.flat_infix(parse_node, result),

            ScriptNode::EXPR => {
                let Some(ScriptNode::Expr { start, .. }) = parse_node.node_ref.deref(self.tree)
                else {
                    return;
                };

                match start.is_nil() {
                    true => self.flat_expr(parse_node, result),
                    false => self.flat_group(parse_node, parent_op, assoc, unwrap, result),
                }
            }

            _ => (),
        }
    }

    fn flat_infix<'b>(&self, parse_node: &'b ParseNode, result: &mut Vec<FlatChild<'b>>) {
        enum ScanState {
            Begin,
            LeftOperand,
            Operator,
            RightOperand,
        }

        let mut scan_state = ScanState::Begin;

        for child in &parse_node.children {
            let ParseNodeChild::Node(child) = &child else {
                continue;
            };

            match child.rule {
                ScriptNode::OP | ScriptNode::CALL_ARGS | ScriptNode::INDEX_ARG => {
                    scan_state = ScanState::Operator;
                }

                _ => {
                    scan_state = match &scan_state {
                        ScanState::Begin | ScanState::LeftOperand => ScanState::LeftOperand,
                        ScanState::Operator | ScanState::RightOperand => ScanState::RightOperand,
                    }
                }
            }
        }

        let has_right_operand = match &scan_state {
            ScanState::RightOperand => true,
            _ => false,
        };

        scan_state = ScanState::Begin;

        let Some(script_node) = parse_node.node_ref.deref(self.tree) else {
            return;
        };

        let parent_op = script_node.precedence(self.tree);

        for child in &parse_node.children {
            let ParseNodeChild::Node(child) = &child else {
                continue;
            };

            match child.rule {
                ScriptNode::MULTILINE_COMMENT | ScriptNode::INLINE_COMMENT => {
                    result.push(FlatChild::Comment(child));
                }

                ScriptNode::OP | ScriptNode::CALL_ARGS | ScriptNode::INDEX_ARG => {
                    match &scan_state {
                        ScanState::Begin => result.push(FlatChild::OperatorLeft(child)),
                        ScanState::LeftOperand => match has_right_operand {
                            true => result.push(FlatChild::OperatorMiddle(child)),
                            false => result.push(FlatChild::OperatorRight(child)),
                        },

                        _ => continue,
                    }

                    scan_state = ScanState::Operator;
                }

                _ => {
                    let Some(script_node) = child.node_ref.deref(self.tree) else {
                        return;
                    };

                    let operand_op = script_node.precedence(self.tree);

                    scan_state = match &scan_state {
                        ScanState::Begin | ScanState::LeftOperand => {
                            self.flat(
                                child,
                                parent_op,
                                Assoc::Left,
                                operand_op.as_operand(Assoc::Left).of(parent_op),
                                result,
                            );

                            ScanState::LeftOperand
                        }

                        ScanState::Operator | ScanState::RightOperand => {
                            self.flat(
                                child,
                                parent_op,
                                Assoc::Right,
                                operand_op.as_operand(Assoc::Right).of(parent_op),
                                result,
                            );

                            ScanState::RightOperand
                        }
                    }
                }
            }
        }
    }

    fn flat_expr<'b>(&self, parse_node: &'b ParseNode, result: &mut Vec<FlatChild<'b>>) {
        let mut unwrap = true;

        for child in &parse_node.children {
            let ParseNodeChild::Node(child) = &child else {
                continue;
            };

            if let ScriptNode::MULTILINE_COMMENT | ScriptNode::INLINE_COMMENT = child.rule {
                unwrap = false;
                break;
            }
        }

        for child in &parse_node.children {
            let ParseNodeChild::Node(child) = &child else {
                continue;
            };

            self.flat(child, Precedence::Outer, Assoc::Left, unwrap, result);
        }
    }

    fn flat_group<'b>(
        &self,
        parse_node: &'b ParseNode,
        parent_op: Precedence,
        assoc: Assoc,
        unwrap: bool,
        result: &mut Vec<FlatChild<'b>>,
    ) {
        if self.config.preserve_expr_groups || !unwrap {
            result.push(FlatChild::Group(parse_node));
            return;
        }

        let mut inner = None;

        for child in &parse_node.children {
            let ParseNodeChild::Node(child) = &child else {
                continue;
            };

            match child.rule {
                ScriptNode::MULTILINE_COMMENT | ScriptNode::INLINE_COMMENT => {
                    result.push(FlatChild::Group(parse_node));
                    return;
                }

                _ => inner = Some(child),
            }
        }

        let Some(inner) = inner else {
            result.push(FlatChild::Group(parse_node));
            return;
        };

        if let ScriptNode::FN = inner.rule {
            result.push(FlatChild::Group(parse_node));
            return;
        }

        let Some(inner_node) = inner.node_ref.deref(self.tree) else {
            result.push(FlatChild::Group(parse_node));
            return;
        };

        let fits = inner_node
            .precedence(self.tree)
            .as_operand(assoc)
            .of(parent_op);

        if !fits {
            result.push(FlatChild::Group(parse_node));
            return;
        }

        self.flat(inner, parent_op, assoc, true, result);
    }
}

enum State {
    Break1,

    PendingBreak2,

    Word {
        stickiness: i8,
        concat_next: bool,
        line: Line,
    },

    ParenOpen {
        line: Line,
        consistent: bool,
        dedent_next: bool,
    },

    BraceOpen {
        line: Line,
        reenter: bool,
        reenter_consistency: bool,
    },

    BracketOpen {
        line: Line,
        consistent: bool,
        dedent_next: bool,
    },

    EmbeddedEnd {
        line: Line,
    },

    PendingSep {
        sep: &'static str,
        line: Line,
        is_last: bool,
        embedded_after: Vec<String>,
    },
}

enum Group {
    Paren,
    Brace,
    Bracket,
}

impl Group {
    #[inline(always)]
    fn from_token_rule(rule: TokenRule) -> Option<(Self, OpenOrClose)> {
        if rule == ScriptToken::ParenOpen as u8 {
            return Some((Self::Paren, OpenOrClose::Open));
        }

        if rule == ScriptToken::BraceOpen as u8 {
            return Some((Self::Brace, OpenOrClose::Open));
        }

        if rule == ScriptToken::BracketOpen as u8 {
            return Some((Self::Bracket, OpenOrClose::Open));
        }

        if rule == ScriptToken::ParenClose as u8 {
            return Some((Self::Paren, OpenOrClose::Close));
        }

        if rule == ScriptToken::BraceClose as u8 {
            return Some((Self::Brace, OpenOrClose::Close));
        }

        if rule == ScriptToken::BracketClose as u8 {
            return Some((Self::Bracket, OpenOrClose::Close));
        }

        None
    }

    #[inline(always)]
    fn open(&self) -> &'static str {
        match self {
            Self::Paren => "(",
            Self::Brace => "{",
            Self::Bracket => "[",
        }
    }

    #[inline(always)]
    fn close(&self) -> &'static str {
        match self {
            Self::Paren => ")",
            Self::Brace => "}",
            Self::Bracket => "]",
        }
    }
}

enum OpenOrClose {
    Open,
    Close,
}

enum BlockUnwrap {
    AsIs,
    UnwrapOuter,
    UnwrapOuterEmpty,
    UnwrapClause,
    UnwrapReturn,
}

impl BlockUnwrap {
    #[inline(always)]
    fn as_is(&self) -> bool {
        match self {
            Self::AsIs => true,
            _ => false,
        }
    }
}

enum MatchArmFormat {
    EndsWithExpr,
    EndsWithBlock,
}

enum FlatChild<'a> {
    OperatorLeft(&'a ParseNode),
    OperatorMiddle(&'a ParseNode),
    OperatorRight(&'a ParseNode),
    Operand(&'a ParseNode),
    Group(&'a ParseNode),
    Comment(&'a ParseNode),
}

impl<'a> FlatChild<'a> {
    fn requires_consistency<C: SourceCode<Token = ScriptToken>>(
        &self,
        tree: &ParseTree<ScriptNode, C>,
    ) -> bool {
        match self {
            Self::Comment(_) => return true,

            Self::OperatorRight(operator) => {
                let Some(script_node) = operator.node_ref.deref(tree) else {
                    return true;
                };

                match script_node {
                    ScriptNode::CallArgs { .. } | ScriptNode::IndexArg { .. } => {
                        for child in &operator.children {
                            match child {
                                ParseNodeChild::Blank(_) => (),
                                ParseNodeChild::Token(_) => (),
                                ParseNodeChild::Node(child) => {
                                    if !is_simple_expr(child) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }

                    _ => (),
                }
            }

            Self::Operand(operand) => {
                if let ScriptNode::FIELD | ScriptNode::ARRAY | ScriptNode::FN | ScriptNode::STRUCT =
                    operand.rule
                {
                    return true;
                }
            }

            _ => (),
        }

        false
    }
}

fn is_simple_expr(parse_node: &ParseNode) -> bool {
    for child in &parse_node.children {
        let ParseNodeChild::Node(child) = &child else {
            continue;
        };

        match child.rule {
            ScriptNode::PACKAGE
            | ScriptNode::ELSE
            | ScriptNode::VAR
            | ScriptNode::STRUCT_ENTRY_KEY
            | ScriptNode::STRING
            | ScriptNode::CRATE
            | ScriptNode::THIS
            | ScriptNode::IDENT
            | ScriptNode::NUMBER
            | ScriptNode::BOOL
            | ScriptNode::OP => (),

            ScriptNode::EXPR => {
                if !is_simple_expr(child) {
                    return false;
                }
            }

            _ => return false,
        };
    }

    true
}

fn dedent_line(string: &str, mut alignment: usize) -> &str {
    let bytes = string.as_bytes();
    let mut index = 0;

    while alignment > 0 {
        let Some(&b' ') = bytes.get(index) else {
            break;
        };

        index += 1;
        alignment -= 1;
    }

    &string[index..]
}
