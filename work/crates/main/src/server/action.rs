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

use std::{collections::HashMap, thread::park_timeout};

use lady_deirdre::{lexis::ToSpan, sync::Trigger};
use log::{error, warn};
use lsp_types::{
    error_codes::{REQUEST_CANCELLED, REQUEST_FAILED},
    request::CodeActionRequest,
    CodeAction,
    CodeActionContext,
    CodeActionKind,
    CodeActionOrCommand,
    Range,
    TextEdit,
    Uri,
    WorkspaceEdit,
};

use crate::{
    analysis::{ModuleError, ModuleRead},
    server::{
        diagnostics::DiagnosticData,
        file::{LspModule, ANALYSIS_PRIORITY},
        logger::LSP_SERVER_LOG,
        rpc::{LspHandle, OutgoingEx, RpcId, RpcLatches},
        tasks::{Task, TaskExecution, COOL_DOWN},
        utils::span_to_range,
        RpcSender,
    },
};

pub(super) struct SendCodeAction {
    pub(super) latches: RpcLatches,
    pub(super) outgoing: RpcSender,
    pub(super) module: LspModule,
}

impl Task for SendCodeAction {
    const EXECUTION: TaskExecution = TaskExecution::ExecuteEach;

    type Config = Self;

    type Message = SendCodeActionMessage;

    #[inline(always)]
    fn init(config: Self::Config) -> Self {
        config
    }

    fn handle(&mut self, message: Self::Message) -> bool {
        loop {
            if message.cancel.is_active() {
                warn!(target: LSP_SERVER_LOG, "[{}] Send code action cancelled by the client.", message.uri.as_str());

                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_CANCELLED,
                    "Send code action cancelled by the client.",
                );

                break;
            }

            let handle = LspHandle::new(&message.cancel);

            let module_read_guard = match self.module.as_ref().read(&handle, ANALYSIS_PRIORITY) {
                Ok(guard) => guard,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send code action cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send code action cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send code action interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send code action error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send code action error.",
                    );

                    break;
                }
            };

            let module_text = module_read_guard.text();

            let mut actions = Vec::new();

            for diagnostic in message.context.diagnostics {
                let Some(data) = &diagnostic.data else {
                    continue;
                };

                let quickfix = DiagnosticData::from(data).0;

                let mut title = String::new();
                let mut edits = Vec::new();

                if let Some(text) = quickfix.set_text_to_origin {
                    if !title.is_empty() {
                        title.push_str(" and ");
                    }

                    title.push_str(&format!("rename to {text:?}"));

                    edits.push(TextEdit {
                        range: diagnostic.range,
                        new_text: text,
                    });
                }

                if let Some(implement_use_of) = quickfix.implement_use_of {
                    let content_origin = module_read_guard.content_origin();

                    if let Some(mut span) = content_origin.to_position_span(&module_text) {
                        if !title.is_empty() {
                            title.push_str(" and ");
                        }

                        title.push_str(&format!("import {implement_use_of:?} package"));

                        span.end = span.start;

                        let range = span_to_range(&span);

                        edits.push(TextEdit {
                            range,
                            new_text: format!("use {implement_use_of};\n\n"),
                        });
                    }
                }

                if !edits.is_empty() {
                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title,
                        kind: Some(CodeActionKind::QUICKFIX),
                        edit: Some(WorkspaceEdit {
                            changes: Some(HashMap::from([(message.uri.clone(), edits)])),

                            ..WorkspaceEdit::default()
                        }),

                        ..CodeAction::default()
                    }));
                }
            }

            self.outgoing.send_ok_response::<CodeActionRequest>(
                &self.latches,
                message.id,
                match actions.is_empty() {
                    true => None,
                    false => Some(actions),
                },
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

pub(super) struct SendCodeActionMessage {
    pub(super) id: RpcId,
    pub(super) uri: Uri,
    pub(super) cancel: Trigger,
    #[allow(unused)]
    pub(super) range: Range,
    pub(super) context: CodeActionContext,
}
