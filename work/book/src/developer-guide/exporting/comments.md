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

# Comments

If the exported item has Rustdoc comments, these comments will be exported as
well, and the script user will see them in the code editor.

```rust,ignore
/// Documentation for the function.
#[export]
pub fn deg(degrees: f64) -> f64 {
    PI * degrees / 180.0
}

/// Documentation for the Vector type.
#[export]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector {
    /// Documentation for the `vector.x` field.
    pub x: f64,
    pub y: f64,
}

#[export]
impl Vector {
    /// Documentation for the Vector constructor.
    #[export(name = "vec")]
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}
```
