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
    any::TypeId,
    fmt::{Debug, Formatter},
    mem::{transmute, ManuallyDrop},
    num::NonZeroUsize,
    ops::Deref,
    ptr::{null, null_mut, NonNull},
    slice::{from_raw_parts, from_raw_parts_mut},
    sync::{Arc, Weak},
};

use ahash::RandomState;
use lady_deirdre::sync::{Lazy, Table};

use crate::{
    report::debug_unreachable,
    runtime::{borrow::BorrowTable, Origin, RuntimeResult, ScriptType, TypeMeta},
};

#[repr(transparent)]
pub struct MemorySlice(MemorySliceInner);

impl Debug for MemorySlice {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, formatter)
    }
}

impl Drop for MemorySlice {
    fn drop(&mut self) {
        let drop_fn = match self.0.drop_fn {
            Some(drop_fn) => drop_fn,
            None => return,
        };

        let head = match self.0.head_mut {
            Some(head) => head,

            // Owned MemorySlice's head_mut and head_ref are always specified and equal.
            None => unsafe { debug_unreachable!("Owned MemorySlice without head_mut.") },
        };

        // Safety: Ownership checked above.
        unsafe { drop_fn(head.as_ptr(), self.0.length, self.0.capacity) }

        if !self.0.is_void() {
            let registry = MemoryRegistry::get();

            // Safety: Owned non-void MemorySlice is registered entry.
            unsafe { registry.deregister(head.address()) };
        }

        #[cfg(debug_assertions)]
        {
            self.0.table.access().assert_empty()
        }
    }
}

impl MemorySlice {
    #[inline(always)]
    pub(super) fn register_string(origin: Origin, string: String) -> RuntimeResult<Arc<Self>> {
        let vector = string.into_bytes();

        // Safety: UNICODE const set to true, because the byte vector
        //         originated from String.
        unsafe { Self::register_owned_slice::<true, u8>(origin, vector) }
    }

    #[inline(always)]
    pub(super) fn register_vec<T: ScriptType>(
        origin: Origin,
        vector: Vec<T>,
    ) -> RuntimeResult<Arc<Self>> {
        // Safety: UNICODE const set to false.
        unsafe { Self::register_owned_slice::<false, T>(origin, vector) }
    }

    #[inline(always)]
    pub(super) fn register_str(origin: Origin, string: &str) -> RuntimeResult<Arc<MemorySlice>> {
        let slice = string.as_bytes();
        let length = slice.len();

        // Safety: Provided arguments correctly describe unicode `slice`.
        unsafe {
            Self::register_raw_slice::<true>(
                origin,
                <u8>::type_meta(),
                slice as *const [u8] as *mut (),
                null_mut(),
                length,
            )
        }
    }

    #[inline(always)]
    pub(super) fn register_slice_ref<T: ScriptType>(
        origin: Origin,
        slice: &[T],
    ) -> RuntimeResult<Arc<MemorySlice>> {
        let length = slice.len();

        // Safety: Provided arguments correctly describe `slice`.
        unsafe {
            Self::register_raw_slice::<false>(
                origin,
                <T>::type_meta(),
                slice as *const [T] as *mut (),
                null_mut(),
                length,
            )
        }
    }

    #[inline(always)]
    pub(super) fn register_slice_mut<T: ScriptType>(
        origin: Origin,
        slice: &mut [T],
    ) -> RuntimeResult<Arc<MemorySlice>> {
        let length = slice.len();

        // Safety: Provided arguments correctly describe `slice`.
        unsafe {
            Self::register_raw_slice::<false>(
                origin,
                <T>::type_meta(),
                null(),
                slice as *mut [T] as *mut (),
                length,
            )
        }
    }

    // Safety:
    //   1. If `by_ref` and `by_mut` non null both, they have the same address.
    //   2. This address points to allocated and fully initialized well-formed
    //      instance of `T`.
    //   3. If `by_ref` non null, it provides immutable access to the instance.
    //   4. If `by_mut` non null, it provides mutable access to the instance.
    #[inline(always)]
    pub(super) unsafe fn register_ptr<T: ScriptType>(
        origin: Origin,
        by_ref: *const T,
        by_mut: *mut T,
    ) -> RuntimeResult<Arc<Self>> {
        unsafe {
            Self::register_raw_slice::<false>(
                origin,
                <T>::type_meta(),
                by_ref as *const (),
                by_mut as *mut (),
                1,
            )
        }
    }

