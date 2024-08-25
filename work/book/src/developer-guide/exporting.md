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

# Exporting

To export semantics from Rust to a script, you would use the `#[export]`
attribute macro.

This macro automatically introspects the underlying Rust item, making it
available in the script environment.

Typically, you would export Rust crate functions, statics, constants, struct
types, and their `impl` blocks, including the implemented operators.

## Export Macro Anatomy

The macro should be applied to the Rust item.

For example, this application is allowed:

```rust,ignore
#[export]
impl Foo {
    pub fn bar(&self) {}
}
```

But the following export is **forbidden** because the implementation method
itself is not a Rust item:

```rust,ignore
// Missing `#[export]` annotation on the impl block.
impl Foo {
    #[export]
    pub fn bar(&self) {}
}
```

The `export` attribute may appear multiple times inside the introspected item and
on the same Rust construct to specify more export details.

```rust,ignore
#[export]
impl Foo {
    // Private methods are not exported by default.
    // By annotating this method with an additional `#[export]` attribute,
    // you enforce the method's export.
    #[export]
    fn bar(&self) {}
}
```
