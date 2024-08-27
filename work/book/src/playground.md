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

<style>
html.adastra-theme {
    --content-max-width: 950px;
}

#example-select {
    background: #e0e0e0;
    border: none;
    font-size: 16px;
    font-weight: bold;
    padding: 5px;
}

#example-select option {
    font-size: 18px;
}

#example-select:focus {
    outline: none;
}

#loading {
    visibility: visible;
    opacity: 1;
}

#loading.loading-visible {
    visibility: visible;
    opacity: 1;
    transition: visibility 0s 100ms, opacity 250ms linear;
}

#loading.loading-hidden {
    visibility: hidden;
    opacity: 0;
    transition: visibility 0s 250ms, opacity 250ms linear;
}
</style>

<script
    src="extra/libs/require.min.js"
    data-main="extra/playground"
></script>

# Playground

<div style="display: flex; flex-direction: column; height: 80vh;">
    <div style="
        padding: 2px 10px;
        background: #e0e0e0;
        border-radius: 10px 10px 0 0;
        border-style: solid;
        border-color: #b3b6b7;
        border-width: 1px 1px 0 1px;
    ">
        <label style="display: flex;">
            <select id="example-select" style="flex-grow: 1" disabled>
                <option value="algebra" selected>Algebra — Using Rust API from scripts</option>
                <option value="collatz">Collatz — Control flow constructions</option>
                <option value="mutability">Mutability — Passing by reference</option>
                <option value="closures">Closures — Functions are first-order objects</option>
                <option value="structs">OOP — Script structs with fields and methods</option>
                <option value="quicksort">Quicksort — Functions recursion</option>
            </select>
        </label>
    </div>
    <div
        id="editor-container"
        style="
            position: relative;
            padding: 0;
            flex: 1;
            background: #424949;
            border-style: solid;
            border-color: #b3b6b7;
            border-width: 1px 1px 0 1px;
        "
    >
        <div
            id="loading"
            class="loading-visible"
            style="
                position: absolute;
                width: 100%;
                height: 100%;
                padding: 0;
                margin: 0;
                background-color:rgba(66, 73, 73, 0.85);
                display: flex;
                align-items: center;
                justify-content: center;
                z-index: 1;
            "
        >
            <div style="
                min-width: 250px;
                padding: 0;
            ">
                <div style="
                    background: #e0e0e0;
                    border-radius: 10px 10px 0 0;
                    border-style: solid;
                    border-color: #b3b6b7;
                    border-width: 1px 1px 0 1px;
                    font-size: 1.1em;
                    font-weight: bold;
                    text-align: center;
                    padding: 5px 0;
                ">Loading...</div>
                <ul style="
                    list-style: none;
                    padding: 20px;
                    margin: 0;
                    background: #b3b6b7;
                    border-color: #b3b6b7;
                    border-radius: 0 0 10px 10px;
                    border-style: solid;
                    border-width: 0 1px 1px 1px;
                ">
                    <li>
                        <i
                            id="loading-client"
                            class="fa fa-check"
                            aria-hidden="true"
                            style="visibility: hidden; color: #229954; margin-right: 5px;"
                        ></i>
                        Language Client
                    </li>
                    <li>
                        <i
                            id="loading-server"
                            class="fa fa-check"
                            aria-hidden="true"
                            style="visibility: hidden; color: #229954; margin-right: 5px;"
                        ></i>
                        Language Server <span id="loading-server-progress"></span>
                    </li>
                    <li>
                        <i
                            id="loading-example"
                            class="fa fa-check"
                            aria-hidden="true"
                            style="visibility: hidden; color: #229954; margin-right: 5px;"
                        ></i>
                        Example File <span id="loading-example-progress"></span>
                    </li>
                </ul>
            </div>
        </div>
        <div
            id="editor"
            style="
                position: absolute;
                width: 100%;
                height: 100%;
                padding: 0;
                background: #424949;
            "
        ></div>
    </div>
    <div style="padding: 0; margin: 0; display: flex; width: 100%;;">
        <div style="
            background: #e0e0e0;
            display: flex;
            flex-direction: column;
            border-radius: 0 0 0 10px;
            border-style: solid;
            border-color: #b3b6b7;
            border-width: 0 0 1px 1px;
        ">
            <div style="display: flex; flex-direction: column; flex-grow: 1;">
                <button
                    id="editor-launch-btn"
                    title="Run Script"
                    style="
                        padding: 0;
                        margin: 0;
                        width: 50px;
                        height: 50px;
                        font-size: 2.25em;
                        color: #229954;
                        background: none;
                        border: none;
                    "
                >
                    <i class="fa fa-play"></i>
                </button>
                <button
                    id="editor-stop-btn"
                    title="Stop Script Evaluation"
                    style="
                        display: none;
                        padding: 0;
                        margin: 0;
                        width: 50px;
                        height: 50px;
                        font-size: 2.25em;
                        color: #ba4a00;
                        background: none;
                        border: none;
                    "
                >
                    <i class="fa fa-stop"></i>
                </button>
                <button
                    id="editor-cleanup-btn"
                    title="Cleanup Debug Messages"
                    style="
                        display: none;
                        padding: 0;
                        margin: 0;
                        width: 50px;
                        height: 50px;
                        font-size: 2.25em;
                        color: #d4ac0d;
                        background: none;
                        border: none;
                    "
                >
                    <i class="fa fa-refresh"></i>
                </button>
            </div>
            <div style="display: flex; flex-direction: column;">
                <button
                    id="editor-hints-btn"
                    title="Show Extra Hints"
                    style="
                        padding: 0;
                        margin: 0;
                        width: 50px;
                        height: 50px;
                        font-size: 2.25em;
                        color: #b3b6b7;
                        background: none;
                        border: none;
                    "
                >
                    <i class="fa fa-commenting-o"></i>
                </button>
            </div>
        </div>
        <code
            id="editor-console"
            class="language-adastra-console"
            style="
                padding: 10px;
                margin: 0;
                flex: 1;
                border: 0 !important;
                border-radius: 0 0 10px 0;
                background: #b3b6b7;
                color: #000;
                height: 160px;
                overflow: auto;
                white-space: nowrap;
            "
        >
        <div style="color: #626567;">
            Shortcuts:<br/>
            - Pressing Ctrl+S in the editor formats the code.<br/>
            - Pressing Ctrl+Space opens the code completion menu.<br/>
            - Pressing Ctrl+Click on a variable or field jumps to its definition.<br/>
            - Press F2 on a variable or field to rename it.<br/>
        </div>
        </code>
    </div>
</div>

## Exported Rust Module

The source code above can use exported constructions from Rust's
[algebra.rs](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/exporting/src/lib.rs)
module via the script's `use algebra;` import statement.

```rust,ignore
{{#include extra/lsp/example.rs}}
```
