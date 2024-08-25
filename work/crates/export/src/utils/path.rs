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

use std::ops::RangeInclusive;

use proc_macro2::Span;
use syn::{spanned::Spanned, GenericArgument, Path, PathArguments, ReturnType, Type};

use crate::utils::Operator;

pub trait PathUtils<'a> {
    fn matches_bracketed(
        self,
        segments: &[&str],
        args: RangeInclusive<usize>,
    ) -> Option<PathMeta<'a>>;

    fn matches_parenthesized(self, segments: &[&str]) -> Option<FnMeta<'a>>;

    fn matches_operator(self) -> Option<Operator>;

    fn matches_fn(self) -> Option<FnMeta<'a>>;

    fn matches_rust_fn(self) -> Option<PathMeta<'a>>;

    fn matches_box(self) -> Option<PathMeta<'a>>;

    fn matches_clone(self) -> Option<PathMeta<'a>>;

    fn matches_copy(self) -> Option<PathMeta<'a>>;

    fn matches_default(self) -> Option<PathMeta<'a>>;

    fn matches_partial_eq(self) -> Option<PathMeta<'a>>;

    fn matches_eq(self) -> Option<PathMeta<'a>>;

    fn matches_partial_ord(self) -> Option<PathMeta<'a>>;

    fn matches_ord(self) -> Option<PathMeta<'a>>;

    fn matches_hash(self) -> Option<PathMeta<'a>>;

    fn matches_debug(self) -> Option<PathMeta<'a>>;

    fn matches_display(self) -> Option<PathMeta<'a>>;

    fn matches_add(self) -> Option<PathMeta<'a>>;

    fn matches_add_assign(self) -> Option<PathMeta<'a>>;

    fn matches_sub(self) -> Option<PathMeta<'a>>;

    fn matches_sub_assign(self) -> Option<PathMeta<'a>>;

    fn matches_mul(self) -> Option<PathMeta<'a>>;

    fn matches_mul_assign(self) -> Option<PathMeta<'a>>;

    fn matches_div(self) -> Option<PathMeta<'a>>;

    fn matches_div_assign(self) -> Option<PathMeta<'a>>;

    fn matches_not(self) -> Option<PathMeta<'a>>;

    fn matches_neg(self) -> Option<PathMeta<'a>>;

    fn matches_bit_and(self) -> Option<PathMeta<'a>>;

    fn matches_bit_and_assign(self) -> Option<PathMeta<'a>>;

    fn matches_bit_or(self) -> Option<PathMeta<'a>>;

    fn matches_bit_or_assign(self) -> Option<PathMeta<'a>>;

    fn matches_bit_xor(self) -> Option<PathMeta<'a>>;

    fn matches_bit_xor_assign(self) -> Option<PathMeta<'a>>;

    fn matches_shl(self) -> Option<PathMeta<'a>>;

    fn matches_shl_assign(self) -> Option<PathMeta<'a>>;

    fn matches_shr(self) -> Option<PathMeta<'a>>;

    fn matches_shr_assign(self) -> Option<PathMeta<'a>>;

    fn matches_rem(self) -> Option<PathMeta<'a>>;

    fn matches_rem_assign(self) -> Option<PathMeta<'a>>;
}