    // Safety:
    //   1. If `by_ref` and `by_mut` non null both, they have the same address.
    //   2. This address points to allocated and fully initialized well-formed
    //      slice of `length` items of type `T`.
    //   3. If `by_ref` non null, it provides immutable access to the slice.
    //   4. If `by_mut` non null, it provides mutable access to the slice.
    //   5. If `UNICODE` set to true, `T` is `u8` and the slice is utf8-safe to decode.
    //   6. If `UNICODE` set to true, `by_mut` is null.
    unsafe fn register_raw_slice<const UNICODE: bool>(
        origin: Origin,
        ty: &'static TypeMeta,
        by_ref: *const (),
        by_mut: *mut (),
        length: usize,
    ) -> RuntimeResult<Arc<Self>> {
        if UNICODE {
            if ty != &TypeId::of::<u8>() {
                // Safety: Upheld by 5.
                unsafe { debug_unreachable!("Unicode slice item is not of u8 type.") }
            }

            if !by_mut.is_null() {
                // Safety: Upheld by 6.
                unsafe { debug_unreachable!("Mutable pointer to unicode slice.") }
            }
        }

        if length > 0 && ty.size() > 0 {
            let address = if !by_ref.is_null() {
                // Safety: Nullability checked above.
                by_ref as usize
            } else if !by_mut.is_null() {
                // Safety: Nullability checked above.
                by_mut as usize
            } else {
                0
            };

            if address > 0 {
                // Safety: Nullability checked above.
                let key = unsafe { NonZeroUsize::new_unchecked(address) };

                let registry = MemoryRegistry::get();

                let shard_index = registry.inner.shard_index_of(&key);

                let shard = match registry.inner.shards().get(shard_index) {
                    Some(shard) => shard,

                    // Safety: shard_of function always returns valid shard index.
                    None => unsafe { debug_unreachable!("Shard index out of bounds") },
                };

                let guard = shard.read().unwrap_or_else(|poison| poison.into_inner());

                if let Some(weak_entry) = guard.get(&key) {
                    if let Some(strong_entry) = weak_entry.upgrade() {
                        if strong_entry.0.ty == ty && strong_entry.0.length == length {
                            return Ok(strong_entry);
                        }
                    }
                }
            }
        }

        Ok(Arc::new(MemorySlice(MemorySliceInner {
            unicode: UNICODE,
            origin,
            ty,
            head_ref: NonNull::new(by_ref as *mut ()),
            head_mut: NonNull::new(by_mut),
            length,
            capacity: length,
            table: BorrowTable::new(),
            drop_fn: None,
        })))
    }

    // Safety: If `UNICODE` set to true, T is `u8` and the slice is utf8-safe to decode.
    unsafe fn register_owned_slice<const UNICODE: bool, T: ScriptType>(
        origin: Origin,
        vector: Vec<T>,
    ) -> RuntimeResult<Arc<Self>> {
        let ty = T::type_meta();

        if UNICODE {
            if ty != &TypeId::of::<u8>() {
                unsafe { debug_unreachable!("Unicode slice item is not of u8 type.") }
            }
        }

        let head;
        let length;
        let capacity;

        {
            let mut vector = ManuallyDrop::new(vector);

            // Safety: `Vector::as_mut_ptr` returns possibly dangling,
            //         but non null pointer.
            head = unsafe { NonNull::new_unchecked(vector.as_mut_ptr() as *mut ()) };
            length = vector.len();
            capacity = vector.capacity();
        }

        let strong_entry = Arc::new(Self(MemorySliceInner {
            unicode: UNICODE,
            origin,
            ty,
            head_ref: Some(head),
            head_mut: Some(head),
            length,
            capacity,
            table: BorrowTable::new(),
            drop_fn: Some(drop_vec::<T>),
        }));

        if length > 0 && ty.size() > 0 {
            let registry = MemoryRegistry::get();

            let key = head.address();
            let shard_index = registry.inner.shard_index_of(&key);

            let shard = match registry.inner.shards().get(shard_index) {
                Some(shard) => shard,

                // Safety: shard_of function always returns valid shard index.
                None => unsafe { debug_unreachable!("Shard index out of bounds") },
            };

            {
                let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

                match guard.insert(key, Arc::downgrade(&strong_entry)) {
                    None => (),

                    Some(_) => {
                        // Safety: Duplicate Box addresses do not exist.
                        unsafe { debug_unreachable!("Duplicate MemorySlice address.") }
                    }
                }
            }
        }

        Ok(strong_entry)
    }

    #[inline(always)]
    pub(super) fn data_origin(&self) -> &Origin {
        &self.0.origin
    }

