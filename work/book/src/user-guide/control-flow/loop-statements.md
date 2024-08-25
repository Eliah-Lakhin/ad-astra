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

# Loop Statements

There are two loop constructs: an unlimited loop and a for-iterator loop.

```adastra
let i = 1;
// Repeats the code block an unlimited number of times until
// a break statement is encountered.
loop {
    if i >= 100 {
        break;
    }
    
    dbg(i);

    i += 1;
}

// Repeats the block for each numeric value in the range,
// or until the loop is explicitly broken.
for i in 1..100 {
    dbg(i);
}
```

The loop statement repeats the code block indefinitely until the program
encounters a `break;` statement.

The for-iterator introduces a new integer variable (specified before the `in`
keyword), which iterates through every numeric value within the range expression
(specified after the `in` keyword).

The range expression can be any Ad Astra expression that returns a numeric
range. In the example above, the script will enter a code block with the `i`
numeric variable iterating from 1 to 99 inclusive. This variable is accessible
only within the body of the for-loop block (and all nested code within that
block).

## Breaking and Continuation

Within the body of loop and for-iterator statements, the code can invoke
`break;` and `continue;` statements.

The `break;` statement immediately ends the loop's execution, while the
`continue;` statement skips the remaining code in the current iteration and
moves on to the next iteration.

```adastra
let i = 0;
loop {
    if i >= 10 {
        break;
    }
    
    if i % 2 == 0 {
        continue;
    }
    
    // Prints 1, 3, 5, 7, 9
    dbg(i);

    i += 1;
}
```

Note that loop control statements affect the nearest loop in which the statement
is nested. If a loop is nested within another loop, breaking the inner loop will
cause the outer loop to continue its execution.

```adastra
loop {
    loop {
        break; // Exits the inner loop.
    }
    
    break; // Exits the outer loop.
}
```
