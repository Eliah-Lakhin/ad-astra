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

# Mutability

In Ad Astra, all data is always passed by reference to heap-allocated memory.

The mutability of an object of a particular type is determined by the set of
implemented operators and methods that collectively shape the type's interface.
This interface may provide full or partial capabilities for mutating the
referred data.

For example, Ad Astra's built-in numbers and booleans are inherently mutable
objects, but strings and functions are fully immutable.

## Assignment Operator

The assignment operator `a = b` is a standard binary operator that may or may
not be implemented for a given type.

The purpose of this operator is to replace the data referred to by the left
operand with the data referred to by the right operand.

Most built-in and exported types usually implement this operator, but there are
exceptions.

For example, all numeric types implement assignment, but the script function
type does not. Therefore, script code can reassign numbers but cannot reassign
functions.

```adastra
let x = 10;

x = 20;

let func = fn(a, b) a + b;

// Assignment to function is forbidden.
// func = fn(a, b) a * b;
```

## Variables are Immutable

Formally, all Ad Astra variables are immutable named slots that store references
to data objects.

Once a variable is initialized with a value, it cannot be reassigned. All
subsequent assignments will be interpreted by the engine as a call to the
binary `=` operator on the type.

```adastra
let x;

x = 10; // Initializes the variable with the value.

x = 20; // Calls the binary assignment operator: "=(x, 20)".
```

## Built-In Types Mutability

| Type                    | Assignment    | Mutability                                       |
|-------------------------|---------------|--------------------------------------------------|
| All `number` types      | Implemented   | Fully mutable.                                   |
| Boolean `bool` type     | Implemented   | Fully mutable.                                   |
| String `str` type       | Unimplemented | Not mutable.                                     |
| Range `range` type      | Unimplemented | Not mutable.                                     |
| Function types          | Unimplemented | Not mutable.                                     |
| Structure `struct` type | Unimplemented | Partially mutable. New fields can be added.      |
| Non-singleton arrays    | Unimplemented | Individual elements of the array may be mutable. |

Note that non-built-in exported types usually implement the assignment operator
and are typically inherently mutable objects.

## Boxing

The built-in semantics of Ad Astra covers only the base use cases, assuming that
growable arrays, strings, and other immutable constructions do not require data
mutation out of the box.

Engine specializations may expose additional APIs that allow the script user to
mutate some or all data types, depending on the specialization domain.

For example, a concrete specialization might implement mutable string builders,
vectors, or even general mutable boxing objects.

```adastra
let sb = string_builder();

sb.push("hello");
sb.push(" ");
sb.push("world");

sb.build() == "hello world";
```
