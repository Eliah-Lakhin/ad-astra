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

# Operators Priority

Operators have precedence and, in the case of binary operators, associativity.

Precedence can be altered using parentheses `(...)`.

For example, `a + b + c` is equivalent to `(a + b) + c`.

In general, Ad Astra's operator precedence is similar to the rules in Rust and
many other languages with C-like syntax.

| Operators                                        | Precedence | Associativity |
|--------------------------------------------------|------------|---------------|
| Assignment: `a = b`, `a += b`, etc               | 1          | Right-to-Left |
| Binary disjunction: `a \|\| b`                   | 2          | Left-to-Right |
| Binary conjunction: `a && b`                     | 3          | Left-to-Right |
| Equality and ordering: `a == b`, `a > b`, etc    | 4          | Left-to-Right |
| Range operator: `10..20`                         | 5          | Left-to-Right |
| Bitwise disjunction: `a \| b`                    | 6          | Left-to-Right |
| Bitwise exclusive disjunction: `a ^ b`           | 7          | Left-to-Right |
| Bitwise conjunction: `a & b`                     | 8          | Left-to-Right |
| Bitwise shift: `a << b` and `a >> b`             | 9          | Left-to-Right |
| Additive: `a + b` and `a - b`                    | 10         | Left-to-Right |
| Multiplicative: `a * b`, `a / b`, and `a % b`    | 11         | Left-to-Right |
| Unary Left: `-a`, `*a`, `!a`                     | 12         | Left-to-Right |
| Unary Right: `a?`, `a(arg)`, `a[idx]`, `a.field` | 13         | Left-to-Right |
| Atomic operand: `ident`, `crate`, `self`, `max`  | 14         | Left-to-Right |

Operators with a higher precedence number take priority over those with a lower
precedence number: `a + b * c` means `a + (b * c)`, because multiplicative
precedence is higher than additive.

Associativity indicates the typical order of operand evaluation. In the
expression `a = b + c`, the `b + c` expression is evaluated before `a`.
