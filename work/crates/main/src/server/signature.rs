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

use lady_deirdre::{
    lexis::{Position as LDPosition, SourceCode, ToSite, ToSpan},
    sync::Trigger,
};
use log::{error, warn};
use lsp_types::{
    error_codes::{REQUEST_CANCELLED, REQUEST_FAILED},
    request::SignatureHelpRequest,
    Documentation,
    ParameterInformation,
    ParameterLabel,
    Position,
    SignatureHelp,
    SignatureInformation,
    Uri,
};

use crate::{
    analysis::{
        symbols::{LookupOptions, ModuleSymbol, SymbolKind},
        ModuleError,
        ModuleRead,
        ModuleText,
    },
    runtime::ScriptOrigin,
    server::{
        file::{LspModule, ANALYSIS_PRIORITY},
        logger::LSP_SERVER_LOG,
        rpc::{LspHandle, OutgoingEx, RpcId, RpcLatches},
        tasks::{Task, TaskExecution, COOL_DOWN},
        utils::{lsp_position_to_ld, make_doc},
        LspServerConfig,
        RpcSender,
    },
};

pub(super) struct SendSignatureHelp {
    pub(super) config: LspServerConfig,
    pub(super) latches: RpcLatches,
    pub(super) outgoing: RpcSender,
    pub(super) module: LspModule,
}

impl Task for SendSignatureHelp {
    const EXECUTION: TaskExecution = TaskExecution::ExecuteEach;

    type Config = Self;

    type Message = SendSignatureHelpMessage;

    #[inline(always)]
    fn init(config: Self::Config) -> Self {
        config
    }

