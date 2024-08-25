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

use std::{collections::hash_map::Entry, ops::Deref, process::exit, str::FromStr, sync::Arc};

use ahash::{AHashMap, RandomState};
use lady_deirdre::sync::{Shared, Table, Trigger};
use log::{debug, error, info, warn};
use lsp_types::{
    error_codes::{REQUEST_FAILED, SERVER_NOT_INITIALIZED},
    notification::{
        Cancel,
        DidChangeTextDocument,
        DidCloseTextDocument,
        DidOpenTextDocument,
        Exit,
        Initialized,
        SetTrace,
    },
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
        Initialize,
        InlayHintRequest,
        LinkedEditingRange,
        PrepareRenameRequest,
        Rename,
        Shutdown,
        SignatureHelpRequest,
        WillRenameFiles,
    },
    CancelParams,
    CodeActionKind,
    CodeActionOptions,
    CodeActionProviderCapability,
    CodeLensOptions,
    CompletionOptions,
    DidChangeTextDocumentParams,
    ExecuteCommandOptions,
    FileOperationFilter,
    FileOperationPattern,
    FileOperationPatternKind,
    FileOperationRegistrationOptions,
    FileRename,
    HoverProviderCapability,
    ImplementationProviderCapability,
    InitializeResult,
    LinkedEditingRangeServerCapabilities,
    OneOf,
    PositionEncodingKind,
    RenameOptions,
    ServerCapabilities,
    ServerInfo,
    SignatureHelpOptions,
    TextDocumentItem,
    TextDocumentSyncCapability,
    TextDocumentSyncKind,
    TextDocumentSyncOptions,
    Uri,
    WorkspaceFileOperationsServerCapabilities,
    WorkspaceServerCapabilities,
};

use crate::{
    runtime::PackageMeta,
    server::{
        command::{CMD_CLEANUP, CMD_LAUNCH, CMD_STOP},
        file::{File, FileConfig, FileMessage},
        logger::{LspLogger, LSP_CLIENT_LOG, LSP_SERVER_LOG},
        rpc::{
            OutgoingEx,
            RpcId,
            RpcLatches,
            RpcMessageInner,
            RpcNotification,
            RpcRequest,
            RpcResponse,
        },
        tasks::LocalOrRemote,
        HealthCheck,
        LspCapabilities,
        LspLoggerConfig,
        LspServerConfig,
        RpcMessage,
        RpcSender,
    },
};

/// A Language Server.
///
/// Typically, you run the server using the [LspServer::startup]
/// function, which performs all necessary preparations to establish
/// communication between the client and the server, and sets up the server
/// itself.
///
/// If you need more control over the server lifecycle, for example,
/// to manually manage the communication channel, you can use
/// the [LspServer::new] constructor function instead.
pub struct LspServer {
    pub(super) config: LspServerConfig,
    pub(super) package: &'static PackageMeta,
    pub(super) outgoing: Arc<RpcSender>,
    pub(super) state: ServerState,
    pub(super) latches: RpcLatches,
    pub(super) health_check: Option<HealthCheck>,
    pub(super) files: AHashMap<String, LocalOrRemote<File>>,
}

