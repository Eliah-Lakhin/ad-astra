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

# Introduction
<img align="right" height="160" style="float: right; margin-left: 10px; width: 160px" alt="Ad Astra Logo" src="https://raw.githubusercontent.com/Eliah-Lakhin/ad-astra/master/work/logo.png" />

Ad Astra is a configurable scripting language designed primarily for embedded
use in Rust applications.

The language features an easy-to-learn, minimalistic syntax that should feel
familiar to users of JavaScript or Python. Developers can expose parts of their
host Rust crate APIs — such as functions, types, type methods, and operators on
types — to the script environment. These APIs collectively form a
domain-specific customization of Ad Astra, enabling the end user to interact
with the Rust application at runtime in a fully dynamic way.

## Built-in Language Server

Usability is one of the key design goals of Ad Astra.

Ad Astra offers a full-featured LSP (Language Server Protocol) server that
supports a wide range of editor features, such as code completions, type
hints, symbol references, and more.

Through the editor environment, users can explore the exported domain-specific
APIs. The user-facing documentation for these APIs mirrors the RustDoc
documentation of the original exported Rust APIs. Overall, the language server
aims to provide the script user with an experience on par with RustAnalyzer.

You can try the language server features in the [Ad Astra Playground](playground.md), a
static web application with the LSP server running in a local web worker.

[![Showcase](https://raw.githubusercontent.com/Eliah-Lakhin/ad-astra/master/work/showcase.gif)](playground.md)

## Exporting

Rust programmers can export Rust APIs directly to the script environment by
annotating the corresponding APIs with the Export attribute macro.

In most cases, developers don't need to maintain an extra abstraction layer
between the Rust static APIs and the fully dynamic script runtime. The export
system automatically performs all necessary Rust code introspections and
exporting. However, the Ad Astra crate also provides low-level exporting
components for fine-grained export edge cases.

```rust
#[export]
pub fn deg(degrees: f64) -> f64 {
    PI * degrees / 180.0
}

#[export]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
}

#[export]
impl Add for Vector {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.x += rhs.x;
        self.y += rhs.y;

        self
    }
}

#[export]
impl Vector {
    pub fn radius(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}
```

## Book

The User Guide sections of this book describe the base language syntax and
semantics.

The Developer Guide is a tutorial that walks you through the Ad Astra exporting
system, as well as the compiler and language server setup steps.

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
