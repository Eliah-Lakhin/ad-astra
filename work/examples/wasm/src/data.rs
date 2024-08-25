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

const CAPACITY_MIN: usize = 1024;
const CAPACITY_MAX: usize = CAPACITY_MIN * 16;

thread_local! {
    static SHARED_BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[inline(always)]
pub(super) unsafe fn export_data(data: impl Into<Vec<u8>>) -> *const u8 {
    let data = data.into();
    let head = data.as_ptr();

    SHARED_BUFFER.set(data);

    head
}

#[inline(always)]
pub(super) unsafe fn import_data() -> String {
    let data = SHARED_BUFFER.take();

    unsafe { String::from_utf8_unchecked(data) }
}

#[no_mangle]
unsafe extern "C" fn buffer_alloc(len: u32) -> *const u8 {
    let len = len as usize;

    SHARED_BUFFER.with_borrow_mut(|buffer| {
        buffer.reserve(len);

        unsafe { buffer.set_len(len) };

        buffer.as_ptr()
    })
}

#[no_mangle]
unsafe extern "C" fn buffer_free() {
    SHARED_BUFFER.with_borrow_mut(|buffer| {
        unsafe { buffer.set_len(0) };

        if buffer.capacity() > CAPACITY_MAX {
            buffer.shrink_to(CAPACITY_MIN);
        }
    });
}

#[no_mangle]
unsafe extern "C" fn buffer_len() -> u32 {
    SHARED_BUFFER.with_borrow(|buffer| buffer.len() as u32)
}
