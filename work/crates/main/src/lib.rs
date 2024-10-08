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

//TODO check warnings regularly
#![allow(warnings)]

//! # Ad Astra API Documentation
//!
//! Ad Astra is a configurable scripting language platform designed for
//! embedding in Rust applications.
//!
//! This documentation provides formal API descriptions. For a general
//! exploration of the Ad Astra language and API usage, please refer to
//! [The Ad Astra Book](https://ad-astra.lakhin.com/).
//!
//! Getting started examples are available in the
//! [GitHub repository](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples).
//!
//! ## Requirements
//!
//! ### Desktop Builds
//!
//! For desktop builds of the script engine, the engine uses link sections to
//! export introspection metadata of the exported Rust code into the
//! script environment.
//!
//! This currently only works with the LLD linker, which is the default in
//! stable Rust releases but optional in unstable releases. For the unstable
//! Rust compiler, you need to reconfigure the linker manually.
//!
//! One way to configure the linker is to add the following configuration to
//! the `.cargo/config.toml` file of your Rust project:
//!
//! ```toml
//! [target.x86_64-unknown-linux-gnu]
//! rustflags=["-Zlinker-features=-lld"]
//! rustdocflags=["-Zlinker-features=-lld"]
//! ```
//!
//! ### WebAssembly Builds
//!
//! The Ad Astra crate supports the WebAssembly build target, but the linker is
//! not available for `wasm32-unknown-unknown` builds. To work around this
//! issue, the exporting system generates "hidden" registration functions that
//! start with the `__ADASTRA_EXPORT_` prefix.
//!
//! You need to manually call these functions before using the loaded wasm
//! module in JavaScript:
//!
//! ```javascript
//! // Loading a WebAssembly file.
//! const assembly = fetch('./wasm.wasm');
//!
//! // Compiling the file in the browser.
//! WebAssembly.instantiateStreaming(assembly, IMPORTS).then(({instance}) => {
//!     // Calling each module-exported function that starts with the special
//!     // `__ADASTRA_EXPORT_` prefix.
//!     //
//!     // These functions are generated by the Export macro.
//!     // By invoking them manually, you register the corresponding item's
//!     // introspection metadata in the Ad Astra script engine's export registry.
//!     for (const property in instance.exports) {
//!         if (property.startsWith('__ADASTRA_EXPORT_')) {
//!             instance.exports[property]();
//!         }
//!     }
//!
//!     // The module is now ready for use.
//! });
//! ```
//!
//! ## Feature Flags
//!
//! - `export` flag: Enabled by default. Disabling this flag prevents the
//!   generation of output code by the `#[export]` attribute macro.
//! - `shallow` flag: Disabled by default. When both the `export` and `shallow`
//!   features are enabled, the `#[export]` macro generates the necessary traits
//!   with dummy implementations. This mode is used for API development purposes
//!   when you don't need to run the script engine, as the Rust compiler
//!   processes the source code much faster with dummy implementations.
//! - `lsp` flag: Enabled by default. When this feature is disabled, the
//!   `server` module of this crate is not available.
//!
//! ## Quick Links
//!
//! - [GitHub Repository](https://github.com/Eliah-Lakhin/ad-astra)
//! - [API Documentation](https://docs.rs/ad-astra)
//! - [Main Crate](https://crates.io/crates/ad-astra)
//! - [Guide Book](https://ad-astra.lakhin.com)
//! - [Examples](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples)
//! - [Playground](https://ad-astra.lakhin.com/playground.html)
//!
//! ## Copyright
//!
//! This work is proprietary software with source-available code.
//!
//! To copy, use, distribute, or contribute to this work, you must agree to the
//! terms and conditions of the
//! [General License Agreement](https://github.com/Eliah-Lakhin/ad-astra/blob/master/EULA.md).
//!
//! For an explanation of the licensing terms, see the
//! [F.A.Q.](https://github.com/Eliah-Lakhin/ad-astra/tree/master/FAQ.md)
//!
//! Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин). All rights reserved.

