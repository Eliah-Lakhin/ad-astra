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

# Module Text

The [ModuleText](https://docs.rs/ad-astra/1.0.0/ad_astra/analysis/struct.ModuleText.html)
object provides access to the original source code of the script.

You can print this object to the terminal, or you can extract a substring of the
source code within specified ranges.

```rust,ignore
let module = ScriptModule::new(
    Package::meta(),
    "let x = 10;\nlet y = 20;\nlet z = 30;",
);

// Assigns a user-facing name to the script module. 
module.rename("Example Module");

let handle = TriggerHandle::new();
let read_guard = module.read(&handle, 1).unwrap();

let module_text = read_guard.text();

// Fetches the second line of the source text.
let second_line = module_text.substring(Position::new(2, 1)..Position::new(3, 1));
assert_eq!(second_line, "let y = 20;\n");

println!("{module_text}");
```

Displaying the `ModuleText` object prints the following snippet:

```text
   ╭──╢ module [‹doctest›.‹Example Module›] ╟──────────────────────────────────╮
 1 │ let x = 10;                                                               │
 2 │ let y = 20;                                                               │
 3 │ let z = 30;                                                               │
   ╰───────────────────────────────────────────────────────────────────────────╯
```

The output snippet's look and feel can be configured via the
[ModuleText::snippet](https://docs.rs/ad-astra/1.0.0/ad_astra/analysis/struct.ModuleText.html#method.snippet)
function. Using this interface, you can set the snippet's header, footer, and
annotate specific fragments of the source code.

```rust,ignore
let mut snippet = module_text.snippet();

snippet.set_caption("Snippet Caption");
snippet.set_summary("Summary line 1.\nSummary line 2.");
snippet.annotate(
    Position::new(2, 5)..Position::new(2, 6),
    AnnotationPriority::Default,
    "Annotation of the variable.",
);

println!("{snippet}");
```

Prints:

```text
   ╭──╢ Snippet Caption [‹doctest›.‹Example Module›] ╟─────────────────────────╮
 1 │ let x = 10;                                                               │
 2 │ let y = 20;                                                               │
   │     ╰╴ Annotation of the variable.                                        │
 3 │ let z = 30;                                                               │
   ├───────────────────────────────────────────────────────────────────────────┤
   │ Summary line 1.                                                           │
   │ Summary line 2.                                                           │
   ╰───────────────────────────────────────────────────────────────────────────╯
```
