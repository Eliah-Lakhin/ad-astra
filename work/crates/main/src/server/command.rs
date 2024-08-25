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

use std::{
    cell::UnsafeCell,
    collections::hash_map::Entry,
    ops::DerefMut,
    sync::{
        mpsc::{sync_channel, SyncSender},
        RwLock,
    },
    thread::{park_timeout, Builder},
    time::Instant,
};

use ahash::AHashMap;
use lady_deirdre::{
    format::TerminalString,
    sync::{Shared, Trigger},
};
use log::{error, info, trace, warn};
use lsp_types::{
    error_codes::{REQUEST_CANCELLED, REQUEST_FAILED},
    request::{CodeLensRefresh, ExecuteCommand, InlayHintRefreshRequest},
    Uri,
};
use serde_json::Value;

use crate::{
    analysis::{ModuleError, ModuleRead},
    interpret::{set_runtime_hook, ScriptFn},
    runtime::{Origin, RuntimeError, ScriptOrigin},
    server::{
        file::{LspModule, COMMAND_PRIORITY},
        logger::{LSP_CLIENT_LOG, LSP_SERVER_LOG},
        rpc::{LspHandle, OutgoingEx, RpcId, RpcLatches},
        tasks::{Task, TaskExecution, COOL_DOWN, TIMEOUT},
        LspServerConfig,
        RpcSender,
    },
};

pub(super) static CMD_LAUNCH: &'static str = "script_launch";
pub(super) static CMD_STOP: &'static str = "script_stop";
pub(super) static CMD_CLEANUP: &'static str = "script_cleanup";

thread_local! {
    static THREAD_RUNNER_STATE: UnsafeCell<Option<RunnerThreadData>> = const {
        UnsafeCell::new(None)
    };
}

/// A helper function that displays custom inlay hints while scripts are being
/// executed in the editor.
///
/// This function is useful for implementing a script-debugging feature that
/// displays underlying debug messages directly in the editor's window.
///
/// The `origin` parameter specifies a script code range where the inlay hint
/// message should appear. The `Origin` object must reference the script module
/// text currently being executed.
///
/// The `hint` parameter is the message that will be displayed in the
/// editor's window. This message should be a short, one-line string, as long
/// hints may be truncated by the editor.
///
/// The `tooltip` parameter provides additional details and can be a custom
/// (potentially multiline) text displayed to the user when they hover over
/// the hint message. You can specify an empty string if you don't want to
/// provide a tooltip.
///
/// This function will be ignored if `origin` points to an invalid source
/// code span or if the span does not correspond to the module currently being
/// executed by the editor.
///
/// ## Example
///
/// ```no_run
/// use ad_astra::{
///     export,
///     runtime::ops::{DynamicArgument, DynamicReturn, DynamicType},
///     server::inlay_hint,
/// };
///
/// #[export]
/// pub fn dbg(arg: DynamicArgument<DynamicType>) -> DynamicReturn<DynamicType> {
///     let message = arg.data.stringify(false);
///     let tooltip = arg.data.stringify(true);
///
///     let tooltip = match message == tooltip {
///         true => String::new(),
///         false => format!("```\n{tooltip}\n```"),
///     };
///
///     inlay_hint(arg.origin, message, tooltip);
///
///     DynamicReturn::new(arg.data)
/// }
/// ```
///
/// Users of the editor will be able to use this exported function to print
/// script values in place:
///
/// ```text
/// let x = 10;
///
/// dbg(x); // Displays an inlay hint in the editor near this function call
///         // with the string " ≈ 10".
/// ```
#[inline(always)]
pub fn inlay_hint(origin: Origin, hint: impl AsRef<str>, tooltip: impl Into<String>) {
    THREAD_RUNNER_STATE.with(move |runner_thread| {
        let Origin::Script(origin) = origin else {
            return;
        };

        // Safety: Access is localized.
        let runner_thread = unsafe { &mut *runner_thread.get() };

        let Some(runner_thread) = runner_thread else {
            return;
        };

        let Some(inlay_hints_refresher) = &runner_thread.inlay_hints_refresher else {
            return;
        };

        let Some(hint) = hint.as_ref().split("\n").next() else {
            return;
        };

        let mut hint = hint.sanitize().to_string().trim().to_string();

        if hint.is_empty() {
            return;
        }

        hint = format!(" ≈ {hint}");

        let mut runner_state_guard = runner_thread
            .runner_state
            .as_ref()
            .write()
            .unwrap_or_else(|poison| poison.into_inner());

        let runner_state = runner_state_guard.deref_mut();
        let message_index = &mut runner_thread.message_index;
        let messages = &mut runner_state.messages;

        let message = CustomMessage {
            origin,
            hint,
            tooltip: tooltip.into(),
        };

        match message_index.entry(origin) {
            Entry::Occupied(entry) => {
                let index = *entry.get();

                let Some(target) = messages.get_mut(index) else {
                    return;
                };

                *target = message;
            }

            Entry::Vacant(entry) => {
                let index = messages.len();

                let _ = entry.insert(index);

                messages.push(message);
            }
        }

        let _ = inlay_hints_refresher.try_send(());
    });
}

