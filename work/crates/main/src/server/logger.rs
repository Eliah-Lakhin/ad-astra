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
    io::{stderr, Write},
    ops::Deref,
    process::id,
    sync::{Arc, Mutex, RwLock, Weak},
    time::Instant,
};

use lady_deirdre::{
    format::{Color, Style, TerminalString},
    sync::Lazy,
};
use log::{info, set_max_level, Level, LevelFilter, Log, Metadata, Record};
use lsp_types::{
    notification::{LogMessage, LogTrace},
    LogMessageParams,
    LogTraceParams,
    MessageType,
    TraceValue,
};
#[cfg(not(target_family = "wasm"))]
use syslog::{Facility, Formatter3164, LoggerBackend};

use crate::server::{
    rpc::RpcNotification,
    LspLoggerClientConfig,
    LspLoggerConfig,
    LspLoggerServerConfig,
    RpcMessage,
    RpcSender,
};

pub(super) static RPC_LOG: &'static str = "ad-astra::$rpc";
pub(super) static LSP_CLIENT_LOG: &'static str = "ad-astra::$client";
pub(super) static LSP_SERVER_LOG: &'static str = "ad-astra::$server";

static PROCESS_NAME: &'static str = "ad-astra-lsp-server";
static TRACE_VALUE: Lazy<RwLock<TraceValue>> = Lazy::new(|| RwLock::new(TraceValue::Off));

pub(super) struct LspLogger {
    level: LevelFilter,
    client: ClientLoggerSetup,
    server: ServerLoggerSetup,
}

impl Log for LspLogger {
    #[inline(always)]
    fn enabled(&self, metadata: &Metadata) -> bool {
        (metadata.level() as usize) <= (self.level as usize)
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        match &self.server {
            ServerLoggerSetup::Off => {}

            ServerLoggerSetup::Stderr { start } => {
                let mut stderr = stderr().lock();
                let _ = writeln!(stderr, "{}", Self::for_stderr(start, record));
                let _ = stderr.flush();
            }

            #[cfg(not(target_family = "wasm"))]
            ServerLoggerSetup::Syslog { logger } => {
                let message = Self::for_syslog(record);

                let mut logger = logger.lock().unwrap_or_else(|poison| poison.into_inner());

                let result = match record.level() {
                    Level::Error => logger.err(&message),
                    Level::Warn => logger.warning(&message),
                    Level::Info => logger.info(&message),
                    Level::Debug => logger.debug(&message),
                    Level::Trace => logger.notice(&message),
                };

                if let Err(error) = result {
                    let mut stderr = stderr().lock();

                    let _ = writeln!(&mut stderr, "Failed to send a message to syslog. {error}");

                    let _ = stderr.flush();
                }
            }

            ServerLoggerSetup::Custom(callback) => {
                callback(record.level(), Self::for_custom(record))
            }
        }

        if record.target() == RPC_LOG {
            return;
        }

        if record.target() == LSP_SERVER_LOG {
            return;
        }

        match &self.client {
            ClientLoggerSetup::Off => {}

            ClientLoggerSetup::Trace { outgoing } => {
                let Some(outgoing) = Weak::upgrade(outgoing) else {
                    return;
                };

                let trace_value_guard = TRACE_VALUE
                    .read()
                    .unwrap_or_else(|poison| poison.into_inner());

                let verbose = match trace_value_guard.deref() {
                    TraceValue::Off => return,
                    TraceValue::Messages => false,
                    TraceValue::Verbose => true,
                };

                drop(trace_value_guard);

                let message = Self::for_client(record);
                let mut parts = message.splitn(2, "\n");

                let Some(message) = parts.next() else {
                    return;
                };

                let _ = outgoing.send(RpcMessage::from(RpcNotification::new::<LogTrace>(
                    LogTraceParams {
                        message: String::from(message),
                        verbose: parts.next().filter(|_| verbose).map(String::from),
                    },
                )));
            }

            ClientLoggerSetup::Window { outgoing } => {
                let Some(outgoing) = Weak::upgrade(outgoing) else {
                    return;
                };

                let typ = match record.level() {
                    Level::Error => MessageType::ERROR,
                    Level::Warn => MessageType::WARNING,
                    Level::Info => MessageType::INFO,
                    Level::Debug => MessageType::INFO,
                    Level::Trace => MessageType::INFO,
                };

                let _ = outgoing.send(RpcMessage::from(RpcNotification::new::<LogMessage>(
                    LogMessageParams {
                        typ,
                        message: Self::for_client(record),
                    },
                )));
            }
        }
    }

    #[inline(always)]
    fn flush(&self) {}
}

impl LspLogger {
    pub(super) fn setup(config: LspLoggerConfig, outgoing: &Arc<RpcSender>) -> bool {
        if !config.enabled {
            return false;
        }

        let logger = LspLogger::new(config, outgoing);

        set_max_level(logger.level);

        #[cfg(not(target_family = "wasm"))]
        {
            log::set_boxed_logger(Box::new(logger)).is_ok()
        }

        #[cfg(target_family = "wasm")]
        {
            struct StaticLspLogger(RwLock<Option<LspLogger>>);

            impl Log for StaticLspLogger {
                #[inline(always)]
                fn enabled(&self, metadata: &Metadata) -> bool {
                    let inner = self.0.read().unwrap_or_else(|poison| poison.into_inner());

                    let Some(inner) = inner.deref() else {
                        return false;
                    };

                    inner.enabled(metadata)
                }

                #[inline(always)]
                fn log(&self, record: &Record) {
                    let inner = self.0.read().unwrap_or_else(|poison| poison.into_inner());

                    let Some(inner) = inner.deref() else {
                        return;
                    };

                    inner.log(record);
                }

                #[inline(always)]
                fn flush(&self) {}
            }

            static LOGGER: StaticLspLogger = StaticLspLogger(RwLock::new(None));

            let mut inner = LOGGER
                .0
                .write()
                .unwrap_or_else(|poison| poison.into_inner());

            if inner.is_some() {
                return false;
            }

            *inner = Some(logger);

            log::set_logger(&LOGGER).is_ok()
        }
    }

