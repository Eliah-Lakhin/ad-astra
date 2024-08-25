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

# Type System

All Ad Astra types, including built-in and custom exported types, are
essentially the types of the host system (Rust types).

Script users cannot define new types within scripts. Instead, they must be
provided by the language specialization in Rust. The system of exported Rust
types defines the structure of the script domain.

The set of all types known to the script engine is global. While types cannot be
referred to directly in scripts, the static analyzer and interpreter are aware
of the types associated with manageable script data objects.

## Arrays

Arrays are a fundamental aspect of the type system's semantics. In Ad Astra,
arrays don't have a dedicated type; rather, every data object in a script is
inherently an array.

For example, both `305` and `[305]` are singleton arrays. Typically, most script
objects are singleton arrays, which are arrays with just one element.

Ad Astra arrays are flat memory allocations with a fixed number of contiguous
elements of the same type.

Arrays are flat in the sense that script code cannot express nested arrays
without boxing: `[1, 2, 3, 4]` and `[1, [2, 3], 4]` are considered equivalent
data entities.

For simplicity, the static script code analyzer does not distinguish between
singleton arrays and arrays with more than one element, assuming that the length
of the array is semantically transparent.

```adastra
// The analyzer assumes that all of the following variables have a `bool`
// type regardless of the array length.

let x = true;
let y = [false];
let z = [true, false, true];
```

## Strings

Ad Astra strings are arrays of unsigned bytes that encode UTF-8 sequences.

The script engine manages strings slightly differently from normal script
arrays. Specifically, the engine infers this type as the built-in `str` type
rather than as a `number` type.

## Nil Type

Another special type is the `nil` type. This type does not have instances that
point to any memory allocations.

In scripts, a Nil object can be constructed as an array without elements: `[]`.

Additionally, script-defined functions that do not return any data have a `nil`
return type. An exported function, method, or operator that returns Rust's `()`
unit type on behalf of the script returns a Nil object.

To check if a data object is not Nil, the script code uses the built-in `?`
unary operator.

```adastra
10? == true; // `10` is not nil.

[]? == false;

let func = fn() {};

func()? == false;

let x = 10;

(x += 5)? == false; // The result of the assignment operator is nil.
```

## Polymorphism and Type Casting

Ad Astra types are unique, monomorphic Rust types.

In general, data objects of distinct types are incompatible. If an exported API
function, method, or operator expects a data object of a specific type, the
script must provide an argument of that expected type.

Depending on the domain-specific specialization, some data types may offer
automatic conversion between objects of distinct types.

For example, the built-in numeric objects support automatic conversion, allowing
script code to pass an integer to a function that expects a floating-point
number.

## Type Families

A type family is a set of types that are semantically related.

They are designed for the convenience of script users. For instance, all
built-in numeric types (e.g., `usize`, `f32`, `i64`, `u8`) belong to a single
`number` type family.

The analyzer refers to them by the family name rather than by their specific
type names, and it typically does not produce a warning if the script code
passes an object of one type to a function that expects another type, as long as
both types belong to the same family.
