////////////////////////////////////////////////////////////////////////////////
// This file is part of "Ad Astra", an embeddable scripting programming       //
// language platform.                                                         //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/ad-astra/blob/master/EULA.md               //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::cell::RefCell;

use ad_astra::{
    analysis::{ModuleRead, ModuleText, ModuleWrite, ScriptModule},
    export,
    interpret::{set_runtime_hook, ScriptFn},
    lady_deirdre::{
        analysis::TriggerHandle,
        arena::Identifiable,
        format::TerminalString,
        lexis::ToSpan,
    },
    runtime::{
        ops::{DynamicArgument, DynamicReturn, DynamicType},
        Origin,
        ScriptPackage,
    },
};

use crate::{
    data::{export_data, import_data},
    logger::console_log,
    worker::set_panic_hook,
    Package,
};

#[link(wasm_import_module = "runner")]
extern "C" {
    fn report(head: *const u8);
}

thread_local! {
    static RUNNER: RefCell<Option<WasmRunner>> = const { RefCell::new(None) };
}

struct WasmRunner {
    module: ScriptModule,
    handle: TriggerHandle,
    assembly: Option<ScriptFn>,
}

impl WasmRunner {
    fn load(uri: &str, text: &str) {
        RUNNER.with_borrow_mut(|state| match state {
            Some(runner) if runner.module.id().name() == uri => {
                let mut write_guard = runner
                    .module
                    .try_write(&runner.handle, 1)
                    .expect("Unable to write to the Runner's module.");

                write_guard
                    .edit(.., text)
                    .expect("Runner's module text update failure.");

                runner.assembly = None;
            }

            _ => {
                let module = ScriptModule::new(Package::meta(), text);
                module.rename(uri);

                *state = Some(WasmRunner {
                    module,
                    handle: TriggerHandle::new(),
                    assembly: None,
                })
            }
        })
    }

    fn compile() {
        RUNNER.with_borrow_mut(|state| {
            let Some(runner) = state else {
                panic!("Runner is not initialized.");
            };

            let read_guard = runner
                .module
                .try_read(&runner.handle, 1)
                .expect("Unable to read Runner's module.");

            let assembly = read_guard
                .compile()
                .expect("Runner's module compilation failure.");

            runner.assembly = Some(assembly);
        });
    }

    fn launch() -> String {
        RUNNER.with_borrow(|state| {
            let Some(runner) = state else {
                panic!("Runner is not initialized.");
            };

            let Some(assembly) = &runner.assembly else {
                panic!("Missing compilation assembly..");
            };

            console_log(format!("{:?}", assembly));

            let read_guard = runner
                .module
                .try_read(&runner.handle, 1)
                .expect("Unable to read Runner's module.");

            match assembly.run() {
                Ok(cell) => {
                    let mut result = String::from("ok]");

                    result.push_str(&cell.stringify(true));

                    result
                }

                Err(error) => {
                    let text = read_guard.text();

                    let mut result = String::from("er]");

                    result.push_str(&error.display(&text).to_string().sanitize().to_string());

                    result
                }
            }
        })
    }

    fn with_text<R>(f: impl FnOnce(&ModuleText<'_>) -> R) -> R {
        RUNNER.with_borrow(move |state| {
            let Some(runner) = state else {
                panic!("Runner is not initialized.");
            };

            let read_guard = runner
                .module
                .try_read(&runner.handle, 1)
                .expect("Unable to read Runner's module.");

            let result = f(&read_guard.text());

            result
        })
    }
}

/// Prints the provided argument and then returns it unchanged.
#[export]
fn dbg(x: DynamicArgument<DynamicType>) -> DynamicReturn<DynamicType> {
    WasmRunner::with_text(move |text| {
        let Origin::Script(origin) = x.origin else {
            return DynamicReturn::new(x.data);
        };

        let Some(span) = origin.to_position_span(text) else {
            return DynamicReturn::new(x.data);
        };

        let line = span.start.line;

        let label = x.data.stringify(false);
        let tooltip = x.data.stringify(true);

        let tooltip = match label == tooltip {
            true => String::new(),
            false => format!("```text\n{tooltip}\n```"),
        };

        let string =
            format!("{{\"kind\":3,\"line\":{line},\"label\":{label:?},\"tooltip\":{tooltip:?}}}");

        let head = unsafe { export_data(string) };

        unsafe { report(head) };

        DynamicReturn::new(x.data)
    })
}

#[no_mangle]
unsafe extern "C" fn runner_init() {
    set_panic_hook();

    set_runtime_hook(|origin| {
        let data = match origin {
            Origin::Script(origin) => {
                WasmRunner::with_text(|text| match origin.to_position_span(text) {
                    Some(span) => {
                        format!("{{\"kind\":1,\"line\":{}}}", span.start.line)
                    }

                    None => String::from("{{\"kind\":1}}"),
                })
            }

            Origin::Rust(origin) => format!("{{\"kind\":2,\"display\":{origin:?}}}"),
        };

        let head = unsafe { export_data(data) };

        unsafe { report(head) };

        true
    })
}

#[no_mangle]
unsafe extern "C" fn runner_load_module() {
    let data = unsafe { import_data() };

    let uri_delimiter = data.find("]").expect("Missing module name.");

    let uri = &data[0..uri_delimiter];
    let text = &data[(uri_delimiter + 1)..];

    WasmRunner::load(uri, text);
}

#[no_mangle]
unsafe extern "C" fn runner_compile_module() {
    WasmRunner::compile();
}

#[no_mangle]
unsafe extern "C" fn runner_launch() -> *const u8 {
    let result = WasmRunner::launch();

    unsafe { export_data(result) }
}
