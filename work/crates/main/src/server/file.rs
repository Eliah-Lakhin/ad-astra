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

use std::{collections::VecDeque, str::FromStr, thread::park_timeout};

use lady_deirdre::{
    analysis::TaskPriority,
    sync::{Shared, Trigger},
};
use log::{error, warn};
use lsp_types::{
    request::{
        CodeActionRequest,
        CodeLensRequest,
        Completion,
        DocumentHighlightRequest,
        ExecuteCommand,
        Formatting,
        GotoDefinition,
        GotoImplementation,
        HoverRequest,
        InlayHintRequest,
        LinkedEditingRange,
        PrepareRenameRequest,
        Rename,
        SignatureHelpRequest,
    },
    CodeActionContext,
    FormattingOptions,
    Position,
    Range,
    TextDocumentContentChangeEvent,
    Uri,
};
use serde_json::Value;

use crate::{
    analysis::{ModuleError, ModuleWrite, ScriptModule},
    runtime::PackageMeta,
    server::{
        action::{SendCodeAction, SendCodeActionMessage},
        command::{SendExecuteCommand, SendExecuteCommandMessage},
        completion::{SendCompletion, SendCompletionMessage},
        definition::{SendGotoDefinition, SendGotoDefinitionMessage},
        diagnostics::{DiagnosticsPublisher, PublishContext},
        format::{SendFormatting, SendFormattingMessage},
        highlight::{SendDocumentHighlight, SendDocumentHighlightMessage},
        hints::{SendInlayHints, SendInlayHintsMessage},
        hover::{SendHover, SendHoverMessage},
        implementation::{SendGotoImplementation, SendGotoImplementationMessage},
        lens::{SendCodeLens, SendCodeLensMessage},
        logger::LSP_CLIENT_LOG,
        rename::{
            SendLinkedEditingRange,
            SendLinkedEditingRangeMessage,
            SendPrepareRename,
            SendPrepareRenameMessage,
            SendRename,
            SendRenameMessage,
        },
        rpc::{LspHandle, OutgoingEx, RpcId, RpcLatches},
        signature::{SendSignatureHelp, SendSignatureHelpMessage},
        tasks::{LocalOrRemote, Task, TaskExecution, COOL_DOWN},
        utils::{range_to_span, uri_to_name},
        HealthCheck,
        LspServerConfig,
        RpcSender,
    },
};

pub(super) type LspModule = Shared<ScriptModule<LspHandle>>;

pub(super) const DIAGNOSTICS_PRIORITY: TaskPriority = 1;
pub(super) const ANALYSIS_PRIORITY: TaskPriority = 2;
pub(super) const COMPLETION_PRIORITY: TaskPriority = 3;
pub(super) const COMMAND_PRIORITY: TaskPriority = 4;
pub(super) const EDIT_PRIORITY: TaskPriority = 5;

pub(super) struct File {
    latches: RpcLatches,
    outgoing: RpcSender,
    module: LspModule,
    uri: Uri,
    version: i32,
    publish_diagnostics_1: Option<LocalOrRemote<DiagnosticsPublisher<1>>>,
    publish_diagnostics_2: Option<LocalOrRemote<DiagnosticsPublisher<2>>>,
    publish_diagnostics_3: Option<LocalOrRemote<DiagnosticsPublisher<3>>>,
    send_inlay_hints: Option<LocalOrRemote<SendInlayHints>>,
    send_formatting: Option<LocalOrRemote<SendFormatting>>,
    send_completion: Option<LocalOrRemote<SendCompletion>>,
    send_hover: Option<LocalOrRemote<SendHover>>,
    send_goto_definition: Option<LocalOrRemote<SendGotoDefinition>>,
    send_document_highlight: Option<LocalOrRemote<SendDocumentHighlight>>,
    send_goto_implementation: Option<LocalOrRemote<SendGotoImplementation>>,
    send_code_action: Option<LocalOrRemote<SendCodeAction>>,
    send_signature_help: Option<LocalOrRemote<SendSignatureHelp>>,
    send_rename: Option<LocalOrRemote<SendRename>>,
    send_prepare_rename: Option<LocalOrRemote<SendPrepareRename>>,
    send_linked_editing_range: Option<LocalOrRemote<SendLinkedEditingRange>>,
    send_code_lens: Option<LocalOrRemote<SendCodeLens>>,
    send_execute_command: Option<LocalOrRemote<SendExecuteCommand>>,
}

