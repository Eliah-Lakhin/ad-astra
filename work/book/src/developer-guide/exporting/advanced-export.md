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

# Advanced Export

In this tutorial, we covered the basic exporting features of Ad Astra that
should address most practical use cases. However, the export system supports a
broader set of features.

To briefly mention a few:

- The export macro supports polymorphic types with type generics, handled
  through type monomorphization.
- The export macro also supports traits and trait implementations. While the
  export system does not export traits themselves, it can export implemented
  members of traits on specified types.
- Exporting custom Rust types (e.g., enums) through type aliases.
- Implementing type casting through the
  [Downcast](https://docs.rs/ad-astra/1.0.0/ad_astra/runtime/trait.Downcast.html)
  and [Upcast](https://docs.rs/ad-astra/1.0.0/ad_astra/runtime/trait.Upcast.html)
  interfaces.
- More script operators, such as implementing the `a = b` assignment operator on
  a type or a custom field access resolver.
- Exporting functions with dynamic parameters and/or result types.

For further reading, refer to the
[Export macro](https://docs.rs/ad-astra/1.0.0/ad_astra/attr.export.html) documentation
and the
[ad_astra::runtime](https://docs.rs/ad-astra/1.0.0/ad_astra/runtime/index.html)
crate module documentation.
