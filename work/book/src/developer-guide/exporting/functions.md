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

# Functions

To export a crate-global function, you should annotate it with the
[#[export]](https://docs.rs/ad-astra/1.0.0/ad_astra/attr.export.html) attribute macro.

```rust,ignore
#[export]
fn round(value: f64) -> i64 {
    value.round() as i64
}
```

The parameter types and the return type must be types that are also exported to
the script environment[^1].

By default, eligible types include:

- All Rust primitive numeric types: `isize`, `f32`, `u8`, etc.
- The boolean type: `bool`.
- Rust string types: `&str` and `String`.
- Ranges of unsigned integers: `Range<usize>`, `RangeFrom<usize>`, etc.
- The unit type `()`.
- Tuples of other eligible types: `(bool, String)`.
- Slices and fixed-size arrays of eligible types: `&[u32]`, `[u32; 6]`, etc.
- A box of an eligible type: `Box<(bool, String)>`.
- An option of an eligible type: `Option<[u8; 12]>`.
- A copy-on-write object of an eligible type with an implicit `'static`
  lifetime: `Cow<str>`.
- A result of an eligible type: `Result<usize, Err>`, where the error variant
  must be a Rust standard error type that is `Send + Sync + 'static`.
- Certain forms of callback functions.

To make additional types eligible, they should be exported either in this crate
or in any dependency crate.

## Function Names

All exported crate functions, regardless of their Rust visibility, will be
available in scripts within a common flat namespace, under the script package of
the crate, using their original Rust function names.

Therefore, their names must be unique within the crate.

To export two independent functions with the same name, you should rename them.

```rust,ignore
#[export]
fn foo() {}

mod bar {
    #[export(name "foo_from_bar")]
    fn foo() {}
}
```

In the script environment, these functions will be exposed as follows:

```adastra
foo();
foo_from_bar();

// Or:

crate.foo();
crate.foo_from_bar();
```

## References

You can export functions with references in the input positions if the lifetimes
of the references are elided. In other words, you can specify references, but
you cannot explicitly specify their lifetimes.

```rust,ignore
#[export]
fn addup(result: &mut usize, arg_1: &usize, arg_2: usize) {
    *result += *arg_1 + arg_2;
}
```

```adastra
let result = 10;
 
addup(result, 7, 2);

result == 19;
```

## Callbacks

You can use callback functions as types for the input and output of exported
functions, but only in certain forms.

```rust,ignore
#[export]
fn foo(
    arg_1: usize,
    arg_2: Box<dyn Fn(usize, usize) -> RuntimeResult<String> + Send + Sync>,
) -> RuntimeResult<String> {
    let mut result = arg_2(arg_1, 10)?;
    
    result.push_str(". Some suffix".);
    
    Ok(result)
}
```

```adastra
let result = foo(50, fn(x, y) ["Result is: ", x + y]);

result == "Result is: 60. Some suffix.";
```

The callback function must be a boxed Rust anonymous function (`Box<dyn>`) that
accepts up to 7 arguments and returns a value wrapped in the
[RuntimeResult](https://docs.rs/ad-astra/1.0.0/ad_astra/runtime/type.RuntimeResult.html)
result object.

The function passed into the Rust code is likely to be a script-defined function
that will be evaluated by the Ad Astra runtime, which can be error-prone.

The exported function implementation can pass the callback's errors back to the
script environment. Therefore, in the example above, the function `foo` also
returns a `RuntimeResult`.

To simplify Rust signatures, you can use one of the predefined type aliases for
callbacks.

```rust,ignore
use ad_astra::runtime::ops::Fn2;

// The first two Fn2 generic parameters are the types of the inputs,
// and the last one is the result type.
#[export]
fn foo(arg_1: usize, arg_2: Fn2<usize, usize, String>) -> RuntimeResult<String> {
    let mut result = arg_2(arg_1, 10)?;
    
    result.push_str(". Some suffix.");
    
    Ok(result)
}
```

[^1]: Or types that can be cast to exported types. For example, the
`Option<f32>` type is not an exported type, but the engine is capable of casting
a Rust `Option` to the exported `f32` type.