impl LspServer {
    /// Creates an instance of the server.
    ///
    /// This function sets up a ready-to-use LSP server, but it does not
    /// manually establish a communication channel, set up the server's logger,
    /// or perform automatic server health checking.
    ///
    /// You are responsible for managing the LSP server lifecycle manually.
    ///
    /// The `config` parameter specifies the general server configuration
    /// options.
    ///
    /// The `package` parameter is the metadata of the
    /// [Script Package](crate::runtime::ScriptPackage) under which the server
    /// will analyze the client's source code files.
    ///
    /// The `outgoing` parameter specifies the [sender end](RpcSender) of the
    /// outgoing RPC messages channel. The server will use this end to add new
    /// messages intended to be sent to the client. You can read the outgoing
    /// messages from the [receiver end](crate::server::RpcReceiver) of the
    /// channel and send them to the client through any communication channel
    /// of your choice. You can create the sender and receiver ends using the
    /// [RpcMessage::channel] function.
    ///
    /// To handle incoming messages sent from the client to the server, use
    /// the [LspServer::handle] function.
    ///
    /// Note that if the `multi_thread` flag in the [LspServerConfig]
    /// configuration is set to false, the server does not spawn any threads.
    /// As a result, the server handles incoming messages synchronously on the
    /// current thread. Thus, outgoing messages may only appear in the
    /// receiver end after calling the [handle](LspServer::handle) function.
    ///
    /// This feature is useful for creating an LSP server in a wasm
    /// container that is intended to run solely on the main thread.
    ///
    /// For a fully automated server setup, consider using the
    /// [LspServer::startup] function instead.
    #[inline(always)]
    pub fn new(
        config: LspServerConfig,
        package: &'static PackageMeta,
        outgoing: RpcSender,
    ) -> Self {
        let latches = match config.multi_thread {
            true => Table::new(),
            false => Table::with_capacity_and_hasher_and_shards(0, RandomState::default(), 1),
        };

        let health_check = match config.health_check {
            Some(timeout) => Some(HealthCheck::new(timeout)),
            None => None,
        };

        Self {
            config,
            package,
            outgoing: Arc::new(outgoing),
            state: ServerState::Uninit,
            latches: Shared::new(latches),
            health_check,
            files: AHashMap::new(),
        }
    }

    /// Sets up the server logger according to the specified `config`.
    ///
    /// Note that the server-side logger can only be set up once, and only if
    /// another logger has not been configured outside of the server.
    ///
    /// The function returns true if the server successfully sets up the
    /// logger.
    #[inline(always)]
    pub fn setup_logger(&self, config: LspLoggerConfig) -> bool {
        LspLogger::setup(config, &self.outgoing)
    }

    /// Provides an object through which you can manually check the health
    /// status of the worker threads spawned by the server.
    ///
    /// The function returns `None` if the `health_check` flag in
    /// [LspServerConfig] is set to false.
    #[inline(always)]
    pub fn health_check(&self) -> Option<&HealthCheck> {
        self.health_check.as_ref()
    }

    /// Returns true if the communication session has been established between
    /// the server and the client, and both the server and the client are fully
    /// initialized and ready to work.
    #[inline(always)]
    pub fn initialized(&self) -> bool {
        match &self.state {
            ServerState::Initialized | ServerState::Shutdown => true,
            _ => false,
        }
    }

    /// Returns `true` if the client has instructed the server to shut down.
    #[inline(always)]
    pub fn shutting_down(&self) -> bool {
        match &&self.state {
            ServerState::Shutdown => true,
            _ => false,
        }
    }

    /// Handles an incoming RPC message from the client.
    ///
    /// You can create this `message` object by deserializing the incoming raw
    /// text of the message using the [RpcMessage::from_input_bytes] function.
    pub fn handle(&mut self, message: RpcMessage) {
        match message.0 {
            RpcMessageInner::Request(message) => self.handle_request(message),
            RpcMessageInner::Response(message) => self.handle_response(message),
            RpcMessageInner::Notification(message) => self.handle_notification(message),
        }
    }

    pub(super) fn check_state(&self, id: Option<&RpcId>) -> bool {
        match &self.state {
            ServerState::Uninit | ServerState::Initializing => {
                let Some(id) = id else {
                    error!(
                        target: LSP_CLIENT_LOG,
                        "Cannot handle incoming message because the server is not initialized.",
                    );
                    return false;
                };

                self.outgoing.send_err_response(
                    &self.latches,
                    id.clone(),
                    SERVER_NOT_INITIALIZED,
                    "Server not initialized.",
                );

                false
            }

            ServerState::Initialized => true,

            ServerState::Shutdown => {
                let Some(id) = id else {
                    error!(
                        target: LSP_CLIENT_LOG,
                        "Cannot handle incoming message because the server is shutting down.",
                    );
                    return false;
                };

                self.outgoing.send_err_response(
                    &self.latches,
                    id.clone(),
                    REQUEST_FAILED,
                    "Server is shutting down.",
                );

                false
            }
        }
    }

