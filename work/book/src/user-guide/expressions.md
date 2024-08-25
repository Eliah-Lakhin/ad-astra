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

# Expressions

Expressions in Ad Astra are constructs that compute data values.

Expressions are distinct from control-flow statements. Unlike in Rust, Ad Astra
statements are not expressions and do not produce any values.

An expression is any combination of atomic operands and operators applied to
other expressions:

- **Literals**:

  - Integer literals: `100`, `200`.

  - Float literals: `10.5`, `0.1234e-6`.

  - Unicode string literals: `"foo"`, `"abra cadabra"`.

  - Boolean literals: `true`, `false`.

  - Nil type constructor: `[]`.

- **Identifiers**:

  - A normal variable, function argument, or exported symbol identifier: `foo`.

  - Built-in context variable: `self`.

    Under `struct` method functions, this variable refers to the struct object
    instance, but in general, it could refer to any object depending on the
    Ad Astra specialization that describes the function's calling context.
    By default (and unless the function is a struct method), `self` is a *nil*
    value.

  - Built-in current package reference: `crate`.

    This identifier always points to the script package under which the script
    code is being evaluated. For example, if you have a variable "foo" that
    shadows a function "foo" from the current package, you can always call the
    package function using the `crate` identifier: `crate.foo()`.

  - Built-in maximum constant: `max`.

    This constant evaluates to the maximum unsigned integer number and is useful
    for the unbound range construct: `10..max` (all numbers from 10 to "infinity").

- **Binary operators** such as `<left_operand> <op> <right_operand>`, where the
  left and right operands are any expressions, and the operator between them is
  any of the following:

  - Assignment operators: `foo = bar`.

  - Arithmetic operators: `foo + bar`, `foo - bar`, `foo * bar`, `foo / bar`,
    `foo % bar`.

  - Bitwise operators: `foo & bar`, `foo | bar`, `foo ^ bar`, `foo << bar`,
    `foo >> bar`.

  - Logical operators: `foo && bar`, `foo || bar`.

  - Composite assignments: `foo += bar`, `foo -= bar`, `foo *= bar`,
    `foo /= bar`, `foo &= bar`, `foo |= bar`, `foo ^= bar`, `foo <<= bar`,
    `foo >>= bar`, `foo %= bar`.

    These operators usually perform the corresponding binary operation on the
    operands and then assign the result to the left-hand operand.

    Note that `&&=` and `||=` are not supported.

  - Equality operators: `foo == bar`, `foo != bar`.

  - Ordering operators: `foo > bar`, `foo >= bar`, `foo < bar`, `foo <= bar`.

- **Unary operators**:

  - Copy operator: `*<expr>`.

    This is a built-in operator that creates a clone of the underlying operand.
    For example, `*"hello world"` creates a copy of a string.

  - Nil testing operator: `<expr>?`.

    Another built-in operator that tests if the operand is a *nil* value or of
    a void-like type.

  - Numeric negation: `-<expr>`.

  - Logical negation: `!<expr>`.

- **Array Constructor**: `[a, b, c]`.

- **Array Length**: `my_array.len`.

- **Array Index**: `foo[index]`, where `index` is a numeric unsigned integer
  value or a range (e.g., `10..20`).

  In the case of ranges, the operator returns a slice of the `foo` array that
  spans a sequence of the original array within the specified range.
  For example, `[10, 20, 30, 40][1..3]` evaluates to the array `[20, 30]`.

- **Range Constructor**: `start..end`, where `start` and `end` are any unsigned
  integer values.

  The constructed range object specifies indices starting from the `start` value
  (inclusive) up until the `end` value (exclusive). The `3..5` range means
  indices 3 and 4. The end bound must be greater than or equal to the start
  bound; otherwise, the range is invalid, which will result in runtime errors
  in most cases.

- **Function Invocation**: `expression(arg1, arg2, arg3)`.

  This syntax can be applied to any expression value that implements Invocation.
  In particular, this operator can be applied to script-defined functions and
  functions exported from packages. The `arg1`, `arg2`, etc., are the argument
  expressions that will be assigned to the function parameters.

  In Ad Astra, the number of function parameters is always fixed. You cannot
  invoke a function with a different number of arguments than the number of
  parameters in the original function signature.

- **Field Access**: `foo.bar` or `foo.3`.

  The field name could be any valid Ad Astra identifier or an integer literal.

  Most data types (usually, most of the custom exported types) support a limited
  set of predefined fields that can be accessed in scripts. The script engine
  refers to these as the type's *components*. Component types are well-defined
  and can be inferred at compile-time. However, some types (such as the script
  structure type) support arbitrary field access semantics. In this case, field
  resolution is fully dynamic.

  The field access operator can return a data object of any arbitrary type, and
  in particular, an invokable function that serves as the type's method.

The majority of the above operators can be overloaded by the host (through
script engine specialization), and their meaning may vary depending on the
operand types.

For some built-in Ad Astra types, the specification establishes concrete,
canonical meanings and implementations for these operators. For example,
`10 + 20` is an addition of two numbers resulting in the number 30, which is
canonical for most programming languages, including Ad Astra.

However, for specific value types, the addition operator may have a
domain-specific meaning. For instance, adding one 3D object to another could
represent the union of those objects.

Overloadable operators are associated with the type of their first operand:
in the addition `10 + 20`, the operator is invoked on the numeric type (because
`10` is a numeric type). In this sense, binary operators, in general, are not
reflexive. The expression `10 + "20"` would attempt to add the number 10 and the
number 20 parsed from a string, but `"10" + 20` is illegal because the string
type does not have an addition operator.
