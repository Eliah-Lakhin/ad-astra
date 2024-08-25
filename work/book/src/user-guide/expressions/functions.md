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

# Functions

In Ad Astra, a function is any object that supports the invocation operator:
`foo(10, "bar")`.

Typically, these objects include script-defined functions, struct methods, and
functions or methods exported by the host system (via Rust).

## Script-Defined Functions

```adastra
let func = fn(a, callback) {
    return a + callback(2);
};

func(10, fn(arg) arg * 5) == 20;
```

Script-defined functions are anonymous first-order objects that are usually
assigned to variables, struct fields, or passed to other functions as callbacks.

Ad Astra does not impose restrictions on input function parameter types or the
output result data type, but the number of arguments must match the number of
the function's formal parameters.

There are two forms of script function syntax:
- A multi-line function with a block of code as its body: `fn() {}`.
- A one-line function: `fn() expr`.

The one-line function is syntactic sugar for the multi-line function that
evaluates the provided expression and returns its value: `fn() { return expr; }`.

By default, a multi-line script-defined function returns nil data unless the
function's body explicitly returns a value (via the `return expr;` statement).

Each Ad Astra script-defined function is an independent unit of execution. When
the script passes a script function as a callback to an exported Rust function,
the host system can evaluate this function in place or independently from the
original script execution.

In particular, Ad Astra specializations can provide multi-threaded script
execution capabilities through this mechanism.

The host system can also transfer a script function defined in one script module
into another script module, thereby enabling multi-module scripting
environments.

## Function Parameters

Function parameters are the variables associated with the values provided to the
function as arguments during the function invocation.

These parameter variables are always considered initialized. As a result, the
script cannot pass an uninitialized variable into a function during its
invocation.

## Closures

A script-defined function can refer to any identifier available in the namespace
where the function was declared.

These references remain valid even if the referred variable outlives the
function. In such cases, the function continues to refer to the variable's data
object for the duration of the function's lifetime.

```adastra
let outer_func = fn() {
    let closure = 10;

    let func = fn(arg) closure + arg;

    func(20) == 30;

    closure *= 5;

    func(20) == 70;

    return func;
};

let inner_func = outer_func();

inner_func(30) == 80;
```