    #[inline(always)]
    fn register_latch(
        &mut self,
        id: &RpcId,
        uri: &Uri,
    ) -> Option<(Trigger, &mut LocalOrRemote<File>)> {
        let Some(file) = self.files.get_mut(uri.as_str()) else {
            error!(target: LSP_CLIENT_LOG, "[{}] Missing document.", uri.as_str());

            self.outgoing.send_err_response(
                &self.latches,
                id.clone(),
                REQUEST_FAILED,
                "Unrecognized document.",
            );

            return None;
        };

        let cancel = Trigger::new();

        let _ = self.latches.as_ref().insert(id.clone(), cancel.clone());

        Some((cancel, file))
    }
}

// Requests

impl LspServer {
    fn handle_request(&mut self, message: RpcRequest) {
        if message.is::<Initialize>() {
            return self.handle_request_initialize(message);
        }

        if message.is::<Shutdown>() {
            return self.handle_request_shutdown(message);
        }

        if message.is::<WillRenameFiles>() {
            return self.handle_request_will_rename_files(message);
        }

        if message.is::<InlayHintRequest>() {
            return self.handle_request_inlay_hint(message);
        }

        if message.is::<Formatting>() {
            return self.handle_formatting(message);
        }

        if message.is::<Completion>() {
            return self.handle_completion(message);
        }

        if message.is::<HoverRequest>() {
            return self.handle_hover(message);
        }

        if message.is::<GotoDefinition>() {
            return self.handle_goto_definition(message);
        }

        if message.is::<DocumentHighlightRequest>() {
            return self.handle_document_highlight(message);
        }

        if message.is::<GotoImplementation>() {
            return self.handle_goto_implementation(message);
        }

        if message.is::<CodeActionRequest>() {
            return self.handle_code_action(message);
        }

        if message.is::<SignatureHelpRequest>() {
            return self.handle_signature_help(message);
        }

        if message.is::<Rename>() {
            return self.handle_rename(message);
        }

        if message.is::<PrepareRenameRequest>() {
            return self.handle_prepare_rename(message);
        }

        if message.is::<LinkedEditingRange>() {
            return self.handle_linked_editing_range(message);
        }

        if message.is::<CodeLensRequest>() {
            return self.handle_code_lens(message);
        }

        if message.is::<ExecuteCommand>() {
            return self.handle_execute_command(message);
        }

        error!(target: LSP_CLIENT_LOG, "Unhandled {:?}.", message);

        self.outgoing.send_err_response(
            &self.latches,
            message.id,
            REQUEST_FAILED,
            format!("Unimplemented request handler for {:?}.", message.method),
        );
    }

