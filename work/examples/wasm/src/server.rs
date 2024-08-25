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

use std::{cell::RefCell, ptr::null, sync::mpsc::TryRecvError};

use ad_astra::{runtime::ScriptPackage, server::*};

use crate::{
    data::{export_data, import_data},
    logger::{console_debug, console_error, console_info, console_log, console_warn},
    worker::set_panic_hook,
    Package,
};

thread_local! {
    static SERVER: RefCell<Option<WasmLspServer>> = const { RefCell::new(None) };
}

struct WasmLspServer {
    server: LspServer,
    outgoing: RpcReceiver,
}

impl WasmLspServer {
    fn init(server: LspServer, outgoing: RpcReceiver) {
        SERVER.with_borrow_mut(|state| {
            if state.is_some() {
                panic!("Server already initialized.");
            }

            *state = Some(WasmLspServer { server, outgoing })
        })
    }

    fn borrow<R>(f: impl FnOnce(&Self) -> R) -> R {
        SERVER.with_borrow(move |state| {
            let Some(state) = state else {
                panic!("Server is not initialized.");
            };

            f(state)
        })
    }

    fn borrow_mut<R>(f: impl FnOnce(&mut Self) -> R) -> R {
        SERVER.with_borrow_mut(move |state| {
            let Some(state) = state else {
                panic!("Server is not initialized.");
            };

            f(state)
        })
    }
}

#[no_mangle]
unsafe extern "C" fn server_init() {
    set_panic_hook();

    let server_config = LspServerConfig::new();

    let (outgoing_sender, outgoing_receiver) = RpcMessage::channel();

    let server = LspServer::new(server_config, Package::meta(), outgoing_sender);

    let mut logger_config = LspLoggerConfig::new();

    logger_config.level = LevelFilter::Debug;
    logger_config.client = LspLoggerClientConfig::Window;
    logger_config.server = LspLoggerServerConfig::Custom(log);

    server.setup_logger(logger_config);

    WasmLspServer::init(server, outgoing_receiver);
}

#[no_mangle]
unsafe extern "C" fn server_input() {
    let input = unsafe { import_data() };

    let Some(input) = RpcMessage::from_input_bytes(input.as_bytes()) else {
        return;
    };

    WasmLspServer::borrow_mut(|wasm_server| {
        wasm_server.server.handle(input);
    });
}

#[no_mangle]
unsafe extern "C" fn server_output() -> *const u8 {
    WasmLspServer::borrow(|wasm_server| {
        let data = match wasm_server.outgoing.try_recv() {
            Ok(message) => message.to_output_bytes().unwrap_or_else(|| Vec::new()),
            Err(TryRecvError::Empty) => Vec::new(),
            Err(TryRecvError::Disconnected) => return null(),
        };

        unsafe { export_data(data) }
    })
}

fn log(level: Level, message: String) {
    match level {
        Level::Error => console_error(message),
        Level::Warn => console_warn(message),
        Level::Info => console_info(message),
        Level::Debug => console_debug(message),
        Level::Trace => console_log(message),
    }
}