#[inline(always)]
fn set_runner_thread_data(runner_thread_data: Option<RunnerThreadData>) {
    THREAD_RUNNER_STATE.with(move |current| {
        // Safety: Access is localized.
        let current = unsafe { &mut *current.get() };

        *current = runner_thread_data;
    });
}

pub(super) struct SendExecuteCommand {
    pub(super) config: LspServerConfig,
    pub(super) latches: RpcLatches,
    pub(super) outgoing: RpcSender,
    pub(super) module: LspModule,
    pub(super) runner_state: SharedRunnerState,
}

impl Task for SendExecuteCommand {
    const EXECUTION: TaskExecution = TaskExecution::ExecuteEach;

    type Config = Self;

    type Message = SendExecuteCommandMessage;

    #[inline(always)]
    fn init(config: Self::Config) -> Self {
        config
    }

    fn handle(&mut self, mut message: Self::Message) -> bool {
        loop {
            if message.cancel.is_active() {
                warn!(
                    target: LSP_SERVER_LOG,
                    "[{}] Send execute command cancelled by the client.",
                    message.uri.as_str(),
                );

                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_CANCELLED,
                    "Send execute command cancelled by the client.",
                );

                break;
            }

            let command = message.command.as_str();

            if command == CMD_LAUNCH {
                message = match self.execute_launch_command(message) {
                    Some(message) => message,
                    None => break,
                };

                continue;
            }

            if command == CMD_STOP {
                message = match self.execute_stop_command(message) {
                    Some(message) => message,
                    None => break,
                };

                continue;
            }

            if command == CMD_CLEANUP {
                message = match self.execute_cleanup_command(message) {
                    Some(message) => message,
                    None => break,
                };

                continue;
            }

            error!(target: LSP_SERVER_LOG, "[{}] Unknown command {command}.", message.uri.as_str());

            self.outgoing.send_err_response(
                &self.latches,
                message.id,
                REQUEST_FAILED,
                format!("Unknown command {command}."),
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

impl Drop for SendExecuteCommand {
    fn drop(&mut self) {
        let mut runner_state_guard = self
            .runner_state
            .as_ref()
            .write()
            .unwrap_or_else(|poison| poison.into_inner());

        runner_state_guard.job += 1;
    }
}

impl SendExecuteCommand {
    fn execute_launch_command(
        &self,
        message: SendExecuteCommandMessage,
    ) -> Option<SendExecuteCommandMessage> {
        let message = self.check_runner_preconditions(message)?;

        let handle = LspHandle::new(&message.cancel);

        let module_read_guard = match self.module.as_ref().read(&handle, COMMAND_PRIORITY) {
            Ok(guard) => guard,

            Err(ModuleError::Interrupted(_)) => {
                if message.cancel.is_active() {
                    warn!(target: LSP_SERVER_LOG, "[{}] Launch command cancelled by the client.", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_CANCELLED,
                        "Launch command cancelled by the client.",
                    );

                    return None;
                }

                warn!(target: LSP_SERVER_LOG, "[{}] Launch command interrupted.", message.uri.as_str());
                park_timeout(COOL_DOWN);
                return Some(message);
            }

            Err(error) => {
                error!(target: LSP_SERVER_LOG, "[{}] Launch command error. {error}", message.uri.as_str());

                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_FAILED,
                    "Launch command error.",
                );

                return None;
            }
        };

        let time = Instant::now();

        let script_fn = match module_read_guard.compile() {
            Ok(script_fn) => script_fn,

            Err(ModuleError::Interrupted(_)) => {
                if message.cancel.is_active() {
                    warn!(target: LSP_SERVER_LOG, "[{}] Launch command cancelled by the client.", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_CANCELLED,
                        "Launch command cancelled by the client.",
                    );

                    return None;
                }

                warn!(target: LSP_SERVER_LOG, "[{}] Launch command interrupted.", message.uri.as_str());
                park_timeout(COOL_DOWN);
                return Some(message);
            }

            Err(error) => {
                error!(target: LSP_SERVER_LOG, "[{}] Launch command error. {error}", message.uri.as_str());

                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_FAILED,
                    "Launch command error.",
                );

                return None;
            }
        };

        let time = time.elapsed();

        if time >= TIMEOUT {
            info!(target: LSP_CLIENT_LOG, "[{}] Script compiled in {time:?}.", message.uri.as_str());
        }

        let _ = self.spawn_launch_job(message.uri, script_fn);

        self.outgoing
            .send_ok_response::<ExecuteCommand>(&self.latches, message.id, None);

        None
    }

    fn execute_stop_command(
        &self,
        message: SendExecuteCommandMessage,
    ) -> Option<SendExecuteCommandMessage> {
        let message = self.check_runner_preconditions(message)?;

        let Some(Value::Number(job)) = message.command_arg else {
            error!(target: LSP_SERVER_LOG, "[{}] Stop command error. Missing job number.", message.uri.as_str());

            self.outgoing.send_err_response(
                &self.latches,
                message.id,
                REQUEST_FAILED,
                "Stop command error. Missing job number.",
            );

            return None;
        };

        let job = job.as_u64().unwrap_or_default();

        {
            let mut runner_state_guard = self
                .runner_state
                .as_ref()
                .write()
                .unwrap_or_else(|poison| poison.into_inner());

            if runner_state_guard.job == job && runner_state_guard.enabled {
                runner_state_guard.enabled = false;

                self.outgoing.request::<CodeLensRefresh>(());
            }
        };

        self.outgoing
            .send_ok_response::<ExecuteCommand>(&self.latches, message.id, None);

        None
    }

    fn execute_cleanup_command(
        &self,
        message: SendExecuteCommandMessage,
    ) -> Option<SendExecuteCommandMessage> {
        let message = self.check_runner_preconditions(message)?;

        {
            let mut runner_state_guard = self
                .runner_state
                .as_ref()
                .write()
                .unwrap_or_else(|poison| poison.into_inner());

            if !runner_state_guard.messages.is_empty() {
                runner_state_guard.messages = Vec::new();

                self.outgoing.request::<CodeLensRefresh>(());
                self.outgoing.request::<InlayHintRefreshRequest>(());
            }
        };

        self.outgoing
            .send_ok_response::<ExecuteCommand>(&self.latches, message.id, None);

        None
    }

    fn check_runner_preconditions(
        &self,
        message: SendExecuteCommandMessage,
    ) -> Option<SendExecuteCommandMessage> {
        if !self.config.scripts_runner {
            warn!(
                target: LSP_SERVER_LOG,
                "[{}] Cannot run the script because the script runner disabled.",
                message.uri.as_str(),
            );

            self.outgoing.send_err_response(
                &self.latches,
                message.id,
                REQUEST_FAILED,
                "Cannot run the script because the script runner disabled.",
            );

            return None;
        }

        if !self.config.multi_thread {
            warn!(
                target: LSP_SERVER_LOG,
                "[{}] Cannot run the script because the server is in a single-thread mode.",
                message.uri.as_str(),
            );

            self.outgoing.send_err_response(
                &self.latches,
                message.id,
                REQUEST_FAILED,
                "Cannot run the script because the server is in a single-thread mode.",
            );

            return None;
        }

        Some(message)
    }

    fn spawn_launch_job(&self, uri: Uri, script_fn: ScriptFn) -> u64 {
        let job = {
            let mut runner_state_guard = self
                .runner_state
                .as_ref()
                .write()
                .unwrap_or_else(|poison| poison.into_inner());

            runner_state_guard.job += 1;

            if runner_state_guard.enabled {
                runner_state_guard.enabled = false;

                self.outgoing.request::<CodeLensRefresh>(());
            }

            if !runner_state_guard.messages.is_empty() {
                runner_state_guard.messages = Vec::new();

                self.outgoing.request::<InlayHintRefreshRequest>(());
            }

            runner_state_guard.job
        };

        let builder = Builder::new().name(format!("[{}] (script runner)", uri.as_str()));

        let result = {
            let uri = uri.clone();
            let module = self.module.clone();
            let runner_state = self.runner_state.clone();
            let outgoing = self.outgoing.clone();
            let inlay_hints_refresher = self.spawn_inlay_hints_refresher(&uri);

            builder.spawn(move || {
                let uri = uri;
                let script_fn = script_fn;
                let module = module;
                let runner_state = runner_state;
                let inlay_hints_refresher = inlay_hints_refresher;
                let outgoing = outgoing;

                {
                    let mut runner_state_guard = runner_state
                        .as_ref()
                        .write()
                        .unwrap_or_else(|poison| poison.into_inner());

                    if runner_state_guard.job != job {
                        return;
                    }

                    if !runner_state_guard.enabled {
                        runner_state_guard.enabled = true;

                        outgoing.request::<CodeLensRefresh>(());
                    }
                }

                set_runtime_hook({
                    let runner_state = runner_state.clone();

                    move |_: &Origin| {
                        let runner_state = &runner_state;

                        let runner_state_guard = runner_state
                            .as_ref()
                            .read()
                            .unwrap_or_else(|poison| poison.into_inner());

                        runner_state_guard.enabled && runner_state_guard.job == job
                    }
                });

                set_runner_thread_data(Some(RunnerThreadData {
                    message_index: AHashMap::new(),
                    runner_state: runner_state.clone(),
                    inlay_hints_refresher: inlay_hints_refresher.clone(),
                }));

                info!(
                    target: LSP_CLIENT_LOG,
                    "[{}] Evaluation started.",
                    uri.as_str(),
                );

                let result = script_fn.run();

                set_runner_thread_data(None);

                let mut runner_state_guard = runner_state
                    .as_ref()
                    .write()
                    .unwrap_or_else(|poison| poison.into_inner());

                if runner_state_guard.job != job {
                    return;
                }

                if runner_state_guard.enabled {
                    runner_state_guard.enabled = false;

                    outgoing.request::<CodeLensRefresh>(());
                }

                match result {
                    Ok(result) => {
                        info!(
                            target: LSP_CLIENT_LOG,
                            "[{}] Evaluation finished.\n{}",
                            uri.as_str(),
                            result.stringify(false),
                        );
                    }

                    Err(error) => {
                        let handle = LspHandle::default();

                        let description = match module.as_ref().read(&handle, COMMAND_PRIORITY) {
                            Ok(module) => {
                                let text = module.text();
                                let description = error.display(&text).to_string();

                                Some(description)
                            }

                            _ => None,
                        };

                        match (error.primary_origin(), inlay_hints_refresher) {
                            (Origin::Script(origin), Some(inlay_hints_refresher)) => {
                                let tooltip = match &description {
                                    Some(description) => {
                                        format!("```text\n{}\n```", description.sanitize())
                                    }
                                    _ => error.summary(),
                                };

                                runner_state_guard.messages.push(CustomMessage {
                                    origin: *origin,
                                    hint: format!(" ❗ {}", error.primary_description()),
                                    tooltip,
                                });

                                let _ = inlay_hints_refresher.try_send(());
                            }

                            _ => (),
                        }

                        if let RuntimeError::Interrupted { .. } = &error {
                            warn!(
                                target: LSP_CLIENT_LOG,
                                "[{}] Evaluation interrupted.",
                                uri.as_str(),
                            );

                            return;
                        };

                        let Some(description) = description else {
                            warn!(
                                target: LSP_CLIENT_LOG,
                                "[{}] Evaluation failed.",
                                uri.as_str(),
                            );

                            return;
                        };

                        warn!(
                            target: LSP_CLIENT_LOG,
                            "[{}] Evaluation error.\n{}",
                            uri.as_str(),
                            description,
                        );
                    }
                }
            })
        };

        if let Err(error) = result {
            error!(
                target: LSP_SERVER_LOG,
                "[{}] Script runner thread spawn error. {error}",
                uri.as_str(),
            );
        }

        job
    }

    fn spawn_inlay_hints_refresher(&self, uri: &Uri) -> Option<SyncSender<()>> {
        if !self.config.capabilities.inlay_hints {
            return None;
        }

        let (sender, receiver) = sync_channel(1);

        let builder = Builder::new().name(format!("[{}] (inlay hints refresher)", uri.as_str()));

        let result = {
            let uri = uri.clone();
            let outgoing = self.outgoing.clone();

            builder.spawn(move || {
                trace!("[{}] (inlay hints refresher) Thread started.", uri.as_str());

                while let Ok(()) = receiver.recv() {
                    park_timeout(TIMEOUT);
                    outgoing.request::<InlayHintRefreshRequest>(());
                }

                trace!(
                    "[{}] (inlay hints refresher) Thread finished.",
                    uri.as_str(),
                );
            })
        };

        if let Err(error) = result {
            error!(
                target: LSP_SERVER_LOG,
                "[{}] Inlay hints refresher thread spawn error. {error}",
                uri.as_str(),
            );

            return None;
        }

        Some(sender)
    }
}

pub(super) struct SendExecuteCommandMessage {
    pub(super) id: RpcId,
    pub(super) uri: Uri,
    pub(super) cancel: Trigger,
    pub(super) command: String,
    pub(super) command_arg: Option<Value>,
}

pub(super) type SharedRunnerState = Shared<RwLock<RunnerState>>;

pub(super) struct RunnerState {
    pub(super) job: u64,
    pub(super) enabled: bool,
    pub(super) messages: Vec<CustomMessage>,
}

impl Default for RunnerState {
    #[inline(always)]
    fn default() -> Self {
        Self {
            job: 0,
            enabled: false,
            messages: Vec::new(),
        }
    }
}

pub(super) struct CustomMessage {
    pub(super) origin: ScriptOrigin,
    pub(super) hint: String,
    pub(super) tooltip: String,
}

struct RunnerThreadData {
    message_index: AHashMap<ScriptOrigin, usize>,
    runner_state: SharedRunnerState,
    inlay_hints_refresher: Option<SyncSender<()>>,
}
