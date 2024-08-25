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

# Evaluation

To evaluate the script, you need to compile the script module into a
[ScriptFn](https://docs.rs/ad-astra/1.0.0/ad_astra/interpret/struct.ScriptFn.html)
object, an Ad Astra Virtual Machine assembly object ready for execution.

```rust,ignore
let module = ScriptModule::new(Package::meta(), "return \"hello world\";");

let handle = TriggerHandle::new();
let mut read_guard = module.read(&handle, 1).unwrap();

// Compiles the script.
let script_fn = read_guard.compile().unwrap();

// Runs the compiled assembly.
match script_fn.run() {
    Ok(result) => {
        // Prints: "hello world".
        println!("{}", result.stringify(false));
    }

    Err(error) => {
        let module_text = read_guard.text();

        println!("{}", error.display(&module_text));
    }
}
```

The [compile](https://docs.rs/ad-astra/1.0.0/ad_astra/analysis/trait.ModuleRead.html#method.compile)
function compiles the script module. Note that this function is capable of
compiling scripts with diagnostic issues, and even if the source code contains
syntax errors. Normally, you should not compile and run such scripts, but if you
do, the compiler will attempt to produce assembly code that aligns as closely as
possible with the original script author's intentions.

To run the script, you call the
[ScriptFn::run](https://docs.rs/ad-astra/1.0.0/ad_astra/interpret/struct.ScriptFn.html#method.run)
function, which executes the compiled assembly on the current thread until
completion and returns a runtime result, which is either a value returned from
the script or a
[RuntimeError](https://docs.rs/ad-astra/1.0.0/ad_astra/runtime/enum.RuntimeError.html).

In the code above, we print the result using the
[Cell::stringify](https://docs.rs/ad-astra/1.0.0/ad_astra/runtime/struct.Cell.html#method.stringify)
function, assuming that the object (a string in this case) implements the
Display or Debug traits. Alternatively, to get the exact value returned from the
script, you can use functions like
[Cell::take](https://docs.rs/ad-astra/1.0.0/ad_astra/runtime/struct.Cell.html#method.take)
instead.

If the script returns a runtime error, this error indicates a bug that occurred
during script evaluation, and you should print this error to the terminal as
well:

```text
   ╭──╢ runtime error [‹doctest›.‹#2›] ╟───────────────────────────────────────────╮
   │        ╭╴ receiver origin                                                     │
 1 │ return "hello world" + 1;                                                     │
   │                      ╰╴ type 'str' does not implement + operator              │
   ├───────────────────────────────────────────────────────────────────────────────┤
   │ The object's type that is responsible to perform specified operation does not │
   │ implement this operator.                                                      │
   ╰───────────────────────────────────────────────────────────────────────────────╯
```

## Isolation

By default, the `ScriptFn::run` function executes the script to completion on
the current thread.

In practice, the script's execution time is unlimited, and the execution process
may never end. For example, the script might contain an infinite loop that would
never terminate.

To limit the execution process, you can set a thread-local hook that triggers on
each Ad Astra assembly command before it is evaluated, allowing the Rust
environment to interrupt the script execution manually.

If the script execution is interrupted, the `run` function will return the
[RuntimeError::Interrupted](https://docs.rs/ad-astra/1.0.0/ad_astra/runtime/enum.RuntimeError.html#variant.Interrupted)
variant.

```rust,ignore
let module = ScriptModule::new(Package::meta(), "loop {}");

let handle = TriggerHandle::new();
let mut read_guard = module.read(&handle, 1).unwrap();

let script_fn = read_guard.compile().unwrap();

let start = Instant::now();

// If the provided hook function returns true, the script runtime continues
// execution. Otherwise, the execution will be interrupted, and the `run`
// function will return an `Interrupted` error.
set_runtime_hook(move |_| start.elapsed().as_secs() <= 5);

match script_fn.run() {
    Ok(result) => {
        println!("{}", result.stringify(false));
    }

    Err(RuntimeError::Interrupted { .. }) => {
        println!("Script execution lasts too long.");
    }

    Err(error) => {
        let module_text = read_guard.text();

        println!("{}", error.display(&module_text));
    }
}
```

Note that although the
[set_runtime_hook](https://docs.rs/ad-astra/1.0.0/ad_astra/interpret/fn.set_runtime_hook.html)
function gives you more control over script evaluation, it slows down the
evaluation process because the provided callback is invoked at each step of
script execution.