    fn handle_request_initialize(&mut self, message: RpcRequest) {
        let (id, params) = match &self.state {
            ServerState::Uninit => message.extract::<Initialize>(),

            ServerState::Initializing | ServerState::Initialized => {
                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_FAILED,
                    "Server already initialized.",
                );
                return;
            }

            ServerState::Shutdown => {
                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_FAILED,
                    "Server is shutting down.",
                );
                return;
            }
        };

        match params.client_info {
            None => info!(target: LSP_SERVER_LOG, "Server initialization."),

            Some(info) => match info.version {
                None => {
                    info!(target: LSP_SERVER_LOG,  "Server initialization for client {:?}.", info.name)
                }

                Some(version) => {
                    info!(target: LSP_SERVER_LOG,  "Server initialization for client {:?} ({version}).", info.name)
                }
            },
        }

        if let Some(trace_value) = params.trace {
            LspLogger::set_trace_value(trace_value);
        }

        self.config
            .capabilities
            .intersect(LspCapabilities::from_client(&params.capabilities));

        self.outgoing.send_ok_response::<Initialize>(
            &self.latches,
            id,
            InitializeResult {
                capabilities: ServerCapabilities {
                    position_encoding: Some(PositionEncodingKind::UTF16),

                    text_document_sync: Some(TextDocumentSyncCapability::Options(
                        TextDocumentSyncOptions {
                            open_close: Some(true),
                            change: Some(TextDocumentSyncKind::INCREMENTAL),

                            ..Default::default()
                        },
                    )),

                    workspace: Some(WorkspaceServerCapabilities {
                        workspace_folders: None,
                        file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                            will_rename: Some(FileOperationRegistrationOptions {
                                filters: vec![FileOperationFilter {
                                    scheme: Some(String::from("file")),
                                    pattern: FileOperationPattern {
                                        glob: format!("**/*.{}", self.config.file_ext),
                                        matches: Some(FileOperationPatternKind::File),
                                        options: None,
                                    },
                                }],
                            }),

                            ..Default::default()
                        }),
                    }),

                    inlay_hint_provider: match self.config.capabilities.inlay_hints {
                        true => Some(OneOf::Left(true)),
                        false => None,
                    },

                    document_formatting_provider: match self.config.capabilities.formatting {
                        true => Some(OneOf::Left(true)),
                        false => None,
                    },

                    completion_provider: match self.config.capabilities.completion {
                        true => Some(CompletionOptions::default()),
                        false => None,
                    },

                    hover_provider: match self.config.capabilities.hover {
                        true => Some(HoverProviderCapability::Simple(true)),
                        false => None,
                    },

                    definition_provider: match self.config.capabilities.goto_definition {
                        true => Some(OneOf::Left(true)),
                        false => None,
                    },

                    document_highlight_provider: match self.config.capabilities.document_highlight {
                        true => Some(OneOf::Left(true)),
                        false => None,
                    },

                    implementation_provider: match self.config.capabilities.goto_implementation {
                        true => Some(ImplementationProviderCapability::Simple(true)),
                        false => None,
                    },

                    code_action_provider: match self.config.capabilities.code_action {
                        true => Some(CodeActionProviderCapability::Options(CodeActionOptions {
                            code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),

                            ..CodeActionOptions::default()
                        })),

                        false => None,
                    },

                    signature_help_provider: match self.config.capabilities.signature_help {
                        true => Some(SignatureHelpOptions {
                            trigger_characters: Some(vec![String::from("(")]),

                            ..SignatureHelpOptions::default()
                        }),

                        false => None,
                    },

                    rename_provider: match self.config.capabilities.rename {
                        true => Some(OneOf::Right(RenameOptions {
                            prepare_provider: Some(self.config.capabilities.rename_prepare),
                            work_done_progress_options: Default::default(),
                        })),

                        false => None,
                    },

                    linked_editing_range_provider: match self
                        .config
                        .capabilities
                        .linked_editing_range
                    {
                        true => Some(LinkedEditingRangeServerCapabilities::Simple(true)),

                        false => None,
                    },

                    code_lens_provider: match self.config.capabilities.code_lens {
                        true => Some(CodeLensOptions {
                            resolve_provider: None,
                        }),

                        false => None,
                    },

                    execute_command_provider: match self.config.capabilities.execute_command {
                        true => Some(ExecuteCommandOptions {
                            commands: {
                                let mut commands = Vec::new();

                                if self.config.scripts_runner {
                                    commands.push(String::from(CMD_LAUNCH));
                                    commands.push(String::from(CMD_STOP));

                                    if self.config.capabilities.inlay_hints {
                                        commands.push(String::from(CMD_CLEANUP));
                                    }
                                }

                                commands
                            },
                            work_done_progress_options: Default::default(),
                        }),

                        false => None,
                    },

                    ..Default::default()
                },

                server_info: Some(ServerInfo {
                    name: String::from(self.package.name()),
                    version: Some(String::from(self.package.version())),
                }),
            },
        );

        self.state = ServerState::Initializing;
    }

    fn handle_request_shutdown(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        self.state = ServerState::Shutdown;

        self.files.clear();

        self.outgoing
            .send_ok_response::<Shutdown>(&self.latches, request.id, ());
    }

    fn handle_request_will_rename_files(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<WillRenameFiles>();

        for FileRename { old_uri, new_uri } in params.files {
            if !new_uri.ends_with(&format!(".{}", self.config.file_ext)) {
                continue;
            }

            let Some(mut file) = self.files.remove(old_uri.as_str()) else {
                error!(target: LSP_CLIENT_LOG, "[{old_uri}] Missing document.");
                continue;
            };

            file.send(FileMessage::RenameFile {
                new_uri: new_uri.clone(),
            });

            if self.files.insert(new_uri.clone(), file).is_some() {
                error!(target: LSP_CLIENT_LOG, "[{new_uri}] Duplicate document.");
                continue;
            }

            debug!(target: LSP_CLIENT_LOG, "[{old_uri}] Will rename to [{new_uri}].");
        }

        self.outgoing
            .send_ok_response::<WillRenameFiles>(&self.latches, id, None);
    }

    fn handle_request_inlay_hint(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<InlayHintRequest>();

        let Some(file) = self.files.get_mut(params.text_document.uri.as_str()) else {
            error!(target: LSP_CLIENT_LOG, "[{}] Missing document.", params.text_document.uri.as_str());
            self.outgoing.send_err_response(
                &self.latches,
                id,
                REQUEST_FAILED,
                "Unrecognized document.",
            );
            return;
        };

        let cancel = Trigger::new();

        let _ = self.latches.as_ref().insert(id.clone(), cancel.clone());

        file.send(FileMessage::InlayHint {
            id,
            cancel,
            range: params.range,
        });
    }

    fn handle_formatting(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<Formatting>();

        let Some(file) = self.files.get_mut(params.text_document.uri.as_str()) else {
            error!(target: LSP_CLIENT_LOG, "[{}] Missing document.", params.text_document.uri.as_str());
            self.outgoing.send_err_response(
                &self.latches,
                id,
                REQUEST_FAILED,
                "Unrecognized document.",
            );
            return;
        };

        let cancel = Trigger::new();

        let _ = self.latches.as_ref().insert(id.clone(), cancel.clone());

        file.send(FileMessage::Formatting {
            id,
            cancel,
            options: params.options,
        });
    }

    fn handle_completion(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<Completion>();

        let Some((cancel, file)) =
            self.register_latch(&id, &params.text_document_position.text_document.uri)
        else {
            return;
        };

        file.send(FileMessage::Completion {
            id,
            cancel,
            position: params.text_document_position.position,
        });
    }

    fn handle_hover(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<HoverRequest>();

        let Some((cancel, file)) =
            self.register_latch(&id, &params.text_document_position_params.text_document.uri)
        else {
            return;
        };

        file.send(FileMessage::Hover {
            id,
            cancel,
            position: params.text_document_position_params.position,
        });
    }

    fn handle_goto_definition(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<GotoDefinition>();

        let Some((cancel, file)) =
            self.register_latch(&id, &params.text_document_position_params.text_document.uri)
        else {
            return;
        };

        file.send(FileMessage::GotoDefinition {
            id,
            cancel,
            position: params.text_document_position_params.position,
        });
    }

    fn handle_document_highlight(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<DocumentHighlightRequest>();

        let Some((cancel, file)) =
            self.register_latch(&id, &params.text_document_position_params.text_document.uri)
        else {
            return;
        };

        file.send(FileMessage::DocumentHighlight {
            id,
            cancel,
            position: params.text_document_position_params.position,
        });
    }

    fn handle_goto_implementation(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<GotoImplementation>();

        let Some((cancel, file)) =
            self.register_latch(&id, &params.text_document_position_params.text_document.uri)
        else {
            return;
        };

        file.send(FileMessage::GotoImplementation {
            id,
            cancel,
            position: params.text_document_position_params.position,
        });
    }

    fn handle_code_action(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<CodeActionRequest>();

        let Some((cancel, file)) = self.register_latch(&id, &params.text_document.uri) else {
            return;
        };

        file.send(FileMessage::CodeAction {
            id,
            cancel,
            range: params.range,
            context: params.context,
        });
    }

    fn handle_signature_help(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<SignatureHelpRequest>();

        let Some((cancel, file)) =
            self.register_latch(&id, &params.text_document_position_params.text_document.uri)
        else {
            return;
        };

        file.send(FileMessage::SignatureHelp {
            id,
            cancel,
            position: params.text_document_position_params.position,
        });
    }

    fn handle_rename(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<Rename>();

        let Some((cancel, file)) =
            self.register_latch(&id, &params.text_document_position.text_document.uri)
        else {
            return;
        };

        file.send(FileMessage::Rename {
            id,
            cancel,
            new_name: params.new_name,
            position: params.text_document_position.position,
        });
    }

    fn handle_prepare_rename(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<PrepareRenameRequest>();

        let Some((cancel, file)) = self.register_latch(&id, &params.text_document.uri) else {
            return;
        };

        file.send(FileMessage::PrepareRename {
            id,
            cancel,
            position: params.position,
        });
    }

    fn handle_linked_editing_range(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<LinkedEditingRange>();

        let Some((cancel, file)) =
            self.register_latch(&id, &params.text_document_position_params.text_document.uri)
        else {
            return;
        };

        file.send(FileMessage::LinkedEditingRange {
            id,
            cancel,
            position: params.text_document_position_params.position,
        });
    }

    fn handle_code_lens(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, params) = request.extract::<CodeLensRequest>();

        let Some((cancel, file)) = self.register_latch(&id, &params.text_document.uri) else {
            return;
        };

        file.send(FileMessage::CodeLens { id, cancel });
    }

    fn handle_execute_command(&mut self, request: RpcRequest) {
        if !self.check_state(Some(&request.id)) {
            return;
        }

        let (id, mut params) = request.extract::<ExecuteCommand>();

        let Some(uri) = params.arguments.first() else {
            self.outgoing.send_err_response(
                &self.latches,
                id,
                REQUEST_FAILED,
                "Missing uri argument in the command arguments.",
            );
            return;
        };

        let Some(uri) = uri.as_str() else {
            self.outgoing.send_err_response(
                &self.latches,
                id,
                REQUEST_FAILED,
                "Uri argument in the command arguments is not a string.",
            );
            return;
        };

        let uri = match Uri::from_str(uri) {
            Ok(uri) => uri,

            Err(error) => {
                self.outgoing.send_err_response(
                    &self.latches,
                    id,
                    REQUEST_FAILED,
                    format!("Uri argument in the command arguments parse error. {error}"),
                );
                return;
            }
        };

        let Some((cancel, file)) = self.register_latch(&id, &uri) else {
            return;
        };

        let command_arg = match params.arguments.len() > 1 {
            false => None,
            true => params.arguments.pop(),
        };

        file.send(FileMessage::Command {
            id,
            cancel,
            command: params.command,
            command_arg,
        });
    }
}

