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

# Ad Astra Examples

This directory contains example Ad Astra scripts, Rust API export example, and
typical setups that demonstrate various use cases of the Ad Astra API.

## Sample Scripts

The [`/scripts`](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/scripts)
directory contains Ad Astra scripts available for analysis and execution within
the provided setups.

- [`/scripts/algebra.adastra`](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/scripts/algebra.adastra)
  demonstrates the usage of exported Rust API within scripts.

- [`/scripts/collatz.adastra`](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/scripts/collatz.adastra)
  demonstrates Ad Astra control-flow constructs using the example of the
  [Collatz conjecture](https://en.wikipedia.org/wiki/Collatz_conjecture).

- [`/scripts/mutability.adastra`](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/scripts/mutability.adastra)
  demonstrates the data passing by reference feature in scripts.

- [`/scripts/closures.adastra`](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/scripts/closures.adastra)
  demonstrates functional programming capabilities.

- [`/scripts/structs.adastra`](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/scripts/structs.adastra)
  demonstrates object-oriented programming capabilities.

- [`/scripts/quicksort.adastra`](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/scripts/quicksort.adastra)
  demonstrates the implementation of the [Quicksort](https://en.wikipedia.org/wiki/Quicksort) algorithm.

## Exporting Example

Available for exploration in the
[`/exporting`](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/exporting)
directory.

This Rust library ("algebra") demonstrates how to export Rust APIs into a script
using the `Export` macro. The API is exported into the script package associated
with the crate.

The setup examples link to this crate in their Cargo.toml dependencies, allowing
the script user to access it via the `use algebra;` construction in Ad Astra.

## Runner Setup

Available for exploration in the
[`/runner`](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/runner)
directory.

This application compiles and runs a script file (by default, the
`/scripts/algebra.adastra` file). If the script contains diagnostic errors or
warnings, the program prints them to the terminal. By default, after the first
run, the application continuously watches for changes in the script file and
reruns the script when the user saves their changes.

Use the `$ ./runner-dbg.sh` and `$ ./runner-prod.sh` shell scripts to run this
Rust program in debug and production modes, respectively.

## Language Server Setup

Available for exploration in the
[`/lsp-server`](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/lsp-server)
directory.

The [LSP](https://microsoft.github.io/language-server-protocol/) language server
works in one of two modes:

- **STD-IO** mode: This is the default mode supported by most code editors.
  In this mode, the client (the editor) typically runs the LSP server program
  automatically and communicates with the server via the server's STD-IO channel.

- **TCP** mode: This mode is activated with the CLI argument `--tcp 127.0.0.1:8081`.
  In this mode, you run the server manually, and the client connects to the
  specified TCP port, which serves as the communication channel.
  Although supported by fewer editors, the TCP mode is more useful for debugging
  purposes.


## Language Client Example

Available for exploration in the
[`/lsp-client`](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/lsp-client)
directory.

This is an example language extension for
[Visual Studio Code](https://code.visualstudio.com/) that utilizes the LSP
language server.

This extension can work with the LSP server in both STD-IO and TCP modes
(configured via the `"adastra.lspServerMode"` setting in VS Code).

For demonstration purposes, the provided implementation runs the language server
through `cargo run` each time the server is started. In a production environment,
you would likely bundle a prebuilt executable with the plugin.

The recommended VS Code settings for the Ad Astra language extension can be
found in the [`./lsp-client/settings.json`](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/lsp-client/settings.json)
file.

To run this extension:

1. Open the directory in VS Code.
2. Ensure the path to the `lsp-server` directory is specified in the
   `"adastra.lspServerPath"` entry of the `settings.json` file.
3. Press F5 to launch the plugin. This will open a separate VS Code editor in
   debug mode with the attached plugin.
4. Open or create a file with the `.adastra` extension. The plugin recognizes
   these files as belonging to Ad Astra scripts and activates the plugin.

Note that upon activation, the plugin automatically compiles and runs the LSP
server via Cargo. On first activation, this may take some time as the plugin
builds the server.

Alternatively, you can launch the LSP server manually in TCP mode. In this case,
change the `"adastra.lspServerMode"` value in the settings.json file to `"TCP"`,
and set `"adastra.lspServerPort"` to match the port specified in the server's
CLI argument `--tcp 127.0.0.1:8081` (8081 in this example).

## WebAssembly Setup

Available for exploration in the
[`./wasm`](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/wasm)
directory.

The Ad Astra crate, including its built-in LSP server, can be built for the
WebAssembly target and launched in the browser. However, this configuration
requires additional setup steps on the server side, such as establishing a
synchronous communication channel and organizing the language client
(a browser's JavaScript application) that will communicate with the WebAssembly
module.

This setup has two parts:

- The client-side JavaScript application, which is the
  [Monaco Editor](https://microsoft.github.io/monaco-editor/) with a custom
  implementation of the LSP client. This implementation is available for
  exploration in the `./wasm/js` subdirectory.

- The server-side Rust application designed for WebAssembly build targets.
  This implementation is located in the `./wasm/src` subdirectory.

The general idea is that the browser launches the WebAssembly module in a
separate web worker and communicates with the worker through post-messages,
treating these messages as LSP communication messages. The worker serializes
these messages into UTF-8 strings and invokes a Rust WebAssembly exported
function to send the serialized data to the Rust code. The Rust code
deserializes the message, handles it with the LSP server, and sends serialized
responses back to the JavaScript web worker. The worker then forwards these
responses to the main JavaScript application, where they are handled by the LSP
client of the Monaco Editor.

The implemented Rust program serves as both the LSP server and the script runner.

To launch this example:

1. Run the `$ wasm-build.sh` build shell script. This script builds the Rust
   application into a WebAssembly module and copies the resulting `.wasm` file
   into the `./wasm` directory.

   Note that the first launch may take some time, as the script compiles the
   Rust code in production mode.

2. Host the content of the `./wasm` directory.

   For example, you can host these files using the
   [https](https://crates.io/crates/https) Rust local web server:
  - Install the server: `$ cargo install-update-config -e RUSTC_BOOTSTRAP=1 https`.
  - From the `./wasm` directory, run `$ https`.
  - Open `http://localhost:8000/` in your browser.

## Quick Links

- [GitHub Repository](https://github.com/Eliah-Lakhin/ad-astra)
- [API Documentation](https://docs.rs/ad-astra)
- [Main Crate](https://crates.io/crates/ad-astra)
- [Guide Book](https://ad-astra.lakhin.com)
- [Examples](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples)
- [Playground](https://ad-astra.lakhin.com/playground.html)

## Copyright

This work is proprietary software with source-available code.

To copy, use, distribute, or contribute to this work, you must agree to the
terms and conditions of the [General License Agreement](https://github.com/Eliah-Lakhin/ad-astra/blob/master/EULA.md).

For an explanation of the licensing terms, see the
[F.A.Q.](https://github.com/Eliah-Lakhin/ad-astra/tree/master/FAQ.md)

Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин). All rights reserved.
