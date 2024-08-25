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

# Ranges

Range objects can be instantiated using the `from..to` syntax. Their primary
purpose is to specify a range of unsigned integer numbers, such as indices for
array slices and iteration ranges in for statements.

```adastra
let array = [10, 20, 30, 40, 50];

array[1..3] == [20, 30];

for i in 7..12 {
    dbg(i); // Prints: 7, 8, 9, 10, and 11.
}
```

The `from` and `to` parts are any expressions that can be evaluated to unsigned
integer numbers. The `from` value specifies the range's lower bound (inclusive),
and the `to` value specifies the upper bound (exclusive).

The upper bound should be greater than or equal to the lower bound. Otherwise,
the range is invalid, which will lead to runtime errors in most cases.

To construct a range with an "unlimited" upper bound, you can use the `max`
built-in constant, which evaluates to the maximum unsigned integer number
available on the current platform: `50..max`.
