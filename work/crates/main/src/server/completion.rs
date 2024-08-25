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

use lady_deirdre::sync::Trigger;
use log::{error, warn};
use lsp_types::{
    error_codes::{REQUEST_CANCELLED, REQUEST_FAILED},
    request::Completion,
    CompletionItem as LSPCompletionItem,
    CompletionItemKind,
    CompletionResponse,
    Documentation,
    InsertTextFormat,
    Position,
    Uri,
};

use crate::{
    analysis::{
        symbols::ModuleSymbol,
        CompletionItem,
        CompletionScope,
        ModuleError,
        ModuleRead,
        ModuleWrite,
    },
    server::{
        file::{LspModule, COMPLETION_PRIORITY},
        logger::LSP_SERVER_LOG,
        rpc::{LspHandle, OutgoingEx, RpcId, RpcLatches},
        snippets::*,
        tasks::{Task, TaskExecution, COOL_DOWN},
        utils::{lsp_position_to_ld, make_doc},
        LspServerConfig,
        RpcSender,
    },
};

pub(super) struct SendCompletion {
    pub(super) config: LspServerConfig,
    pub(super) latches: RpcLatches,
    pub(super) outgoing: RpcSender,
    pub(super) module: LspModule,
}

impl Task for SendCompletion {
    const EXECUTION: TaskExecution = TaskExecution::ExecuteEach;

    type Config = Self;

    type Message = SendCompletionMessage;

    #[inline(always)]
    fn init(config: Self::Config) -> Self {
        config
    }

    fn handle(&mut self, message: Self::Message) -> bool {
        loop {
            if message.cancel.is_active() {
                warn!(target: LSP_SERVER_LOG, "[{}] Send completion cancelled by the client.", message.uri.as_str());

                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_CANCELLED,
                    "Send completion cancelled by the client.",
                );

                break;
            }

            let handle = LspHandle::new(&message.cancel);

            let mut module_write_guard = match self
                .module
                .as_ref()
                .write(&handle, COMPLETION_PRIORITY)
            {
                Ok(guard) => guard,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send completion cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send completion cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send completion interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send completion error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send completion error.",
                    );

                    break;
                }
            };

            let position = lsp_position_to_ld(&message.position);

            let completions = match module_write_guard.completions(position) {
                Ok(completions) => completions,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send completion cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send completion cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send completion interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send completion error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send completion error.",
                    );

                    break;
                }
            };

            let text = module_write_guard.text();

            let mut result = Vec::new();

            for item in completions.items {
                let mut label = item.label.to_string();

                let kind = completion_kind(completions.scope, &item);
                let insert_text;
                let insert_text_format;

                match kind {
                    CompletionItemKind::FUNCTION | CompletionItemKind::METHOD
                        if self.config.capabilities.completion_snippets =>
                    {
                        match self.config.capabilities.completion_snippets {
                            true => {
                                insert_text = Some(format!("{label}($1)"));
                                insert_text_format = Some(InsertTextFormat::SNIPPET)
                            }

                            false => {
                                label.push_str("()");
                                insert_text = None;
                                insert_text_format = None;
                            }
                        }
                    }

                    _ => {
                        insert_text = None;
                        insert_text_format = None;
                    }
                }

                let detail = match item.desc.type_hint.is_dynamic() {
                    true => None,
                    false => Some(item.desc.type_hint.to_string()),
                };

                let doc = make_doc(
                    &module_write_guard,
                    &text,
                    self.config.capabilities.completion_markdown,
                    self.config.language_id,
                    false,
                    &item.desc,
                );

                result.push(LSPCompletionItem {
                    label,
                    kind: Some(kind),
                    detail,
                    documentation: doc.map(|doc| Documentation::MarkupContent(doc)),
                    insert_text,
                    insert_text_format,

                    ..Default::default()
                });
            }

            if self.config.capabilities.completion_snippets {
                match completions.scope {
                    CompletionScope::Expression => {
                        result.push(SnippetFn::item(&self.config));
                        result.push(SnippetStruct::item(&self.config));
                        result.push(SnippetSelf::item(&self.config));
                        result.push(SnippetCrate::item(&self.config));
                        result.push(SnippetTrue::item(&self.config));
                        result.push(SnippetFalse::item(&self.config));
                        result.push(SnippetMax::item(&self.config));
                    }

                    CompletionScope::Statement => {
                        result.push(SnippetUse::item(&self.config));
                        result.push(SnippetLet::item(&self.config));
                        result.push(SnippetFor::item(&self.config));
                        result.push(SnippetLoop::item(&self.config));
                        result.push(SnippetIf::item(&self.config));
                        result.push(SnippetMatch::item(&self.config));
                        result.push(SnippetReturn::item(&self.config));
                        result.push(SnippetBreak::item(&self.config));
                        result.push(SnippetContinue::item(&self.config));
                        result.push(SnippetSelf::item(&self.config));
                        result.push(SnippetCrate::item(&self.config));
                        result.push(SnippetTrue::item(&self.config));
                        result.push(SnippetFalse::item(&self.config));
                        result.push(SnippetMax::item(&self.config));
                    }

                    CompletionScope::MatchArm => {
                        result.push(SnippetMatchArm::item(&self.config));
                        result.push(SnippetMatchElse::item(&self.config));
                    }

                    CompletionScope::Field => {
                        result.push(SnippetLen::item(&self.config));
                    }

                    _ => (),
                }
            }

            self.outgoing.send_ok_response::<Completion>(
                &self.latches,
                message.id,
                Some(CompletionResponse::Array(result)),
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

pub(super) struct SendCompletionMessage {
    pub(super) id: RpcId,
    pub(super) uri: Uri,
    pub(super) cancel: Trigger,
    pub(super) position: Position,
}

fn completion_kind(scope: CompletionScope, item: &CompletionItem) -> CompletionItemKind {
    match scope {
        CompletionScope::Unknown => CompletionItemKind::TEXT,

        CompletionScope::Import => CompletionItemKind::MODULE,

        CompletionScope::Expression | CompletionScope::Statement | CompletionScope::MatchArm => {
            if item.desc.type_hint.is_fn() {
                return CompletionItemKind::FUNCTION;
            }

            if item.desc.type_hint.is_package() {
                return CompletionItemKind::MODULE;
            }

            if let ModuleSymbol::Package(..) = &item.desc.impl_symbol {
                return CompletionItemKind::CONSTANT;
            }

            CompletionItemKind::VARIABLE
        }

        CompletionScope::Field => {
            if item.desc.type_hint.is_fn() {
                return CompletionItemKind::METHOD;
            }

            if item.desc.type_hint.is_package() {
                return CompletionItemKind::MODULE;
            }

            CompletionItemKind::FIELD
        }
    }
}
