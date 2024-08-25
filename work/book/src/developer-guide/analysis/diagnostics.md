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

# Diagnostics

The first thing you should do with the script module before compiling and
running user script code is analyzing the source code for syntax and semantic
issues.

There are two types of issues:

- **Diagnostic Errors**: These are hard errors. The analyzer is confident that
  these issues must be fixed in the source code at all costs.
- **Diagnostic Warnings**: The analyzer has detected a piece of code that is
  likely problematic, but it is unsure if it would lead to a runtime bug due to
  the dynamic nature of the Ad Astra script. For example, passing an argument of
  the wrong type to a function would result in a warning. The analyzer
  recommends fixing these issues, but it's ultimately up to the user.

Additionally, there are three levels of analysis depth, ordered by their
severity:

1. Syntax errors.
2. Semantic errors and warnings inferred locally.
3. Deep semantic analysis of the code for possible diagnostic warnings.

The issues at the lower depth levels are easier to detect and are the most
severe.

Before compiling the script, it is recommended to check the script module at
least for diagnostic errors at the first two levels of diagnostics.

```rust,ignore
let module = ScriptModule::new(
    Package::meta(),
    "let x = 10;\nlet 20;\nlet z = 30;",
);
module.rename("Example Module");

let handle = TriggerHandle::new();
let read_guard = module.read(&handle, 1).unwrap();

for depth in 1..=3 {
    let diagnostics = read_guard.diagnostics(depth).unwrap();

    // The `!0` argument is the severity mask.
    // In this case, we are checking for both diagnostic errors and warnings.
    if diagnostics.len(!0) == 0 {
        continue;
    }

    let module_text = read_guard.text();

    // Prints diagnostic errors and warnings.
    // The `highlight` function returns an annotated snippet.
    println!("{}", diagnostics.highlight(&module_text, !0));

    return;
}

println!("No issues detected.");
```

Prints:

```text
   ╭──╢ diagnostics [‹doctest›.‹Example Module›] ╟─────────────────────────────╮
 1 │ let x = 10;                                                               │
 2 │ let 20;                                                                   │
   │     ╰╴ missing var name in 'let <var> = <expr>;'                          │
 3 │ let z = 30;                                                               │
   ├───────────────────────────────────────────────────────────────────────────┤
   │ Errors: 1                                                                 │
   │ Warnings: 0                                                               │
   ╰───────────────────────────────────────────────────────────────────────────╯
```
