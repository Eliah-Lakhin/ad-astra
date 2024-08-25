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

use ahash::AHashSet;
use lady_deirdre::{lexis::ToSpan, sync::Trigger};
use log::{error, warn};
use lsp_types::{
    error_codes::{REQUEST_CANCELLED, REQUEST_FAILED},
    request::{LinkedEditingRange, PrepareRenameRequest, Rename},
    LinkedEditingRanges,
    Position,
    PrepareRenameResponse,
    TextEdit,
    Uri,
    WorkspaceEdit,
};

use crate::{
    analysis::{
        symbols::{
            EntrySymbol,
            FieldSymbol,
            IdentKind,
            IdentSymbol,
            LookupOptions,
            ModuleSymbol,
            SymbolKind,
            VarRef,
            VarSymbol,
        },
        ModuleError,
        ModuleRead,
        ModuleReadGuard,
        ModuleResult,
    },
    runtime::ScriptOrigin,
    server::{
        file::{LspModule, ANALYSIS_PRIORITY},
        logger::LSP_SERVER_LOG,
        rpc::{LspHandle, OutgoingEx, RpcId, RpcLatches},
        tasks::{Task, TaskExecution, COOL_DOWN},
        utils::{lsp_position_to_ld, span_to_range},
        RpcSender,
    },
};

const RENAME_SYMBOLS: u32 = (SymbolKind::Var as u32)
    | (SymbolKind::Ident as u32)
    | (SymbolKind::Field as u32)
    | (SymbolKind::Entry as u32);

const PREPARE_RENAME_SYMBOLS: u32 = (SymbolKind::Package as u32)
    | (SymbolKind::Var as u32)
    | (SymbolKind::Entry as u32)
    | (SymbolKind::Ident as u32)
    | (SymbolKind::Field as u32)
    | (SymbolKind::Literal as u32)
    | (SymbolKind::Operator as u32);

pub(super) struct SendRename {
    pub(super) latches: RpcLatches,
    pub(super) outgoing: RpcSender,
    pub(super) module: LspModule,
}

impl Task for SendRename {
    const EXECUTION: TaskExecution = TaskExecution::ExecuteEach;

    type Config = Self;

    type Message = SendRenameMessage;

    #[inline(always)]
    fn init(config: Self::Config) -> Self {
        config
    }

