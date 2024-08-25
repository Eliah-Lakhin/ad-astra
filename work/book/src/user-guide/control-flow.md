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

# Control Flow

The script code is the body of an implicit function with zero parameters, which
serves as the entry point of the script.

The body of a function (including the entry-point code) consists of statements
that are executed sequentially, except for control flow statements such as loops
and conditionals.

```adastra
// Injects additional APIs from the sub-package "algebra"
// into the current namespace.
use algebra;

let x = 10; // Variable declaration.

foo(x + 20); // Expression statement.

// Simple conditional statement.
if x == 10 {
    do_something();
}

// Complex conditional statement.
match x {
    10 => {},
    20 => {},
    else => {},
}

// Infinite loop statement.
loop {
    x += 1;
    
    if x > 10 {
        break; // Breaks the loop.
    }
}

// For loop that iterates through the numeric range from 10 to 19 inclusive.
for i in 10..20 {
    dbg(i);
}

// Nested statement block.
{
    let inner_var;
    func_1();
    func_2();
}

// Returning from a function (from the script's main function in this case).
return "end";
```
