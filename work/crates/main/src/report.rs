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

macro_rules! debug_unreachable (
    ($message:expr) => {
        {
            #[cfg(debug_assertions)]
            {
                $crate::report::system_panic!($message);
            }

            #[allow(unreachable_code)]
            ::std::hint::unreachable_unchecked()
        }
    };

    ($message:expr, $($args:tt)*) => {
        $crate::report::debug_unreachable!(::std::format!($message, $($args)*))
    };
);

macro_rules! system_panic (
    ($message:expr) => {{
        #[cfg(not(target_family = "wasm"))]
        {
            let std_error = ::std::io::stderr();
            let mut handle = std_error.lock();

            let message = $crate::report::error_message!($message);
            let _ = ::std::io::Write::write(
                &mut handle,
                ::std::ops::Deref::deref(&message).as_bytes(),
            );
            let _ = ::std::io::Write::flush(&mut handle);

            ::std::process::abort();
        }

        #[cfg(target_family = "wasm")]
        {
            let message = $crate::report::error_message!($message);
            ::std::panic!("{}", message);
        }
    }};

    ($message:expr, $($args:tt)*) => {
        $crate::report::system_panic!(::std::format!($message, $($args)*))
    };
);

macro_rules! error_message (
    ($message:expr) => {
        ::std::format!(
r#" !! AD ASTRA INTERNAL ERROR
 !!
 !! This is a bug.
 !! If you see this message, please open an Issue: https://github.com/Eliah-Lakhin/ad-astra/issues
 !!
 !! Message: {}
 !! File: {}
 !! Line: {}
 !! Column: {}
"#,
            $message,
            ::std::panic::Location::caller().file(),
            ::std::panic::Location::caller().line(),
            ::std::panic::Location::caller().column(),
        )
    };

    ($message:expr, $($args:tt)*) => {
        $crate::report::error_message!(::std::format!($message, $($args)*))
    };
);

pub(crate) use debug_unreachable;
pub(crate) use error_message;
pub(crate) use system_panic;