/// Creation, editing, and incremental analysis of script modules.
///
/// This API allows you to load scripts from disk, print their source code
/// diagnostics (errors and warnings) to the terminal, and compile these modules
/// into Ad Astra VM assembly ready for execution.
///
/// Additionally, the analysis module provides a low-level API for developing
/// your own source code analysis tools from scratch. The script module objects
/// created with this API are editable, and they include methods for querying
/// individual semantic features of the source code. For example, you can
/// request the type of a variable in the source code or find all usages of
/// identifiers across the code.
///
/// For further details, see the [ScriptModule](analysis::ScriptModule)
/// documentation.
///
/// ## Script Modules Creation
///
/// ```rust
/// # use ad_astra::{
/// #     analysis::ScriptModule, export, lady_deirdre::analysis::TriggerHandle,
/// #     runtime::ScriptPackage,
/// # };
/// #
/// // To instantiate modules, you must declare a script package for your crate.
/// //
/// // The script analysis and execution occur within the package environment,
/// // which keeps track of all Rust items you make available for the script
/// // runtime.
/// //
/// // Usually, you declare the Package object in the lib.rs or main.rs file of
/// // your crate.
/// #[export(package)]
/// #[derive(Default)]
/// struct Package;
///
/// let _module = ScriptModule::<TriggerHandle>::new(
///     // A reference to the package under which the source code will be analyzed.
///     Package::meta(),
///     // The source code text. This could be text that you load from disk.
///     "let foo = 10;",
/// );
/// ```
///
/// ## Source Code Diagnostics
///
/// ```rust
/// # use ad_astra::{
/// #     analysis::{IssueSeverity, ModuleRead, ScriptModule},
/// #     export,
/// #     lady_deirdre::analysis::TriggerHandle,
/// #     runtime::ScriptPackage,
/// # };
/// #
/// # #[export(package)]
/// # #[derive(Default)]
/// # struct Package;
/// #
/// let module = ScriptModule::new(Package::meta(), "let foo = ;");
///
/// // First, you need to access the module for reading (you cannot read and
/// // write to the module at the same time).
/// //
/// // A handle object is required. Using this object, you can signal the
/// // read part to interrupt this job from another thread where you want to
/// // write to the module.
/// let handle = TriggerHandle::new();
/// let module_read = module.read(&handle, 1).unwrap();
///
/// // Diagnostics Level 1 -- all syntax errors.
/// // Diagnostics Level 2 -- superficial semantic errors and warnings.
/// // Diagnostics Level 3 -- deep analysis for semantic warnings.
/// let diagnostics_1 = module_read.diagnostics(1 /* all syntax errors */).unwrap();
///
/// // To print errors you need access to the module's source code text.
/// let module_text = module_read.text();
///
/// println!(
///     "{}",
///     diagnostics_1.highlight(
///         &module_text,
///         IssueSeverity::Error as u8, /* print error messages only  */
///     ),
/// );
/// ```
///
/// ## Changing Source Code Text
///
/// ```rust
/// # use ad_astra::{
/// #     analysis::{ModuleRead, ModuleWrite, ScriptModule},
/// #     export,
/// #     lady_deirdre::{
/// #         analysis::TriggerHandle,
/// #         lexis::{Position, SourceCode},
/// #     },
/// #     runtime::ScriptPackage,
/// # };
/// #
/// # #[export(package)]
/// # #[derive(Default)]
/// # struct Package;
/// #
/// let module = ScriptModule::new(Package::meta(), "let foo = 10;");
///
/// // Access the module for writing.
/// let handle = TriggerHandle::new();
/// let mut module_write = module.write(&handle, 1).unwrap();
///
/// // Change the text on line 1, columns 11 to 13 (exclusive).
/// module_write
///     .edit(Position::new(1, 11)..Position::new(1, 13), "20")
///     .unwrap();
///
/// let module_text = module_write.text();
///
/// assert_eq!(module_text.substring(..), "let foo = 20;");
/// ```
///
/// ## Query Symbol Semantics
///
/// ```rust
/// # use ad_astra::{
/// #     analysis::{
/// #         symbols::{LookupOptions, ModuleSymbol},
/// #         ModuleRead,
/// #         ScriptModule,
/// #     },
/// #     export,
/// #     lady_deirdre::{analysis::TriggerHandle, lexis::Position},
/// #     runtime::ScriptPackage,
/// # };
/// #
/// # #[export(package)]
/// # #[derive(Default)]
/// # struct Package;
/// #
/// let module = ScriptModule::new(Package::meta(), "let foo = 10; let bar = foo;");
///
/// let handle = TriggerHandle::new();
/// let module_read = module.read(&handle, 1).unwrap();
///
/// // Reads symbol(s) in the specified source code span.
/// let symbols = module_read
///     .symbols(
///         Position::new(1, 19)..Position::new(1, 22), // "bar"
///         LookupOptions::default(), // You can filter specific symbol types.
///     )
///     .unwrap();
///
/// let ModuleSymbol::Var(var_symbol) = symbols.first().unwrap() else {
///     panic!();
/// };
///
/// let var_type = var_symbol.var_type(&module_read).unwrap().type_hint;
///
/// assert_eq!("number", var_type.to_string());
/// ```
pub mod analysis;