    fn handle(&mut self, message: Self::Message) -> bool {
        loop {
            if message.cancel.is_active() {
                warn!(target: LSP_SERVER_LOG, "[{}] Send rename cancelled by the client.", message.uri.as_str());

                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_CANCELLED,
                    "Send rename cancelled by the client.",
                );

                break;
            }

            let handle = LspHandle::new(&message.cancel);

            let module_read_guard = match self.module.as_ref().read(&handle, ANALYSIS_PRIORITY) {
                Ok(guard) => guard,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send rename cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send rename cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send rename interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send rename error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send rename error.",
                    );

                    break;
                }
            };

            let position = lsp_position_to_ld(&message.position);

            let symbols = match module_read_guard.symbols(
                position..position,
                LookupOptions::new().filter(RENAME_SYMBOLS),
            ) {
                Ok(symbols) => symbols,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send rename cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send rename cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send rename interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send rename error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send rename error.",
                    );

                    break;
                }
            };

            let Some(symbol) = symbols.first() else {
                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    0,
                    "nothing to rename in this position",
                );

                break;
            };

            let text = module_read_guard.text();

            let mut analyzer = RenameAnalyzer {
                read: &module_read_guard,
                edits: AHashSet::new(),
                error: "",
            };

            match analyzer.rename(symbol, message.new_name.as_str()) {
                Ok(()) => (),

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send rename cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send rename cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send rename interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send rename error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send rename error.",
                    );

                    break;
                }
            };

            if !analyzer.error.is_empty() {
                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    0,
                    String::from(analyzer.error),
                );

                break;
            };

            let mut edits = Vec::with_capacity(analyzer.edits.len());

            for edit in analyzer.edits {
                let Some(span) = edit.to_position_span(&text) else {
                    continue;
                };

                let range = span_to_range(&span);

                edits.push(TextEdit {
                    range,
                    new_text: message.new_name.clone(),
                });
            }

            if edits.is_empty() {
                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    0,
                    "nothing to rename in this position",
                );

                break;
            }

            self.outgoing.send_ok_response::<Rename>(
                &self.latches,
                message.id,
                Some(WorkspaceEdit {
                    changes: Some(HashMap::from([(message.uri, edits)])),

                    ..WorkspaceEdit::default()
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

pub(super) struct SendPrepareRename {
    pub(super) latches: RpcLatches,
    pub(super) outgoing: RpcSender,
    pub(super) module: LspModule,
}

impl Task for SendPrepareRename {
    const EXECUTION: TaskExecution = TaskExecution::ExecuteEach;

    type Config = Self;

    type Message = SendPrepareRenameMessage;

    #[inline(always)]
    fn init(config: Self::Config) -> Self {
        config
    }

    fn handle(&mut self, message: Self::Message) -> bool {
        loop {
            if message.cancel.is_active() {
                warn!(target: LSP_SERVER_LOG, "[{}] Send prepare rename cancelled by the client.", message.uri.as_str());

                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_CANCELLED,
                    "Send prepare rename cancelled by the client.",
                );

                break;
            }

            let handle = LspHandle::new(&message.cancel);

            let module_read_guard = match self.module.as_ref().read(&handle, ANALYSIS_PRIORITY) {
                Ok(guard) => guard,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send prepare rename cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send prepare rename cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send prepare rename interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send prepare rename error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send prepare rename error.",
                    );

                    break;
                }
            };

            let position = lsp_position_to_ld(&message.position);

            let symbols = match module_read_guard.symbols(
                position..position,
                LookupOptions::new().filter(PREPARE_RENAME_SYMBOLS),
            ) {
                Ok(symbols) => symbols,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send prepare rename cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send prepare rename cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send prepare rename interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send prepare rename error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send prepare rename error.",
                    );

                    break;
                }
            };

            let Some(symbol) = symbols.first() else {
                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    0,
                    "nothing to rename in this position",
                );

                break;
            };

            let text = module_read_guard.text();

            let mut analyzer = RenameAnalyzer {
                read: &module_read_guard,
                edits: AHashSet::new(),
                error: "",
            };

            let origin = match analyzer.prepare_rename(symbol) {
                Ok(origin) => origin,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send prepare rename cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send prepare rename cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send prepare rename interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send prepare rename error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send prepare rename error.",
                    );

                    break;
                }
            };

            if !analyzer.error.is_empty() {
                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    0,
                    String::from(analyzer.error),
                );

                break;
            };

            let Some(span) = origin.to_position_span(&text) else {
                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    0,
                    "nothing to rename in this position",
                );

                break;
            };

            let range = span_to_range(&span);

            self.outgoing.send_ok_response::<PrepareRenameRequest>(
                &self.latches,
                message.id,
                Some(PrepareRenameResponse::Range(range)),
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
pub(super) struct SendLinkedEditingRange {
    pub(super) latches: RpcLatches,
    pub(super) outgoing: RpcSender,
    pub(super) module: LspModule,
}

impl Task for SendLinkedEditingRange {
    const EXECUTION: TaskExecution = TaskExecution::ExecuteEach;

    type Config = Self;

    type Message = SendLinkedEditingRangeMessage;

    #[inline(always)]
    fn init(config: Self::Config) -> Self {
        config
    }

    fn handle(&mut self, message: Self::Message) -> bool {
        loop {
            if message.cancel.is_active() {
                warn!(target: LSP_SERVER_LOG, "[{}] Send linked editing range cancelled by the client.", message.uri.as_str());

                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_CANCELLED,
                    "Send linked editing range cancelled by the client.",
                );

                break;
            }

            let handle = LspHandle::new(&message.cancel);

            let module_read_guard = match self.module.as_ref().read(&handle, ANALYSIS_PRIORITY) {
                Ok(guard) => guard,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send linked editing range cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send linked editing range cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send linked editing range interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send linked editing range error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send linked editing range error.",
                    );

                    break;
                }
            };

            let position = lsp_position_to_ld(&message.position);

            let symbols = match module_read_guard.symbols(
                position..position,
                LookupOptions::new().filter(RENAME_SYMBOLS),
            ) {
                Ok(symbols) => symbols,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send linked editing range cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send linked editing range cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send linked editing range interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send linked editing range error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send linked editing range error.",
                    );

                    break;
                }
            };

            let Some(symbol) = symbols.first() else {
                self.outgoing.send_ok_response::<LinkedEditingRange>(
                    &self.latches,
                    message.id,
                    None,
                );

                break;
            };

            let text = module_read_guard.text();

            let mut analyzer = RenameAnalyzer {
                read: &module_read_guard,
                edits: AHashSet::new(),
                error: "",
            };

            match analyzer.rename(symbol, "") {
                Ok(()) => (),

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send linked editing range cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send linked editing range cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send linked editing range interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send linked editing range error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send linked editing range error.",
                    );

                    break;
                }
            };

            let mut ranges = Vec::with_capacity(analyzer.edits.len());

            for edit in analyzer.edits {
                let Some(span) = edit.to_position_span(&text) else {
                    continue;
                };

                let range = span_to_range(&span);

                ranges.push(range);
            }

            self.outgoing.send_ok_response::<LinkedEditingRange>(
                &self.latches,
                message.id,
                match ranges.is_empty() {
                    true => None,
                    false => Some(LinkedEditingRanges {
                        ranges,
                        word_pattern: None,
                    }),
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

pub(super) struct SendRenameMessage {
    pub(super) id: RpcId,
    pub(super) uri: Uri,
    pub(super) cancel: Trigger,
    pub(super) new_name: String,
    pub(super) position: Position,
}

pub(super) struct SendPrepareRenameMessage {
    pub(super) id: RpcId,
    pub(super) uri: Uri,
    pub(super) cancel: Trigger,
    pub(super) position: Position,
}

pub(super) struct SendLinkedEditingRangeMessage {
    pub(super) id: RpcId,
    pub(super) uri: Uri,
    pub(super) cancel: Trigger,
    pub(super) position: Position,
}

struct RenameAnalyzer<'a> {
    read: &'a ModuleReadGuard<'a, LspHandle>,
    edits: AHashSet<ScriptOrigin>,
    error: &'static str,
}

impl<'a> RenameAnalyzer<'a> {
    fn prepare_rename(&mut self, symbol: &ModuleSymbol) -> ModuleResult<ScriptOrigin> {
        match symbol {
            ModuleSymbol::Nil => self.error = "nothing to rename in this position",
            ModuleSymbol::Use(_) => self.error = "cannot rename keyword",
            ModuleSymbol::Package(_) => self.error = "cannot rename import package",
            ModuleSymbol::Var(symbol) => return Ok(symbol.origin(self.read)),
            ModuleSymbol::Loop(_) => self.error = "cannot rename keyword",
            ModuleSymbol::Break(_) => self.error = "cannot rename keyword",
            ModuleSymbol::Fn(_) => self.error = "cannot rename keyword",
            ModuleSymbol::Return(_) => self.error = "cannot rename keyword",
            ModuleSymbol::Struct(_) => self.error = "cannot rename keyword",
            ModuleSymbol::Array(_) => self.error = "nothing to rename in this position",
            ModuleSymbol::Entry(symbol) => return Ok(symbol.origin(self.read)),
            ModuleSymbol::Ident(symbol) => return self.prepare_rename_ident(symbol),
            ModuleSymbol::Field(symbol) => return self.prepare_rename_field(symbol),
            ModuleSymbol::Literal(_) => self.error = "literals cannot be renamed",
            ModuleSymbol::Operator(_) => self.error = "operators cannot be renamed",
            ModuleSymbol::Call(_) => self.error = "nothing to rename in this position",
            ModuleSymbol::Index(_) => self.error = "nothing to rename in this position",
        }

        Ok(ScriptOrigin::nil())
    }

    fn prepare_rename_ident(&mut self, symbol: &IdentSymbol) -> ModuleResult<ScriptOrigin> {
        match symbol.kind(self.read)? {
            IdentKind::Invalid | IdentKind::VarAccess | IdentKind::VarDefinition => (),

            IdentKind::CrateAccess | IdentKind::PackageAccess => {
                self.error = "cannot rename an entity from a package"
            }

            IdentKind::CrateIdent | IdentKind::SelfIdent => {
                self.error = "cannot rename special variable";
            }
        }

        Ok(symbol.origin(self.read))
    }

    fn prepare_rename_field(&mut self, symbol: &FieldSymbol) -> ModuleResult<ScriptOrigin> {
        if symbol.declaration(self.read)?.is_none() {
            self.error = "this field is not declared in the script struct"
        }

        Ok(symbol.origin(self.read))
    }

    fn rename(&mut self, symbol: &ModuleSymbol, new_name: &str) -> ModuleResult<()> {
        match symbol {
            ModuleSymbol::Nil => self.error = "nothing to rename in this position",
            ModuleSymbol::Use(_) => self.error = "cannot rename keyword",
            ModuleSymbol::Package(_) => self.error = "cannot rename import package",
            ModuleSymbol::Var(symbol) => self.rename_var(symbol, new_name)?,
            ModuleSymbol::Loop(_) => self.error = "cannot rename keyword",
            ModuleSymbol::Break(_) => self.error = "cannot rename keyword",
            ModuleSymbol::Fn(_) => self.error = "cannot rename keyword",
            ModuleSymbol::Return(_) => self.error = "cannot rename keyword",
            ModuleSymbol::Struct(_) => self.error = "cannot rename keyword",
            ModuleSymbol::Array(_) => self.error = "nothing to rename in this position",
            ModuleSymbol::Entry(symbol) => self.rename_entry(symbol, new_name)?,
            ModuleSymbol::Ident(symbol) => self.rename_ident(symbol, new_name)?,
            ModuleSymbol::Field(symbol) => self.rename_field(symbol, new_name)?,
            ModuleSymbol::Literal(_) => self.error = "literals cannot be renamed",
            ModuleSymbol::Operator(_) => self.error = "operators cannot be renamed",
            ModuleSymbol::Call(_) => self.error = "nothing to rename in this position",
            ModuleSymbol::Index(_) => self.error = "nothing to rename in this position",
        }

        Ok(())
    }

    fn rename_var(&mut self, symbol: &VarSymbol, new_name: &str) -> ModuleResult<()> {
        self.check_new_ident_name(new_name);

        let _ = self.edits.insert(symbol.origin(self.read));

        let refs = symbol.references(self.read)?;

        for var_ref in refs {
            let var_ref = match var_ref {
                VarRef::Access(var_ref) => var_ref,
                VarRef::Definition(var_ref) => var_ref,
            };

            let _ = self.edits.insert(var_ref.origin(self.read));
        }

        Ok(())
    }

    fn rename_entry(&mut self, symbol: &EntrySymbol, new_name: &str) -> ModuleResult<()> {
        self.check_new_entry_key_name(new_name);

        let _ = self.edits.insert(symbol.origin(self.read));

        if let Some(struct_symbol) = symbol.struct_symbol(self.read) {
            for entry in struct_symbol.entries(self.read) {
                if &entry == symbol {
                    continue;
                }

                let Some(name) = entry.name(self.read) else {
                    continue;
                };

                if name == new_name {
                    self.error = "duplicate struct field name";
                }
            }
        }

        let _ = self.edits.insert(symbol.origin(self.read));

        let refs = symbol.references(self.read)?;

        for field_ref in refs {
            let _ = self.edits.insert(field_ref.origin(self.read));
        }

        Ok(())
    }

    fn rename_ident(&mut self, symbol: &IdentSymbol, new_name: &str) -> ModuleResult<()> {
        let _ = self.edits.insert(symbol.origin(self.read));

        match symbol.kind(self.read)? {
            IdentKind::Invalid => self.check_new_ident_name(new_name),

            IdentKind::CrateAccess | IdentKind::PackageAccess => {
                self.error = "cannot rename an entity from a package"
            }

            IdentKind::VarAccess | IdentKind::VarDefinition => {
                match symbol.declaration(self.read)? {
                    ModuleSymbol::Var(decl) => self.rename_var(&decl, new_name)?,
                    _ => self.check_new_ident_name(new_name),
                }
            }

            IdentKind::CrateIdent | IdentKind::SelfIdent => {
                self.error = "cannot rename special variable";
            }
        }

        Ok(())
    }

    fn rename_field(&mut self, symbol: &FieldSymbol, new_name: &str) -> ModuleResult<()> {
        let _ = self.edits.insert(symbol.origin(self.read));

        match symbol.declaration(self.read)? {
            Some(entry_symbol) => self.rename_entry(&entry_symbol, new_name)?,
            _ => self.error = "this field is not declared in the script struct",
        }

        Ok(())
    }

    fn check_new_ident_name(&mut self, new_name: &str) {
        if new_name.is_empty() {
            self.error = "identifier's name cannot be empty";
            return;
        }

        let mut first = true;

        for ch in new_name.chars() {
            match first {
                true => {
                    if ch.is_ascii_digit() {
                        self.error = "identifier's name cannot start with digit";
                        return;
                    }

                    if !ch.is_ascii_alphabetic() && ch != '_' {
                        self.error = "identifier's name must start with ascii \
                        alphabetic char or '_' char";
                        return;
                    }

                    first = false
                }

                false => {
                    if !ch.is_ascii_alphanumeric() && ch != '_' {
                        self.error = "identifier's name must contain ascii \
                        alphabetic, numeric, or '_' chars only";
                        return;
                    }
                }
            }
        }
    }

    fn check_new_entry_key_name(&mut self, new_name: &str) {
        if new_name.is_empty() {
            self.error = "entry's name cannot be empty";
            return;
        }

        let mut first = true;
        let mut numeric = false;

        for ch in new_name.chars() {
            match first {
                true => {
                    if ch.is_ascii_digit() {
                        numeric = true;
                        first = false;
                        continue;
                    }

                    if !ch.is_ascii_alphabetic() && ch != '_' {
                        self.error = "entry's name must contain ascii alphabetic, \
                        numeric, or '_' chars only";
                        return;
                    }

                    first = false
                }

                false => match numeric {
                    true => {
                        if !ch.is_ascii_digit() {
                            self.error =
                                "numeric entry names must contain ascii numeric chars only";
                            return;
                        }
                    }

                    false => {
                        if !ch.is_ascii_alphanumeric() && ch != '_' {
                            self.error = "entry's name must contain ascii \
                            alphabetic, numeric, or '_' chars only";

                            return;
                        }
                    }
                },
            }
        }
    }
}
