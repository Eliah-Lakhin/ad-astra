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

use std::{
    cell::RefCell,
    fmt::{Debug, Display, Formatter},
};

use crate::runtime::{Cell, Origin, RuntimeError};

pub(crate) struct Stringifier<'a> {
    pub(crate) origin: Origin,
    pub(crate) cell: &'a Cell,
    pub(crate) error: RefCell<Option<RuntimeError>>,
    pub(crate) fallback_to_type: bool,
}

impl<'a> Display for Stringifier<'a> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let result = self
            .cell
            .clone()
            .into_object()
            .display(self.origin, self.origin, formatter);

        if let Err(error) = result {
            let is_format_error = match &error {
                RuntimeError::FormatError { .. } => true,
                _ => false,
            };

            *self.error.borrow_mut() = Some(error);

            if is_format_error {
                return Err(std::fmt::Error);
            }

            if self.fallback_to_type {
                formatter.write_str(&format!("<{}>", self.cell.ty()))?;
            }
        }

        Ok(())
    }
}

impl<'a> Debug for Stringifier<'a> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let result = self
            .cell
            .clone()
            .into_object()
            .debug(self.origin, self.origin, formatter);

        if let Err(error) = result {
            let is_format_error = match &error {
                RuntimeError::FormatError { .. } => true,
                _ => false,
            };

            *self.error.borrow_mut() = Some(error);

            if is_format_error {
                return Err(std::fmt::Error);
            }

            if self.fallback_to_type {
                formatter.write_str(&format!("<{}>", self.cell.ty()))?;
            }
        }

        Ok(())
    }
}

impl Cell {
    /// A convenient utility function that formats Cell's data for debugging
    /// purposes.
    ///
    /// If the underlying [type](Cell::ty) of the Cell implements
    /// [display operations](crate::runtime::Prototype::implements_display), the
    /// function uses the Display implementation to format the data. Otherwise,
    /// if the type implements
    /// [debug operations](crate::runtime::Prototype::implements_debug),
    /// the function falls back to the Debug implementation.
    ///
    /// This function temporarily [borrows](Cell::borrow_ref) the underlying
    /// data. If the borrowing fails, or if the data type does not implement
    /// either Display or Debug traits, the result will be the signature of
    /// the type.
    ///
    /// The `alt` parameter specifies whether the function should use alternate
    /// formatting (such as `format!("{:#}")`) or not.
    #[inline(always)]
    pub fn stringify(&self, alt: bool) -> String {
        let stringifier = Stringifier {
            origin: Origin::nil(),
            cell: self,
            error: RefCell::new(None),
            fallback_to_type: true,
        };

        let result = match (self.ty().prototype().implements_display(), alt) {
            (true, true) => format!("{stringifier:#}"),
            (true, false) => format!("{stringifier:}"),
            (false, true) => format!("{stringifier:#?}"),
            (false, false) => format!("{stringifier:?}"),
        };

        result
    }
}

macro_rules! transparent_upcast {
    ($ty:ty) => {
        impl<'a> $crate::runtime::Upcast<'a> for $ty {
            type Output = ::std::boxed::Box<$ty>;

            #[inline(always)]
            fn upcast(
                _origin: $crate::runtime::Origin,
                this: Self,
            ) -> $crate::runtime::RuntimeResult<Self::Output> {
                ::std::result::Result::Ok(::std::boxed::Box::new(this))
            }

            #[inline(always)]
            fn hint() -> $crate::runtime::TypeHint {
                $crate::runtime::TypeHint::Type(<$ty as $crate::runtime::ScriptType>::type_meta())
            }
        }

        impl<'a> $crate::runtime::Upcast<'a> for &'a $ty {
            type Output = &'a $ty;

            #[inline(always)]
            fn upcast(
                _origin: $crate::runtime::Origin,
                this: Self,
            ) -> $crate::runtime::RuntimeResult<Self::Output> {
                ::std::result::Result::Ok(this)
            }

            #[inline(always)]
            fn hint() -> $crate::runtime::TypeHint {
                $crate::runtime::TypeHint::Type(<$ty as $crate::runtime::ScriptType>::type_meta())
            }
        }

        impl<'a> $crate::runtime::Upcast<'a> for &'a mut $ty {
            type Output = &'a mut $ty;

            #[inline(always)]
            fn upcast(
                _origin: $crate::runtime::Origin,
                this: Self,
            ) -> $crate::runtime::RuntimeResult<Self::Output> {
                ::std::result::Result::Ok(this)
            }

            #[inline(always)]
            fn hint() -> $crate::runtime::TypeHint {
                $crate::runtime::TypeHint::Type(<$ty as $crate::runtime::ScriptType>::type_meta())
            }
        }
    };
}

pub(crate) use transparent_upcast;
