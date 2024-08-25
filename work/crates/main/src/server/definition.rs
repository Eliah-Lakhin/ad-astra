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

use lady_deirdre::{lexis::ToSpan, sync::Trigger};
use log::{error, warn};
use lsp_types::{
    error_codes::{REQUEST_CANCELLED, REQUEST_FAILED},
    request::GotoDefinition,
    GotoDefinitionResponse,
    Location,
    Position,
    Uri,
};

use crate::{
    analysis::{
        symbols::{LookupOptions, ModuleSymbol, SymbolKind},
        ModuleError,
        ModuleRead,
    },
    server::{
        file::{LspModule, ANALYSIS_PRIORITY},
        logger::LSP_SERVER_LOG,
        rpc::{LspHandle, OutgoingEx, RpcId, RpcLatches},
        tasks::{Task, TaskExecution, COOL_DOWN},
        utils::{lsp_position_to_ld, span_to_range},
        RpcSender,
    },
};

pub(super) struct SendGotoDefinition {
    pub(super) latches: RpcLatches,
    pub(super) outgoing: RpcSender,
    pub(super) module: LspModule,
}

impl Task for SendGotoDefinition {
    const EXECUTION: TaskExecution = TaskExecution::ExecuteEach;

    type Config = Self;

    type Message = SendGotoDefinitionMessage;

    #[inline(always)]
    fn init(config: Self::Config) -> Self {
        config
    }

    fn handle(&mut self, message: Self::Message) -> bool {
        loop {
            if message.cancel.is_active() {
                warn!(target: LSP_SERVER_LOG, "[{}] Send goto definition cancelled by the client.", message.uri.as_str());

                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_CANCELLED,
                    "Send goto definition cancelled by the client.",
                );

                break;
            }

            let handle = LspHandle::new(&message.cancel);

            let module_read_guard = match self.module.as_ref().read(&handle, ANALYSIS_PRIORITY) {
                Ok(guard) => guard,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send goto definition cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send goto definition cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send goto definition interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send goto definition error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send goto definition error.",
                    );

                    break;
                }
            };

            let position = lsp_position_to_ld(&message.position);

            const GOTO_SYMBOLS: u32 = (SymbolKind::Break as u32)
                | (SymbolKind::Return as u32)
                | (SymbolKind::Ident as u32)
                | (SymbolKind::Field as u32);

            let symbols = match module_read_guard.symbols(
                position..position,
                LookupOptions::new().filter(GOTO_SYMBOLS),
            ) {
                Ok(symbols) => symbols,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send goto definition cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send goto definition cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send goto definition interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send goto definition error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send goto definition error.",
                    );

                    break;
                }
            };

            let definition =
                match symbols.first() {
                    Some(ModuleSymbol::Break(symbol)) => symbol
                        .loop_symbol(&module_read_guard)
                        .map(|symbol| match symbol {
                            Some(symbol) => ModuleSymbol::Loop(symbol),
                            None => ModuleSymbol::Nil,
                        }),

                    Some(ModuleSymbol::Return(symbol)) => {
                        symbol
                            .fn_symbol(&module_read_guard)
                            .map(|symbol| match symbol {
                                Some(symbol) => ModuleSymbol::Fn(symbol),
                                None => ModuleSymbol::Nil,
                            })
                    }

                    Some(ModuleSymbol::Ident(symbol)) => symbol.declaration(&module_read_guard),

                    Some(ModuleSymbol::Field(symbol)) => symbol
                        .declaration(&module_read_guard)
                        .map(|symbol| match symbol {
                            Some(symbol) => ModuleSymbol::Entry(symbol),
                            None => ModuleSymbol::Nil,
                        }),

                    _ => {
                        self.outgoing.send_ok_response::<GotoDefinition>(
                            &self.latches,
                            message.id,
                            None,
                        );

                        break;
                    }
                };

            let definition = match definition {
                Ok(symbol) => symbol,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send goto definition cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send goto definition cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send goto definition interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send goto definition error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send goto definition error.",
                    );

                    break;
                }
            };

            let origin = definition.origin(&module_read_guard);

            let text = module_read_guard.text();

            let Some(span) = origin.to_position_span(&text) else {
                self.outgoing
                    .send_ok_response::<GotoDefinition>(&self.latches, message.id, None);

                break;
            };

            let range = span_to_range(&span);

            self.outgoing.send_ok_response::<GotoDefinition>(
                &self.latches,
                message.id,
                Some(GotoDefinitionResponse::Scalar(Location {
                    uri: message.uri,
                    range,
                })),
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

pub(super) struct SendGotoDefinitionMessage {
    pub(super) id: RpcId,
    pub(super) uri: Uri,
    pub(super) cancel: Trigger,
    pub(super) position: Position,
}
