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

# Advanced Analysis

The script module interface offers additional features for syntax and semantic
analysis of the script's source code:

- The [completions](https://docs.rs/ad-astra/1.0.0/ad_astra/analysis/trait.ModuleWrite.html#method.completions)
  function returns all possible code completion candidates at the specified
  cursor point.
- The [symbols](https://docs.rs/ad-astra/1.0.0/ad_astra/analysis/trait.ModuleRead.html#method.symbols)
  function allows manual inspection of the source code's syntax constructions
  and the semantic relations between them.

These and other features provide low-level components for the development of
language servers and source code analysis tools for the Ad Astra language from
scratch, which are usually unnecessary for typical use case scenarios.
For further reading, see the
[analysis](https://docs.rs/ad-astra/1.0.0/ad_astra/analysis/index.html) and
[analysis::symbols](https://docs.rs/ad-astra/1.0.0/ad_astra/analysis/symbols/index.html)
API documentation.
