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

# API Overview

The Ad Astra project aims to develop an embedded scripting language
infrastructure integrated into end applications.

Typically, this infrastructure includes the following components:

- A script compiler and interpreter.
- A static code analyzer that verifies the syntax and semantic consistency of
  script source code, printing possible diagnostic errors and warnings to the
  terminal (or another output).
- Language extensions for code editors to assist users during script
  development.
- A source code formatter program.

The Ad Astra [crate](https://crates.io/crates/ad-astra) provides the necessary
APIs to implement these components for a custom domain-specific scripting
language based on the Ad Astra language.

The [examples](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples)
directory in the Ad Astra repository contains sample setups.

This tutorial will guide you through the most common API functions of the crate.
For in-depth learning, refer to the
[API Documentation](https://docs.rs/ad-astra).

## Language Customization

To extend the base language, you can export Rust APIs such as module types,
functions, methods, operators, etc., into the script environment using the
[#[export]](https://docs.rs/ad-astra/1.0.0/ad_astra/attr.export.html) attribute
macro.

```rust
#[export]
pub fn round(value: f64) -> i64 {
    value.round() as i64
}

#[export]
impl Matrix {
    pub fn rotation(angle: f64) -> Self {
        let (sin, cos) = angle.sin_cos();

        Self {
            x: Vector { x: cos, y: -sin },
            y: Vector { x: sin, y: cos },
        }
    }
}
```

The underlying Rust items will be introspected by the macro and exposed in the
script, fulfilling the script's domain-specific environment.

The `#[export]` macro has certain configuration options that will be explained
in the tutorial, but typically, exporting works without extra effort.

The [Algebra Example](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/algebra/src/lib.rs)
demonstrates a typical use of the export macro.

## Analyzer and Interpreter

To implement the analyzer for a script module, you instantiate a
[ScriptModule](https://docs.rs/ad-astra/1.0.0/ad_astra/analysis/struct.ScriptModule.html)
object into which you load the source text of the script.

For example, you can load the text from disk:

```rust,ignore
let text = read_to_string(&cli.path).expect("Script file read error.");

let module = ScriptModule::new(Package::meta(), text);
```

After creating the script module, you can query this object for diagnostic 
errors and warnings and print them to the terminal.

```rust,ignore
let handle = TriggerHandle::new();
let read_guard = module.read(&handle, 1).expect("Module read error.");

let diagnostics = read_guard.diagnostics(1).expect("Module analysis error.");

if !diagnostics.is_empty() {
    // Prints all errors and warnings, if any.
    println!("{}", diagnostics.highlight(&read_guard.text(), !0));
}
```

If there are no errors, you can compile the script module into assembly and run
this assembly in the Ad Astra Virtual Machine.

```rust,ignore
let assembly = read_guard.compile().expect("Script compilation error.");

match script_fn.run() {
    Ok(_) => println!("Script execution finished."),

    // Otherwise, prints the runtime error.
    Err(error) => println!(
        "Script execution failure:\n{}",
        error.display(&read_guard.text()),
    ),
}
```

Depending on your implementation goals, you can continuously watch the source
code file for changes and repeat the above steps to provide continuous script
execution.

The [Runner Example](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/runner)
demonstrates this kind of setup.

## Code Editor Extension

As a separate program, you can configure and run a language server that
interacts with the code editor through the
[LSP](https://microsoft.github.io/language-server-protocol/) protocol. This
server assists the script user in the editor with code completions, type hints,
identifier references, and many other features useful for live code development.

```rust,ignore
fn main() {
    LspServer::startup(
        LspServerConfig::new(),
        LspLoggerConfig::new(),
        LspTransportConfig::Stdio,
        Package::meta(),
    );
}
```

The [LSP Server Example](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/lsp-server)
demonstrates a language server setup.

To implement the code editor extension, you need to create an LSP client as a
plugin for the code editor. This implementation is editor-specific, so you
should consult the documentation of the particular editor you are targeting.

The [LSP Client Example](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/lsp-client)
is a sample LSP client setup for VS Code.

## Code Formatter

Additionally, you can set up a separate Rust program that formats the source
text of the Ad Astra script into a canonical form.

The [ad_astra::format::format_script_text](https://docs.rs/ad-astra/1.0.0/ad_astra/format/fn.format_script_text.html)
function takes a string of the source code, formats it, and returns the
formatted version of the text.

This setup is optional, as the LSP server offers built-in formatting
capabilities.

## Web-Assembly Build

The Ad Astra crate is WASM-compatible. Specifically, you can implement a special
setup of the script language infrastructure, including the language's runner and
code editor, that works entirely within a web browser without requiring a
separate web server.

The [WebAssembly Example](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/wasm)
demonstrates a setup of the script language runner and the LSP server compatible
with the `wasm32` target. In this setup, the server-side operates in the
browser's web worker, while the client side is a customized Monaco editor that
interacts with the local LSP server running in the web worker.

This example is also available in the [Ad Astra Playground](../playground.md).
