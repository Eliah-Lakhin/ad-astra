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

use std::hash::BuildHasher;

use ahash::{AHashMap, AHashSet, AHasher, RandomState};

const SEED: [u64; 4] = [16854046535748765453, 34234, 123174618, 888452233587344];

#[inline(always)]
pub fn seed_random_state() -> RandomState {
    RandomState::with_seeds(SEED[0], SEED[1], SEED[2], SEED[3])
}

#[inline(always)]
pub fn seed_hasher() -> AHasher {
    seed_random_state().build_hasher()
}

#[inline(always)]
pub fn seed_hash_map<K, V>() -> AHashMap<K, V> {
    AHashMap::with_hasher(seed_random_state())
}

#[inline(always)]
pub fn seed_hash_map_with_capacity<K, V>(capacity: usize) -> AHashMap<K, V> {
    AHashMap::with_capacity_and_hasher(capacity, seed_random_state())
}

#[inline(always)]
pub fn seed_hash_set<K>() -> AHashSet<K> {
    AHashSet::with_hasher(seed_random_state())
}

#[inline(always)]
#[allow(unused)]
pub fn seed_hash_set_with_capacity<K>(capacity: usize) -> AHashSet<K> {
    AHashSet::with_capacity_and_hasher(capacity, seed_random_state())
}