mod exports;

/// Scripts formatting and printing to the terminal.
///
/// This module contains two useful components:
///
/// 1. A source code formatting algorithm (available through the
///    [format_script_text](format::format_script_text) function). This
///    function applies canonical formatting rules to source code text
///    written in the Ad Astra language.
///
/// 2. The [ScriptSnippet](format::ScriptSnippet) object, which allows you
///    to print snippets with syntax highlighting and annotated fragments of
///    Ad Astra source code to the terminal.
pub mod format;

/// Ad Astra Virtual Machine.
///
/// The primary object of this module is the [ScriptFn](interpret::ScriptFn),
/// which contains the compiled assembly of the source code ready for execution.
/// You create this object using the [compile](analysis::ModuleRead::compile)
/// function.
///
/// You can find more information about the internal design of the Virtual
/// Machine in the ScriptFn API documentation.
pub mod interpret;

mod report;

/// Building blocks of the script evaluation runtime.
///
/// This module provides both high-level and low-level APIs related to the
/// script execution engine, memory management, and the export system:
///
/// - The [ScriptPackage](runtime::ScriptPackage) and
///   [PackageMeta](runtime::PackageMeta) interfaces describe the package of
///   your crate into which the export system exports introspection metadata
///   of the Rust code related to your crate.
///
/// - The [Cell](runtime::Cell) object provides a generic interface to interact
///   with the memory managed by the Script Engine.
///
/// - The [TypeMeta](runtime::TypeMeta), [Prototype](runtime::Prototype), and
///   other related interfaces expose introspection information about the Rust
///   types known to the script engine.
///
/// - The [Origin](runtime::Origin), [Ident](runtime::Ident), and other related
///   objects provide ways to address both Rust and Script source code text
///   ranges.
///
/// - The [runtime::ops] submodule provides interfaces for low-level modeling of
///   Rust code exporting.
pub mod runtime;

mod semantics;

