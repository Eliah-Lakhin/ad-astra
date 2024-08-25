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

use crate::data::export_data;

#[link(wasm_import_module = "logger")]
extern "C" {
    fn log(head: *const u8);
    fn info(head: *const u8);
    fn warn(head: *const u8);
    fn debug(head: *const u8);
    fn error(head: *const u8);
}

#[inline(always)]
pub(super) fn console_log(string: impl Into<String>) {
    let head = unsafe { export_data(string.into()) };

    unsafe { log(head) }
}

#[inline(always)]
pub(super) fn console_info(string: impl Into<String>) {
    let head = unsafe { export_data(string.into()) };

    unsafe { info(head) }
}

#[inline(always)]
pub(super) fn console_warn(string: impl Into<String>) {
    let head = unsafe { export_data(string.into()) };

    unsafe { warn(head) }
}

#[inline(always)]
pub(super) fn console_debug(string: impl Into<String>) {
    let head = unsafe { export_data(string.into()) };

    unsafe { debug(head) }
}

#[inline(always)]
pub(super) fn console_error(string: impl Into<String>) {
    let head = unsafe { export_data(string.into()) };

    unsafe { error(head) }
}
