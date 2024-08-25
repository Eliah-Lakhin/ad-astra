<!------------------------------------------------------------------------------
  This file is part of "Ad Astra", an embeddable scripting programming
  language platform.

  This work is proprietary software with source-available code.

  To copy, use, distribute, or contribute to this work, you must agree to
  the terms of the General License Agreement:

  https://github.com/Eliah-Lakhin/ad-astra/blob/master/EULA.md

  The agreement grants a Basic Commercial License, allowing you to use
  this work in non-commercial and limited commercial products with a total
  gross revenue cap. To remove this commercial limit for one of your
  products, you must acquire a Full Commercial License.

  If you contribute to the source code, documentation, or related materials,
  you must grant me an exclusive license to these contributions.
  Contributions are governed by the "Contributions" section of the General
  License Agreement.

  Copying the work in parts is strictly forbidden, except as permitted
  under the General License Agreement.

  If you do not or cannot agree to the terms of this Agreement,
  do not use this work.

  This work is provided "as is", without any warranties, express or implied,
  except where such disclaimers are legally invalid.

  Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).
  All rights reserved.
------------------------------------------------------------------------------->

# Language Server

The language server is a Rust program that provides semantic analysis metadata
for the code editor (the language client) through the
[LSP](https://microsoft.github.io/language-server-protocol/) protocol.

The LSP language server is an essential component of the code editor's language 
xtension plugin.

The Ad Astra crate includes a built-in LSP server that you can run using the
[LspServer::startup](https://docs.rs/ad-astra/1.0.0/ad_astra/server/struct.LspServer.html#method.startup)
function. This function performs all necessary preparations to establish
communication with the language client according to the specified configuration
and runs the actual server that assists the code editor user.

```rust,ignore
// General LSP server configuration features.
// Through this object, you can enable or disable certain LSP features.
// The default configuration suits the majority of practical needs.
let server_config = LspServerConfig::new();

// Sets up the server-side and client-side loggers.
//
// The client-side logs are end-user facing messages that will be shown
// in the editor's console (this may vary depending on the editor's user
// interface). Usually, these logs are less verbose and include only general
// messages about the server state.
//
// The server-side logs usually include client-side messages too, but they also
// include additional messages useful for server debugging.
//
// By default, the server uses the STDERR channel (because the STDIO channel may
// be used as the actual LSP communication transport between the server and the
// client). You can manually configure this option to redirect log messages to
// the Unix Syslog or to a custom function.
let logger_config = LspLoggerConfig::new();

// The LSP communication transport. The STDIO channel is the default option
// supported by the majority of code editors that support the LSP protocol.
let transport_config = LspTransportConfig::Stdio;

LspServer::startup(
    server_config,
    logger_config,
    transport_config,
    
    // A script package on behalf of which the files opened in the code editor
    // will be analyzed.
    Package::meta(),
);
```

Usually, you would implement the LSP server as a separate Rust executable
program and bundle it with the code editor extension. The editor would run this
program, establishing communication through the STDIO channel of the program's
process.

The [Language Server Setup](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/lsp-server)
example provides a sample setup of the language server.

In addition to the STDIO transport, some language clients also support TCP
communication transport, where the LSP server is started independently from the
client, and the client connects to the TCP port opened by the server (or by the
client).

This communication mode is less common than the STDIO transport but is more
useful for server debugging during the development of the code editor extension.
In particular, if you start the server's process manually in the terminal, you
will also see its STDERR debugging logs in the terminal.

The Ad Astra built-in server supports both types of communication transports.

The [Language Client Example](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/lsp-client)
demonstrates a VS Code extension that works with the Ad Astra LSP Server
through one of the transports, depending on the user's preference.
