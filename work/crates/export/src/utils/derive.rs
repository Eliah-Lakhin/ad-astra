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

use proc_macro2::Span;
use syn::{Attribute, Result};

use crate::utils::PathUtils;

#[derive(Default)]
pub struct DeriveMeta {
    clone: Option<Span>,
    copy: Option<Span>,
    default: Option<Span>,
    partial_eq: Option<Span>,
    eq: Option<Span>,
    partial_ord: Option<Span>,
    ord: Option<Span>,
    hash: Option<Span>,
    debug: Option<Span>,
    display: Option<Span>,
}

impl DeriveMeta {
    #[inline]
    pub fn impls_clone(&self) -> Option<Span> {
        if self.clone.is_some() {
            return self.clone;
        }

        if self.copy.is_some() {
            return self.copy;
        }

        None
    }

    #[inline]
    pub fn impls_debug(&self) -> Option<Span> {
        self.debug
    }

    #[inline]
    pub fn impls_partial_eq(&self) -> Option<Span> {
        if self.partial_eq.is_some() {
            return self.partial_eq;
        }

        if self.eq.is_some() {
            return self.eq;
        }

        None
    }

    #[inline]
    pub fn impls_default(&self) -> Option<Span> {
        self.default
    }

    #[inline]
    pub fn impls_partial_ord(&self) -> Option<Span> {
        if self.partial_ord.is_some() {
            return self.partial_ord;
        }

        if self.ord.is_some() {
            return self.ord;
        }

        None
    }

    #[inline]
    pub fn impls_ord(&self) -> Option<Span> {
        self.ord
    }

    #[inline]
    pub fn impls_hash(&self) -> Option<Span> {
        self.hash
    }

    pub(super) fn enrich(&mut self, attribute: &Attribute) -> Result<()> {
        if !attribute.path().is_ident("derive") {
            return Ok(());
        }

        attribute.parse_nested_meta(|meta| {
            if let Some(meta) = meta.path.matches_clone() {
                self.clone = Some(meta.span);
            }

            if let Some(meta) = meta.path.matches_copy() {
                self.copy = Some(meta.span);
            }

            if let Some(meta) = meta.path.matches_default() {
                self.default = Some(meta.span);
            }

            if let Some(meta) = meta.path.matches_partial_eq() {
                self.partial_eq = Some(meta.span);
            }

            if let Some(meta) = meta.path.matches_eq() {
                self.eq = Some(meta.span);
            }

            if let Some(meta) = meta.path.matches_partial_ord() {
                self.partial_ord = Some(meta.span);
            }

            if let Some(meta) = meta.path.matches_ord() {
                self.ord = Some(meta.span);
            }

            if let Some(meta) = meta.path.matches_hash() {
                self.hash = Some(meta.span);
            }

            if let Some(meta) = meta.path.matches_debug() {
                self.debug = Some(meta.span);
            }

            if let Some(meta) = meta.path.matches_display() {
                self.display = Some(meta.span);
            }

            Ok(())
        })?;

        Ok(())
    }
}
