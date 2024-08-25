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

# Arrays

In Ad Astra, every data object is an array, usually an array with just one
element.

For example, the expression `100` creates an array with a single numeric value.

Most semantic constructions, operators, and exported functions typically create
singleton arrays (arrays with one element). Singleton arrays are convenient
operands, allowing the script code to apply operations like addition on
singletons: `10 + 20`. However, this operator is inapplicable to multi-element
arrays: `[10, 20] + [30, 40]`.

To create a new array, you can use the array constructor: `[10, 20, 30]`.

To access a single element of the array, you can use the index operator:
`foo[10]`. The index operator also accepts a range value that maps the array to
a slice: `foo[10..20]`.

To get the length of the array, you can use the special `.len` built-in field:
`[10, 20, 30].len == 3`.

Since every data object is an array, this field is available for any data object
regardless of its type: `10.len == 1`.

```adastra
let my_array = [10, 20, 30, 40];

for i in 0..my_array.len {

    // Prints 10, 20, 30, and 40.
    dbg(my_array[i]);
}
```

## Mutability

In Ad Astra, arrays are immutable in length; however, the individual elements of
an array can be mutated if the corresponding data type supports mutation.

For example, numeric types support mutation, so you can change the elements of
an array.

```adastra
let my_array = [10, 20, 30, 40];

for i in 0..my_array.len {
    my_array[i] /= 10;

    // Prints 1, 2, 3, and 4.
    dbg(my_array[i]);
}
```

Ad Astra does not provide variable-sized arrays out of the box. Ad Astra arrays
are analogous to Rust's fixed-size arrays, which cannot be resized or reallocated.

For vector-like data types with dynamic resizing, the underlying engine
specialization may provide corresponding higher-level APIs.

## Arrays Concatenation

The array constructor operator `[a, b, c]` is an overloadable operator that
typically concatenates the provided arguments into a single array of elements of
the same type.

The implementation of this operator is type-dependent, but the canonical
implementation simply constructs a new array from the provided elements:

- If the argument is *nil* or an empty array, the implementation skips this
  element.
- Otherwise, the implementation attempts to cast each element of the argument's
  array into a target type and adds these casted elements to the resulting
  array.

The expression `[10, 20, 30]` creates an array with 10, 20, and 30 numeric
values.

The expression `[[10, 20], [], [30]]` creates the same flat array of 10, 20,
and 30 numeric values.

The constructor `[[10]]` simply creates the number value 10: `[[10]] == 10`.

As a target data type into which each argument will be cast, the canonical
implementation uses the first non-nil argument.

The `[10, "20"]` constructor creates an array of numbers, attempting to parse
the second argument into a number, while the `["10", 20]` expression creates
the string "1020" because the first non-nil argument is a string. Therefore,
the rest of the arguments will be stringified as well.
