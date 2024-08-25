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

use ahash::AHashMap;
use lady_deirdre::{
    lexis::{PositionSpan, ToSpan},
    sync::Trigger,
};
use log::{error, warn};
use lsp_types::{
    error_codes::{REQUEST_CANCELLED, REQUEST_FAILED},
    request::DocumentHighlightRequest,
    DocumentHighlight,
    DocumentHighlightKind,
    Position,
    Uri,
};

use crate::{
    analysis::{
        symbols::{
            BreakSymbol,
            EntrySymbol,
            FieldSymbol,
            FnSymbol,
            IdentKind,
            IdentSymbol,
            LookupOptions,
            LoopSymbol,
            ModuleSymbol,
            PackageSymbol,
            ReturnSymbol,
            StructSymbol,
            SymbolKind,
            UseSymbol,
            VarRef,
            VarSymbol,
        },
        ModuleError,
        ModuleRead,
        ModuleReadGuard,
        ModuleResult,
        ModuleText,
    },
    server::{
        file::{LspModule, ANALYSIS_PRIORITY},
        logger::LSP_SERVER_LOG,
        rpc::{LspHandle, OutgoingEx, RpcId, RpcLatches},
        tasks::{Task, TaskExecution, COOL_DOWN},
        utils::{lsp_position_to_ld, span_to_range},
        RpcSender,
    },
};

pub(super) struct SendDocumentHighlight {
    pub(super) latches: RpcLatches,
    pub(super) outgoing: RpcSender,
    pub(super) module: LspModule,
}

impl Task for SendDocumentHighlight {
    const EXECUTION: TaskExecution = TaskExecution::ExecuteEach;

    type Config = Self;

    type Message = SendDocumentHighlightMessage;

    #[inline(always)]
    fn init(config: Self::Config) -> Self {
        config
    }

    fn handle(&mut self, message: Self::Message) -> bool {
        loop {
            if message.cancel.is_active() {
                warn!(target: LSP_SERVER_LOG, "[{}] Send inlay hints cancelled by the client.", message.uri.as_str());

                self.outgoing.send_err_response(
                    &self.latches,
                    message.id,
                    REQUEST_CANCELLED,
                    "Send inlay hints cancelled by the client.",
                );

                break;
            }

            let handle = LspHandle::new(&message.cancel);

            let module_read_guard = match self.module.as_ref().read(&handle, ANALYSIS_PRIORITY) {
                Ok(guard) => guard,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send document highlight cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send document highlight cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send document highlight interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send document highlight error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send document highlight error.",
                    );

                    break;
                }
            };

            const HIGHLIGHT_SYMBOLS: u32 = (SymbolKind::Use as u32)
                | (SymbolKind::Package as u32)
                | (SymbolKind::Var as u32)
                | (SymbolKind::Loop as u32)
                | (SymbolKind::Break as u32)
                | (SymbolKind::Fn as u32)
                | (SymbolKind::Return as u32)
                | (SymbolKind::Struct as u32)
                | (SymbolKind::Entry as u32)
                | (SymbolKind::Ident as u32)
                | (SymbolKind::Field as u32);

            let position = lsp_position_to_ld(&message.position);

            let symbols = match module_read_guard.symbols(
                position..position,
                LookupOptions::new().filter(HIGHLIGHT_SYMBOLS),
            ) {
                Ok(symbols) => symbols,

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send document highlight cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send document highlight cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send document highlight interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send document highlight error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send document highlight error.",
                    );

