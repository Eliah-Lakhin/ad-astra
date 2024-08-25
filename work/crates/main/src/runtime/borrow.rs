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
    cell::UnsafeCell,
    fmt::{Debug, Formatter},
    hint::spin_loop,
    mem::replace,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    report::{debug_unreachable, system_panic},
    runtime::{Origin, RuntimeError, RuntimeResult},
};

const BORROW_LIMIT: u32 = 64;

#[repr(transparent)]
pub(super) struct BorrowTable(SpinMutex<BorrowTableInner>);

impl Debug for BorrowTable {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let access = self.0.lock();

        let mut debug_struct = formatter.debug_struct("BorrowTable");

        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        unsafe {
            let value_refs = &*access.value_refs.get();

            debug_struct.field("value_refs", value_refs);
        }

        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        unsafe {
            let value_mut = &*access.value_mut.get();

            match value_mut {
                None => debug_struct.field("value_mut", &[] as &[&Origin; 0]),
                Some(origin) => debug_struct.field("value_mut", &[origin]),
            };
        }

        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        unsafe {
            let place_refs = &*access.place_refs.get();

            debug_struct.field("place_refs", place_refs);
        }

        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        unsafe {
            let place_muts = &*access.place_muts.get();

            debug_struct.field("place_muts", place_muts);
        }

        debug_struct.finish()
    }
}

impl BorrowTable {
    #[inline(always)]
    pub(super) fn new() -> Self {
        Self(SpinMutex::new(BorrowTableInner {
            value_refs: UnsafeCell::new(BorrowStack::new()),
            value_mut: UnsafeCell::new(None),
            place_refs: UnsafeCell::new(BorrowStack::new()),
            place_muts: UnsafeCell::new(BorrowStack::new()),
        }))
    }

    #[inline(always)]
    pub(super) fn access(&self) -> BorrowTableAccess {
        BorrowTableAccess(self.0.lock())
    }
}

#[repr(transparent)]
pub(super) struct BorrowTableAccess<'a>(SpinGuard<'a, BorrowTableInner>);

