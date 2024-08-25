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

# Script Packages

The script package is the only structure required for exporting from the Rust
crate into the script environment.

```rust,ignore
#[export(package)]
// The script package must implement the Default trait.
#[derive(Default)]
struct Package;
```

This object represents the metadata of the current crate, and there should be no
more than a single exported script package per crate.

Typically, you place this object in the `lib.rs` or `main.rs` entry point of the
crate. However, the location is optional. The Ad Astra engine can recognize
exported Rust items regardless of their implementation location and visibility
level.

Other exported crate functions, statics, and constants will be exported on
behalf of the crate's script package. Script modules will be evaluated based on
the semantics exported into the script package.

For example, if you have an exported function `deg`, running the script code on
behalf of this crate's script package will make this function available in the
script.

```rust,ignore
#[export(package)]
#[derive(Default)]
struct Package;

#[export]
pub fn deg(degrees: f64) -> f64 {
    PI * degrees / 180.0
}

let script_module = ScriptModule::new(Package::meta(), "deg(120);");
```

## Package Visibility

The visibility level of the exported package object is up to the implementation.

By making this object public, you allow your crate's API users to run Ad Astra
scripts on behalf of this crate directly. This may or may not be desirable
depending on the level of encapsulation you want for your crate's API.

## Package Dependencies

The dependencies of your script package are the dependencies of your crate
(as specified in the `Cargo.toml` file) that also have exported script packages,
regardless of the script package object's visibility.

For instance, in the [Exporting Example](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/exporting),
the `algebra` crate is a Rust library that exports some Rust APIs into the local
script package of this library.
The [Runner Example](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/runner)
is a Rust program that has `algebra` as a Cargo dependency and its own
ScriptPackage as well.

When the Runner program runs scripts, the script code imports semantics from the
`algebra` package using the import statement.

```adastra
use algebra;

// `vec` is an exported function in the `algebra` package.

let v = vec(0.0, 1.0);

// Alternatively, the script code can refer to identifiers from dependencies
// by the package names.

let v = algebra.vec(0.0, 1.0);

// The `crate` keyword refers to the current package, where all exported
// semantics reside, including the names of the dependent packages.

let v = crate.algebra.vec(0.0, 1.0);
```