                    break;
                }
            };

            let Some(symbol) = symbols.first() else {
                self.outgoing.send_ok_response::<DocumentHighlightRequest>(
                    &self.latches,
                    message.id,
                    None,
                );

                break;
            };

            let text = module_read_guard.text();

            let mut builder = HighlightsBuilder {
                read: &module_read_guard,
                text: &text,
                highlights: AHashMap::new(),
            };

            match builder.highlight_symbol(symbol) {
                Ok(()) => (),

                Err(ModuleError::Interrupted(_)) => {
                    if message.cancel.is_active() {
                        warn!(target: LSP_SERVER_LOG, "[{}] Send document highlight cancelled by the client.", message.uri.as_str());

                        self.outgoing.send_err_response(
                            &self.latches,
                            message.id,
                            REQUEST_CANCELLED,
                            "Send document highlight cancelled by the client.",
                        );

                        break;
                    }

                    warn!(target: LSP_SERVER_LOG, "[{}] Send document highlight interrupted.", message.uri.as_str());
                    park_timeout(COOL_DOWN);
                    continue;
                }

                Err(error) => {
                    error!(target: LSP_SERVER_LOG, "[{}] Send document highlight error. {error}", message.uri.as_str());

                    self.outgoing.send_err_response(
                        &self.latches,
                        message.id,
                        REQUEST_FAILED,
                        "Send document highlight error.",
                    );

                    break;
                }
            }

            let highlights = builder.highlights.into_values().collect::<Vec<_>>();

            self.outgoing.send_ok_response::<DocumentHighlightRequest>(
                &self.latches,
                message.id,
                match highlights.is_empty() {
                    true => None,
                    false => Some(highlights),
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

pub(super) struct SendDocumentHighlightMessage {
    pub(super) id: RpcId,
    pub(super) uri: Uri,
    pub(super) cancel: Trigger,
    pub(super) position: Position,
}

struct HighlightsBuilder<'a> {
    read: &'a ModuleReadGuard<'a, LspHandle>,
    text: &'a ModuleText<'a>,
    highlights: AHashMap<PositionSpan, DocumentHighlight>,
}

impl<'a> HighlightsBuilder<'a> {
    fn highlight_symbol(&mut self, symbol: &ModuleSymbol) -> ModuleResult<()> {
        match symbol {
            ModuleSymbol::Nil => Ok(()),
            ModuleSymbol::Use(symbol) => self.highlight_use(symbol),
            ModuleSymbol::Package(symbol) => self.highlight_package(symbol),
            ModuleSymbol::Var(symbol) => self.highlight_var(symbol),
            ModuleSymbol::Loop(symbol) => self.highlight_loop(symbol),
            ModuleSymbol::Break(symbol) => self.highlight_break(symbol),
            ModuleSymbol::Fn(symbol) => self.highlight_fn(symbol),
            ModuleSymbol::Return(symbol) => self.highlight_return(symbol),
            ModuleSymbol::Struct(symbol) => self.highlight_struct(symbol),
            ModuleSymbol::Array(_) => Ok(()),
            ModuleSymbol::Entry(symbol) => self.highlight_entry(symbol),
            ModuleSymbol::Ident(symbol) => self.highlight_ident(symbol),
            ModuleSymbol::Field(symbol) => self.highlight_field(symbol),
            ModuleSymbol::Literal(_) => Ok(()),
            ModuleSymbol::Operator(_) => Ok(()),
            ModuleSymbol::Call(_) => Ok(()),
            ModuleSymbol::Index(_) => Ok(()),
        }
    }

    fn highlight_use(&mut self, symbol: &UseSymbol) -> ModuleResult<()> {
        if symbol.resolution(self.read)?.is_none() {
            return Ok(());
        }

        let Some(package) = symbol.last_package(self.read) else {
            return Ok(());
        };

        let package_origin = package.origin(self.read);

        let Some(package_span) = package_origin.to_position_span(self.text) else {
            return Ok(());
        };

        self.insert(&package_span, DocumentHighlightKind::TEXT);

        let refs = symbol.all_references(self.read)?;

        for ident_symbol in refs {
            let ident_origin = ident_symbol.origin(self.read);

            let Some(ident_span) = ident_origin.to_position_span(self.text) else {
                continue;
            };

            self.insert(&ident_span, DocumentHighlightKind::TEXT);
        }

        Ok(())
    }

    fn highlight_package(&mut self, symbol: &PackageSymbol) -> ModuleResult<()> {
        let Some(use_symbol) = symbol.use_symbol(self.read) else {
            return Ok(());
        };

        self.highlight_use(&use_symbol)
    }

    fn highlight_var(&mut self, symbol: &VarSymbol) -> ModuleResult<()> {
        let var_origin = symbol.origin(self.read);

        let Some(var_span) = var_origin.to_position_span(self.text) else {
            return Ok(());
        };

        match symbol.let_value(self.read).is_nil() {
            true => self.insert(&var_span, DocumentHighlightKind::TEXT),
            false => self.insert(&var_span, DocumentHighlightKind::WRITE),
        };

        let refs = symbol.references(self.read)?;

        for var_ref in refs {
            let (ident_symbol, kind) = match var_ref {
                VarRef::Access(symbol) => (symbol, DocumentHighlightKind::READ),
                VarRef::Definition(symbol) => (symbol, DocumentHighlightKind::WRITE),
            };

            let ident_origin = ident_symbol.origin(self.read);

            let Some(ident_span) = ident_origin.to_position_span(self.text) else {
                continue;
            };

            self.insert(&ident_span, kind);
        }

        Ok(())
    }

    fn highlight_loop(&mut self, symbol: &LoopSymbol) -> ModuleResult<()> {
        let loop_origin = symbol.origin(self.read);

        let Some(loop_span) = loop_origin.to_position_span(self.text) else {
            return Ok(());
        };

        self.insert(&loop_span, DocumentHighlightKind::TEXT);

        let breaks = symbol.breaks(self.read)?;

        for break_symbol in breaks {
            let break_origin = break_symbol.origin(self.read);

            let Some(break_span) = break_origin.to_position_span(self.text) else {
                continue;
            };

            self.insert(&break_span, DocumentHighlightKind::TEXT);
        }

        Ok(())
    }

    fn highlight_break(&mut self, symbol: &BreakSymbol) -> ModuleResult<()> {
        let Some(loop_symbol) = symbol.loop_symbol(self.read)? else {
            return Ok(());
        };

        self.highlight_loop(&loop_symbol)
    }

    fn highlight_fn(&mut self, symbol: &FnSymbol) -> ModuleResult<()> {
        let fn_origin = symbol.origin(self.read);

        let Some(fn_span) = fn_origin.to_position_span(self.text) else {
            return Ok(());
        };

        self.insert(&fn_span, DocumentHighlightKind::TEXT);

        let returns = symbol.return_symbols(self.read)?;

        for return_symbol in returns {
            let return_origin = return_symbol.origin(self.read);

            let Some(return_span) = return_origin.to_position_span(self.text) else {
                continue;
            };

            self.insert(&return_span, DocumentHighlightKind::TEXT);
        }

        Ok(())
    }

    fn highlight_return(&mut self, symbol: &ReturnSymbol) -> ModuleResult<()> {
        let returns = symbol.fn_returns(self.read)?;

        for return_symbol in returns {
            let return_origin = return_symbol.origin(self.read);

            let Some(return_span) = return_origin.to_position_span(self.text) else {
                continue;
            };

            self.insert(&return_span, DocumentHighlightKind::TEXT);
        }

        let Some(fn_symbol) = symbol.fn_symbol(self.read)? else {
            return Ok(());
        };

        let fn_origin = fn_symbol.origin(self.read);

        let Some(fn_span) = fn_origin.to_position_span(self.text) else {
            return Ok(());
        };

        self.insert(&fn_span, DocumentHighlightKind::TEXT);

        Ok(())
    }

    fn highlight_struct(&mut self, symbol: &StructSymbol) -> ModuleResult<()> {
        let struct_origin = symbol.origin(self.read);

        let Some(struct_span) = struct_origin.to_position_span(self.text) else {
            return Ok(());
        };

        self.insert(&struct_span, DocumentHighlightKind::TEXT);

        let refs = symbol.references(self.read)?;

        for ref_symbol in refs {
            let origin = match ref_symbol {
                ModuleSymbol::Ident(symbol) => symbol.origin(self.read),
                ModuleSymbol::Field(symbol) => symbol.origin(self.read),
                _ => continue,
            };

            let Some(span) = origin.to_position_span(self.text) else {
                continue;
            };

            self.insert(&span, DocumentHighlightKind::READ);
        }

        Ok(())
    }

    fn highlight_entry(&mut self, symbol: &EntrySymbol) -> ModuleResult<()> {
        let entry_origin = symbol.origin(self.read);

        let Some(entry_span) = entry_origin.to_position_span(self.text) else {
            return Ok(());
        };

        self.insert(&entry_span, DocumentHighlightKind::WRITE);

        let refs = symbol.references(self.read)?;

        for field_symbol in refs {
            let field_origin = field_symbol.origin(self.read);

            let Some(field_span) = field_origin.to_position_span(self.text) else {
                continue;
            };

            self.insert(&field_span, DocumentHighlightKind::READ);
        }

        Ok(())
    }

    fn highlight_ident(&mut self, symbol: &IdentSymbol) -> ModuleResult<()> {
        match symbol.declaration(self.read)? {
            ModuleSymbol::Var(var_symbol) => self.highlight_var(&var_symbol),

            ModuleSymbol::Struct(struct_symbol) => {
                let struct_origin = struct_symbol.origin(self.read);

                let Some(struct_span) = struct_origin.to_position_span(self.text) else {
                    return Ok(());
                };

                self.insert(&struct_span, DocumentHighlightKind::TEXT);

                let refs = struct_symbol.references(self.read)?;

                for ref_symbol in refs {
                    let ModuleSymbol::Ident(ref_symbol) = ref_symbol else {
                        continue;
                    };

                    let IdentKind::SelfIdent = ref_symbol.kind(self.read)? else {
                        continue;
                    };

                    let ref_origin = ref_symbol.origin(self.read);

                    let Some(ref_span) = ref_origin.to_position_span(self.text) else {
                        return Ok(());
                    };

                    self.insert(&ref_span, DocumentHighlightKind::READ);
                }

                Ok(())
            }

            ModuleSymbol::Package(package_symbol) => {
                let package_origin = package_symbol.origin(self.read);

                let Some(package_span) = package_origin.to_position_span(self.text) else {
                    return Ok(());
                };

                self.insert(&package_span, DocumentHighlightKind::TEXT);

                let Some(use_symbol) = package_symbol.use_symbol(self.read) else {
                    return Ok(());
                };

                let Some(name) = symbol.name(self.read) else {
                    return Ok(());
                };

                let refs = use_symbol.references_by_name(self.read, &name)?;

                for ident_symbol in refs {
                    let ident_origin = ident_symbol.origin(self.read);

                    let Some(ident_span) = ident_origin.to_position_span(self.text) else {
                        continue;
                    };

                    self.insert(&ident_span, DocumentHighlightKind::READ);
                }

                Ok(())
            }

            _ => {
                let similar_symbols = symbol.similar_idents(self.read)?;

                for similar_symbol in similar_symbols {
                    let origin = similar_symbol.origin(self.read);

                    let Some(span) = origin.to_position_span(self.text) else {
                        continue;
                    };

                    self.insert(&span, DocumentHighlightKind::READ);
                }

                Ok(())
            }
        }
    }

    fn highlight_field(&mut self, symbol: &FieldSymbol) -> ModuleResult<()> {
        if let Some(decl) = symbol.declaration(self.read)? {
            return self.highlight_entry(&decl);
        };

        let similar_fields = symbol.similar_fields(self.read)?;

        for field_ref in similar_fields {
            let field_origin = field_ref.origin(self.read);

            let Some(field_span) = field_origin.to_position_span(self.text) else {
                continue;
            };

            self.insert(&field_span, DocumentHighlightKind::READ);
        }

        Ok(())
    }

    fn insert(&mut self, span: &PositionSpan, kind: DocumentHighlightKind) {
        if self.highlights.contains_key(span) {
            return;
        }

        let _ = self.highlights.insert(
            span.clone(),
            DocumentHighlight {
                range: span_to_range(span),
                kind: Some(kind),
            },
        );
    }
}