impl Drop for File {
    fn drop(&mut self) {
        self.module.as_ref().deny_access();
    }
}

impl Task for File {
    const EXECUTION: TaskExecution = TaskExecution::ExecuteEach;

    type Config = FileConfig;
    type Message = FileMessage;

    fn init(config: FileConfig) -> Self {
        let module = Shared::new(ScriptModule::new(config.package, config.text));

        if let Some(name) = uri_to_name(&config.uri) {
            module.as_ref().rename(name);
        }

        let runner_state = Shared::default();

        let mut publish_diagnostics_1 = None;
        let mut publish_diagnostics_2 = None;
        let mut publish_diagnostics_3 = None;

        if config.config.capabilities.publish_diagnostics {
            let diagnostics = Shared::default();

            publish_diagnostics_1 = Some(LocalOrRemote::new(
                format!("[{}] (diagnostics 1)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                DiagnosticsPublisher {
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                    diagnostics: diagnostics.clone(),
                },
            ));

            publish_diagnostics_2 = Some(LocalOrRemote::new(
                format!("[{}] (diagnostics 2)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                DiagnosticsPublisher {
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                    diagnostics: diagnostics.clone(),
                },
            ));

            publish_diagnostics_3 = Some(LocalOrRemote::new(
                format!("[{}] (diagnostics 3)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                DiagnosticsPublisher {
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                    diagnostics,
                },
            ));
        }

        let mut send_inlay_hints = None;

        if config.config.capabilities.inlay_hints {
            send_inlay_hints = Some(LocalOrRemote::new(
                format!("[{}] (send hints)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendInlayHints {
                    config: config.config,
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                    runner_state: runner_state.clone(),
                },
            ));
        }

        let mut send_formatting = None;

        if config.config.capabilities.formatting {
            send_formatting = Some(LocalOrRemote::new(
                format!("[{}] (send formatting)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendFormatting {
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                },
            ));
        }

        let mut send_completion = None;

        if config.config.capabilities.completion {
            send_completion = Some(LocalOrRemote::new(
                format!("[{}] (send completion)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendCompletion {
                    config: config.config,
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                },
            ));
        }

        let mut send_hover = None;

        if config.config.capabilities.hover {
            send_hover = Some(LocalOrRemote::new(
                format!("[{}] (send hover)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendHover {
                    config: config.config,
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                },
            ));
        }

        let mut send_goto_definition = None;

        if config.config.capabilities.goto_definition {
            send_goto_definition = Some(LocalOrRemote::new(
                format!("[{}] (send goto definition)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendGotoDefinition {
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                },
            ));
        }

        let mut send_document_highlight = None;

        if config.config.capabilities.document_highlight {
            send_document_highlight = Some(LocalOrRemote::new(
                format!("[{}] (send goto definition)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendDocumentHighlight {
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                },
            ));
        }

        let mut send_goto_implementation = None;

        if config.config.capabilities.goto_implementation {
            send_goto_implementation = Some(LocalOrRemote::new(
                format!("[{}] (send goto implementation)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendGotoImplementation {
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                },
            ));
        }

        let mut send_code_action = None;

        if config.config.capabilities.code_action {
            send_code_action = Some(LocalOrRemote::new(
                format!("[{}] (send code action)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendCodeAction {
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                },
            ));
        }

        let mut send_signature_help = None;

        if config.config.capabilities.signature_help {
            send_signature_help = Some(LocalOrRemote::new(
                format!("[{}] (send signature help)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendSignatureHelp {
                    config: config.config,
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                },
            ));
        }

        let mut send_rename = None;

        if config.config.capabilities.rename {
            send_rename = Some(LocalOrRemote::new(
                format!("[{}] (send rename)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendRename {
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                },
            ));
        }

        let mut send_prepare_rename = None;

        if config.config.capabilities.rename_prepare {
            send_prepare_rename = Some(LocalOrRemote::new(
                format!("[{}] (send prepare rename)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendPrepareRename {
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                },
            ));
        }

        let mut send_linked_editing_range = None;

        if config.config.capabilities.linked_editing_range {
            send_linked_editing_range = Some(LocalOrRemote::new(
                format!("[{}] (send linked editing range)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendLinkedEditingRange {
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                },
            ));
        }

        let mut send_code_lens = None;

        if config.config.capabilities.code_lens {
            send_code_lens = Some(LocalOrRemote::new(
                format!("[{}] (send code lens)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendCodeLens {
                    config: config.config,
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                    runner_state: runner_state.clone(),
                },
            ));
        }

        let mut send_execute_command = None;

        if config.config.capabilities.execute_command {
            send_execute_command = Some(LocalOrRemote::new(
                format!("[{}] (send execute command)", config.uri.as_str()),
                config.config.multi_thread,
                &config.health_check,
                SendExecuteCommand {
                    config: config.config,
                    latches: config.latches.clone(),
                    outgoing: config.outgoing.clone(),
                    module: module.clone(),
                    runner_state,
                },
            ));
        }

        let mut result = Self {
            latches: config.latches,
            outgoing: config.outgoing,
            module,
            uri: config.uri,
            version: config.version,
            publish_diagnostics_1,
            publish_diagnostics_2,
            publish_diagnostics_3,
            send_inlay_hints,
            send_formatting,
            send_completion,
            send_hover,
            send_goto_definition,
            send_document_highlight,
            send_goto_implementation,
            send_code_action,
            send_signature_help,
            send_rename,
            send_prepare_rename,
            send_linked_editing_range,
            send_code_lens,
            send_execute_command,
        };

        result.trigger_diagnostics();

        result
    }

