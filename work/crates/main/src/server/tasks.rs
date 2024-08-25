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
    any::Any,
    collections::VecDeque,
    fmt::{Debug, Formatter},
    mem::take,
    ops::Deref,
    sync::{
        mpsc::{channel, sync_channel, Receiver, SendError, Sender, TryRecvError},
        Mutex,
    },
    thread::{current, park_timeout, Builder, JoinHandle, ThreadId},
    time::{Duration, Instant},
};

use ahash::RandomState;
use lady_deirdre::sync::{Shared, Table};
use log::{error, trace, warn};

use crate::{
    report::{debug_unreachable, system_panic},
    server::{
        file::LspModule,
        logger::{LSP_CLIENT_LOG, LSP_SERVER_LOG},
    },
};

pub(super) const COOL_DOWN: Duration = Duration::from_millis(100);
pub(super) const TIMEOUT: Duration = Duration::from_millis(500);

const STATS: usize = 10;

/// An object through which you can monitor the health status of the separate
/// job threads spawned by the server.
///
/// The health check status is available only if the `health_check` option
/// in [LspServerConfig](crate::server::LspServerConfig) is enabled.
///
/// You can access the server's HealthCheck object using the
/// [LspServer::health_check](crate::server::LspServer::health_check) function.
///
/// The [Debug] implementation of this object prints a list of threads that
/// haven't responded for a prolonged period.
#[derive(Clone)]
pub struct HealthCheck {
    timeout: Shared<Mutex<Duration>>,
    tasks: Shared<Table<ThreadId, Health, RandomState>>,
}

impl Debug for HealthCheck {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut report = self.check(false);

        report.sort_by(|(_, a), (_, b)| a.elapsed().cmp(&b.elapsed()).reverse());

        let mut debug_struct = f.debug_struct("HealthCheck");

        for (name, pong) in report {
            let _ = debug_struct.field(name.as_str(), &format_args!("{:?}", pong.elapsed()));
        }

        debug_struct.finish()
    }
}

impl HealthCheck {
    #[inline(always)]
    pub(super) fn new(timeout: Duration) -> Self {
        Self {
            timeout: Shared::new(Mutex::new(timeout)),
            tasks: Shared::default(),
        }
    }

    /// Sets the duration of time allocated for worker threads to finish
    /// their job tasks.
    ///
    /// The default `duration` is 5 seconds.
    ///
    /// If you set the `duration` to zero, automatic health checking will be
    /// disabled permanently.
    pub fn set_timeout(&self, timeout: Duration) {
        let mut timeout_guard = self
            .timeout
            .as_ref()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        *timeout_guard = timeout;
    }

    /// Returns the current duration of time allocated for worker threads to
    /// complete their job tasks.
    pub fn timeout(&self) -> Duration {
        let timeout_guard = self
            .timeout
            .as_ref()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        *timeout_guard
    }

    /// Sends ping signals to all currently running worker threads of the
    /// server.
    ///
    /// In response, the threads should update their health check timestamps.
    pub fn ping(&self) {
        for shard in self.tasks.as_ref().shards() {
            let shard_read_guard = shard.read().unwrap_or_else(|poison| poison.into_inner());

            for health in shard_read_guard.deref().values() {
                let _ = health.sender.send(TaskThreadCommand::Ping);
            }
        }
    }

    /// Manually checks if the worker thread timestamps are up to date
    /// within the current [timeout](Self::timeout) duration.
    ///
    /// The function returns a list of job task names and their timestamps if
    /// these timestamps exceed the timeout (i.e., problematic jobs).
    ///
    /// If the `interrupt` flag is set to `true`, the function will attempt to
    /// forcefully cancel problematic tasks.
    ///
    /// The function returns an empty vector if the timestamp is set to zero.
    pub fn check(&self, interrupt: bool) -> Vec<(String, Instant)> {
        let timeout = self.timeout();

        if timeout.is_zero() {
            return Vec::new();
        }

        let mut problematic = Vec::new();

        for shard in self.tasks.as_ref().shards() {
            let shard_read_guard = shard.read().unwrap_or_else(|poison| poison.into_inner());

            for health in shard_read_guard.deref().values() {
                if health.pong.elapsed() > timeout {
                    if interrupt {
                        health.module.as_ref().deny_access();
                    }

                    problematic.push((health.name.clone(), health.pong));
                }
            }
        }

        problematic
    }
}

pub(super) enum LocalOrRemote<T: Task> {
    Local(T),
    Remote(TaskHandle),
}

impl<T: Task> LocalOrRemote<T> {
    pub(super) fn new(
        name: impl AsRef<str>,
        multi_thread: bool,
        health_check: &Option<HealthCheck>,
        config: T::Config,
    ) -> Self {
        if !multi_thread {
            return Self::Local(T::init(config));
        }

        match T::spawn(name, health_check.clone(), config) {
            Ok(handle) => Self::Remote(handle),
            Err(local) => Self::Local(local),
        }
    }

