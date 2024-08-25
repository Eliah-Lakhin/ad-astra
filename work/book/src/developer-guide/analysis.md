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

# Analysis

You are loading script files into the
[ScriptModule](https://docs.rs/ad-astra/1.0.0/ad_astra/analysis/struct.ScriptModule.html)
objects for preliminary static analysis of the script's code semantics and
compilation into the Ad Astra Virtual Machine assembly.

```rust,ignore
let module = ScriptModule::new(
    Package::meta(),
    "let x = 10; x + 2; dbg(x);",
);
```

The first argument, `Package::meta()`, of the script module constructor is a
reference to the metadata of the script package from which the script will be
analyzed and interpreted.

The second argument is the script's source code, which you can load from a
script file.

A script module is a mutable object that does not perform code analysis
instantly. Instead, it provides interfaces to incrementally query certain
features of the source code, in accordance with recent edits to the source text.