impl BorrowTableAccess<'_> {
    #[inline(always)]
    pub(super) fn grant_value_ref(&self, origin: Origin) -> RuntimeResult<u32> {
        {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let value_mut = unsafe { &*self.0.value_mut.get() };

            if let Some(cause) = value_mut {
                return Err(RuntimeError::WriteToRead {
                    access_origin: origin,
                    borrow_origin: *cause,
                });
            }
        }

        {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let place_muts = unsafe { &*self.0.place_muts.get() };

            if let Some(cause) = place_muts.last() {
                return Err(RuntimeError::WriteToRead {
                    access_origin: origin,
                    borrow_origin: *cause,
                });
            }
        }

        let index = {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let value_refs = unsafe { &mut *self.0.value_refs.get() };

            value_refs.occupy(origin)?
        };
        Ok(index)
    }

    #[inline(always)]
    pub(super) fn grant_value_mut(&self, origin: Origin) -> RuntimeResult<u32> {
        {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let value_refs = unsafe { &*self.0.value_refs.get() };

            if let Some(cause) = value_refs.last() {
                return Err(RuntimeError::ReadToWrite {
                    access_origin: origin,
                    borrow_origin: *cause,
                });
            }
        }

        {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let place_refs = unsafe { &*self.0.place_refs.get() };

            if let Some(cause) = place_refs.last() {
                return Err(RuntimeError::ReadToWrite {
                    access_origin: origin,
                    borrow_origin: *cause,
                });
            }
        }

        {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let place_muts = unsafe { &*self.0.place_muts.get() };

            if let Some(cause) = place_muts.last() {
                return Err(RuntimeError::WriteToWrite {
                    access_origin: origin,
                    borrow_origin: *cause,
                });
            }
        }

        {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let value_mut = unsafe { &mut *self.0.value_mut.get() };

            if let Some(cause) = value_mut {
                return Err(RuntimeError::WriteToWrite {
                    access_origin: origin,
                    borrow_origin: *cause,
                });
            }

            *value_mut = Some(origin);
        };

        Ok(0)
    }

    #[inline(always)]
    pub(super) fn grant_place_ref(&self, origin: Origin) -> RuntimeResult<u32> {
        {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let value_mut = unsafe { &*self.0.value_mut.get() };

            if let Some(cause) = value_mut {
                return Err(RuntimeError::WriteToRead {
                    access_origin: origin,
                    borrow_origin: *cause,
                });
            }
        }

        let index = {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let place_refs = unsafe { &mut *self.0.place_refs.get() };

            place_refs.occupy(origin)?
        };

        Ok(index)
    }

    #[inline(always)]
    pub(super) fn grant_place_mut(&self, origin: Origin) -> RuntimeResult<u32> {
        {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let value_mut = unsafe { &*self.0.value_mut.get() };

            if let Some(cause) = value_mut {
                return Err(RuntimeError::WriteToWrite {
                    access_origin: origin,
                    borrow_origin: *cause,
                });
            }
        }

        {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let value_refs = unsafe { &*self.0.value_refs.get() };

            if let Some(cause) = value_refs.last() {
                return Err(RuntimeError::ReadToWrite {
                    access_origin: origin,
                    borrow_origin: *cause,
                });
            }
        }

        let index = {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let place_muts = unsafe { &mut *self.0.place_muts.get() };

            place_muts.occupy(origin)?
        };

        Ok(index)
    }

    // Safety: index was granted by corresponding function and never released before.
    #[inline(always)]
    pub(super) unsafe fn release_value_ref(&self, index: u32) {
        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let value_refs = unsafe { &mut *self.0.value_refs.get() };

        // Safety: Upheld by the caller.
        let _ = unsafe { value_refs.release(index) };
    }

    // Safety: index was granted by corresponding function and never released before.
    #[inline(always)]
    pub(super) unsafe fn release_value_mut(&self, _index: u32) {
        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let value_mut = unsafe { &mut *self.0.value_mut.get() };

        if replace(value_mut, None).is_none() {
            // Safety: Upheld by the caller.
            unsafe { debug_unreachable!("ValueMut releasing failure.") }
        }
    }

    // Safety: index was granted by corresponding function and never released before.
    #[inline(always)]
    pub(super) unsafe fn release_place_ref(&self, index: u32) {
        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let place_refs = unsafe { &mut *self.0.place_refs.get() };

        // Safety: Upheld by the caller.
        let _ = unsafe { place_refs.release(index) };
    }

    // Safety: index was granted by corresponding function and never released before.
    #[inline(always)]
    pub(super) unsafe fn release_place_mut(&self, index: u32) {
        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let place_muts = unsafe { &mut *self.0.place_muts.get() };

        // Safety: Upheld by the caller.
        let _ = unsafe { place_muts.release(index) };
    }

    // Safety: index was granted by corresponding function and never released before.
    #[inline(always)]
    pub(super) unsafe fn get_value_ref(&self, index: u32) -> Origin {
        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let value_refs = unsafe { &*self.0.value_refs.get() };

        // Safety: Upheld by the caller.
        *unsafe { value_refs.get(index) }
    }

    // Safety: index was granted by corresponding function and never released before.
    #[inline(always)]
    pub(super) unsafe fn get_value_mut(&self, _index: u32) -> Origin {
        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let value_mut = unsafe { &*self.0.value_mut.get() };

        match value_mut {
            Some(origin) => *origin,

            None => {
                // Safety: Upheld by the caller.
                unsafe { debug_unreachable!("ValueMut releasing failure.") }
            }
        }
    }

    // Safety: index was granted by corresponding function and never released before.
    #[inline(always)]
    pub(super) unsafe fn get_place_ref(&self, index: u32) -> Origin {
        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let place_refs = unsafe { &*self.0.place_refs.get() };

        // Safety: Upheld by the caller.
        *unsafe { place_refs.get(index) }
    }

    // Safety: index was granted by corresponding function and never released before.
    #[inline(always)]
    pub(super) unsafe fn get_place_mut(&self, index: u32) -> Origin {
        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let place_muts = unsafe { &*self.0.place_muts.get() };

        // Safety: Upheld by the caller.
        *unsafe { place_muts.get(index) }
    }

    #[cfg(debug_assertions)]
    #[inline(always)]
    pub(super) fn assert_value(&self) {
        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let value_refs = unsafe { &*self.0.value_refs.get() };

        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let value_mut = unsafe { &*self.0.value_mut.get() };

        if value_refs.is_empty() && value_mut.is_none() {
            system_panic!("Neither ValueRef nor ValueMut access granted.")
        }
    }

    #[cfg(debug_assertions)]
    #[inline(always)]
    pub(super) fn assert_value_mut(&self) {
        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let value_mut = unsafe { &*self.0.value_mut.get() };

        if value_mut.is_none() {
            system_panic!("ValueMut access is not granted.")
        }
    }

    #[cfg(debug_assertions)]
    #[inline(always)]
    pub(super) fn assert_place(&self) {
        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let place_refs = unsafe { &*self.0.place_refs.get() };

        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let place_muts = unsafe { &*self.0.place_muts.get() };

        if place_refs.is_empty() && place_muts.is_empty() {
            system_panic!("Neither PlaceRef nor PlaceMut access granted.")
        }
    }

    #[cfg(debug_assertions)]
    #[inline(always)]
    pub(super) fn assert_place_mut(&self) {
        // Safety:
        //   1. Data race is guarded by the mutex.
        //   2. BorrowTable access is always localized within a single thread.
        let place_muts = unsafe { &*self.0.place_muts.get() };

        if place_muts.is_empty() {
            system_panic!("PlaceMut access is not granted.")
        }
    }

    #[cfg(debug_assertions)]
    #[inline(always)]
    pub(super) fn assert_empty(&self) {
        {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let value_refs = unsafe { &*self.0.value_refs.get() };

            if let Some(origin) = value_refs.last() {
                system_panic!("Unreleased ValueRef access {:?}.", origin,)
            }
        }

        {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let value_mut = unsafe { &*self.0.value_mut.get() };

            if let Some(origin) = value_mut {
                system_panic!("Unreleased ValueMut access {:?}.", origin,)
            }
        }

        {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let place_refs = unsafe { &*self.0.place_refs.get() };

            if let Some(origin) = place_refs.last() {
                system_panic!("Unreleased PlaceRef access {:?}.", origin,)
            }
        }

        {
            // Safety:
            //   1. Data race is guarded by the mutex.
            //   2. BorrowTable access is always localized within a single thread.
            let place_muts = unsafe { &*self.0.place_muts.get() };

            if let Some(origin) = place_muts.last() {
                system_panic!("Unreleased PlaceMut access {:?}.", origin,)
            }
        }
    }
}

