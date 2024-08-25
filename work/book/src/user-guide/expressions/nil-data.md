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

# Nil Data

Ad Astra has a concept of nil data, a special object that intentionally does not
represent any value.

Similar concepts exist in many scripting languages such as JavaScript, Python, 
nd Lua. However, in Ad Astra, nil data is less severe. For example, script code
cannot access a variable that does not exist and receive nil data. Instead, such
access would result in a hard compile-time error. Additionally, most built-in
APIs usually never accept nil data types as arguments.

One practical case where nil data may appear is when an exported Rust function
returns `Option<T>` and this option is `None`. Such a possibility should be
clear to the user because the editor displays the original Rust function
signature via the LSP server.

To check if a value is not nil, you can use the built-in `foo?` operator.

```adastra
// fn exported_function() -> Option<usize>
let foo = exported_function();

match foo? {
    true => dbg(foo + 10),
    false => dbg("The result is nil"),
}
```

To manually construct a nil data object, you can use an array constructor
without arguments.

```adastra
[]? == false;
```

In general, it is recommended to avoid using nil data in scripts to prevent
possible "null-pointer" bugs, but this decision ultimately depends on the
script's design.
