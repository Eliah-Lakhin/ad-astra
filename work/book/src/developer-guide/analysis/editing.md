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

# Editing

The script module is an editable object. You can change the entire source code
or just a part of it whenever the user modifies corresponding fragments of the
code.

Due to the incremental nature of script analysis, source code edits are
efficient operations. The analyzer does not rebuild the entire inner semantic
representation of the code with every edit. Instead, it patches its inner
structures according to the changes whenever the API user queries corresponding
semantic features (e.g., when you query code diagnostics).

To edit the code, you should obtain the script module's write guard.

```rust,ignore
let module = ScriptModule::new(Package::meta(), "let x = 10;");
module.rename("Example Module");

let handle = TriggerHandle::new();
let mut write_guard = module.write(&handle, 1).unwrap();

// An absolute source code character range.
//
// Alternatively, you can use a line-column range:
//     `Position::new(1, 5)..Position::new(1, 6)`.
//
// The `..` range specifies the entire text range:
//     `write_guard.edit(.., "let new_variable_name = 10;").unwrap();`
write_guard.edit(4..5, "new_variable_name").unwrap();

let module_text = write_guard.text();

println!("{module_text}");
```

Prints:

```text
   ╭──╢ module [‹doctest›.‹Example Module›] ╟──────────────────────────────────╮
 1 │ let new_variable_name = 10;                                               │
   ╰───────────────────────────────────────────────────────────────────────────╯
```