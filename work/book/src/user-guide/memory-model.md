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

# Memory Model

Data objects in scripts are always passed by reference to a heap memory
allocation.

This includes primitive types as well. For instance, if the script code
introduces a boolean value using the `true` keyword, the result of this
expression is a reference to the heap where the boolean data object is
allocated.

The script engine maintains a counter for the active references to script data,
ensuring that the object's data allocation persists until the last reference is
released.

```adastra
let x = 500; // Variable `x` contains a reference to the numeric object "500".

x /= 10; // `x` is passed by reference.

dbg(x); // `x` is passed by reference too.

// The end of the `x` lifetime.

dbg("Done");
```

Function closures prolong data lifetimes:

```adastra
let func;

{
    let x = 5;
    
    // The function captures the data referred to by `x`.
    func = fn() { return x; };
    
    // The lifetime of the variable `x` ends here,
    // but the data it refers to remains alive.
}

dbg(func()); // Prints "5".

// The lifetime of the function ends here, along with its closure
// to the numeric object "5".
```

## Data Projections

Script code can create a projection of one referential data object to a subset
of its memory.

For example, when script code refers to an array by index or range, the
interpreter does not create a copy of the element(s). Instead, it returns a
reference to the array slice.

```adastra
let array = [10, 20, 30];

// The `array[1]` expression returns a reference to the second element
// of the array.
let second = array[1];

// Mutates an element of the original array.
second = 200;

// Prints "200".
dbg(array[1]);
```

If this behavior is not desirable, the script code can copy the referred data
using the `*foo` cloning unary operator.

```adastra
let array = [10, 20, 30];

let second_copy = *(array[1]);

second_copy = 200;

dbg(array[1]); // Prints "20".
```

## Dereferencing

All data operations requested from scripts are eventually performed by the host
system (Rust code).

When a script sums two numbers, the operation is executed by the Rust function
that implements the `a + b` operator, taking both operands by value.

Passing by value means transferring the data allocation from the script memory
to the host system (or implicitly cloning the data if there are multiple active
script references to it).

In general, the host code can access script data immutably, mutably, or by
taking the data by value (which may require immutable access for cloning).

The type of data access is usually indicated by the corresponding exported
function signature. For example, the host function
`fn foo(a: &usize, b: &mut bool)` accesses the first argument immutably and the
second argument mutably.

Script code does not need to manually dereference provided arguments; data
dereferencing is handled automatically by the script engine.

However, this implicit dereferencing must comply with Rust's general rules of
exclusive access:

- There can be as many simultaneous active immutable dereferences of the same
  data allocation as needed.
- But if the data is dereferenced mutably, other simultaneous mutable or
  immutable dereferences are forbidden.

Data dereferencing is managed by the script engine, and failure to comply with
these rules results in runtime errors.

Such errors are rare because, typically, data dereferencing is localized.