    #[inline(always)]
    fn handle(&mut self, message: FileMessage) -> bool {
        match message {
            FileMessage::RenameFile { new_uri } => self.handle_rename_file(new_uri),

            FileMessage::ChangeText { version, changes } => {
                self.handle_change_text(version, changes)
            }

            FileMessage::InlayHint { id, cancel, range } => {
                self.handle_inlay_hint(id, cancel, range)
            }

            FileMessage::Formatting {
                id,
                cancel,
                options,
            } => self.handle_formatting(id, cancel, options),

            FileMessage::Completion {
                id,
                cancel,
                position,
            } => self.handle_completion(id, cancel, position),

            FileMessage::Hover {
                id,
                cancel,
                position,
            } => self.handle_hover(id, cancel, position),

            FileMessage::GotoDefinition {
                id,
                cancel,
                position,
            } => self.handle_goto_definition(id, cancel, position),

            FileMessage::DocumentHighlight {
                id,
                cancel,
                position,
            } => self.handle_document_highlight(id, cancel, position),

            FileMessage::GotoImplementation {
                id,
                cancel,
                position,
            } => self.handle_goto_implementation(id, cancel, position),

            FileMessage::CodeAction {
                id,
                cancel,
                range,
                context,
            } => self.handle_code_action(id, cancel, range, context),

            FileMessage::SignatureHelp {
                id,
                cancel,
                position,
            } => self.handle_signature_help(id, cancel, position),

            FileMessage::Rename {
                id,
                cancel,
                new_name,
                position,
            } => self.handle_rename(id, cancel, new_name, position),

            FileMessage::PrepareRename {
                id,
                cancel,
                position,
            } => self.handle_prepare_rename(id, cancel, position),

            FileMessage::LinkedEditingRange {
                id,
                cancel,
                position,
            } => self.handle_linked_editing_range(id, cancel, position),

            FileMessage::CodeLens { id, cancel } => self.handle_code_lens(id, cancel),

            FileMessage::Command {
                id,
                cancel,
                command,
                command_arg,
            } => self.handle_command(id, cancel, command, command_arg),
        }

        true
    }