    #[inline(always)]
    pub(super) fn ty(&self) -> &'static TypeMeta {
        self.0.ty
    }

    #[inline(always)]
    pub(super) fn length(&self) -> usize {
        self.0.length
    }

    #[inline(always)]
    pub(super) fn is_owned(&self) -> bool {
        self.0.is_owned()
    }

    #[inline(always)]
    pub(super) fn is_unicode(&self) -> bool {
        self.0.unicode
    }

    #[inline(always)]
    pub(super) fn is_readable(&self) -> bool {
        self.0.head_ref.is_some()
    }

    #[inline(always)]
    pub(super) fn is_writeable(&self) -> bool {
        self.0.head_mut.is_some() && !self.0.unicode
    }

    // Safety:
    //   1. If the MemorySlice is readable, PlaceRef or PlaceMut access granted.
    //   2. If the MemorySlice is writeable, PlaceMut access granted.
    //   3. `start_bound <= end_bound`.
    //   4. If the underlying item type size > 0, then `end_bound <= self.length()`.
    pub(super) unsafe fn subslice(
        &self,
        origin: Origin,
        start_bound: usize,
        end_bound: usize,
    ) -> RuntimeResult<Arc<Self>> {
        if start_bound > end_bound {
            // Safety: Upheld by 3.
            unsafe { debug_unreachable!("Malformed range.") }
        }

        let size = self.0.ty.size();

        if end_bound > self.0.length && size > 0 {
            // Safety: Upheld by 4.
            unsafe { debug_unreachable!("Range out of bounds.") }
        }

        let by_ref = match self.0.head_ref {
            Some(head_ref) => unsafe {
                (head_ref.as_ptr() as *const ()).byte_add(start_bound * size)
            },
            None => null(),
        };

        let by_mut = match self.0.head_mut {
            Some(head_mut) if !self.0.unicode => unsafe {
                head_mut.as_ptr().byte_add(start_bound * size)
            },
            _ => null_mut(),
        };

        unsafe {
            Self::register_raw_slice::<false>(
                origin,
                self.0.ty,
                by_ref,
                by_mut,
                end_bound - start_bound,
            )
        }
    }

    // Safety:
    //   1. MemorySlice is owned.
    //   2. T properly describes underlying item type.
    //   3. There are no borrow-grants into this MemorySlice.
    pub(super) unsafe fn into_vec<T: 'static>(self) -> Vec<T> {
        // Safety: Transparent layout transmutation.
        let this = unsafe { transmute::<MemorySlice, MemorySliceInner>(self) };

        if !this.is_owned() {
            // Safety: Upheld by 1.
            unsafe { debug_unreachable!("MemorySlice is not owned.") }
        }

        if this.ty != &TypeId::of::<T>() {
            // Safety: Upheld by 2.
            unsafe { debug_unreachable!("MemorySlice type mismatch.") }
        }

        #[cfg(debug_assertions)]
        {
            this.table.access().assert_empty();
        }

        let head = match this.head_mut {
            Some(head) => head,

            // Owned MemorySlice's head_mut and head_ref are always specified and equal.
            None => unsafe { debug_unreachable!("Owned MemorySlice without head_mut.") },
        };

        // Safety:
        //  1. Type checked above.
        //  2. Slice source(owned) checked above.
        //  3. Downcasting slice to vector without additional capacity.
        let vector =
            unsafe { Vec::from_raw_parts(head.as_ptr() as *mut T, this.length, this.capacity) };

        if !this.is_void() {
            let registry = MemoryRegistry::get();

            // Safety: Non-void owned MemorySlice always registered.
            unsafe { registry.deregister(head.address()) }
        }

        vector
    }

    // Safety:
    //   1. ValueRef or ValueMut access granted.
    //   2. T properly describes underlying item type.
    //   3. MemorySlice is readable.
    #[inline(always)]
    pub(super) unsafe fn as_slice_ref<'a, T: 'static>(&'a self) -> &'a [T] {
        let head = match self.0.head_ref {
            Some(head) => head,

            // Safety: Upheld by 3.
            None => unsafe { debug_unreachable!("Borrowing non-readable MemorySlice.") },
        };

        if self.0.ty != &TypeId::of::<T>() {
            // Safety: Upheld by 2.
            unsafe { debug_unreachable!("MemorySlice type mismatch.") }
        }

        #[cfg(debug_assertions)]
        if !self.0.is_void() {
            self.0.table.access().assert_value();
        }

        unsafe { from_raw_parts::<'a, T>(head.as_ptr() as *const T, self.0.length) }
    }

    // Safety:
    //   1. ValueMut access granted.
    //   2. T properly describes underlying item type.
    //   3. MemorySlice is writable.
    #[inline(always)]
    pub(super) unsafe fn as_slice_mut<'a, T: 'static>(&'a self) -> &'a mut [T] {
        let head = match self.0.head_mut {
            Some(head) if !self.0.unicode => head,

            // Safety: Upheld by 3.
            _ => unsafe { debug_unreachable!("Borrowing non-writable MemorySlice.") },
        };

        if self.0.ty != &TypeId::of::<T>() {
            // Safety: Upheld by 2.
            unsafe { debug_unreachable!("MemorySlice type mismatch.") }
        }

        #[cfg(debug_assertions)]
        if !self.0.is_void() {
            self.0.table.access().assert_value_mut();
        }

        unsafe { from_raw_parts_mut::<'a, T>(head.as_ptr() as *mut T, self.0.length) }
    }

    // Safety:
    //   1. PlaceRef or PlaceMut access granted.
    //   2. T properly describes underlying item type.
    //   3. MemorySlice is readable.
    //   4. MemorySlice is not empty.
    #[inline(always)]
    pub(super) unsafe fn as_ptr_ref<T: 'static>(&self) -> *const T {
        let head = match self.0.head_ref {
            Some(head) => head,

            // Safety: Upheld by 3.
            None => unsafe { debug_unreachable!("Accessing non-readable MemorySlice.") },
        };

        if self.0.ty != &TypeId::of::<T>() {
            // Safety: Upheld by 2.
            unsafe { debug_unreachable!("MemorySlice type mismatch.") }
        }

        if self.0.length == 0 {
            // Safety: Upheld by 4.
            unsafe { debug_unreachable!("Empty MemorySlice access.") }
        }

        #[cfg(debug_assertions)]
        if !self.0.is_void() {
            self.0.table.access().assert_place();
        }

        head.as_ptr() as *const T
    }

    // Safety:
    //   1. PlaceMut access granted.
    //   2. T properly describes underlying item type.
    //   3. MemorySlice is writeable.
    //   4. MemorySlice is not empty.
    #[inline(always)]
    pub(super) unsafe fn as_ptr_mut<T: 'static>(&self) -> *mut T {
        let head = match self.0.head_mut {
            Some(head) if !self.0.unicode => head,

            // Safety: Upheld by 3.
            _ => unsafe { debug_unreachable!("Accessing non-writeable MemorySlice.") },
        };

        if self.0.ty != &TypeId::of::<T>() {
            // Safety: Upheld by 2.
            unsafe { debug_unreachable!("MemorySlice type mismatch.") }
        }

        if self.0.length == 0 {
            // Safety: Upheld by 4.
            unsafe { debug_unreachable!("Empty MemorySlice access.") }
        }

        #[cfg(debug_assertions)]
        if !self.0.is_void() {
            self.0.table.access().assert_place_mut();
        }

        head.as_ptr() as *mut T
    }

    #[inline(always)]
    pub(super) fn grant_value_ref(&self, origin: Origin) -> RuntimeResult<Grant> {
        if self.0.is_void() {
            return Ok(Grant::ValueRef(u32::MAX));
        }

        Ok(Grant::ValueRef(
            self.0.table.access().grant_value_ref(origin)?,
        ))
    }

    #[inline(always)]
    pub(super) fn grant_value_mut(&self, origin: Origin) -> RuntimeResult<Grant> {
        if self.0.is_void() {
            return Ok(Grant::ValueMut(u32::MAX));
        }

        Ok(Grant::ValueMut(
            self.0.table.access().grant_value_mut(origin)?,
        ))
    }

    #[inline(always)]
    pub(super) fn grant_place_ref(&self, origin: Origin) -> RuntimeResult<Grant> {
        if self.0.is_void() {
            return Ok(Grant::PlaceRef(u32::MAX));
        }

        Ok(Grant::PlaceRef(
            self.0.table.access().grant_place_ref(origin)?,
        ))
    }

    #[inline(always)]
    pub(super) fn grant_place_mut(&self, origin: Origin) -> RuntimeResult<Grant> {
        if self.0.is_void() {
            return Ok(Grant::PlaceMut(u32::MAX));
        }

        Ok(Grant::PlaceMut(
            self.0.table.access().grant_place_mut(origin)?,
        ))
    }

    // Safety: Grant belongs to this MemorySlice.
    pub(super) unsafe fn grant_origin(&self, grant: &Grant) -> Origin {
        if self.0.is_void() {
            return self.0.origin;
        }

        let access = self.0.table.access();

        // Safety: Upheld by the caller.
        match grant {
            Grant::ValueRef(index) => unsafe { access.get_value_ref(*index) },
            Grant::ValueMut(index) => unsafe { access.get_value_mut(*index) },
            Grant::PlaceRef(index) => unsafe { access.get_place_ref(*index) },
            Grant::PlaceMut(index) => unsafe { access.get_place_mut(*index) },
        }
    }

    // Safety: Grant belongs to this MemorySlice.
    #[inline(always)]
    pub(super) unsafe fn release_grant(&self, grant: Grant) {
        if self.0.is_void() {
            return;
        }

        let access = self.0.table.access();

        // Safety: Upheld by the caller.
        match grant {
            Grant::ValueRef(index) => unsafe { access.release_value_ref(index) },
            Grant::ValueMut(index) => unsafe { access.release_value_mut(index) },
            Grant::PlaceRef(index) => unsafe { access.release_place_ref(index) },
            Grant::PlaceMut(index) => unsafe { access.release_place_mut(index) },
        }
    }
}

#[derive(Debug)]
pub(super) enum Grant {
    ValueRef(u32),
    ValueMut(u32),
    PlaceRef(u32),
    PlaceMut(u32),
}

struct MemorySliceInner {
    unicode: bool,
    origin: Origin,
    ty: &'static TypeMeta,
    head_ref: Option<AnyPointer>,
    head_mut: Option<AnyPointer>,
    length: usize,
    capacity: usize,
    table: BorrowTable,
    drop_fn: Option<unsafe fn(head: *mut (), length: usize, capacity: usize)>,
}

impl Debug for MemorySliceInner {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MemorySlice")
            .field("owned", &self.is_owned())
            .field("unicode", &self.unicode)
            .field("origin", &self.origin)
            .field("ty", &self.ty)
            .field("head_ref", &self.head_ref)
            .field("head_mut", &self.head_mut)
            .field("length", &self.length)
            .field("table", &self.table)
            .finish()
    }
}