    pub(super) fn send(&mut self, message: T::Message) {
        match self {
            Self::Local(task) => {
                let _ = task.handle(message);
            }

            Self::Remote(task) => {
                if task
                    .sender()
                    .send(TaskThreadCommand::Handle(Box::new(message)))
                    .is_err()
                {
                    error!(target: LSP_CLIENT_LOG, "Task channel closed.");
                }
            }
        }
    }
}

pub(super) enum TaskExecution {
    ExecuteEach,
    ExecuteLatest,
}

pub(super) trait Task: Sized {
    const EXECUTION: TaskExecution;

    type Config: Sized + Send + 'static;

    type Message: Sized + Send + 'static;

    fn init(config: Self::Config) -> Self;

    fn handle(&mut self, message: Self::Message) -> bool;

    fn module(&self) -> &LspModule;

    fn spawn(
        name: impl AsRef<str>,
        health_check: Option<HealthCheck>,
        config: Self::Config,
    ) -> Result<TaskHandle, Self> {
        let name = name.as_ref();
        let (init_sender, init_receiver) = sync_channel(1);
        let (commends_sender, commands_receiver) = channel();

        let result = {
            let name = String::from(name);
            let commends_sender = commends_sender.clone();

            Builder::new().name(name.clone()).spawn(move || {
                let initializer = TaskThreadInitializer::<Self> {
                    name,
                    init_receiver,
                    commends_sender,
                    commands_receiver,
                };

                let Some(handler) = initializer.init() else {
                    return;
                };

                handler.run();
            })
        };

        match result {
            Ok(handle) => {
                if let Err(SendError((_, config))) = init_sender.send((health_check, config)) {
                    error!(target: LSP_CLIENT_LOG, "{name} Thread config channel disconnected.");
                    return Err(Self::init(config));
                }

                drop(init_sender);

                Ok(TaskHandle {
                    inner: Some((commends_sender, handle)),
                })
            }

            Err(error) => {
                trace!(target: LSP_CLIENT_LOG, "{name} Thread creation failure. {error}");
                Err(Self::init(config))
            }
        }
    }
}

pub(super) struct TaskHandle {
    inner: Option<(Sender<TaskThreadCommand>, JoinHandle<()>)>,
}

impl Drop for TaskHandle {
    fn drop(&mut self) {
        let Some((sender, handle)) = take(&mut self.inner) else {
            return;
        };

        let _ = sender.send(TaskThreadCommand::Finish);

        drop(sender);

        let name = String::from(handle.thread().name().unwrap_or(""));

        trace!(target: LSP_CLIENT_LOG, "{name} Thread join...");

        match handle.join() {
            Ok(()) => {
                trace!(target: LSP_CLIENT_LOG, "{name} Thread released.");
            }

            Err(_) => {
                error!(target: LSP_CLIENT_LOG, "{name} Thread release failure.");
            }
        }
    }
}

impl TaskHandle {
    #[inline(always)]
    pub(super) fn sender(&self) -> &Sender<TaskThreadCommand> {
        let Some((sender, _)) = &self.inner else {
            // Inner state is always available when the instance is externally accessed.
            unsafe { debug_unreachable!("Missing inner state.") }
        };

        sender
    }

    #[inline(always)]
    #[allow(unused)]
    pub(super) fn detach(mut self) -> Sender<TaskThreadCommand> {
        let Some((sender, _)) = take(&mut self.inner) else {
            // Inner state is always available when the instance is externally accessed.
            unsafe { debug_unreachable!("Missing inner state.") }
        };

        sender
    }
}

pub(super) enum TaskThreadCommand {
    Handle(Box<dyn Any + Send>),
    Ping,
    Continue,
    Finish,
}

struct Health {
    name: String,
    pong: Instant,
    sender: Sender<TaskThreadCommand>,
    module: LspModule,
}

struct TaskThreadInitializer<T: Task> {
    name: String,
    init_receiver: Receiver<(Option<HealthCheck>, T::Config)>,
    commends_sender: Sender<TaskThreadCommand>,
    commands_receiver: Receiver<TaskThreadCommand>,
}

impl<T: Task> TaskThreadInitializer<T> {
    fn init(self) -> Option<TaskThreadExecutor<T>> {
        let Ok((health_check, config)) = self.init_receiver.recv() else {
            error!(target: LSP_CLIENT_LOG, "{} Thread was not initialized.", self.name);
            return None;
        };

        drop(self.init_receiver);

        let task = T::init(config);

        let thread_id = current().id();

        if let Some(health_check) = &health_check {
            let _ = health_check.tasks.as_ref().insert(
                thread_id,
                Health {
                    name: self.name.clone(),
                    pong: Instant::now(),
                    sender: self.commends_sender.clone(),
                    module: task.module().clone(),
                },
            );
        }

        let handler = TaskThreadExecutor {
            name: self.name.clone(),
            thread_id,
            health_check,
            commands_receiver: self.commands_receiver,
            task,
            stats: VecDeque::with_capacity(STATS),
        };

        trace!(target: LSP_CLIENT_LOG, "{} Thread initialized.", self.name);

        Some(handler)
    }
}

