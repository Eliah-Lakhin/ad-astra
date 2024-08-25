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

# Numbers

The script code creates numeric objects using integral literals such as
`1234567`, and floating-point literals such as `123.456`, `123.456e2`, or
`123e-3`.

## Numeric Operations

For numeric types, the following operators are available:

- Arithmetic operations: `a + b`, `a - b`, `a * b`, `a / b`.
  
- Bitwise operations (for integer numbers only): `a & b`, `a | b`, `a ^ b`,
  `a << b`, `a >> b`.

- Remainder of division (for integer numbers only): `a % b`.

- Assignment operator: `a = b`.

- Composite assignment of any of the above: `a += b`, `a &= b`, etc.

- Equality and ordering: `a == b`, `a > b`, `a >= b`, `a < b`, `a <= b`.

- Numeric negation: `-a`.

## Numbers Conversion

The underlying type of a numeric value is platform-specific and can be any
Rust primitive numeric type such as `usize`, `isize`, `f64`, `i32`, etc.

The script engine selects the best type that suits the needs of the underlying
value representation. In scripts, numerics are represented as the generalized
`number` type, and the engine performs automatic number type conversions as
needed.

In general, Ad Astra numbers behave similarly to those in many other scripting
languages that do not distinguish between numeric types.

For this reason, script code can perform numeric operations on numbers of
different types transparently most of the time.

```adastra
10 + 4.5 == 14;
10.3 + 2 == 12.3;
[18, 3.6, -9]; // Creates an array of floats.
```

When applying numeric binary operators, the script attempts to cast the
right-hand operand to the type of the left-hand operand.

For this reason, the script will not be able to perform this subtraction:
`10 - 30`, because the left-hand side is an unsigned integer. To make it signed,
you can prefix the literal with the `+` sign: `+10 - 30`.

In general, to enforce casting to a desired type, you can start the numeric
operation with a numeric literal of that type.

```adastra
10 + 4.5 == 14;
0.0 + 10 + 4.5 == 14.5;
+10 - 30 == -20;
```

When the script code calls an exported Rust function with a parameter of a
specific numeric type, the script engine performs numeric conversion
automatically whenever possible.

```adastra
// fn foo(usize);

foo(10);
foo(10.5); // Passes 10 by truncating the fractional part.
foo(-20); // Leads to a runtime error because -20 cannot be converted to usize.
```
