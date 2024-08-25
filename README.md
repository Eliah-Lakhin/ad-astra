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

# Ad Astra

[![Crate](https://img.shields.io/crates/v/ad-astra?label=Crate)](https://crates.io/crates/ad-astra)
[![API Docs](https://img.shields.io/docsrs/ad-astra?label=API%20Docs)](https://docs.rs/ad-astra)
[![Book](https://img.shields.io/badge/Book-616161)](https://ad-astra.lakhin.com/)
[![Examples](https://img.shields.io/badge/Examples-616161)](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples)
[![Playground](https://img.shields.io/badge/Playground-616161)](https://ad-astra.lakhin.com/playground.html)

<img align="right" height="220" style="float: right; margin-left: 10px; width: 220px" alt="Ad Astra Logo" src="https://raw.githubusercontent.com/Eliah-Lakhin/ad-astra/master/work/logo.png" />

Ad Astra is a configurable scripting language platform designed for embedding
in Rust applications.

Unlike system programming languages such as Rust or C++, the primary advantage
of scripting languages is their short edit-compile-run loop, allowing users to
quickly re-run scripts and see the results almost instantly, without needing to
stop and rebuild the entire host application. Additionally, scripting languages,
including Ad Astra, have a lightweight and easy-to-learn syntax.

These features are essential for live coding, making them an ideal choice as the
foundation for dynamic plugin systems in end-user applications. For example, they
can be used as scripting systems for video game engines or any other applications
requiring dynamic reconfiguration capabilities.

## Key Features

What makes Ad Astra stand out among alternative solutions are:

1. **Advanced Built-In Language Server**

   With the built-in LSP server, you can provide your users with code editor
   language extensions that help them explore exported APIs and navigate
   through script code in real-time.

2. **Rust Integration**

   Using the Export macro, you can export Rust crate APIs into a fully dynamic
   script environment with minimal changes.

3. **Configurable Language Semantics**

   Most language constructs, including operators and types, are configurable
   and customizable. You can define your own domain-specific language semantics
   to meet the specific needs of your application domain.

## Language Server

[![Showcase](https://raw.githubusercontent.com/Eliah-Lakhin/ad-astra/master/work/showcase.gif)](https://ad-astra.lakhin.com/playground.html)

You can experience the features of the Ad Astra language server yourself in the
interactive [Playground](https://ad-astra.lakhin.com/playground.html).

This demo editor runs locally in your web browser without needing a remote web
server. The client-side uses a customized Monaco editor, while the server-side
is powered by the Ad Astra LSP server, built for the WebAssembly target and
running in the browser's web worker.

The same editor infrastructure can be set up with any IDE that supports the LSP
protocol, such as VS Code. Example setups are available in the
[examples](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples)
directory.

Supported LSP features:

- Live syntax and semantic diagnostics for source code.
- Code completion suggestions.
- Jump to definitions and declarations of identifiers.
- Inlay hints for variable types and function parameters.
- Tooltips for variables, functions, and other identifiers.
- Highlighting of semantically related identifiers.
- Function signature hints.
- Identifier renaming.
- Built-in code formatter.
- Quickfix suggestions for issues.
- Script runner.

## Exporting Rust API

The `#[export]` macro from the crate introspects Rust module items and exports
them to the script engine.

The script user can then utilize the exported Rust API in a fully dynamic script
environment.

```rust
// Exports the `deg` function to the script environment.
#[export]
pub fn deg(degrees: f64) -> f64 {
    PI * degrees / 180.0
}

// Exports the Rust struct `Vector` with public fields.
#[export]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
}

// Defines the `vector_1 + vector_2` operator in the script.
#[export]
impl Add for Vector {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.x += rhs.x;
        self.y += rhs.y;

        self
    }
}

// Exports type methods to the script.
#[export]
impl Vector {
    pub fn radius(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}
```

## Ad Astra Language

The Ad Astra language is a dynamically-typed, interpreted scripting language
that incorporates elements from functional, object-oriented, and concatenative
paradigms.

The language semantics is inspired by JavaScript, Lua, and Python, while the
syntax is designed to visually resemble Rust.

This language is easy to learn for most programmers and does not require
advanced programming concepts to get started.

```
// Imports the Rust API from the exported crate "my_crate".
use my_crate;

// Variable declaration.
let foo = 10;

// The language supports for-loops and unbounded loops.
for i in 0..50 {
    // Prints formatted strings: "Step 0", "Step 1", etc.
    dbg(["Step ", i]);
}

// Function declaration.
let func = fn(a) {
    // Utilizes the closure "foo".
    return foo + a;
};

func(20) == 30;

// A struct is a key-value data type similar to a JavaScript object
// or Lua table. With the "struct" type, users can emulate objects
// with fields and methods.
let my_object = struct {
    field: 123,
    
    method: fn(a) {
        self.field += a;
    },
};

my_object.method(5);
my_object.field == 128;
```

The base language deliberately features minimalistic syntax and built-in APIs,
allowing you to create a customized language environment through Rust exports
that suit your domain-specific needs.

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