struct TaskThreadExecutor<T: Task> {
    name: String,
    thread_id: ThreadId,
    health_check: Option<HealthCheck>,
    commands_receiver: Receiver<TaskThreadCommand>,
    task: T,
    stats: VecDeque<Duration>,
}

impl<T: Task> TaskThreadExecutor<T> {
    #[inline(always)]
    fn run(mut self) {
        match T::EXECUTION {
            TaskExecution::ExecuteEach => self.run_each(),
            TaskExecution::ExecuteLatest => self.run_latest(),
        }

        if let Some(health_check) = &self.health_check {
            let _ = health_check.tasks.as_ref().remove(&self.thread_id);
        }

        trace!(target: LSP_CLIENT_LOG, "{} Thread finished.", self.name);
    }

    fn run_each(&mut self) {
        loop {
            let message = match self.get_message() {
                Ok(message) => message,
                Err(true) => continue,
                Err(false) => break,
            };

            if !self.task.handle(message) {
                break;
            }
        }
    }

    fn run_latest(&mut self) {
        loop {
            let message = match self.get_latest_message() {
                Ok(message) => message,
                Err(true) => continue,
                Err(false) => break,
            };

            let start = Instant::now();

            if !self.task.handle(message) {
                continue;
            }

            let end = start.elapsed();

            if end >= Self::mean(&self.stats) {
                warn!(
                    target: LSP_CLIENT_LOG,
                    "{} Execution time: {end:?}.",
                    self.name
                );
            }

            if self.stats.len() == STATS {
                let _ = self.stats.pop_front();
            }

            self.stats.push_back(end);
        }
    }

    fn get_message(&self) -> Result<T::Message, bool> {
        match self.receive_command() {
            TaskThreadCommand::Handle(message) => Ok(self.downcast(message)),

            TaskThreadCommand::Ping => {
                self.pong();
                Err(true)
            }

            TaskThreadCommand::Continue => Err(true),

            TaskThreadCommand::Finish => Err(false),
        }
    }

    fn get_latest_message(&self) -> Result<T::Message, bool> {
        let mut message = match self.receive_command() {
            TaskThreadCommand::Handle(message) => message,

            TaskThreadCommand::Ping => {
                self.pong();
                return Err(true);
            }

            TaskThreadCommand::Continue => return Err(true),

            TaskThreadCommand::Finish => return Err(false),
        };

        loop {
            park_timeout(COOL_DOWN);

            let mut new = false;

            loop {
                message = match self.try_receive_command() {
                    TaskThreadCommand::Handle(message) => {
                        new = true;
                        message
                    }

                    TaskThreadCommand::Ping => {
                        self.pong();
                        continue;
                    }

                    TaskThreadCommand::Continue => break,

                    TaskThreadCommand::Finish => return Err(false),
                };
            }

            if !new {
                break;
            }
        }

        Ok(self.downcast(message))
    }

    #[inline(always)]
    fn receive_command(&self) -> TaskThreadCommand {
        let Ok(task_thread_message) = self.commands_receiver.recv() else {
            return TaskThreadCommand::Finish;
        };

        task_thread_message
    }

    #[inline(always)]
    fn try_receive_command(&self) -> TaskThreadCommand {
        match self.commands_receiver.try_recv() {
            Ok(task_thread_message) => task_thread_message,
            Err(TryRecvError::Empty) => TaskThreadCommand::Continue,
            Err(TryRecvError::Disconnected) => TaskThreadCommand::Finish,
        }
    }

    #[inline(always)]
    fn pong(&self) {
        let Some(health_check) = &self.health_check else {
            return;
        };

        let Some(mut health) = health_check.tasks.as_ref().get_mut(&self.thread_id) else {
            error!(target: LSP_SERVER_LOG, "{} Missing health-check record.", self.name);
            return;
        };

        health.pong = Instant::now();
    }

    #[inline(always)]
    fn downcast(&self, message: Box<dyn Any + Send>) -> T::Message {
        let Ok(boxed) = message.downcast() else {
            system_panic!("{} Invalid thread message type.", self.name,);
        };

        *boxed
    }

    #[inline(always)]
    fn mean(stats: &VecDeque<Duration>) -> Duration {
        let mut delay = TIMEOUT;

        if !stats.is_empty() {
            delay += stats.iter().sum::<Duration>() / (stats.len() as u32)
        }

        delay
    }
}
