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

# Structs

By exporting a Rust `struct` type, you register this type in the script engine,
allowing other exported items (e.g., Rust functions) to refer to it.

```rust,ignore
#[export]
#[derive(Debug, Clone, Copy, PartialEq)]
struct Vector {
    // Only the public fields will be exposed in scripts by default.
    pub x: f64,
    
    // Enforce private field exposure by annotating the field with `#[export]`.
    #[export]
    y: f64,
    
    // Read-only fields will be exposed as read-only in scripts.
    #[export(readonly)]
    pub z: f64,
}

// Referring to Vector as an exported function parameter.
#[export]
fn foo(v: &Vector) {}
```

The exported Rust structure must be of a type that is `Send + Sync + 'static`.
Therefore, you cannot export a structure with non-static lifetime references.

## Fields

By default, the macro exposes only the public fields to the scripting
environment. However, you can enforce exposure by annotating the field with the
`#[export]` or `#[export(include)]` attribute (both are synonyms when applied to
struct fields).

The field type must be one of the following:

- Any Rust numeric type: `f32`, `usize`, etc.
- The `bool` type.
- The unit `()` type.
- A range (`Range`) type.
- Any exported struct type.

Note that, in contrast to functions, you cannot expose a structure field with a
type like `Option<usize>`.

To bypass this limitation, you can prevent public field exposure using the
`#[export(exclude)]` annotation and expose the struct field value using
corresponding getters and setters.

```rust,ignore
#[export]
struct Foo {
    #[export(exclude)]
    pub bar: Option<usize>,
}

#[export]
impl Foo {
    pub fn get_bar(&self) -> &Option<usize> {
        self.bar
    }
    
    pub fn set_bar(&mut self, bar: Option<usize>) {
        self.bar = bar;
    }
}
```

## Methods

To export associated implementation members of the exported structure, you
should export the corresponding `impl` block of the structure.

```rust,ignore
#[export]
impl Vector {
    #[export(name "vec")]
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn radius(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&mut self) -> &mut Self {
        let r = self.radius();

        self.x /= r;
        self.y /= r;
        self.z /= r;

        self
    }
}
```

```adastra
let v = vec(1.0, 3.7, 9.0);

v.normalize();

v.radius() == 1.0;
```

Similarly to struct fields, the exporting system exposes only public methods by
default. Therefore, if an implementation has a non-public method that you want
to expose, or a public method that you don't want to export, you should
annotate them with the `#[export(include)]` and `#[export(exclude)]` attributes,
respectively.

There are two types of associated functions:

- Object methods: functions that have `self`, `&self`, or `&mut self` as a receiver.
- Non-methods, such as the `Vector::new` constructor from the example above.

Non-methods will be exported on behalf of the script package, just like normal
crate-global functions. Their names must be unique across the exported crate
functions namespace.

Usually, you would assign more type-specific names to constructors, such as
renaming the `Vector::new` function using the `#[export(name = "vec")]`
attribute.

In contrast, type methods with a receiver belong to the namespace of the
exported type. Their names must be unique only within the type's namespace.
For example, `Vector::radius` does not need renaming even if you export another
type with a method of the same name.

Finally, exported methods may return references with the same lifetime as the
receiver's lifetime. The `Vector::normalize` is an example of such a method.
