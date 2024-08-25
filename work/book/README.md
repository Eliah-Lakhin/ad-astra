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

# The Ad Astra Book

The source materials for the [Guide Book](https://ad-astra.lakhin.com).

To set up the book locally:

1. Install [mdbook](https://crates.io/crates/mdbook): `$ cargo install mdbook`.

2. Clone [Ad Astra repository](https://github.com/Eliah-Lakhin/ad-astra) to your
   local machine.

3. Run the build script: `$ ./build.sh` from the `work/book` directory of the
   repository.

   This bash script performs the initial book build, including building the
   playground files and downloading required dependencies from remote CDNs.

   Subsequent builds can be performed using the `$ mdbook build` or
   `$ mdbook watch` commands.

4. Host the files from the `book/output` directory on your local machine.

   For example, you can host these files using the
   [https](https://crates.io/crates/https) Rust local web server:
   - Install the server: `$ cargo install-update-config -e RUSTC_BOOTSTRAP=1 https`.
   - From the `book/output` directory, run `$ https`.
   - Open `http://localhost:8000/` in your browser.

## Quick Links

- [GitHub Repository](https://github.com/Eliah-Lakhin/ad-astra)
- [API Documentation](https://docs.rs/ad-astra)
- [Main Crate](https://crates.io/crates/ad-astra)
- [Guide Book](https://ad-astra.lakhin.com)
- [Examples](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples)
- [Playground](https://ad-astra.lakhin.com/playground.html)

## Copyright

This work is proprietary software with source-available code.

To copy, use, distribute, or contribute to this work, you must agree to the
terms and conditions of the [General License Agreement](https://github.com/Eliah-Lakhin/ad-astra/blob/master/EULA.md).

For an explanation of the licensing terms, see the
[F.A.Q.](https://github.com/Eliah-Lakhin/ad-astra/tree/master/FAQ.md)

Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин). All rights reserved.
