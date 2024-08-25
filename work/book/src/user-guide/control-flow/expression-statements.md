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

# Expression Statements

An expression statement is any Ad Astra expression that ends with a semicolon (`;`).

```adastra
(10 + 20) * (30 + foo(40));
```

Note that, unlike in Rust, Ad Astra statements are not expressions. The following
syntax is not allowed: `return match x { 10 => true, else => false};`.

## Variable Initialization Statement

The engine interprets the `<variable_name> = <expr>;` assignment syntax as a
special expression statement that initializes the variable if it is not yet
initialized. Otherwise, it treats this expression as a standard assignment
operation.

Note that you cannot initialize uninitialized variables in any other way. Inner
assignment expressions will also be interpreted as assignment operations.

```adastra
let x;

// This will not initialize variable x:
// foo(x = 10);

// But this expression is an initialization expression.
x = 10;
```
