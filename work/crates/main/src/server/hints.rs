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

use std::thread::park_timeout;

use ahash::AHashSet;
use lady_deirdre::{
    arena::Identifiable,
    lexis::{Position, SourceCode, ToSite, ToSpan},
    sync::Trigger,
};
use log::{error, warn};
use lsp_types::{
    error_codes::{REQUEST_CANCELLED, REQUEST_FAILED},
    request::InlayHintRequest,
    InlayHint,
    InlayHintKind,
    InlayHintLabel,
    InlayHintTooltip,
    MarkupContent,
    MarkupKind,
    Range,
    Uri,
};

use crate::{
    analysis::{
        symbols::{
            ArraySymbol,
            CallSymbol,
            FieldSymbol,
            FnKind,
            FnSymbol,
            IdentKind,
            IdentSymbol,
            IndexSymbol,
            LookupOptions,
            ModuleSymbol,
            SymbolKind,
            VarSymbol,
        },
        Closeness,
        Description,
        ModuleError,
        ModuleRead,
        ModuleReadGuard,
        ModuleResult,
        ModuleText,
        StringEstimation,
    },
    runtime::Ident,
    server::{
        command::{CustomMessage, SharedRunnerState},
        file::{LspModule, ANALYSIS_PRIORITY},
        logger::LSP_SERVER_LOG,
        rpc::{LspHandle, OutgoingEx, RpcId, RpcLatches},
        tasks::{Task, TaskExecution, COOL_DOWN},
        utils::{ld_position_to_lsp, make_doc, range_to_span},
        LspServerConfig,
        RpcSender,
    },
    syntax::ScriptToken,
};

pub(super) struct SendInlayHints {
    pub(super) config: LspServerConfig,
    pub(super) latches: RpcLatches,
    pub(super) outgoing: RpcSender,
    pub(super) module: LspModule,
    pub(super) runner_state: SharedRunnerState,
}

impl Task for SendInlayHints {
    const EXECUTION: TaskExecution = TaskExecution::ExecuteEach;

    type Config = Self;

    type Message = SendInlayHintsMessage;

    #[inline(always)]
    fn init(config: Self::Config) -> Self {
        config
    }

    fn handle(&mut self, message: Self::Message) -> bool {
        'outer: loop {
            if message.cancel.is_active() {
                warn!(target: LSP_SERVER_LOG, "[{}] Send inlay hints cancelled by the client.", message.uri.as_str());

                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_CANCELLED,
                    "Send inlay hints cancelled by the client.",
                );

                break;
            }

            let handle = LspHandle::new(&message.cancel);

            let module_read_guard = match self.module.as_ref().read(&handle, ANALYSIS_PRIORITY) {
                Ok(guard) => guard,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send inlay hints cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send inlay hints cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send inlay hints interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send inlay hints error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send inlay hints error.",
                    );

                    break;
                }
            };

            let span = range_to_span(&message.range);

            const HINT_SYMBOLS: u32 = (SymbolKind::Var as u32)
                | (SymbolKind::Fn as u32)
                | (SymbolKind::Array as u32)
                | (SymbolKind::Ident as u32)
                | (SymbolKind::Field as u32)
                | (SymbolKind::Call as u32)
                | (SymbolKind::Index as u32);

            let symbols = match module_read_guard
                .symbols(span, LookupOptions::new().filter(HINT_SYMBOLS))
            {
                Ok(symbols) => symbols,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send inlay hints cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send inlay hints cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send inlay hints interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send inlay hints error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send inlay hints error.",
                    );

                    break;
                }
            };

            let text = module_read_guard.text();

            let mut builder = HintsBuilder {
                markdown: true,
                language_id: self.config.language_id,
                read: &module_read_guard,
                text: &text,
                hint_index: AHashSet::with_capacity(symbols.len()),
                hints: Vec::with_capacity(symbols.len()),
            };

            {
                let runner_state_guard = self
                    .runner_state
                    .as_ref()
                    .read()
                    .unwrap_or_else(|poison| poison.into_inner());

                builder.add_custom_messages(&runner_state_guard.messages);
            }

            for symbol in symbols {
                match builder.add_symbol(symbol) {
                    Ok(()) => (),

                    Err(ModuleError::Interrupted(_)) => {
                        if message.cancel.is_active() {
                            warn!(target: LSP_SERVER_LOG, "[{}] Send inlay hints cancelled by the client.", message.uri.as_str());

                            self.outgoing.send_err_response(
                                &self.latches,
                                message.id,
                                REQUEST_CANCELLED,
                                "Send inlay hints cancelled by the client.",
                            );

                            break 'outer;
                        }

                        warn!(target: LSP_SERVER_LOG, "[{}] Send inlay hints interrupted.", message.uri.as_str());
                        park_timeout(COOL_DOWN);
                        continue 'outer;
                    }

                    Err(error) => {
                        error!(target: LSP_SERVER_LOG, "[{}] Send inlay hints error. {error}", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_FAILED,
                            "Send inlay hints error.",
                        );

                        break 'outer;
                    }
                }
            }

            self.outgoing.send_ok_response::<InlayHintRequest>(
                &self.latches,
                message.id,
                Some(builder.hints),
            );

            break;
        }

        true
    }

    #[inline(always)]
    fn module(&self) -> &LspModule {
        &self.module
    }
}

