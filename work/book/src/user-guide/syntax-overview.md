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

# Syntax Overview

Ad Astra is a dynamically typed imperative scripting language with elements of
functional and concatenative programming paradigms.

The base semantics of the language is similar to JavaScript: a typical script
consists of anonymous functions and structural objects that form the program's
design.

```adastra
let my_object = struct {
    field: 10,
    method: fn(param) {
        self.field += param;
    },
};

my_object.method(20);

my_object.field == 30;
```

Visually, the language attempts to mimic Rust's syntax: variables are introduced
with the `let` keyword, objects with the `struct` keyword, functions with the
`fn` keyword, and so on.

In general, Ad Astra is a dynamically typed language, meaning that variable and
expression types are inferred during script evaluation. However, there are
certain static restrictions imposed on language constructs. For instance, you
cannot change the type of a variable once it has been assigned a value, nor can
you invoke a function with an arbitrary number of arguments.

```adastra
let x = 10;

x = 20; // But you cannot assign a string literal to `x`.
```

These restrictions make the code architecture less ambiguous than, for example,
JavaScript or Python programs. At the same time, they make static source code
analysis more predictable within local evaluation contexts.

The primary source of program polymorphism is user-defined functions. A function
introduced by the user does not impose restrictions on input parameter types or
the output type. Essentially, all script-defined functions are polymorphic.

```adastra
let concat_two_args = fn(a, b) {
    return [a, b];
};

concat_two_args(10, 20); // Creates an array of numbers: [10, 20].

concat_two_args("hello ", "world"); // Creates a new string: "hello world".
```

In contrast, the APIs exported from Rust have well-defined typed signatures.

For example, if you have a Rust function `fn deg_to_rad(x: f64) -> f64`, you
cannot call this function in scripts by providing a `struct` argument. Doing so
would result in a runtime error, and the static analyzer will detect such misuse
in most cases.

Finally, the script user cannot introduce new types or operators on the types.
All types and the operations on them (including type methods) are exported from
Rust to the script, defining the domain-specific environment for the script code.