impl<'a> PathUtils<'a> for &'a Path {
    fn matches_bracketed(
        self,
        segments: &[&str],
        args: RangeInclusive<usize>,
    ) -> Option<PathMeta<'a>> {
        if self.segments.len() != segments.len() {
            return None;
        }

        let mut countdown = self.segments.len();

        for (this, pattern) in self.segments.iter().zip(segments.iter()) {
            if this.ident.to_string() != *pattern {
                return None;
            }

            countdown -= 1;

            match &this.arguments {
                PathArguments::None => (),
                PathArguments::Parenthesized(..) => return None,
                PathArguments::AngleBracketed(arguments) => {
                    if countdown > 0 {
                        return None;
                    }

                    if arguments.args.len() > *args.end() {
                        return None;
                    }

                    if arguments.args.len() < *args.start() {
                        return None;
                    }

                    let mut args = Vec::with_capacity(arguments.args.len());

                    for argument in arguments.args.iter() {
                        let GenericArgument::Type(argument) = argument else {
                            return None;
                        };

                        args.push(argument);
                    }

                    return Some(PathMeta {
                        span: self.span(),
                        args,
                    });
                }
            }
        }

        Some(PathMeta {
            span: self.span(),
            args: Vec::new(),
        })
    }

    fn matches_parenthesized(self, segments: &[&str]) -> Option<FnMeta<'a>> {
        if self.segments.len() != segments.len() {
            return None;
        }

        let mut countdown = self.segments.len();

        for (this, pattern) in self.segments.iter().zip(segments.iter()) {
            if this.ident.to_string() != *pattern {
                return None;
            }

            countdown -= 1;

            match &this.arguments {
                PathArguments::None => (),
                PathArguments::Parenthesized(arguments) => {
                    if countdown > 0 {
                        return None;
                    }

                    return Some(FnMeta {
                        span: self.span(),
                        inputs: arguments.inputs.iter().collect(),
                        output: match &arguments.output {
                            ReturnType::Default => None,
                            ReturnType::Type(_, ty) => Some(ty.as_ref()),
                        },
                    });
                }
                PathArguments::AngleBracketed(..) => return None,
            }
        }

        None
    }

    fn matches_operator(self) -> Option<Operator> {
        for operator in Operator::enumerate() {
            let trait_name = format!("Script{}", operator.to_string());

            if let Some(_) = self.matches_bracketed(&[trait_name.as_str()], 0..=0) {
                return Some(*operator);
            }

            if let Some(_) =
                self.matches_bracketed(&["ad_astra", "runtime", "ops", trait_name.as_str()], 0..=0)
            {
                return Some(*operator);
            }
        }

        None
    }

    fn matches_fn(self) -> Option<FnMeta<'a>> {
        static FREE: [&'static str; 1] = ["Fn"];
        static STD: [&'static str; 3] = ["std", "ops", "Fn"];
        static CORE: [&'static str; 3] = ["core", "ops", "Fn"];

        if let Some(result) = self.matches_parenthesized(&FREE) {
            return Some(result);
        }

        if let Some(result) = self.matches_parenthesized(&STD) {
            return Some(result);
        }

        if let Some(result) = self.matches_parenthesized(&CORE) {
            return Some(result);
        }

        None
    }

    fn matches_rust_fn(self) -> Option<PathMeta<'a>> {
        for params in 0..8 {
            let type_name = format!("Fn{params}");
            let args = params + 1;

            if let Some(meta) = self.matches_bracketed(&[type_name.as_str()], args..=args) {
                return Some(meta);
            }

            if let Some(meta) = self.matches_bracketed(
                &["ad_astra", "runtime", "ops", type_name.as_str()],
                args..=args,
            ) {
                return Some(meta);
            }
        }

        None
    }

    fn matches_box(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Box"];
        static STD: [&'static str; 3] = ["std", "boxed", "Box"];
        static CORE: [&'static str; 3] = ["core", "boxed", "Box"];

        if let Some(result) = self.matches_bracketed(&FREE, 1..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 1..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 1..=1) {
            return Some(result);
        }

        None
    }

    fn matches_clone(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Clone"];
        static STD: [&'static str; 3] = ["std", "clone", "Clone"];
        static CORE: [&'static str; 3] = ["core", "clone", "Clone"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=0) {
            return Some(result);
        }

        None
    }

    fn matches_copy(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Copy"];
        static STD: [&'static str; 3] = ["std", "marker", "Copy"];
        static CORE: [&'static str; 3] = ["core", "marker", "Copy"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=0) {
            return Some(result);
        }

        None
    }

    fn matches_default(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Default"];
        static STD: [&'static str; 3] = ["std", "default", "Default"];
        static CORE: [&'static str; 3] = ["core", "default", "Default"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=0) {
            return Some(result);
        }

        None
    }

    fn matches_partial_eq(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["PartialEq"];
        static STD: [&'static str; 3] = ["std", "cmp", "PartialEq"];
        static CORE: [&'static str; 3] = ["core", "cmp", "PartialEq"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_eq(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Eq"];
        static STD: [&'static str; 3] = ["std", "cmp", "Eq"];
        static CORE: [&'static str; 3] = ["core", "cmp", "Eq"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=0) {
            return Some(result);
        }

        None
    }

    fn matches_partial_ord(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["PartialOrd"];
        static STD: [&'static str; 3] = ["std", "cmp", "PartialOrd"];
        static CORE: [&'static str; 3] = ["core", "cmp", "PartialOrd"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_ord(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Ord"];
        static STD: [&'static str; 3] = ["std", "cmp", "Ord"];
        static CORE: [&'static str; 3] = ["core", "cmp", "Ord"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=0) {
            return Some(result);
        }

        None
    }

    fn matches_hash(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Hash"];
        static STD: [&'static str; 3] = ["std", "hash", "Hash"];
        static CORE: [&'static str; 3] = ["core", "hash", "Hash"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=0) {
            return Some(result);
        }

        None
    }

    fn matches_debug(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Debug"];
        static STD: [&'static str; 3] = ["std", "fmt", "Debug"];
        static CORE: [&'static str; 3] = ["core", "fmt", "Debug"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=0) {
            return Some(result);
        }

        None
    }

    fn matches_display(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Display"];
        static STD: [&'static str; 3] = ["std", "fmt", "Display"];
        static CORE: [&'static str; 3] = ["core", "fmt", "Display"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=0) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=0) {
            return Some(result);
        }

        None
    }

    fn matches_add(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Add"];
        static STD: [&'static str; 3] = ["std", "ops", "Add"];
        static CORE: [&'static str; 3] = ["core", "ops", "Add"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_add_assign(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["AddAssign"];
        static STD: [&'static str; 3] = ["std", "ops", "AddAssign"];
        static CORE: [&'static str; 3] = ["core", "ops", "AddAssign"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_sub(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Sub"];
        static STD: [&'static str; 3] = ["std", "ops", "Sub"];
        static CORE: [&'static str; 3] = ["core", "ops", "Sub"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_sub_assign(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["SubAssign"];
        static STD: [&'static str; 3] = ["std", "ops", "SubAssign"];
        static CORE: [&'static str; 3] = ["core", "ops", "SubAssign"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_mul(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Mul"];
        static STD: [&'static str; 3] = ["std", "ops", "Mul"];
        static CORE: [&'static str; 3] = ["core", "ops", "Mul"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_mul_assign(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["MulAssign"];
        static STD: [&'static str; 3] = ["std", "ops", "MulAssign"];
        static CORE: [&'static str; 3] = ["core", "ops", "MulAssign"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_div(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Div"];
        static STD: [&'static str; 3] = ["std", "ops", "Div"];
        static CORE: [&'static str; 3] = ["core", "ops", "Div"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_div_assign(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["DivAssign"];
        static STD: [&'static str; 3] = ["std", "ops", "DivAssign"];
        static CORE: [&'static str; 3] = ["core", "ops", "DivAssign"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_not(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Not"];
        static STD: [&'static str; 3] = ["std", "ops", "Not"];
        static CORE: [&'static str; 3] = ["core", "ops", "Not"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_neg(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Neg"];
        static STD: [&'static str; 3] = ["std", "ops", "Neg"];
        static CORE: [&'static str; 3] = ["core", "ops", "Neg"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_bit_and(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["BitAnd"];
        static STD: [&'static str; 3] = ["std", "ops", "BitAnd"];
        static CORE: [&'static str; 3] = ["core", "ops", "BitAnd"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_bit_and_assign(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["BitAndAssign"];
        static STD: [&'static str; 3] = ["std", "ops", "BitAndAssign"];
        static CORE: [&'static str; 3] = ["core", "ops", "BitAndAssign"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_bit_or(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["BitOr"];
        static STD: [&'static str; 3] = ["std", "ops", "BitOr"];
        static CORE: [&'static str; 3] = ["core", "ops", "BitOr"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_bit_or_assign(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["BitOrAssign"];
        static STD: [&'static str; 3] = ["std", "ops", "BitOrAssign"];
        static CORE: [&'static str; 3] = ["core", "ops", "BitOrAssign"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_bit_xor(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["BitXor"];
        static STD: [&'static str; 3] = ["std", "ops", "BitXor"];
        static CORE: [&'static str; 3] = ["core", "ops", "BitXor"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_bit_xor_assign(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["BitXorAssign"];
        static STD: [&'static str; 3] = ["std", "ops", "BitXorAssign"];
        static CORE: [&'static str; 3] = ["core", "ops", "BitXorAssign"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_shl(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Shl"];
        static STD: [&'static str; 3] = ["std", "ops", "Shl"];
        static CORE: [&'static str; 3] = ["core", "ops", "Shl"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_shl_assign(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["ShlAssign"];
        static STD: [&'static str; 3] = ["std", "ops", "ShlAssign"];
        static CORE: [&'static str; 3] = ["core", "ops", "ShlAssign"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_shr(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Shr"];
        static STD: [&'static str; 3] = ["std", "ops", "Shr"];
        static CORE: [&'static str; 3] = ["core", "ops", "Shr"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_shr_assign(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["ShrAssign"];
        static STD: [&'static str; 3] = ["std", "ops", "ShrAssign"];
        static CORE: [&'static str; 3] = ["core", "ops", "ShrAssign"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_rem(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["Rem"];
        static STD: [&'static str; 3] = ["std", "ops", "Rem"];
        static CORE: [&'static str; 3] = ["core", "ops", "Rem"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }

    fn matches_rem_assign(self) -> Option<PathMeta<'a>> {
        static FREE: [&'static str; 1] = ["RemAssign"];
        static STD: [&'static str; 3] = ["std", "ops", "RemAssign"];
        static CORE: [&'static str; 3] = ["core", "ops", "RemAssign"];

        if let Some(result) = self.matches_bracketed(&FREE, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&STD, 0..=1) {
            return Some(result);
        }

        if let Some(result) = self.matches_bracketed(&CORE, 0..=1) {
            return Some(result);
        }

        None
    }
}

pub struct PathMeta<'a> {
    pub span: Span,
    pub args: Vec<&'a Type>,
}

pub struct FnMeta<'a> {
    pub span: Span,
    pub inputs: Vec<&'a Type>,
    pub output: Option<&'a Type>,
}