struct BorrowTableInner {
    value_refs: UnsafeCell<BorrowStack>,
    value_mut: UnsafeCell<Option<Origin>>,
    place_refs: UnsafeCell<BorrowStack>,
    place_muts: UnsafeCell<BorrowStack>,
}

pub(super) struct BorrowStack {
    entries: Vec<BorrowEntry>,
    next: u32,
    last: u32,
}

impl Debug for BorrowStack {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let mut debug_list = formatter.debug_list();

        let mut index = self.last as usize;

        loop {
            let occupied = match self.entries.get(index) {
                None => break,
                Some(occupied) => occupied,
            };

            match occupied {
                BorrowEntry::Occupied {
                    previous, origin, ..
                } => {
                    debug_list.entry(origin);
                    index = *previous as usize;
                }

                BorrowEntry::Vacant(..) => {
                    // Safety: If `last` is a valid index, it always points to
                    // Occupied entry.
                    unsafe { debug_unreachable!("Invalid BorrowStack entry index.") }
                }
            }
        }

        debug_list.finish()
    }
}

impl BorrowStack {
    #[inline(always)]
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            next: 0,
            last: u32::MAX,
        }
    }

    #[must_use]
    #[inline]
    fn last(&self) -> Option<&Origin> {
        let occupied = self.entries.get(self.last as usize)?;

        match occupied {
            BorrowEntry::Occupied { origin, .. } => Some(origin),

            BorrowEntry::Vacant(..) => {
                // Safety: If `last` is a valid index, it always points to
                // Occupied entry.
                unsafe { debug_unreachable!("Last index points to vacant entry.") }
            }
        }
    }

    #[cfg(debug_assertions)]
    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.last >= self.entries.len() as u32
    }

    #[inline]
    fn occupy(&mut self, origin: Origin) -> RuntimeResult<u32> {
        let index = self.next;

        if index == BORROW_LIMIT {
            return Err(RuntimeError::BorrowLimit {
                access_origin: origin,
                limit: BORROW_LIMIT as usize,
            });
        }

        match self.entries.get_mut(self.next as usize) {
            None => {
                let previous = self.last;

                self.last = index;
                self.next += 1;
                self.entries.push(BorrowEntry::Occupied {
                    previous,
                    next: self.next,
                    origin,
                });
            }

            Some(vacant) => {
                debug_assert!(
                    matches!(vacant, BorrowEntry::Vacant(..)),
                    "Occupied entry in the next position.",
                );

                let previous = self.last;

                self.last = index;
                self.next = match vacant {
                    BorrowEntry::Vacant(next) => *next,

                    _ => {
                        // Safety: `next` always points to Vacant entry.
                        unsafe { debug_unreachable!("Occupied entry in the next position") }
                    }
                };

                *vacant = BorrowEntry::Occupied {
                    previous,
                    next: self.next,
                    origin,
                }
            }
        }

        Ok(index)
    }

    // Safety: `index` points to occupied entry.
    #[must_use]
    #[inline]
    unsafe fn get(&self, index: u32) -> &Origin {
        debug_assert!(
            (index as usize) < self.entries.len(),
            "Index out of bounds."
        );

        match self.entries.get(index as usize) {
            Some(occupied) => {
                match occupied {
                    BorrowEntry::Occupied { origin, .. } => origin,

                    BorrowEntry::Vacant(..) => {
                        // Safety: Upheld by the caller.
                        unsafe { debug_unreachable!("Vacant entry in occupied position.") }
                    }
                }
            }

            None => {
                // Safety: Upheld by the caller.
                unsafe { debug_unreachable!("Index out of bounds.") }
            }
        }
    }

    // Safety: `index` points to occupied entry.
    #[must_use]
    #[inline]
    unsafe fn release(&mut self, index: u32) -> Origin {
        debug_assert!(
            (index as usize) < self.entries.len(),
            "Index out of bounds."
        );

        match self.entries.get_mut(index as usize) {
            Some(occupied) => {
                let origin = match replace(occupied, BorrowEntry::Vacant(self.next)) {
                    BorrowEntry::Occupied {
                        previous,
                        next,
                        origin,
                    } => {
                        if index == self.last {
                            self.last = previous;
                        }

                        match self.entries.get_mut(previous as usize) {
                            Some(BorrowEntry::Occupied {
                                next: previous_next,
                                ..
                            }) => {
                                *previous_next = next;
                            }

                            _ => (),
                        }

                        match self.entries.get_mut(next as usize) {
                            Some(BorrowEntry::Occupied {
                                previous: next_previous,
                                ..
                            }) => {
                                *next_previous = previous;
                            }

                            _ => (),
                        }

                        origin
                    }

                    BorrowEntry::Vacant(..) => {
                        // Safety: Upheld by the caller.
                        unsafe { debug_unreachable!("Vacant entry in occupied position.") }
                    }
                };

                self.next = index;

                origin
            }

            None => {
                // Safety: Upheld by the caller.
                unsafe { debug_unreachable!("Index out of bounds.") }
            }
        }
    }
}

enum BorrowEntry {
    Occupied {
        previous: u32,
        next: u32,
        origin: Origin,
    },
    Vacant(u32),
}

struct SpinMutex<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> SpinMutex<T> {
    #[inline(always)]
    fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    #[inline(always)]
    fn lock(&self) -> SpinGuard<T> {
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            while self
                .lock
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {
                spin_loop()
            }
        }

        SpinGuard { mutex: &self }
    }
}

#[repr(transparent)]
struct SpinGuard<'a, T> {
    mutex: &'a SpinMutex<T>,
}

impl<'a, T> Drop for SpinGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.lock.store(false, Ordering::Release);
    }
}

impl<'a, T> Deref for SpinGuard<'a, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        // Safety: SpinGuard provides unique access.
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for SpinGuard<'a, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: SpinGuard provides unique access.
        unsafe { &mut *self.mutex.data.get() }
    }
}