    #[inline(always)]
    fn module(&self) -> &LspModule {
        &self.module
    }
}

impl File {
    fn handle_rename_file(&mut self, new_uri: String) {
        let new_uri = match Uri::from_str(new_uri.as_str()) {
            Ok(uri) => uri,

            Err(error) => {
                error!(target: LSP_CLIENT_LOG, "[{new_uri}] URI parser error. {error}");
                return;
            }
        };

        if let Some(name) = uri_to_name(&new_uri) {
            self.module.as_ref().rename(name);
        }

        self.uri = new_uri;
    }

    fn handle_change_text(&mut self, version: i32, changes: Vec<TextDocumentContentChangeEvent>) {
        let mut changes = changes.into_iter().collect::<VecDeque<_>>();

        'outer: loop {
            let handle = LspHandle::default();

            let mut module_write_guard = match self.module.as_ref().write(&handle, EDIT_PRIORITY) {
                Ok(guard) => guard,

                Err(ModuleError::Interrupted(_)) => {
                    if !self.module.as_ref().is_access_allowed() {
                        warn!(target: LSP_CLIENT_LOG, "[{}] Text change cancelled.", self.uri.as_str());
                        return;
                    }

                    warn!(target: LSP_CLIENT_LOG, "[{}] Text change interrupted.", self.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_CLIENT_LOG, "[{}] Text change error. {error}", self.uri.as_str());
                    return;
                }
            };

            while let Some(change) = changes.pop_front() {
                let result = match change.range {
                    Some(range) => module_write_guard.edit(range_to_span(&range), change.text),
                    None => module_write_guard.edit(.., change.text),
                };

                match result {
                    Ok(()) => (),

                    Err(ModuleError::Interrupted(_)) => {
                        if !self.module.as_ref().is_access_allowed() {
                            warn!(target: LSP_CLIENT_LOG, "[{}] Text change cancelled.", self.uri.as_str());
                            return;
                        }

                        warn!(target: LSP_CLIENT_LOG, "[{}] Text change interrupted.", self.uri.as_str());
                        park_timeout(COOL_DOWN);
                        continue 'outer;
                    }

                    Err(error) => {
                        error!(target: LSP_CLIENT_LOG, "[{}] Text change error. {error}", self.uri.as_str());
                        return;
                    }
                }
            }

            break;
        }

        self.version = version;