pub(super) struct SendInlayHintsMessage {
    pub(super) id: RpcId,
    pub(super) uri: Uri,
    pub(super) cancel: Trigger,
    pub(super) range: Range,
}

struct HintsBuilder<'a> {
    markdown: bool,
    language_id: &'a str,
    read: &'a ModuleReadGuard<'a, LspHandle>,
    text: &'a ModuleText<'a>,
    hint_index: AHashSet<Position>,
    hints: Vec<InlayHint>,
}

impl<'a> HintsBuilder<'a> {
    fn add_custom_messages(&mut self, messages: &[CustomMessage]) {
        for CustomMessage {
            origin,
            hint,
            tooltip,
        } in messages.iter().rev()
        {
            let Some(span) = origin.to_position_span(self.text) else {
                continue;
            };

            let position = Position::new(
                span.start.line,
                self.text.lines().line_length(span.start.line).max(1),
            );

            let tooltip = match tooltip.is_empty() {
                true => None,
                false => Some(MarkupContent {
                    kind: match self.markdown {
                        true => MarkupKind::Markdown,
                        false => MarkupKind::PlainText,
                    },
                    value: tooltip.clone(),
                }),
            };

            self.insert_hint(position, String::from(hint), None, tooltip);
        }
    }

    fn add_symbol(&mut self, symbol: ModuleSymbol) -> ModuleResult<()> {
        if self.read.is_interrupted() {
            return Err(ModuleError::Interrupted(self.read.id()));
        }

        match symbol {
            ModuleSymbol::Nil => Ok(()),
            ModuleSymbol::Use(_) => Ok(()),
            ModuleSymbol::Package(_) => Ok(()),
            ModuleSymbol::Var(symbol) => self.add_var(symbol),
            ModuleSymbol::Loop(_) => Ok(()),
            ModuleSymbol::Break(_) => Ok(()),
            ModuleSymbol::Fn(symbol) => self.add_fn(symbol),
            ModuleSymbol::Return(_) => Ok(()),
            ModuleSymbol::Struct(_) => Ok(()),
            ModuleSymbol::Array(symbol) => self.add_array(symbol),
            ModuleSymbol::Entry(_) => Ok(()),
            ModuleSymbol::Ident(symbol) => self.add_ident(symbol),
            ModuleSymbol::Field(symbol) => self.add_field(symbol),
            ModuleSymbol::Literal(_) => Ok(()),
            ModuleSymbol::Operator(_) => Ok(()),
            ModuleSymbol::Call(symbol) => self.add_call(symbol),
            ModuleSymbol::Index(symbol) => self.add_index(symbol),
        }
    }

    fn add_var(&mut self, symbol: VarSymbol) -> ModuleResult<()> {
        let origin = symbol.origin(self.read);

        let Some(span) = origin.to_position_span(self.text) else {
            return Ok(());
        };

        let ty = symbol.var_type(self.read)?;

        if ty.type_hint.is_dynamic() {
            return Ok(());
        }

        if let ModuleSymbol::Struct(..) | ModuleSymbol::Literal(..) = symbol.let_value(self.read) {
            return Ok(());
        }

        let tooltip = make_doc(
            self.read,
            self.text,
            self.markdown,
            self.language_id,
            false,
            &ty,
        );

        self.insert_hint(
            span.end,
            format!(": {}", ty.type_hint),
            Some(InlayHintKind::TYPE),
            tooltip,
        );

        Ok(())
    }

