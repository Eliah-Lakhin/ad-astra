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

use std::sync::Mutex;

use lady_deirdre::{lexis::ToSpan, sync::Shared};
use log::{error, warn};
use lsp_types::{
    notification::PublishDiagnostics,
    Diagnostic,
    DiagnosticSeverity,
    NumberOrString,
    PublishDiagnosticsParams,
    Uri,
};
use serde_json::{Map, Value};

use crate::{
    analysis::{
        DiagnosticsDepth,
        IssueQuickfix,
        IssueSeverity,
        ModuleDiagnostics,
        ModuleError,
        ModuleRead,
        ModuleText,
    },
    server::{
        file::{LspModule, DIAGNOSTICS_PRIORITY},
        logger::{LSP_CLIENT_LOG, LSP_SERVER_LOG},
        rpc::{LspHandle, OutgoingEx},
        tasks::{Task, TaskExecution},
        utils::span_to_range,
        RpcSender,
    },
};

pub(super) struct DiagnosticsPublisher<const DEPTH: DiagnosticsDepth> {
    pub(super) outgoing: RpcSender,
    pub(super) module: LspModule,
    pub(super) diagnostics: Shared<Mutex<FileDiagnostics>>,
}

impl<const DEPTH: DiagnosticsDepth> Task for DiagnosticsPublisher<DEPTH> {
    const EXECUTION: TaskExecution = TaskExecution::ExecuteLatest;

    type Config = Self;

    type Message = PublishContext;

    #[inline(always)]
    fn init(config: Self::Config) -> Self {
        config
    }

    #[inline(always)]
    fn handle(&mut self, message: Self::Message) -> bool {
        let handle = LspHandle::default();

        let module_read_guard = match self.module.as_ref().read(&handle, DIAGNOSTICS_PRIORITY) {
            Ok(guard) => guard,

            Err(ModuleError::Interrupted(_)) => {
                warn!(target: LSP_SERVER_LOG, "[{}] Diagnostics {DEPTH:?} interrupted.", message.uri.as_str());
                return false;
            }

            Err(error) => {
                error!(target: LSP_CLIENT_LOG, "[{}] Diagnostics {DEPTH:?} error. {error}", message.uri.as_str());
                return true;
            }
        };

        let diagnostics = match module_read_guard.diagnostics(DEPTH) {
            Ok(diagnostics) => diagnostics,

            Err(ModuleError::Interrupted(_)) => {
                warn!(target: LSP_SERVER_LOG, "[{}] Diagnostics {DEPTH:?} interrupted.", message.uri.as_str());
                return false;
            }

            Err(error) => {
                error!(target: LSP_CLIENT_LOG, "[{}] Diagnostics {DEPTH:?} error. {error}", message.uri.as_str());
                return true;
            }
        };

        let mut field_diagnostics_guard = self
            .diagnostics
            .as_ref()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        if !field_diagnostics_guard.update(diagnostics) {
            return true;
        };

        let text = module_read_guard.text();

        self.outgoing
            .notify::<PublishDiagnostics>(PublishDiagnosticsParams {
                uri: message.uri,
                diagnostics: field_diagnostics_guard.snapshot(&text),
                version: Some(message.version),
            });

        true
    }

    #[inline(always)]
    fn module(&self) -> &LspModule {
        &self.module
    }
}

pub(super) struct PublishContext {
    pub(super) uri: Uri,
    pub(super) version: i32,
}

#[derive(Default)]
pub(super) struct FileDiagnostics {
    diagnostics: [Option<ModuleDiagnostics>; 3],
}

impl FileDiagnostics {
    fn update(&mut self, diagnostics: ModuleDiagnostics) -> bool {
        let current = &mut self.diagnostics[diagnostics.depth() as usize - 1];

        if let Some(current) = &current {
            if current.revision() >= diagnostics.revision() {
                return false;
            }
        }

        *current = Some(diagnostics);
        true
    }

    fn snapshot(&self, text: &ModuleText) -> Vec<Diagnostic> {
        let mut result = Vec::new();

        for disagnostics in &self.diagnostics {
            let Some(diagnostics) = disagnostics else {
                continue;
            };

            for issue in diagnostics {
                let Some(span) = issue.origin(text).to_position_span(text) else {
                    continue;
                };

                let range = span_to_range(&span);

                let severity = match issue.severity() {
                    IssueSeverity::Error => DiagnosticSeverity::ERROR,
                    IssueSeverity::Warning => DiagnosticSeverity::WARNING,
                };

                let code = NumberOrString::Number(issue.code() as i32);

                let message = issue.verbose_message(text);

                let data = issue
                    .quickfix()
                    .map(|quickfix| Value::from(DiagnosticData(quickfix)));

                result.push(Diagnostic {
                    range,
                    severity: Some(severity),
                    code: Some(code),
                    //todo consider providing a link to the RustDoc
                    code_description: None,
                    message,
                    data,

                    ..Diagnostic::default()
                });
            }
        }

        result
    }
}

pub(super) struct DiagnosticData(pub(super) IssueQuickfix);

impl From<DiagnosticData> for Value {
    fn from(value: DiagnosticData) -> Self {
        let mut map = Map::new();

        if let Some(set_text_to_origin) = value.0.set_text_to_origin {
            let _ = map.insert(
                String::from("set_text_to_origin"),
                Value::String(set_text_to_origin),
            );
        }

        if let Some(implement_use_of) = value.0.implement_use_of {
            let _ = map.insert(
                String::from("implement_use_of"),
                Value::String(implement_use_of),
            );
        }

        Self::Object(map)
    }
}

impl From<&Value> for DiagnosticData {
    fn from(value: &Value) -> Self {
        let mut issue = IssueQuickfix::default();

        let Some(value) = value.as_object() else {
            return DiagnosticData(issue);
        };

        if let Some(set_text_to_origin) = value.get("set_text_to_origin") {
            if let Some(set_text_to_origin) = set_text_to_origin.as_str() {
                issue.set_text_to_origin = Some(String::from(set_text_to_origin));
            }
        }

        if let Some(implement_use_of) = value.get("implement_use_of") {
            if let Some(implement_use_of) = implement_use_of.as_str() {
                issue.implement_use_of = Some(String::from(implement_use_of));
            }
        }

        DiagnosticData(issue)
    }
}
