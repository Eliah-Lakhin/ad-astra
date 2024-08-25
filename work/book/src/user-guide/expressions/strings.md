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

# Strings

The `"hello world"` string literal creates a string object.

Ad Astra strings are immutable arrays of unsigned bytes that encode Unicode
strings. These values are compatible with Rust's immutable `str` type.

Since strings are arrays of bytes, script code can concatenate them using the
array constructor.

```adastra
["hello", " ", "world"] == "hello world";
```

The script engine interprets strings slightly differently than normal byte
arrays, considering that these arrays encode text data:

1. The array constructor attempts to stringify each argument into a string
   during the argument type casting. This feature is particularly useful for
   constructing formatted strings.

   ```adastra
   let x = 10;
   
   // Prints "The value of x is 10".
   dbg(["The value of x is ", x]);
   ```

2. The string index operator indexes by the string's Unicode characters rather
   than the underlying bytes.

   ```adastra
   "hello world"[1] == "e";
   "hello world"[1..7] == "ello w";
   ```