impl MemorySliceInner {
    #[inline(always)]
    fn is_void(&self) -> bool {
        self.length == 0 || self.ty.size() == 0
    }

    #[inline(always)]
    fn is_owned(&self) -> bool {
        self.drop_fn.is_some()
    }
}

// Safety:
//   1. `head` points to slice of exactly `capacity` allocated
//      and properly aligned items.
//   2. First `length` items are properly initialized.
//   3. The slice fully covers allocated memory.
//   4. The slice was allocated by the global allocator.
#[inline(always)]
unsafe fn drop_vec<T>(head: *mut (), length: usize, capacity: usize) {
    if length > capacity {
        unsafe { debug_unreachable!("Vector length is larger than capacity.") }
    }

    // Safety: Upheld by the caller.
    let _ = unsafe { Vec::from_raw_parts(head.cast::<T>(), length, capacity) };
}

#[repr(transparent)]
struct MemoryRegistry {
    inner: Table<NonZeroUsize, Weak<MemorySlice>, RandomState>,
}

// Safety: MemoryRegistry access operations guarded by the RwLock.
unsafe impl Send for MemoryRegistry {}

// Safety: MemoryRegistry access operations guarded by the RwLock.
unsafe impl Sync for MemoryRegistry {}

impl MemoryRegistry {
    #[inline(always)]
    fn get() -> &'static Self {
        static REGISTRY: Lazy<MemoryRegistry> = Lazy::new(|| MemoryRegistry {
            inner: Table::new(),
        });

        REGISTRY.deref()
    }

    // Safety: There is registered MemorySlice that belongs to specified address.
    #[inline]
    unsafe fn deregister(&self, address: NonZeroUsize) {
        let shard = match self.inner.shards().get(self.inner.shard_index_of(&address)) {
            Some(shard) => shard,

            // Safety: RegistryKey's shard always contains valid index into
            //         related registry.
            None => unsafe { debug_unreachable!("Incorrect MemorySlice shard.") },
        };

        let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

        if guard.remove(&address).is_none() {
            // Safety: Upheld by the caller.
            unsafe { debug_unreachable!("Dropping unregistered MemorySlice.") }
        }

        drop(guard);
    }
}

type AnyPointer = NonNull<()>;

trait AnyPointerImpl {
    fn address(self) -> NonZeroUsize;
}

impl AnyPointerImpl for AnyPointer {
    #[inline(always)]
    fn address(self) -> NonZeroUsize {
        // Safety: NonNull casting to NonZeroUsize.
        unsafe { NonZeroUsize::new_unchecked(self.as_ptr() as *mut u8 as usize) }
    }
}