    fn add_fn(&mut self, symbol: FnSymbol) -> ModuleResult<()> {
        let FnKind::Multiline = symbol.kind(self.read) else {
            return Ok(());
        };

        let params_origin = symbol.params_origin(self.read);

        let Some(span) = params_origin.to_position_span(self.text) else {
            return Ok(());
        };

        let return_type = symbol.return_type(self.read)?;

        if return_type.type_hint.is_dynamic() {
            return Ok(());
        }

        let tooltip = make_doc(
            self.read,
            self.text,
            self.markdown,
            self.language_id,
            false,
            &return_type,
        );

        self.insert_hint(
            span.end,
            format!(" -> {}", return_type.type_hint),
            Some(InlayHintKind::TYPE),
            tooltip,
        );

        Ok(())
    }

    fn add_array(&mut self, symbol: ArraySymbol) -> ModuleResult<()> {
        let origin = symbol.origin(self.read);

        let Some(span) = origin.to_position_span(self.text) else {
            return Ok(());
        };

        if !self.is_eol(&span.end) {
            return Ok(());
        }

        let ty = symbol.ty(self.read)?;

        if ty.type_hint.is_dynamic() {
            return Ok(());
        }

        let tooltip = make_doc(
            self.read,
            self.text,
            self.markdown,
            self.language_id,
            false,
            &ty,
        );

        self.insert_hint(
            span.end,
            format!(": {}", ty.type_hint),
            Some(InlayHintKind::TYPE),
            tooltip,
        );

        Ok(())
    }

    fn add_ident(&mut self, symbol: IdentSymbol) -> ModuleResult<()> {
        match symbol.kind(self.read)? {
            IdentKind::Invalid => return Ok(()),
            IdentKind::CrateAccess => (),
            IdentKind::PackageAccess => (),
            IdentKind::VarAccess => (),
            IdentKind::VarDefinition => return Ok(()),
            IdentKind::CrateIdent => return Ok(()),
            IdentKind::SelfIdent => (),
        };

        let origin = symbol.origin(self.read);

        let Some(span) = origin.to_position_span(self.text) else {
            return Ok(());
        };

        if !self.is_eol(&span.end) {
            return Ok(());
        }

        let ty = symbol.ty(self.read)?;

        if ty.type_hint.is_dynamic() {
            return Ok(());
        }

        let tooltip = make_doc(
            self.read,
            self.text,
            self.markdown,
            self.language_id,
            false,
            &ty,
        );

        self.insert_hint(
            span.end,
            format!(": {}", ty.type_hint),
            Some(InlayHintKind::TYPE),
            tooltip,
        );

        Ok(())
    }

    fn add_field(&mut self, symbol: FieldSymbol) -> ModuleResult<()> {
        let origin = symbol.origin(self.read);

        let Some(span) = origin.to_position_span(self.text) else {
            return Ok(());
        };

        if !self.is_eol(&span.end) {
            return Ok(());
        }

        let ty = symbol.ty(self.read)?;

        if ty.type_hint.is_dynamic() {
            return Ok(());
        }

        let tooltip = make_doc(
            self.read,
            self.text,
            self.markdown,
            self.language_id,
            false,
            &ty,
        );

        self.insert_hint(
            span.end,
            format!(": {}", ty.type_hint),
            Some(InlayHintKind::TYPE),
            tooltip,
        );

        Ok(())
    }

