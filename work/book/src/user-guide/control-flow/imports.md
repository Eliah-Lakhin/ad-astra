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

# Import Statements

If the script environment includes additional packages through which the host
system exports extra APIs (such as Rust functions or constants), the script can
inject these API identifiers into the current namespace scope using the
`use <package>;` statement.

```adastra
use algebra;

// Calls the function "vec" from the imported package "algebra".
let v = vec(0.0, 1.0);
```

By importing identifiers from a package, they shadow any previously introduced
identifiers, similar to variable shadowing.

```adastra
let vec = 10;

{
    use algebra;
    
    vec(0.0, 1.0); // Refers to the "algebra.vec" function.
}

// Outside of the block, the original `vec` variable is no longer shadowed.
vec == 10;
```

Note that you can refer to package identifiers directly without importing them.
Additionally, you can always refer to the current package using the built-in
`crate` identifier, which cannot be shadowed.

```adastra
let vec = 10;

algebra.vec(0.0, 1.0);

let algebra = 20;

crate.algebra.vec(0.0, 1.0);

// Since the "algebra" package is shadowed by the `algebra` variable, you can
// import this package using the `crate` built-in identifier.
use crate.algebra;

// Refers to the "algebra.vec" function.
vec(0.0, 1.0);
```
