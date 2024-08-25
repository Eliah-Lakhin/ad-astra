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

use std::{net::SocketAddr, time::Duration};

use log::{Level, LevelFilter};
use lsp_types::{ClientCapabilities, MarkupKind};

/// A general configuration object for the Language Server.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub struct LspServerConfig {
    /// If set to true, the server's job manager will use dedicated threads.
    /// Otherwise, all LSP messages will be handled by a single thread.
    ///
    /// In general, multi-threading mode improves the server's performance.
    ///
    /// The default value is true unless the build target is a wasm target.
    /// For wasm targets, the default value is false.
    pub multi_thread: bool,

    /// If set to true, the server provides script running functionality in
    /// the code editor.
    ///
    /// The script runner also requires the `capabilities.code_lens`,
    /// `capabilities.execute_command`, and `multi_thread` flags to be enabled.
    ///
    /// The default value of this option is true if the build target is not a
    /// wasm target; otherwise, the default value is false.
    pub scripts_runner: bool,

    /// If specified, the server spawns a dedicated thread that periodically
    /// checks the health of other tasks. If a worker thread is not responding,
    /// the server attempts to cancel its task and respawn the thread.
    ///
    /// This option requires the `multi_thread` flag to be enabled.
    ///
    /// By default, the `health_check` feature is enabled and set to 5 seconds
    /// for non-wasm build targets. Otherwise, the feature is disabled.
    pub health_check: Option<Duration>,

    /// The name of the script language.
    ///
    /// The default value is "adastra", but you can rename the language using
    /// this option.
    pub language_id: &'static str,

    /// The extension of script files (e.g., "foo_file.<extension>").
    ///
    /// The language server filters files opened by the client based on their
    /// extension.
    ///
    /// The default value is "adastra", but you can change the file extension
    /// using this option.
    pub file_ext: &'static str,

    /// Configures the client-side and server-side logger.
    pub logger: LspLoggerConfig,

    /// Configures the LSP capabilities of the server.
    pub capabilities: LspCapabilities,
}

impl Default for LspServerConfig {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl LspServerConfig {
    /// The default constructor for this configuration object.
    #[inline(always)]
    pub const fn new() -> Self {
        let multi_thread;
        let scripts_runtime;
        let health_check;

        #[cfg(not(target_family = "wasm"))]
        {
            multi_thread = true;
            scripts_runtime = true;
            health_check = Some(Duration::from_secs(5));
        }

        #[cfg(target_family = "wasm")]
        {
            multi_thread = false;
            scripts_runtime = false;
            health_check = None;
        }

        Self {
            multi_thread,
            scripts_runner: scripts_runtime,
            health_check,
            language_id: "adastra",
            file_ext: "adastra",
            logger: LspLoggerConfig::new(),
            capabilities: LspCapabilities::new(),
        }
    }
}

/// An LSP capabilities configuration object for the Language Server.
///
/// By default, all flags are set to true.
///
/// Note that these features will only be available in the editor if both the
/// server and the client (the editor) support the corresponding capabilities.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub struct LspCapabilities {
    /// The server performs ongoing analysis for script errors and warnings,
    /// and periodically publishes diagnostic results to the client.
    pub publish_diagnostics: bool,

    /// When enabled, the editor shows inlay hints in the source code text that
    /// indicate value types and function signatures.
    pub inlay_hints: bool,

    /// When enabled, the editor supports script code formatting.
    pub formatting: bool,

    /// When enabled, the editor supports code completion features.
    pub completion: bool,

    /// When enabled, the completion menu uses a Markdown renderer for the
    /// description text related to completion suggestions.
    ///
    /// This capability will be ignored if the `completion` flag is disabled.
    pub completion_markdown: bool,

    /// When enabled, the completion menu includes snippets for common
    /// script language constructs.
    ///
    /// This capability will be ignored if the `completion` flag is disabled.
    pub completion_snippets: bool,

    /// When enabled, the editor shows hint text when the user moves the
    /// cursor over code symbols. For example, this might display RustDoc
    /// documentation related to a function or a field.
    pub hover: bool,

    /// When enabled, the editor uses a Markdown renderer for hover hints.
    ///
    /// This capability will be ignored if the `hover` flag is disabled.
    pub hover_markdown: bool,

    /// When enabled, the editor provides a jump-to-definition feature for
    /// code symbols. For example, this allows jumping to a variable's
    /// definition from a variable reference.
    pub goto_definition: bool,

    /// When enabled, the editor highlights related code symbols when the cursor
    /// touches them. For example, the editor will highlight all occurrences
    /// of a variable declaration and all references to that variable.
    pub document_highlight: bool,

    /// When enabled, the editor provides a jump-to-implementation feature for
    /// code symbols. For example, the user can jump from a function call to its
    /// implementation.
    pub goto_implementation: bool,

    /// When enabled, the editor provides quickfix refactoring suggestions to
    /// address some of the diagnostic messages.
    ///
    /// This feature also requires the `publish_diagnostics` capability to be
    /// enabled.
    pub code_action: bool,