    fn add_call(&mut self, symbol: CallSymbol) -> ModuleResult<()> {
        let origin = symbol.origin(self.read);

        let Some(span) = origin.to_position_span(self.text) else {
            return Ok(());
        };

        let receiver_type = symbol.receiver(self.read).expr_ty(self.read)?;

        match receiver_type.impl_symbol {
            ModuleSymbol::Fn(receiver) => {
                let fn_params = receiver.params(self.read);

                let mut params = Vec::with_capacity(fn_params.len());

                for param_var in fn_params {
                    let Some(name) = param_var.var_name(self.read) else {
                        params.push(None);
                        continue;
                    };

                    let ty = param_var.var_type(self.read)?;

                    params.push(Some((Ident::Script(name), ty)));
                }

                let args = symbol.args(self.read);

                self.add_args(params, args)?;
            }

            _ => {
                if let Some(meta) = receiver_type.type_hint.invocation() {
                    if let Some(inputs) = &meta.inputs {
                        let mut params = Vec::with_capacity(inputs.len());

                        for param in inputs {
                            let Some(name) = &param.name else {
                                params.push(None);
                                continue;
                            };

                            let doc = param.hint.doc();

                            let desc = Description {
                                type_hint: param.hint,
                                impl_symbol: ModuleSymbol::Nil,
                                doc,
                            };

                            params.push(Some((name.clone(), desc)));
                        }

                        let args = symbol.args(self.read);

                        self.add_args(params, args)?;
                    }
                }
            }
        }

        if !self.is_eol(&span.end) {
            return Ok(());
        }

        let ty = symbol.ty(self.read)?;

        if ty.type_hint.is_dynamic() {
            return Ok(());
        }

        let tooltip = make_doc(
            self.read,
            self.text,
            self.markdown,
            self.language_id,
            false,
            &ty,
        );

        self.insert_hint(
            span.end,
            format!(": {}", ty.type_hint),
            Some(InlayHintKind::TYPE),
            tooltip,
        );

        Ok(())
    }

    fn add_args(
        &mut self,
        params: Vec<Option<(Ident, Description)>>,
        args: Vec<ModuleSymbol>,
    ) -> ModuleResult<()> {
        for (param, arg) in params.into_iter().zip(args.into_iter()) {
            let Some((param, param_desc)) = param else {
                continue;
            };

            let origin = arg.expr_outer_origin(self.read);

            let Some(span) = origin.to_position_span(self.text) else {
                continue;
            };

            let mut param_string = param.to_string();

            if param_string.is_empty() || param_string.starts_with("_") {
                continue;
            }

            let arg_string = self.text.substring(&span);

            if arg_string.estimate(param_string.as_str()) >= Closeness::half() {
                continue;
            };

            let tooltip = make_doc(
                self.read,
                self.text,
                self.markdown,
                self.language_id,
                true,
                &param_desc,
            );

            param_string.push_str(": ");

            self.insert_hint(
                span.start,
                param_string,
                Some(InlayHintKind::PARAMETER),
                tooltip,
            );
        }

        Ok(())
    }

    fn add_index(&mut self, symbol: IndexSymbol) -> ModuleResult<()> {
        let origin = symbol.origin(self.read);

        let Some(span) = origin.to_position_span(self.text) else {
            return Ok(());
        };

        if !self.is_eol(&span.end) {
            return Ok(());
        }

        let ty = symbol.ty(self.read)?;

        if ty.type_hint.is_dynamic() {
            return Ok(());
        }

        let tooltip = make_doc(
            self.read,
            self.text,
            self.markdown,
            self.language_id,
            false,
            &ty,
        );

        self.insert_hint(
            span.end,
            format!(": {}", ty.type_hint),
            Some(InlayHintKind::TYPE),
            tooltip,
        );

        Ok(())
    }

    fn insert_hint(
        &mut self,
        position: Position,
        label: String,
        kind: Option<InlayHintKind>,
        tooltip: Option<MarkupContent>,
    ) {
        if !self.hint_index.insert(position) {
            return;
        }

        self.hints.push(InlayHint {
            position: ld_position_to_lsp(&position),
            label: InlayHintLabel::String(label),
            kind,
            text_edits: None,
            tooltip: tooltip.map(InlayHintTooltip::MarkupContent),
            padding_left: None,
            padding_right: None,
            data: None,
        });
    }

    fn is_eol(&self, position: &Position) -> bool {
        let Some(site) = position.to_site(self.text) else {
            return false;
        };

        if site == self.text.length() {
            return true;
        }

        for chunk in self.text.chunks((site + 1)..) {
            match chunk.token {
                ScriptToken::Whitespace => (),
                ScriptToken::Linebreak => return true,
                _ => break,
            }
        }

        false
    }
}
