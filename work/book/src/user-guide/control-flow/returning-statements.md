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

# Returning Statements

The `return;` and `return <expr>;` statements immediately stop the current
function's execution and return the provided expression as the function's
result. The variant without an expression returns a *nil* value from the
function. If the function reaches its end without an explicit return, it returns
a *nil* value implicitly.

```adastra
let foo = fn() {
    // Returns the value 100 from the foo function.
    return 100;
};

let one_hundred = foo();

// Returns the value 200 from the script.
return one_hundred * 2;
```

Depending on the Ad Astra specialization, the value returned from the script may
represent the result of the script's execution.