    /// When enabled, the editor shows a function's signature description when
    /// the user attempts to call the function inside the script.
    pub signature_help: bool,

    /// When enabled, the signature helper uses a Markdown renderer to display
    /// the signature text.
    ///
    /// This capability will be ignored if the `signature_help` flag is
    /// disabled.
    pub signature_help_markdown: bool,

    /// When enabled, the editor allows renaming identifiers.
    pub rename: bool,

    /// When enabled, the renaming feature in the editor will be more
    /// interactive, highlighting related symbols that are subject to renaming.
    ///
    /// This capability will be ignored if the `rename` flag is disabled.
    pub rename_prepare: bool,

    /// When enabled, the editor automatically renames related identifiers as
    /// the user edits one of them, without requiring the user to enter renaming
    /// mode.
    pub linked_editing_range: bool,

    /// When enabled, the editor supports common code actions, such as running
    /// code.
    ///
    /// This capability and the `execute_command` capability are required for
    /// the script execution feature.
    pub code_lens: bool,

    /// When enabled, the editor supports common code actions, such as running
    /// code.
    ///
    /// This capability and the `code_lens` capability are required for
    /// the script execution feature.
    pub execute_command: bool,
}

impl Default for LspCapabilities {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl LspCapabilities {
    /// The default constructor that enables all server capabilities.
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            publish_diagnostics: true,
            inlay_hints: true,
            formatting: true,
            completion: true,
            completion_markdown: true,
            completion_snippets: true,
            hover: true,
            hover_markdown: true,
            goto_definition: true,
            document_highlight: true,
            goto_implementation: true,
            code_action: true,
            signature_help: true,
            signature_help_markdown: true,
            rename: true,
            rename_prepare: true,
            linked_editing_range: true,
            code_lens: true,
            execute_command: true,
        }
    }

    /// A constructor that disables all server capabilities.
    #[inline(always)]
    pub const fn none() -> Self {
        Self {
            publish_diagnostics: false,
            inlay_hints: false,
            formatting: false,
            completion: false,
            completion_markdown: false,
            completion_snippets: false,
            hover: false,
            hover_markdown: false,
            goto_definition: false,
            document_highlight: false,
            goto_implementation: false,
            code_action: false,
            signature_help: false,
            signature_help_markdown: false,
            rename: false,
            rename_prepare: false,
            linked_editing_range: false,
            code_lens: false,
            execute_command: false,
        }
    }

    pub(super) fn from_client(capabilities: &ClientCapabilities) -> Self {
        let mut result = Self::none();

        if let Some(text_document) = &capabilities.text_document {
            result.publish_diagnostics = text_document.publish_diagnostics.is_some();
            result.inlay_hints = text_document.inlay_hint.is_some();
            result.formatting = text_document.formatting.is_some();

            if let Some(completion) = &text_document.completion {
                result.completion = true;

                if let Some(completion_item) = &completion.completion_item {
                    if let Some(documentation_format) = &completion_item.documentation_format {
                        result.completion_markdown = documentation_format
                            .iter()
                            .any(|kind| kind == &MarkupKind::Markdown);
                    }

                    result.completion_snippets = completion_item
                        .snippet_support
                        .filter(|flag| *flag == true)
                        .is_some()
                }
            }

            if let Some(hover) = &text_document.hover {
                result.hover = true;

                if let Some(content_format) = &hover.content_format {
                    result.hover_markdown = content_format
                        .iter()
                        .any(|kind| kind == &MarkupKind::Markdown);
                }
            }

            result.goto_definition = text_document.definition.is_some();
            result.document_highlight = text_document.document_highlight.is_some();
            result.goto_implementation = text_document.implementation.is_some();
            result.code_action = text_document.code_action.is_some();

            if let Some(signature_help) = &text_document.signature_help {
                result.signature_help = true;

                if let Some(signature_information) = &signature_help.signature_information {
                    if let Some(documentation_format) = &signature_information.documentation_format
                    {
                        result.signature_help_markdown = documentation_format
                            .iter()
                            .any(|kind| kind == &MarkupKind::Markdown);
                    }
                }
            }

            if let Some(rename) = &text_document.rename {
                result.rename = true;
                result.rename_prepare = rename.prepare_support.filter(|flag| *flag).is_some()
            }

            result.linked_editing_range = text_document.linked_editing_range.is_some();
            result.code_lens = text_document.code_lens.is_some();
        }

        if let Some(workspace) = &capabilities.workspace {
            result.execute_command = workspace.execute_command.is_some();
        }

        result
    }

    #[inline(always)]
    pub(super) fn intersect(&mut self, other: Self) {
        self.publish_diagnostics = self.publish_diagnostics && other.publish_diagnostics;
        self.inlay_hints = self.inlay_hints && other.inlay_hints;
        self.formatting = self.formatting && other.formatting;
        self.completion = self.completion && other.completion;
        self.completion_markdown = self.completion_markdown && other.completion_markdown;
        self.completion_snippets = self.completion_snippets && other.completion_snippets;
        self.hover = self.hover && other.hover;
        self.hover_markdown = self.hover_markdown && other.hover_markdown;
        self.goto_definition = self.goto_definition && other.goto_definition;
        self.document_highlight = self.document_highlight && other.document_highlight;
        self.goto_implementation = self.goto_implementation && other.goto_implementation;
        self.code_action = self.code_action && other.code_action;
        self.signature_help = self.signature_help && other.signature_help;
        self.signature_help_markdown =
            self.signature_help_markdown && other.signature_help_markdown;
        self.rename = self.rename && other.rename;
        self.rename_prepare = self.rename_prepare && other.rename_prepare;
        self.linked_editing_range = self.linked_editing_range && other.linked_editing_range;
        self.code_lens = self.code_lens && other.code_lens;
        self.execute_command = self.execute_command && other.execute_command;
    }
}

