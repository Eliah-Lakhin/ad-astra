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
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
};

use strsim::normalized_damerau_levenshtein;

const EPSILON: f32 = 0.0001;

/// A score representing the distance between two strings.
///
/// The score is measured in terms of percentage with fractional precision.
///
/// "100%" indicates that the estimated string exactly matches the pattern
/// string, while "0%" indicates they are completely distinct.
///
/// The Debug and Display implementations of this object round the underlying
/// percentage to the nearest integer. The default value is "0%".
///
/// The [StringEstimation::estimate] function estimates the distance
/// between two strings and returns a `Closeness` value.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Closeness(f32);

impl Debug for Closeness {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, formatter)
    }
}

impl Display for Closeness {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_fmt(format_args!("{}%", self.percents()))
    }
}

impl PartialEq for Closeness {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.normalized().eq(&other.normalized())
    }
}

impl Eq for Closeness {}

impl PartialOrd for Closeness {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Closeness {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.normalized().cmp(&other.normalized())
    }
}

impl Hash for Closeness {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.normalized().hash(state)
    }
}

impl Default for Closeness {
    #[inline(always)]
    fn default() -> Self {
        Self::zero()
    }
}

impl Closeness {
    /// Returns a "0%" closeness value.
    #[inline(always)]
    pub const fn zero() -> Self {
        Self(0.0)
    }

    /// Returns a "50%" closeness value.
    #[inline(always)]
    pub const fn half() -> Self {
        Self(0.5)
    }

    /// Returns a "100%" closeness value.
    #[inline(always)]
    pub const fn one() -> Self {
        Self(1.0)
    }

    /// Returns the underlying percentage value rounded up to the nearest
    /// integer.
    #[inline(always)]
    pub fn percents(self) -> u16 {
        ((self.0 * 1000.0).round() / 10.0) as u16
    }

    #[inline(always)]
    fn normalized(self) -> u32 {
        (self.0 / EPSILON) as u32
    }
}

/// An extension trait for strings that estimates the distance between two
/// strings.
pub trait StringEstimation {
    /// Estimates the similarity between two strings.
    ///
    /// The returned [Closeness] object represents the similarity between
    /// the provided string and the specified `pattern` in terms of percentage,
    /// with fractional precision. A value of "100%" ([Closeness::one]) means
    /// that the string fully matches the pattern.
    ///
    /// ```rust
    /// use ad_astra::analysis::{Closeness, StringEstimation};
    ///
    /// assert_eq!("foo".estimate("foo"), Closeness::one());
    /// assert_eq!("foo".estimate("aaa"), Closeness::zero());
    ///
    /// println!("{}", "foo".estimate("Foo")); // ~ 66%
    /// println!("{}", "Bra".estimate("bar")); // ~ 33%
    /// ```
    fn estimate(&self, pattern: impl AsRef<str>) -> Closeness;
}

impl<S: AsRef<str>> StringEstimation for S {
    fn estimate(&self, pattern: impl AsRef<str>) -> Closeness {
        let this = self.as_ref();
        let pattern = pattern.as_ref();

        let closeness = normalized_damerau_levenshtein(pattern, this);

        Closeness((closeness as f32 / EPSILON) as usize as f32 * EPSILON)
    }
}