// Responses

impl LspServer {
    fn handle_response(&mut self, _message: RpcResponse) {
        //todo Valid client responses (e.g. from the Inlay Hint Refresh requests)
        //     are currently not tracked.

        // error!(target: LSP_CLIENT_LOG, "Unhandled {:?}.", message);
    }
}

// Notifications

impl LspServer {
    fn handle_notification(&mut self, message: RpcNotification) {
        if message.is::<Exit>() {
            return self.handle_notification_exit(message);
        }

        if message.is::<Initialized>() {
            return self.handle_notification_initialized(message);
        }

        if message.is::<SetTrace>() {
            return self.handle_notification_set_trace(message);
        }

        if message.is::<Cancel>() {
            return self.handle_notification_cancel(message);
        }

        if message.is::<DidOpenTextDocument>() {
            return self.handle_notification_did_open_text_document(message);
        }

        if message.is::<DidCloseTextDocument>() {
            return self.handle_notification_did_close_text_document(message);
        }

        if message.is::<DidChangeTextDocument>() {
            return self.handle_notification_did_change_text_document(message);
        }

        error!(target: LSP_CLIENT_LOG, "Unhandled {:?}.", message);
    }

    fn handle_notification_exit(&mut self, _message: RpcNotification) {
        if !self.shutting_down() {
            error!(target: LSP_SERVER_LOG, "Abnormal exit.");
            exit(1);
        }

        info!(target: LSP_SERVER_LOG, "Normal exit.");
    }

