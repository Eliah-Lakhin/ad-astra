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

use lady_deirdre::sync::Trigger;
use log::warn;
use lsp_types::{error_codes::REQUEST_CANCELLED, request::CodeLensRequest, CodeLens, Command, Uri};
use serde_json::{Number, Value};

use crate::server::{
    command::{SharedRunnerState, CMD_CLEANUP, CMD_LAUNCH, CMD_STOP},
    file::LspModule,
    logger::LSP_SERVER_LOG,
    rpc::{OutgoingEx, RpcId, RpcLatches},
    tasks::{Task, TaskExecution},
    LspServerConfig,
    RpcSender,
};

pub(super) struct SendCodeLens {
    pub(super) config: LspServerConfig,
    pub(super) latches: RpcLatches,
    pub(super) outgoing: RpcSender,
    pub(super) module: LspModule,
    pub(super) runner_state: SharedRunnerState,
}

impl Task for SendCodeLens {
    const EXECUTION: TaskExecution = TaskExecution::ExecuteEach;

    type Config = Self;

    type Message = SendCodeLensMessage;

    #[inline(always)]
    fn init(config: Self::Config) -> Self {
        config
    }

    fn handle(&mut self, message: Self::Message) -> bool {
        if message.cancel.is_active() {
            warn!(target: LSP_SERVER_LOG, "[{}] Send code lens cancelled by the client.", message.uri.as_str());

            self.outgoing.send_err_response(
                &self.latches,
                message.id,
                REQUEST_CANCELLED,
                "Send code lens cancelled by the client.",
            );

            return true;
        }

        let mut result = Vec::new();

        if self.config.scripts_runner && self.config.capabilities.execute_command {
            let runner_state_guard = self
                .runner_state
                .as_ref()
                .read()
                .unwrap_or_else(|poison| poison.into_inner());

            match runner_state_guard.enabled {
                false => {
                    result.push(CodeLens {
                        range: Default::default(),
                        command: Some(Command {
                            title: String::from("▶️ Launch"),
                            command: String::from(CMD_LAUNCH),
                            arguments: Some(vec![Value::String(message.uri.to_string())]),
                        }),
                        data: None,
                    });

                    let cleanup = self.config.capabilities.inlay_hints
                        && !runner_state_guard.messages.is_empty();

                    if cleanup {
                        result.push(CodeLens {
                            range: Default::default(),
                            command: Some(Command {
                                title: String::from("🔁 Cleanup"),
                                command: String::from(CMD_CLEANUP),
                                arguments: Some(vec![Value::String(message.uri.to_string())]),
                            }),
                            data: None,
                        });
                    }
                }

                true => {
                    result.push(CodeLens {
                        range: Default::default(),
                        command: Some(Command {
                            title: String::from("⏹️ Stop"),
                            command: String::from(CMD_STOP),
                            arguments: Some(vec![
                                Value::String(message.uri.to_string()),
                                Value::Number(Number::from(runner_state_guard.job)),
                            ]),
                        }),
                        data: None,
                    });
                }
            }
        }

        self.outgoing
            .send_ok_response::<CodeLensRequest>(&self.latches, message.id, Some(result));

        true
    }

    #[inline(always)]
    fn module(&self) -> &LspModule {
        &self.module
    }
}

pub(super) struct SendCodeLensMessage {
    pub(super) id: RpcId,
    pub(super) uri: Uri,
    pub(super) cancel: Trigger,
}