/// Built-in language server for code editors that support the LSP protocol.
///
/// This module is available under the `lsp` feature of the crate, which is
/// enabled by default.
///
/// ## LSP Protocol Overview
///
/// The language server is a standalone program that is typically bundled with
/// a language extension for the code editor. Usually, the extension (acting as
/// a "client") runs this program and then connects to it, for example, through
/// an IO channel or a TCP socket.
///
/// Once the connection is established, the client and server communicate via
/// the [LSP](https://microsoft.github.io/language-server-protocol/) protocol,
/// which consists of identifiable two-way messages and one-way notifications.
///
/// The communication session is an ongoing process. During the initial setup
/// phase, the client and server negotiate which code editor features they
/// support. The session ends when the user closes the editor; in this case,
/// the client either sends a special exit signal to the server or simply
/// terminates the language server process.
///
/// During normal operation (after initialization and before session closing),
/// the client notifies the server about changes in the source code of the
/// project files being developed by the user. This includes events like files
/// being opened or closed, and changes to the text of opened files.
/// The server, in turn, creates and maintains an internal representation of the
/// files open in the editor, including their text, syntax, and semantics.
///
/// The client periodically queries the server for information about the opened
/// files. For example, the client might request type hints for the code
/// currently visible in the editor's window. If the user starts typing, the
/// client may ask the server for code completion suggestions. If the user
/// hovers over a variable, the client queries the server for all semantically
/// related usages of that variable, and so on.
///
/// Additionally, the server performs background tasks such as analyzing the
/// source code for syntax and semantic errors and warnings, and periodically
/// sends diagnostic information to the client.
///
/// Overall, the purpose of the language server is to provide the editor with
/// useful information about the evolving codebase, which the editor uses to
/// assist the user in developing the source code.
///
/// ## Setup
///
/// Typically, you run the language server from the main function using
/// the [LspServer::startup](server::LspServer::startup) function, providing
/// the server configuration objects and a reference to the
/// [Package](runtime::PackageMeta) under which the server should analyze
/// the source code.
///
/// ```no_run
/// # use std::{net::SocketAddr, str::FromStr};
/// #
/// # use ad_astra::{
/// #     export,
/// #     runtime::ScriptPackage,
/// #     server::{LspLoggerConfig, LspServer, LspServerConfig, LspTransportConfig},
/// # };
/// #
/// #[export(package)]
/// #[derive(Default)]
/// struct Package;
///
/// let server_config = LspServerConfig::new();
///
/// let logger_config = LspLoggerConfig::new();
///
/// let transport_config =
///     LspTransportConfig::TcpServer(SocketAddr::from_str("127.0.0.1:8081").unwrap());
///
/// LspServer::startup(
///     // General server and LSP features configuration.
///     server_config,
///     // Server-side and client-side logger configuration.
///     logger_config,
///     // Transport configuration between the server and the editor (client).
///     transport_config,
///     // Script package under which the opened files will be analyzed.
///     Package::meta(),
/// );
/// ```
///
/// This startup function creates the server and establishes a communication
/// session with the client. When the communication session ends, the function
/// returns, and typically the server process terminates at this stage as well.
///
/// For the communication channel, most editors in production prefer the STD-IO
/// channel rather than the TCP socket. However, in debug mode, a TCP socket
/// can be more convenient as it provides an easier way to restart the server
/// and the client.
///
/// ## WebAssembly
///
/// The Ad Astra crate (including the LSP server) supports WebAssembly (wasm)
/// build targets, allowing you to potentially run the LSP server directly in a
/// web browser.
///
/// However, this setup requires additional effort to establish the
/// communication channel between the wasm module and the client.
///
/// First, you need to disable the multi-threaded mode of the LSP server
/// by setting the `multi_thread` option of the
/// [LspServerConfig](server::LspServerConfig) to false, as wasm builds
/// typically do not support multi-threading.
///
/// Next, create the server object manually using the
/// [LspServer::new](server::LspServer::new) constructor. By manually
/// instantiating the server, communication is not automatically established.
/// Instead, the server provides manual functions to handle incoming and
/// outgoing messages.
///
/// The [LspServer::handle](server::LspServer::handle) function accepts
/// incoming [RpcMessage](server::RpcMessage) objects from the client. You can
/// deserialize these objects from a byte array or a UTF-8 encoded string
/// using [RpcMessage::from_input_bytes](server::RpcMessage::from_input_bytes).
///
/// To send outgoing messages from the server to the client, you should create
/// a Rust standard channel for outgoing RPC messages using
/// [RpcMessage::channel](server::RpcMessage::channel). One end of the
/// channel is passed to the server's constructor, while the other end
/// is used to read pending messages that the server wants to send to the client.
///
/// ```no_run
/// use ad_astra::{
///     export,
///     runtime::ScriptPackage,
///     server::{LspServer, LspServerConfig, RpcMessage},
/// };
///
/// #[export(package)]
/// #[derive(Default)]
/// struct Package;
///
/// let mut server_config = LspServerConfig::new();
///
/// server_config.multi_thread = false;
///
/// let (outgoing_sender, outgoing_receiver) = RpcMessage::channel();
///
/// let mut server = LspServer::new(server_config, Package::meta(), outgoing_sender);
///
/// loop {
///     let in_message = RpcMessage::from_input_bytes(
///         // Should be a message received from the wasm host (from the client).
///         &[],
///     )
///     .unwrap();
///
///     if in_message.is_exit() {
///         return;
///     }
///
///     server.handle(in_message);
///
///     while let Ok(out_message) = outgoing_receiver.try_recv() {
///         let out_bytes = out_message.to_output_bytes().unwrap();
///
///         // Send the out_bytes back to the wasm host (the client).
///     }
/// }
/// ```
#[cfg(feature = "lsp")]
pub mod server;
mod syntax;

extern crate self as ad_astra;

pub use ad_astra_export::export;
pub use lady_deirdre;