    fn handle_notification_initialized(&mut self, _message: RpcNotification) {
        if let ServerState::Initializing = &self.state {
            info!(target: LSP_CLIENT_LOG, "Server initialized.");
            self.state = ServerState::Initialized;
            return;
        }

        let _ = self.check_state(None);
    }

    fn handle_notification_set_trace(&mut self, message: RpcNotification) {
        if !self.check_state(None) {
            return;
        }

        let params = message.extract::<SetTrace>();

        LspLogger::set_trace_value(params.value);
    }

    fn handle_notification_cancel(&mut self, message: RpcNotification) {
        if !self.check_state(None) {
            return;
        }

        let CancelParams { id } = message.extract::<Cancel>();

        let id = RpcId::from(id);

        let Some(latch_read_guard) = self.latches.as_ref().get(&id) else {
            warn!(target: LSP_SERVER_LOG, "Client unable to cancel request {id:?}.");
            return;
        };

        latch_read_guard.deref().activate();
    }

    fn handle_notification_did_open_text_document(&mut self, message: RpcNotification) {
        if !self.check_state(None) {
            return;
        }

        let params = message.extract::<DidOpenTextDocument>();

        let TextDocumentItem {
            uri,
            language_id,
            version,
            text,
        } = params.text_document;

        match self.files.entry(uri.to_string()) {
            Entry::Occupied(entry) => match language_id == self.config.language_id {
                true => {
                    debug!(
                        target: LSP_CLIENT_LOG,
                        "[{}] Document preserved (language preserved).",
                        uri.as_str(),
                    );
                }

                false => {
                    let _ = entry.remove();

                    debug!(
                        target: LSP_CLIENT_LOG,
                        "[{}] Document detached, because it's language has been changed.",
                        uri.as_str(),
                    );
                }
            },

            Entry::Vacant(entry) => {
                if language_id != self.config.language_id {
                    error!(
                        target: LSP_CLIENT_LOG,
                        "[{}] Incorrect language id. Expected {:?}, but {language_id:?} provided.",
                        uri.as_str(),
                        self.config.language_id,
                    );
                    return;
                }

                debug!(target: LSP_CLIENT_LOG, "[{}] Document attached.", entry.key());

                let value = LocalOrRemote::new(
                    format!("[{}] (file)", entry.key()),
                    self.config.multi_thread,
                    &self.health_check,
                    FileConfig {
                        config: self.config,
                        package: self.package,
                        health_check: self.health_check.clone(),
                        latches: self.latches.clone(),
                        outgoing: self.outgoing.deref().clone(),
                        uri,
                        version,
                        text,
                    },
                );

                let _ = entry.insert(value);
            }
        }
    }

    fn handle_notification_did_close_text_document(&mut self, message: RpcNotification) {
        if !self.check_state(None) {
            return;
        }

        let uri = message.extract::<DidCloseTextDocument>().text_document.uri;

        if self.files.remove(uri.as_str()).is_none() {
            return;
        }

        debug!(target: LSP_CLIENT_LOG, "[{}] Document detached.", uri.as_str());
    }

    fn handle_notification_did_change_text_document(&mut self, message: RpcNotification) {
        if !self.check_state(None) {
            return;
        }

        let DidChangeTextDocumentParams {
            text_document,
            content_changes,
        } = message.extract::<DidChangeTextDocument>();

        let Some(file) = self.files.get_mut(text_document.uri.as_str()) else {
            error!(target: LSP_CLIENT_LOG, "[{}] Missing document.", text_document.uri.as_str());
            return;
        };

        file.send(FileMessage::ChangeText {
            version: text_document.version,
            changes: content_changes,
        });
    }
}

#[derive(Clone)]
pub(super) enum ServerState {
    Uninit,
    Initializing,
    Initialized,
    Shutdown,
}
