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

# Variables

```adastra
let x = 10;
let y;

y = 10;
```

Variable introduction starts with the `let` keyword, followed by the variable
name, and ends with a semicolon (`;`).

The optional `= 10` part initializes the variable immediately with the provided
expression. You can delay a variable's initialization (as in the case of the `y`
variable), but you cannot use an uninitialized variable until it is fully
initialized.

```adastra
let x;

if something() {
    x = 10;
    
    // The variable `x` is considered fully initialized here, and you can use it.
}

// However, outside of the condition's block, the variable `x` might be
// uninitialized.

let y;

match something() {
    true => { y = 10; }
    false => { y = 20; }
}

// The variable `y` is fully initialized here because the `match`
// statement covers all possible control-flow branches.
```

You can use any Ad Astra identifier as a variable name, which is a sequence of
alphanumeric and "_" ASCII characters. Note that Ad Astra currently does not
support arbitrary Unicode identifiers.

Variables allow users to introduce new functions and structure instances in the
code. In Ad Astra, structures and functions are anonymous, and by assigning them
to variables, users create "named" functions and structures.

```adastra
let sum = fn(x, y) {
    return x + y;
};

let st = struct { field: 10 };

st.field = sum(10, 20);
```

## Identifier Shadowing

A variable introduction statement shadows any identifier with the same name that
was previously introduced in the scope.

```adastra
let x = 10;
let x = 20; // Shadows the previously introduced `x`.

{
    let x = 30; // Shadows the previous `x`, but only within the block context.
    
    x == 30;
}

x == 20;
```