    fn handle(&mut self, message: Self::Message) -> bool {
        loop {
            if message.cancel.is_active() {
                warn!(target: LSP_SERVER_LOG, "[{}] Send signature help cancelled by the client.", message.uri.as_str());

                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_CANCELLED,
                    "Send signature help cancelled by the client.",
                );

                break;
            }

            let handle = LspHandle::new(&message.cancel);

            let module_read_guard = match self.module.as_ref().read(&handle, ANALYSIS_PRIORITY) {
                Ok(guard) => guard,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send signature help cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send signature help cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send signature help interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send signature help error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send signature help error.",
                    );

                    break;
                }
            };

            let position = lsp_position_to_ld(&message.position);

            const SIGNATURE_SYMBOLS: u32 = SymbolKind::Call as u32;

            let symbols = match module_read_guard.symbols(
                position..position,
                LookupOptions::new().outer().filter(SIGNATURE_SYMBOLS),
            ) {
                Ok(symbols) => symbols,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send signature help cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send signature help cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send signature help interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send signature help error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send signature help error.",
                    );

                    break;
                }
            };

            let Some(ModuleSymbol::Call(symbol)) = symbols.last() else {
                self.outgoing.send_ok_response::<SignatureHelpRequest>(
                    &self.latches,
                    message.id,
                    None,
                );

                break;
            };

            let text = module_read_guard.text();

            let args_origin = symbol.origin(&module_read_guard);
            let param_index = infer_param_index(&text, &position, args_origin);

            let receiver = symbol.receiver(&module_read_guard);

            let ty = match receiver.expr_ty(&module_read_guard) {
                Ok(ty) => ty,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send signature help cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send signature help cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send signature help interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send signature help error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send signature help error.",
                    );

                    break;
                }
            };

            let Some(invocation) = ty.type_hint.invocation() else {
                self.outgoing.send_ok_response::<SignatureHelpRequest>(
                    &self.latches,
                    message.id,
                    None,
                );

                break;
            };

            let doc = make_doc(
                &module_read_guard,
                &text,
                self.config.capabilities.signature_help_markdown,
                self.config.language_id,
                false,
                &ty,
            );

            let mut params = Vec::with_capacity(invocation.arity().unwrap_or(0));

            if let Some(inputs) = &invocation.inputs {
                for param in inputs {
                    let documentation = match param.hint.is_dynamic() {
                        true => None,
                        false => Some(Documentation::String(param.hint.to_string())),
                    };

                    params.push(ParameterInformation {
                        label: ParameterLabel::Simple(
                            param
                                .name
                                .as_ref()
                                .map(|name| name.to_string())
                                .unwrap_or(String::from("?")),
                        ),

                        documentation,
                    });
                }
            }

            self.outgoing.send_ok_response::<SignatureHelpRequest>(
                &self.latches,
                message.id,
                Some(SignatureHelp {
                    signatures: vec![SignatureInformation {
                        label: invocation.to_string(),
                        documentation: doc.map(|content| Documentation::MarkupContent(content)),
                        parameters: Some(params),
                        active_parameter: None,
                    }],
                    active_signature: Some(0),
                    active_parameter: param_index.map(|index| index as u32),
                }),
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

pub(super) struct SendSignatureHelpMessage {
    pub(super) id: RpcId,
    pub(super) uri: Uri,
    pub(super) cancel: Trigger,
    pub(super) position: Position,
}

fn infer_param_index(
    text: &ModuleText,
    position: &LDPosition,
    args_origin: ScriptOrigin,
) -> Option<usize> {
    enum State {
        Args(usize),
        InlineComment(usize),
        MultilineComment(usize, usize),
    }

    let mut param_index = 0;

    let site = position.to_site(text)?;
    let mut args_span = args_origin.to_site_span(text)?;

    if args_span.start < args_span.end {
        args_span.start += 1;
    }

    if args_span.start < args_span.end {
        args_span.end -= 1;
    }

    let mut state = State::Args(0);

    for chunk in text.chunks(args_span) {
        match chunk.string {
            "," => {
                if let State::Args(1) = &state {
                    if chunk.end() <= site {
                        param_index += 1;
                        continue;
                    };

                    break;
                }
            }

            "//" => match &state {
                State::Args(nesting) => state = State::InlineComment(*nesting),
                State::InlineComment(_) => (),
                State::MultilineComment(_, _) => (),
            },

            "/*" => match &mut state {
                State::Args(nesting) => state = State::MultilineComment(*nesting, 1),
                State::InlineComment(_) => (),
                State::MultilineComment(_, nesting) => *nesting += 1,
            },

            "*/" => match &mut state {
                State::Args(..) => return None,
                State::InlineComment(_) => (),
                State::MultilineComment(args_nesting, comment_nesting) => {
                    *comment_nesting = match comment_nesting.checked_sub(1) {
                        Some(nesting) => nesting,
                        None => return None,
                    };

                    if *comment_nesting == 0 {
                        state = State::Args(*args_nesting);
                    }
                }
            },

            "\n" | "\r\n" => match &mut state {
                State::Args(..) => (),
                State::InlineComment(args_nesting) => state = State::Args(*args_nesting),
                State::MultilineComment(_, _) => (),
            },

            "{" | "[" | "(" => match &mut state {
                State::Args(nesting) => *nesting += 1,
                State::InlineComment(_) => (),
                State::MultilineComment(_, _) => (),
            },

            "}" | "]" => match &mut state {
                State::Args(nesting) => {
                    *nesting = match nesting.checked_sub(1) {
                        Some(nesting) => {
                            if nesting == 0 {
                                return None;
                            }

                            nesting
                        }
                        None => return None,
                    };
                }
                State::InlineComment(_) => {}
                State::MultilineComment(_, _) => {}
            },

            ")" => match &mut state {
                State::Args(nesting) => {
                    *nesting = match nesting.checked_sub(1) {
                        Some(nesting) => {
                            if nesting == 0 {
                                break;
                            }

                            nesting
                        }
                        None => return None,
                    };
                }
                State::InlineComment(_) => {}
                State::MultilineComment(_, _) => {}
            },

            _ => (),
        }
    }

    Some(param_index)
}