    #[inline(always)]
    fn new(config: LspLoggerConfig, outgoing: &Arc<RpcSender>) -> Self {
        let client = match config.client {
            LspLoggerClientConfig::Off => ClientLoggerSetup::Off,

            LspLoggerClientConfig::Trace => ClientLoggerSetup::Trace {
                outgoing: Arc::downgrade(outgoing),
            },

            LspLoggerClientConfig::Window => ClientLoggerSetup::Window {
                outgoing: Arc::downgrade(outgoing),
            },
        };

        let server = match config.server {
            LspLoggerServerConfig::Off => ServerLoggerSetup::Off,

            LspLoggerServerConfig::Stderr => ServerLoggerSetup::Stderr {
                start: Instant::now(),
            },

            #[cfg(not(target_family = "wasm"))]
            LspLoggerServerConfig::Syslog => match syslog::unix(Formatter3164 {
                facility: Facility::LOG_USER,
                hostname: None,
                process: String::from(PROCESS_NAME),
                pid: id(),
            }) {
                Ok(logger) => ServerLoggerSetup::Syslog {
                    logger: Mutex::new(logger),
                },

                Err(error) => {
                    eprintln!("Syslog setup error. Switching to stderr as a fallback. {error}");

                    ServerLoggerSetup::Stderr {
                        start: Instant::now(),
                    }
                }
            },

            #[cfg(target_family = "wasm")]
            LspLoggerServerConfig::Syslog => {
                panic!("Syslog not available under the wasm target.");
            }

            LspLoggerServerConfig::Custom(callback) => ServerLoggerSetup::Custom(callback),
        };

        let mut level = config.level;

        if let (ClientLoggerSetup::Off, ServerLoggerSetup::Off) = (&client, &server) {
            level = LevelFilter::Off;
        }

        Self {
            level,
            client,
            server,
        }
    }

    pub(super) fn set_trace_value(new_value: TraceValue) {
        let mut old_value = TRACE_VALUE
            .write()
            .unwrap_or_else(|poison| poison.into_inner());

        if old_value.deref() == &new_value {
            return;
        }

        info!(target: LSP_SERVER_LOG, "New trace value: {new_value:?}.");

        *old_value = new_value;
    }

    fn for_client(record: &Record) -> String {
        record.args().to_string().sanitize()
    }

    fn for_stderr(start: &Instant, record: &Record) -> String {
        let target = {
            let target = record.target();

            if target == LSP_CLIENT_LOG || target == LSP_SERVER_LOG {
                String::from("[lsp]")
            } else if target == RPC_LOG {
                format!("[{}]", "rpc".apply(Style::new().invert()))
            } else {
                match record.line() {
                    Some(line) => format!("[{target}::{line}]"),
                    None => format!("[{target}]"),
                }
            }
        };

        let color = match record.level() {
            Level::Error => Color::Red,
            Level::Warn => Color::Yellow,
            Level::Info => Color::Green,
            Level::Debug => Color::BrightBlue,
            Level::Trace => Color::BrightBlack,
        };

        let mut result = String::with_capacity(1024);

        let duration = start.elapsed();

        let mut seconds = duration.as_secs();
        let mut minutes = seconds / 60;
        let hours = minutes / 60;

        seconds -= minutes * 60;
        minutes -= hours * 60;

        result.push_str(&format!("{hours:02}:{minutes:02}:{seconds:02} "));

        result.push_str(&target.apply(Style::new().bold().fg(color)));

        let args = record.args().to_string();

        if args.len() > 0 {
            if !args.starts_with('\n') {
                result.push(' ');
            }

            result.push_str(&args);
        }

        result
    }

    fn for_syslog(record: &Record) -> String {
        let target = record.target();
        let args = record.args().to_string().sanitize();

        if target == RPC_LOG || target == LSP_SERVER_LOG || target == LSP_CLIENT_LOG {
            return args;
        }

        let Some(line) = args.lines().next().filter(|line| !line.is_empty()) else {
            return match record.line() {
                Some(line) => format!("[{target}::{line}]"),
                None => format!("[{target}]"),
            };
        };

        line.to_string()
    }

    fn for_custom(record: &Record) -> String {
        let target = {
            let target = record.target();

            if target == LSP_CLIENT_LOG || target == LSP_SERVER_LOG {
                String::from("[lsp]")
            } else if target == RPC_LOG {
                String::from("[rpc]")
            } else {
                match record.line() {
                    Some(line) => format!("[{target}::{line}]"),
                    None => format!("[{target}]"),
                }
            }
        };

        let mut result = String::with_capacity(1024);

        result.push_str(&target);

        let args = record.args().to_string();

        if args.len() > 0 {
            if !args.starts_with('\n') {
                result.push(' ');
            }

            result.push_str(&args.sanitize());
        }

        result
    }
}

enum ClientLoggerSetup {
    Off,

    Trace { outgoing: Weak<RpcSender> },

    Window { outgoing: Weak<RpcSender> },
}

enum ServerLoggerSetup {
    Off,
    Stderr {
        start: Instant,
    },

    #[cfg(not(target_family = "wasm"))]
    Syslog {
        logger: Mutex<syslog::Logger<LoggerBackend, Formatter3164>>,
    },

    Custom(fn(Level, String)),
}