        self.trigger_diagnostics();
    }

    fn handle_inlay_hint(&mut self, id: RpcId, cancel: Trigger, range: Range) {
        let Some(send_inlay_hints) = &mut self.send_inlay_hints else {
            error!(target: LSP_CLIENT_LOG, "[{}] Inlay hints sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<InlayHintRequest>(&self.latches, id, None);

            return;
        };

        send_inlay_hints.send(SendInlayHintsMessage {
            id,
            uri: self.uri.clone(),
            cancel,
            range,
        });
    }

    fn handle_formatting(&mut self, id: RpcId, cancel: Trigger, options: FormattingOptions) {
        let Some(send_formatting) = &mut self.send_formatting else {
            error!(target: LSP_CLIENT_LOG, "[{}] Formatting sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<Formatting>(&self.latches, id, None);

            return;
        };

        send_formatting.send(SendFormattingMessage {
            id,
            uri: self.uri.clone(),
            cancel,
            options,
        });
    }

    fn handle_completion(&mut self, id: RpcId, cancel: Trigger, position: Position) {
        let Some(send_formatting) = &mut self.send_completion else {
            error!(target: LSP_CLIENT_LOG, "[{}] Completion sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<Completion>(&self.latches, id, None);

            return;
        };

        send_formatting.send(SendCompletionMessage {
            id,
            uri: self.uri.clone(),
            cancel,
            position,
        });
    }

    fn handle_hover(&mut self, id: RpcId, cancel: Trigger, position: Position) {
        let Some(send_hover) = &mut self.send_hover else {
            error!(target: LSP_CLIENT_LOG, "[{}] Hover sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<HoverRequest>(&self.latches, id, None);

            return;
        };

        send_hover.send(SendHoverMessage {
            id,
            uri: self.uri.clone(),
            cancel,
            position,
        });
    }

    fn handle_goto_definition(&mut self, id: RpcId, cancel: Trigger, position: Position) {
        let Some(send_goto_definition) = &mut self.send_goto_definition else {
            error!(target: LSP_CLIENT_LOG, "[{}] Goto definition sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<GotoDefinition>(&self.latches, id, None);

            return;
        };

        send_goto_definition.send(SendGotoDefinitionMessage {
            id,
            uri: self.uri.clone(),
            cancel,
            position,
        });
    }

    fn handle_document_highlight(&mut self, id: RpcId, cancel: Trigger, position: Position) {
        let Some(send_document_highlight) = &mut self.send_document_highlight else {
            error!(target: LSP_CLIENT_LOG, "[{}] Document highlight sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<DocumentHighlightRequest>(&self.latches, id, None);

            return;
        };

        send_document_highlight.send(SendDocumentHighlightMessage {
            id,
            uri: self.uri.clone(),
            cancel,
            position,
        });
    }

    fn handle_goto_implementation(&mut self, id: RpcId, cancel: Trigger, position: Position) {
        let Some(send_goto_implementation) = &mut self.send_goto_implementation else {
            error!(target: LSP_CLIENT_LOG, "[{}] Goto implementation sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<GotoImplementation>(&self.latches, id, None);

            return;
        };

        send_goto_implementation.send(SendGotoImplementationMessage {
            id,
            uri: self.uri.clone(),
            cancel,
            position,
        });
    }

    fn handle_code_action(
        &mut self,
        id: RpcId,
        cancel: Trigger,
        range: Range,
        context: CodeActionContext,
    ) {
        let Some(send_code_action) = &mut self.send_code_action else {
            error!(target: LSP_CLIENT_LOG, "[{}] Code action sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<CodeActionRequest>(&self.latches, id, None);

            return;
        };

        send_code_action.send(SendCodeActionMessage {
            id,
            uri: self.uri.clone(),
            cancel,
            range,
            context,
        });
    }

    fn handle_signature_help(&mut self, id: RpcId, cancel: Trigger, position: Position) {
        let Some(send_goto_implementation) = &mut self.send_signature_help else {
            error!(target: LSP_CLIENT_LOG, "[{}] Signature help sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<SignatureHelpRequest>(&self.latches, id, None);

            return;
        };

        send_goto_implementation.send(SendSignatureHelpMessage {
            id,
            uri: self.uri.clone(),
            cancel,
            position,
        });
    }

    fn handle_rename(&mut self, id: RpcId, cancel: Trigger, new_name: String, position: Position) {
        let Some(send_rename) = &mut self.send_rename else {
            error!(target: LSP_CLIENT_LOG, "[{}] Rename sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<Rename>(&self.latches, id, None);

            return;
        };

        send_rename.send(SendRenameMessage {
            id,
            uri: self.uri.clone(),
            cancel,
            new_name,
            position,
        });
    }

    fn handle_prepare_rename(&mut self, id: RpcId, cancel: Trigger, position: Position) {
        let Some(send_prepare_rename) = &mut self.send_prepare_rename else {
            error!(target: LSP_CLIENT_LOG, "[{}] Prepare rename sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<PrepareRenameRequest>(&self.latches, id, None);

            return;
        };

        send_prepare_rename.send(SendPrepareRenameMessage {
            id,
            uri: self.uri.clone(),
            cancel,
            position,
        });
    }

    fn handle_linked_editing_range(&mut self, id: RpcId, cancel: Trigger, position: Position) {
        let Some(send_linked_editing_range) = &mut self.send_linked_editing_range else {
            error!(target: LSP_CLIENT_LOG, "[{}] Linked editing range sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<LinkedEditingRange>(&self.latches, id, None);

            return;
        };

        send_linked_editing_range.send(SendLinkedEditingRangeMessage {
            id,
            uri: self.uri.clone(),
            cancel,
            position,
        });
    }

    fn handle_code_lens(&mut self, id: RpcId, cancel: Trigger) {
        let Some(send_code_lens) = &mut self.send_code_lens else {
            error!(target: LSP_CLIENT_LOG, "[{}] Code lens sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<CodeLensRequest>(&self.latches, id, None);

            return;
        };

        send_code_lens.send(SendCodeLensMessage {
            id,
            uri: self.uri.clone(),
            cancel,
        });
    }

    fn handle_command(
        &mut self,
        id: RpcId,
        cancel: Trigger,
        command: String,
        command_arg: Option<Value>,
    ) {
        let Some(execute_command) = &mut self.send_execute_command else {
            error!(target: LSP_CLIENT_LOG, "[{}] Command execution sender is not initialized.", self.uri.as_str());

            self.outgoing
                .send_ok_response::<ExecuteCommand>(&self.latches, id, None);

            return;
        };

        execute_command.send(SendExecuteCommandMessage {
            id,
            uri: self.uri.clone(),
            cancel,
            command,
            command_arg,
        });
    }

    fn trigger_diagnostics(&mut self) {
        if let Some(publisher) = &mut self.publish_diagnostics_1 {
            publisher.send(PublishContext {
                uri: self.uri.clone(),
                version: self.version,
            });
        }

        if let Some(publisher) = &mut self.publish_diagnostics_2 {
            publisher.send(PublishContext {
                uri: self.uri.clone(),
                version: self.version,
            });
        }

        if let Some(publisher) = &mut self.publish_diagnostics_3 {
            publisher.send(PublishContext {
                uri: self.uri.clone(),
                version: self.version,
            });
        }
    }
}

pub(super) struct FileConfig {
    pub(super) config: LspServerConfig,
    pub(super) package: &'static PackageMeta,
    pub(super) health_check: Option<HealthCheck>,
    pub(super) latches: RpcLatches,
    pub(super) outgoing: RpcSender,
    pub(super) uri: Uri,
    pub(super) version: i32,
    pub(super) text: String,
}

pub(super) enum FileMessage {
    RenameFile {
        new_uri: String,
    },

    ChangeText {
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    },

    InlayHint {
        id: RpcId,
        cancel: Trigger,
        range: Range,
    },

    Formatting {
        id: RpcId,
        cancel: Trigger,
        options: FormattingOptions,
    },

    Completion {
        id: RpcId,
        cancel: Trigger,
        position: Position,
    },

    Hover {
        id: RpcId,
        cancel: Trigger,
        position: Position,
    },

    GotoDefinition {
        id: RpcId,
        cancel: Trigger,
        position: Position,
    },

    DocumentHighlight {
        id: RpcId,
        cancel: Trigger,
        position: Position,
    },

    GotoImplementation {
        id: RpcId,
        cancel: Trigger,
        position: Position,
    },

    CodeAction {
        id: RpcId,
        cancel: Trigger,
        range: Range,
        context: CodeActionContext,
    },

    SignatureHelp {
        id: RpcId,
        cancel: Trigger,
        position: Position,
    },

    Rename {
        id: RpcId,
        cancel: Trigger,
        new_name: String,
        position: Position,
    },

    PrepareRename {
        id: RpcId,
        cancel: Trigger,
        position: Position,
    },

    LinkedEditingRange {
        id: RpcId,
        cancel: Trigger,
        position: Position,
    },

    CodeLens {
        id: RpcId,
        cancel: Trigger,
    },

    Command {
        id: RpcId,
        cancel: Trigger,
        command: String,
        command_arg: Option<Value>,
    },
}