/// A configuration of the communication channel between the server and
/// the client (editor).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub enum LspTransportConfig {
    /// Communication will occur through the STD-IO stream of the server's
    /// process.
    ///
    /// This is the default option, widely supported by the majority
    /// of code editors (clients).
    Stdio,

    /// The server will connect to the specified TCP socket, opened by the
    /// client.
    TcpClient(SocketAddr),

    /// The server will open the specified TCP socket, and the client will
    /// connect to this socket.
    TcpServer(SocketAddr),
}

impl Default for LspTransportConfig {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl LspTransportConfig {
    /// The default constructor for this enum.
    ///
    /// The default variant is `Stdio`, representing the communication stream
    /// of the server's process.
    #[inline(always)]
    pub const fn new() -> Self {
        Self::Stdio
    }
}

/// A configuration for the server and client loggers.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub struct LspLoggerConfig {
    /// If set to false, both client and server logging will be disabled,
    /// and the rest of the configuration options will be ignored.
    pub enabled: bool,

    /// The general logging level.
    ///
    /// In debug builds, the default option is [LevelFilter::Debug], while in
    /// production builds, the default value is [LevelFilter::Info].
    pub level: LevelFilter,

    /// Configuration of the log messages that will be shown in the editor.
    pub client: LspLoggerClientConfig,

    /// Configuration of the log messages that will be shown on the server side.
    ///
    /// Typically, the server logs also include client log messages. However,
    /// the server logs contain more detailed messages useful for debugging,
    /// which are less relevant for the editor's user.
    pub server: LspLoggerServerConfig,
}

impl Default for LspLoggerConfig {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl LspLoggerConfig {
    /// The default constructor for this configuration object.
    #[inline(always)]
    pub const fn new() -> Self {
        #[cfg(debug_assertions)]
        let level = LevelFilter::Debug;

        #[cfg(not(debug_assertions))]
        let level = LevelFilter::Info;

        Self {
            enabled: true,
            level,
            client: LspLoggerClientConfig::new(),
            server: LspLoggerServerConfig::new(),
        }
    }

    /// Sets the `enabled` flag to false, effectively disabling the loggers.
    #[inline(always)]
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::new()
        }
    }
}

/// A configuration for the client-side logger (editor logger).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub enum LspLoggerClientConfig {
    /// The server will not send any log messages to the client.
    Off,

    /// The log messages sent from the server to the client will be shown
    /// in the trace channel of the editor.
    Trace,

    /// The log messages sent from the server to the client will be shown
    /// in the debug window of the editor.
    ///
    /// This is the default variant.
    Window,
}

impl Default for LspLoggerClientConfig {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl LspLoggerClientConfig {
    /// The default constructor for this enum.
    ///
    /// The default variant is `Window`, meaning that log messages will be
    /// shown in the editor's debug window.
    #[inline(always)]
    pub const fn new() -> Self {
        Self::Window
    }
}

/// A configuration for the server-side logger.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub enum LspLoggerServerConfig {
    /// The server will not show any log messages on the server side.
    Off,

    /// The server prints log messages to the STDERR stream of the process.
    ///
    /// This is the default option.
    Stderr,

    /// The server prints log messages to the syslog. This option is only
    /// available on Unix operating systems.
    Syslog,

    /// A custom logger. The server-side log messages will be sent to the
    /// specified function.
    ///
    /// This feature is particularly useful for wasm builds of the server,
    /// intended to run in browser-based WASM modules.
    ///
    /// In WASM environments, general logging options are not available
    /// (WASM does not have an STDERR stream and does not have access to
    /// syslog). Using this function, you can set up a separate logging channel
    /// that sends log messages to the WASM host.
    Custom(fn(Level, String)),
}

impl Default for LspLoggerServerConfig {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl LspLoggerServerConfig {
    /// The default constructor for this enum.
    ///
    /// The default variant is `Stderr`, meaning that log messages will be
    /// sent to the STDERR stream of the server's process.
    #[inline(always)]
    pub const fn new() -> Self {
        return Self::Stderr;
    }
}
