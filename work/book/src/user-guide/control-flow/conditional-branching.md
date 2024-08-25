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

# Conditional Branching

Ad Astra provides two conditional statements that control the flow of execution
based on a conditional expression: the simple single-branching `if` statement
and the multi-branching `match` statement.

## If Statement

```adastra
if 20 > 10 {
    // Body
}
```

The if-statement evaluates the provided expression to a boolean value. If the
value is `true`, the body block will be executed; otherwise, the block will be
skipped.

In Ad Astra, if statements do not have "else" branches
(e.g., `if foo {} else {}` syntax is **forbidden**). For multi-branching logic,
you should use match statements instead.

## Match Statement

In Ad Astra, the multi-branching match statement serves the purpose of
"switching" conditional branching, and it comes in two forms: a match statement
with a subject and a match statement without a subject.

```adastra
match subject {
    10 => {},
    20 => {},
    else => {},
}

match {
    foo > 10 => {},
    bar < 20 => {},
    else => {},
}
```

The body of the match statement (the code enclosed in `{...}` curly braces)
consists of match arms. Each arm contains a testing *expression* specified
before the `=>` arrow and an arm *body* specified after the arrow.

Match arms are separated by `,` commas, with an optional trailing comma. If the
arm's body is a code block, the comma separator can be omitted.

As the body, the user can specify either a code block or an expression, which
will be interpreted as a block with a single expression statement.

The script engine executes match arms one by one in the order they are
specified.

If the match statement has a subject expression, the engine tests for equality
between the subject value and the arm's expression. If the statement does not
have a subject, the engine interprets the arm's expression as a boolean.

Once the engine finds the first truthful arm, it executes its body and ends the
match statement.

For example, an "if-else" branching could be expressed as follows:

```adastra
match foo > 10 {
    true => dbg("foo is greater than 10"),
    false => dbg("foo is less than or equal to 10"),
}
```

The "switch" branching could be expressed as follows:

```adastra
match foo {
    2 => dbg("foo is equal to 2"),
    7 => dbg("foo is equal to 7"),
    else =>  dbg("foo is neither 2 nor 7"),
}
```

## Exhaustiveness

Exhaustiveness means that the conditional branching covers all possible
conditions.

The `if` statement is never exhaustive unless the conditional expression is a
trivial `true` or `false` literal, because this statement does not have a
fallback "else" case.

The `match` statement can be exhaustive if it covers all possibilities.
For example, if the statement has match arms covering both `true` and `false`
values.

To make the match branching explicitly exhaustive, you can introduce a special
fallback arm: `else => {}` (the "else" keyword is a built-in construct).

This fallback arm should be the last one in the list of match arms, and it will
be executed as the final option if all previous conditions fail.

## Variable Initialization

You can conditionally initialize a variable using an *exhaustive* match
statement.

```adastra
let x;

match foo {
    "bar" => x = 10,
    "baz" => x = 20,
    else => x = 30,
}

// Variable `x` is fully initialized here.

dbg(x);
```
