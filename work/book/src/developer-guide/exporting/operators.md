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

# Operators

The export macro recognizes the derives of standard Rust traits such as `Clone`,
`PartialEq`, `Debug`, etc.

By exporting these trait implementations, you enable certain features that the
script user can utilize with the type instances.

If the exported structure has a `#[derive(...)]` attribute, this attribute must
follow the `#[export]` attribute.

```rust,ignore
#[export]
// These derives will be recognized by the export macro.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
}
```

Alternatively, you can export these traits manually by exporting the
corresponding implementations.

```rust,ignore
#[export]
impl Display for Vector {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("vec({}, {})", self.x, self.y))
    }
}
```

## Supported Traits

In addition to standard derivable traits, the export system supports the
majority of traits from the [std::ops](https://doc.rust-lang.org/std/ops/index.html)
module and some other standard Rust traits.

| Rust Trait                | Script Operators                                  |
|---------------------------|---------------------------------------------------|
| `Clone` and/or `Copy`     | Copying: `*foo`                                   |
| `Debug` and/or `Display`  | Stringification: `["Foo is ", foo]`               |
| `PartialEq` and/or `Eq`   | Equality: `a == b` and `a != b`                   |
| `PartialOrd` and/or `Ord` | Comparison: `a >= b`, `a < b`, etc.               |
| `Hash`                    | Used implicitly                                   |
| `Default`                 | Used implicitly                                   |
| `Add` / `AddAssign`       | Addition: `a + b` / `a += b`                      |
| `Sub` / `SubAssign`       | Subtraction: `a - b` / `a -= b`                   |
| `Mul` / `MulAssign`       | Multiplication: `a * b` / `a *= b`                |
| `Div` / `DivAssign`       | Division: `a / b` / `a /= b`                      |
| `Not`                     | Logical negation: `!a`                            |
| `Neg`                     | Numeric negation: `-a`                            |
| `BitAnd` / `BitAndAssign` | Bitwise conjunction: `a & b` / `a &= b`           |
| `BitOr` / `BitOrAssign`   | Bitwise disjunction: `a \| b` / `a \|= b`         |
| `BitXor` / `BitXorAssign` | Bitwise exclusive disjunction: `a ^ b` / `a ^= b` |
| `Shl` / `ShlAssign`       | Bitwise left shift: `a << b` / `a <<= b`          |
| `Shr` / `ShrAssign`       | Bitwise right shift: `a >> b` / `a >>= b`         |
| `Rem` / `RemAssign`       | Remainder of division: `a % b` / `a %= b`         |

Note that the assignment script operator (`a = b`) is implicitly implemented for
exported Rust structures. For this reason, exporting just the `Add` trait
implementation is enough to enable the `a += b` script operator.

```rust,ignore
// Implements + operator between two vectors.
#[export]
impl Add for Vector {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.x += rhs.x;
        self.y += rhs.y;

        self
    }
}
```

```adastra
let v = vec(0.0, 1.0) + vec(1.0, 0.5);

// In this case, this is a syntax sugar for using `v + vec(-5.0, 0.0)`,
// where the result is then assigned to the left-hand side.
v += vec(-5.0, 0.0);

v == vec(-4.0, 1.5);
```
