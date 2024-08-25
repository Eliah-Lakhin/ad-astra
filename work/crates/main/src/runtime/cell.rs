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
    mem::{replace, take, transmute, transmute_copy},
    ops::{Bound, RangeBounds},
    ptr::{null, null_mut},
    str::from_utf8,
    sync::Arc,
};

use crate::{
    report::{debug_unreachable, system_panic},
    runtime::{
        coercion::{Upcasted, UpcastedChain},
        memory::{Grant, MemorySlice},
        NumericOperationKind,
        Origin,
        RuntimeError,
        RuntimeResult,
        ScriptType,
        TypeMeta,
        Upcast,
    },
};

pub trait MapRef<'a, From: 'static> {
    type To: Upcast<'a>;

    fn map(self, from: &'a From) -> RuntimeResult<Self::To>;
}

impl<'a, From, To, F> MapRef<'a, From> for F
where
    From: 'static,
    To: Upcast<'a>,
    F: FnOnce(&'a From) -> RuntimeResult<To>,
{
    type To = To;

    #[inline(always)]
    fn map(self, from: &'a From) -> RuntimeResult<Self::To> {
        self(from)
    }
}

pub trait MapMut<'a, From: 'static> {
    type To: Upcast<'a>;

    fn map(self, from: &'a mut From) -> RuntimeResult<Self::To>;
}

impl<'a, From, To, F> MapMut<'a, From> for F
where
    From: 'static,
    To: Upcast<'a>,
    F: FnOnce(&'a mut From) -> RuntimeResult<To>,
{
    type To = To;

    #[inline(always)]
    fn map(self, from: &'a mut From) -> RuntimeResult<Self::To> {
        self(from)
    }
}

/// A reference-counting pointer to heap-allocated memory managed by the
/// Script Engine.
///
/// The Cell object provides the foundational mechanism for modeling Script
/// memory and ensuring interoperability between Script data and Rust data.
///
/// ## Cell Design
///
/// A unit of Script memory allocation is an array of elements, all of the same
/// [type known to the Engine](ScriptType). Typically, this is an array with
/// just one element.
///
/// The Cell represents a pointer to this memory allocation. This pointer can
/// either reference memory owned by the Engine, a projection of the memory
/// allocation, or a reference to a portion of the allocation (not necessarily
/// owned by the Engine).
///
/// Each Cell is a reference-counting pointer. Cloning a Cell increases the
/// reference count, while dropping an instance decreases it. When the last
/// instance of a Cell pointing to memory owned by the Script Engine is dropped,
/// the Engine deallocates the memory.
///
/// The Engine tracks the borrowing status of pointers. For example, if you
/// obtain an immutable reference from a Cell's pointer, you cannot acquire a
/// mutable reference from another clone of this Cell until the immutable
/// reference is released.
///
/// Additionally, the Engine ensures that dereferencing a Cell pointer that is a
/// projection of another Cell pointer adheres to Rust's borrowing rules. For
/// example, if one Cell refers to a chunk of memory of another Cell,
/// immutable dereferencing of either Cell and mutable dereferencing of the
/// other are prohibited by the Script Engine.
///
/// These borrowing rules are compatible with Rust's borrowing rules and
/// the [Miri](https://github.com/rust-lang/miri) machine.
///
/// ## Creation
///
/// You can create a Cell object using one of the memory-transfer functions
/// that hand control over the Rust object (or an array of objects) to the
/// Script Engine:
///
/// - [Cell::give]: Creates a Cell from the specified object. Typically, this
///   function creates a Cell that points to a singleton array containing the
///   object you provided. However, depending on the type's
///   [upcasting](Upcast) procedure, the Engine may create an array of multiple
///   elements instead. For example, the Script Engine interprets [String] as an
///   array of bytes.
///
/// - [Cell::give_vec]: Explicitly creates an array of objects from the
///   specified vector.
///
/// - [Cell::nil]: Creates a special kind of Cell that intentionally does not
///   reference any memory allocation. Most of the Cell access functions will
///   yield [RuntimeError::Nil] if you attempt to access the underlying data.
///   Additionally, Nil is the default value of the Cell object.

///
/// ```
/// # use ad_astra::runtime::{Cell, Origin};
/// #
/// let cell = Cell::give(Origin::nil(), 125u16).unwrap();
///
/// assert_eq!(cell.length(), 1);
/// assert!(cell.is::<u16>());
/// assert!(!cell.is_nil());
///
/// let cell = Cell::give_vec(Origin::nil(), vec![10u8, 20, 30]).unwrap();
///
/// assert_eq!(cell.length(), 3);
/// ```
///
/// ## Cell Type Inspection
///
/// You can obtain runtime metadata about the underlying data type using the
/// [Cell::ty] function, and determine the length of the array using the
/// [Cell::length] function.
///
/// Within the type metadata, you can further inspect the operations available
/// for this type using the [TypeMeta::prototype] function.
///
/// ## Data Access
///
/// Operations you can perform on the Cell's underlying data can be divided
/// into higher-level and lower-level memory operations.
///
/// Higher-level operations involve working with the data as a type exported
/// from Rust and in accordance with the
/// [Prototype](crate::runtime::Prototype) of this type.
///
/// Lower-level operations involve using the Cell's API functions directly,
/// such as data borrowing and data projections.
///
/// Most operations (both high- and low-level) either consume the Cell or
/// render the instance obsolete. Therefore, if you need to perform more than
/// one operation, you should [Clone] the Cell and use the cloned instances
/// for each operation separately.
///
/// ## Object Operations
///
/// To work with a Cell as an object of the exported Rust type, you
/// should convert the Cell into an [Object] using the [Cell::into_object]
/// function.
///
/// ```
/// # use ad_astra::runtime::{Arg, Cell, Origin};
/// #
/// let lhs = Cell::give(Origin::nil(), 10usize).unwrap();
/// let rhs = Cell::give(Origin::nil(), 20usize).unwrap();
///
/// let result = lhs
///     .into_object()
///     .add(Origin::nil(), Origin::nil(), Arg::new(Origin::nil(), rhs))
///     .unwrap();
///
/// let result_value = result.take::<usize>(Origin::nil()).unwrap();
///
/// assert_eq!(result_value, 30);
/// ```
///
/// ## Taking Data Back
///
/// You can retrieve the data from a Cell using one of the [Cell::take],
/// [Cell::take_vec], or [Cell::take_string] functions.
///
/// If the `Cell` is the last instance pointing to the data owned by the Script
/// Engine, the function will return the data without cloning. If there are
/// additional clones of the Cell, or if the data is a projection of another
/// memory allocation, the Cell will attempt to clone the data. If the type
/// does not support cloning, you will receive a
/// [RuntimeError::UndefinedOperator] error.
///
/// ```
/// # use ad_astra::runtime::{Cell, Origin};
/// #
/// let cell = Cell::give(Origin::nil(), 125u16).unwrap();
/// let cell2 = cell.clone();
///
/// assert!(cell.take::<f32>(Origin::nil()).is_err());
/// assert_eq!(cell2.take::<u16>(Origin::nil()).unwrap(), 125);
/// ```
///
/// ## Data Borrowing
///
/// The [Cell::borrow_ref], [Cell::borrow_mut], [Cell::borrow_slice_ref],
/// [Cell::borrow_slice_mut], and [Cell::borrow_str] functions provide a way
/// to obtain normal Rust references to the underlying data without cloning or
/// transferring ownership.
///
/// These functions return Rust references with the same lifetime as the Cell
/// instance, ensuring that the reference cannot outlive the Cell's instance.
///
/// When borrowing data, the current instance of the Cell becomes obsolete.
/// You cannot use it anymore, and you should drop this Cell after dropping
/// the reference.
///
/// ```
/// # use ad_astra::runtime::{Cell, Origin};
/// #
/// let mut cell1 = Cell::give(Origin::nil(), 125u16).unwrap();
/// let mut cell2 = cell1.clone();
/// let mut cell3 = cell1.clone();
///
/// assert_eq!(cell1.borrow_ref::<u16>(Origin::nil()).unwrap(), &125);
///
/// // Cannot mutably borrow the same data because an immutable reference is
/// // still active.
/// assert!(cell2.borrow_mut::<u16>(Origin::nil()).is_err());
///
/// // Dropping the immutable reference.
/// drop(cell1);
///
/// assert_eq!(cell3.borrow_mut::<u16>(Origin::nil()).unwrap(), &mut 125);
/// ```
///
/// ## Data Projections
///
/// You can map a Cell's data to another Cell that refers to the memory
/// allocation related to the original data.
///
/// For example, using [Cell::map_ptr], you can project a pointer to a Rust
/// structure to one of its fields without dereferencing the struct pointer.
/// Alternatively, with [Cell::map_ref], you can map a reference to the
/// underlying object to a reference of another object with the same lifetime.
///
/// ```
/// # use ad_astra::runtime::{Cell, Origin};
/// #
/// let cell = Cell::give_vec(Origin::nil(), vec![10u16, 20, 30, 40]).unwrap();
///
/// let mut mapped_cell = cell.map_slice(Origin::nil(), 2..).unwrap();
///
/// assert_eq!(
///     mapped_cell.borrow_slice_ref::<u16>(Origin::nil()).unwrap(),
///     &[30, 40],
/// );
/// ```
///
/// ## Strings
///
/// The Ad Astra Engine has a special case for string data (`[str]` type).
/// The Cell stores strings as arrays of bytes (`[u8]`), but it also sets a
/// flag indicating that this array is safe for UTF-8 decoding.
///
/// The Cell API includes special functions (e.g., [Cell::borrow_str]) that
/// interpret arrays of bytes as Unicode strings.
///
/// ## Zero-Sized Types
///
/// Zero-Sized Types (ZSTs) have special handling in Ad Astra. If you create an
/// array of ZST elements, the Cell stores the length of the array but allows
/// addressing of elements outside of the array bounds without errors.
///
/// Additionally, data access operations on ZST Cells are exempt from some
/// of the borrowing rules. For example, you can obtain mutable and immutable
/// references to the same zero-sized data without conflicts.
///
/// One exception is the `()` [unit] type. When you [give](Cell::give) a unit
/// value `()` to the function, it returns a [Cell::nil] value, with the Nil
/// Cell representing the Rust unit value.

#[derive(Clone)]
#[repr(transparent)]
pub struct Cell(Option<Arc<Chain>>);

impl Debug for Cell {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            None => formatter.write_str("Nil"),
            Some(chain) => Debug::fmt(chain, formatter),
        }
    }
}

impl Default for Cell {
    #[inline(always)]
    fn default() -> Self {
        Self::nil()
    }
}

impl Cell {
    /// Returns an instance of Cell that intentionally does not reference any
    /// memory allocation.
    ///
    /// Most functions in the Cell API will return a [RuntimeError::Nil] error
    /// when called on this instance.
    #[inline(always)]
    pub const fn nil() -> Self {
        Self(None)
    }

    /// Gives ownership of the data to the Script Engine and returns a Cell
    /// that points to this data.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// where this object was created.
    ///
    /// The `data` parameter is the data of the created Cell.
    ///
    /// This argument does not need to be of a type known to the Script Engine;
    /// it can be any Rust type that can be [upcasted](Upcast) to a known type.
    /// For example, the generic [Option] type is not known to the
    /// Script Engine. If the Option is Some, the upcasting procedure unwraps
    /// the value; otherwise, it returns a unit `()` value (which is interpreted
    /// as [Cell::nil]).
    ///
    /// The function returns a [`RuntimeError`] if the upcasting procedure fails
    /// to upcast the `data` object.
    pub fn give(origin: Origin, data: impl Upcast<'static>) -> RuntimeResult<Self> {
        let to = match Upcast::upcast(origin, data)?.into_chain(origin)? {
            UpcastedChain::Cell(cell) => return Ok(cell),
            UpcastedChain::Slice(memory_slice) => memory_slice,
        };

        Ok(Self(Some(Arc::new(Chain(ChainInner {
            from: Default::default(),
            to,
            grant: None,
        })))))
    }

    /// A lower-level alternative to the [Cell::give] function. Unlike the
    /// give function, give_vec does not perform [upcasting](Upcast) of the
    /// data object. Instead, it transfers ownership of the vector's array to
    /// the Script Engine as-is and returns a Cell that points to the vector's
    /// memory allocation.
    ///
    /// The generic parameter `T` is the type of the elements in the array and
    /// must be a type known to the Script Engine.
    pub fn give_vec<T: ScriptType>(origin: Origin, data: Vec<T>) -> RuntimeResult<Self> {
        let to = match data.into_chain(origin)? {
            UpcastedChain::Cell(cell) => return Ok(cell),
            UpcastedChain::Slice(memory_slice) => memory_slice,
        };

        Ok(Self(Some(Arc::new(Chain(ChainInner {
            from: Default::default(),
            to,
            grant: None,
        })))))
    }

    /// Returns true if this Cell is [Cell::nil].
    ///
    /// For example, if you [give](Cell::give) a unit `()` value, the resulting
    /// Cell is a Nil Cell.
    #[inline(always)]
    pub fn is_nil(&self) -> bool {
        self.0.is_none()
    }

    /// Returns the Rust or Script source code range indicating where the Cell's
    /// data was created.
    #[inline(always)]
    pub fn origin(&self) -> Origin {
        match &self.0 {
            None => Origin::Rust(TypeMeta::nil().origin()),
            Some(chain) => chain.0.data_origin(),
        }
    }

    /// Returns runtime metadata for the Rust type of the underlying Cell's data
    /// object (or the type of the elements in the Cell's array).
    ///
    /// If the Cell points to a `[u8]` array of bytes, and these bytes encode a
    /// Unicode string, the function returns metadata for the [str] type.
    ///
    /// If the Cell is a [Nil](Cell::nil) cell, the function returns metadata
    /// for the [unit] type.
    #[inline(always)]
    pub fn ty(&self) -> &'static TypeMeta {
        match &self.0 {
            None => TypeMeta::nil(),

            Some(chain) => {
                match chain.0.to.is_unicode() && chain.0.to.ty() == &TypeId::of::<u8>() {
                    true => <str>::type_meta(),
                    false => chain.0.to.ty(),
                }
            }
        }
    }

    /// Returns true if the Cell's data has a type specified by the `T` generic
    /// parameter.
    ///
    /// Note that if the Cell stores a string, the function will return true
    /// for both [str] and [u8] types.
    #[inline(always)]
    pub fn is<T: ScriptType + ?Sized>(&self) -> bool {
        let id = TypeId::of::<T>();

        match &self.0 {
            None => id == TypeId::of::<()>(),

            Some(chain) => {
                let ty = chain.0.to.ty();

                if ty == &id {
                    return true;
                }

                let str_type = TypeId::of::<str>();
                let u8_type = TypeId::of::<u8>();

                chain.0.to.is_unicode() && ty == &u8_type && id == str_type
            }
        }
    }

    /// Returns the number of elements in the array to which this Cell points.
    ///
    /// Typically, the Cell object stores arrays with a single element, and the
    /// function returns 1.
    ///
    /// Note that the Script Engine stores Rust strings as arrays of bytes.
    /// In this case, the function returns the number of bytes encoding the
    /// string.
    ///
    /// If the Cell is a [Nil Cell](Cell::nil), the function returns 0.
    #[inline(always)]
    pub fn length(&self) -> usize {
        match &self.0 {
            None => 0,
            Some(chain) => chain.0.to.length(),
        }
    }

    /// Retrieves the data to which this Cell points.
    ///
    /// If there are no other clones of this Cell, the function takes the
    /// value from the Script Engine and returns it as-is. Otherwise, the
    /// function attempts to clone the underlying data.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// from where the Cell's data was accessed.
    ///
    /// The function returns a [RuntimeError] if:
    ///
    /// - The data requires cloning but cannot be immutably borrowed (e.g., the
    ///   data is already borrowed mutably), or the [type](Self::ty) does not
    ///   implement the [cloning](crate::runtime::Prototype::implements_clone)
    ///   operator.
    /// - The generic parameter `T` is not a [type](Self::is) of the Cell's
    ///   data, or if the Cell points to an array with zero non-zero-sized (ZST)
    ///   elements.
    ///
    /// If the `Cell` points to an array with more than one element, the
    /// function returns the first element in the array.
    ///
    /// For strings, this function can retrieve the first byte of the UTF-8
    /// encoding (by specifying [u8] as `T`), but it is recommended to use the
    /// [take_string](Self::take_string) function instead, which returns the
    /// entire string data.
    #[inline]
    pub fn take<T: ScriptType>(mut self, origin: Origin) -> RuntimeResult<T> {
        match take(&mut self.0) {
            None => {
                if TypeId::of::<T>() == TypeId::of::<()>() {
                    // Safety: Type checked above.
                    return Ok(unsafe { transmute_copy::<(), T>(&()) });
                }

                Err(RuntimeError::TypeMismatch {
                    access_origin: origin,
                    data_type: TypeMeta::nil(),
                    expected_types: Vec::from([T::type_meta()]),
                })
            }

            Some(chain) => chain.take_first(origin),
        }
    }

    /// Similar to the [take](Self::take) function, but returns all elements
    /// of the underlying Cell's array as a vector, even if the array has zero
    /// elements.
    pub fn take_vec<T: ScriptType>(mut self, origin: Origin) -> RuntimeResult<Vec<T>> {
        match take(&mut self.0) {
            None => {
                if TypeId::of::<T>() == TypeId::of::<()>() {
                    let vector = Vec::from([(); 1]);

                    // Safety: Type checked above.
                    return Ok(unsafe { transmute_copy::<Vec<()>, Vec<T>>(&vector) });
                }

                Err(RuntimeError::TypeMismatch {
                    access_origin: origin,
                    data_type: TypeMeta::nil(),
                    expected_types: Vec::from([T::type_meta()]),
                })
            }

            Some(chain) => chain.take_vec(origin),
        }
    }

    /// Similar to the [take_vec](Self::take_vec) function, but attempts to
    /// decode the underlying Cell's array into a [String].
    ///
    /// If the [type](Self::is) of the data is neither [u8] nor [str], the
    /// function returns an error.
    ///
    /// If the type is [u8] but it is unknown whether the underlying array of
    /// bytes is a UTF-8 encoding of a string, the function will attempt to
    /// decode the array. If decoding fails, the function returns an error too.
    pub fn take_string(mut self, origin: Origin) -> RuntimeResult<String> {
        match take(&mut self.0) {
            None => Err(RuntimeError::TypeMismatch {
                access_origin: origin,
                data_type: TypeMeta::nil(),
                expected_types: Vec::from([<str>::type_meta()]),
            }),

            Some(chain) => {
                let is_unicode = chain.0.to.is_unicode();

                let bytes = chain.take_vec::<u8>(origin)?;

                match is_unicode {
                    true => {
                        #[cfg(debug_assertions)]
                        {
                            match String::from_utf8(bytes) {
                                Ok(string) => Ok(string),

                                Err(error) => {
                                    system_panic!(format!(
                                        "Unicode byte slice decoding failure.\n{}",
                                        error.utf8_error()
                                    ))
                                }
                            }
                        }

                        #[cfg(not(debug_assertions))]
                        {
                            // Safety: MemorySlice labeled as unicode-safe.
                            Ok(unsafe { String::from_utf8_unchecked(bytes) })
                        }
                    }

                    false => Ok(String::from_utf8_lossy(bytes.as_ref()).into_owned()),
                }
            }
        }
    }

    /// Returns an immutable reference to the Cell's data.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// from where the Cell's data was accessed.
    ///
    /// The function dereferences the first element of the array to which this
    /// Cell points. If the array consists of more than one element or zero
    /// elements (which can be checked via the [Cell::length] function), the
    /// function returns a [RuntimeError].
    ///
    /// The function also returns an error if the Script Engine cannot grant
    /// immutable access (e.g., if the data is already borrowed mutably), or if
    /// the generic parameter `T` is not a [type](Self::is) of the Cell's data.
    ///
    /// The returned reference has the same lifetime as the Cell's instance.
    /// The borrow_ref function turns the Cell's instance into an invalid state,
    /// so you cannot use this instance again. You should drop the Cell after
    /// releasing the reference. If you need to use the Cell's data after
    /// releasing the reference, you should clone the Cell before calling
    /// borrow_ref.
    #[inline]
    pub fn borrow_ref<T: ScriptType>(&mut self, origin: Origin) -> RuntimeResult<&T> {
        let length = self.length();

        if length != 1 {
            return Err(RuntimeError::NonSingleton {
                access_origin: origin,
                actual: length,
            });
        }

        let slice = self.borrow_slice_ref::<T>(origin)?;

        match slice.first() {
            Some(singleton) => Ok(singleton),

            // Safety: Slice length checked above.
            None => unsafe { debug_unreachable!("Missing slice first item.") },
        }
    }

    /// Similar to [borrow_ref](Self::borrow_ref), but returns a reference to
    /// the entire array to which this Cell points, regardless of the array's
    /// [length](Self::length).
    pub fn borrow_slice_ref<T: ScriptType>(&mut self, origin: Origin) -> RuntimeResult<&[T]> {
        match take(&mut self.0) {
            None => Err(RuntimeError::Nil {
                access_origin: origin,
            }),

            Some(chain) => {
                let data_type = chain.0.to.ty();
                let expected_type = T::type_meta();

                if data_type != expected_type {
                    return Err(RuntimeError::TypeMismatch {
                        access_origin: origin,
                        data_type,
                        expected_types: Vec::from([expected_type]),
                    });
                }

                self.0 = Some(chain.value_ref(origin)?);

                match &self.0 {
                    // Safety: Just set above.
                    None => unsafe { debug_unreachable!("Nil Cell borrowing.") },

                    // Safety:
                    //   1. ValueRef granted if and only if the MemoryCell is readable.
                    //   2. Item type checked above.
                    Some(chain) => Ok(unsafe { chain.0.to.as_slice_ref::<T>() }),
                }
            }
        }
    }

    /// Similar to [borrow_slice_ref](Self::borrow_slice_ref), but attempts to
    /// decode the underlying Cell's array into a [str].
    ///
    /// If the [type](Self::is) of the data is neither [u8] nor [str], the
    /// function returns an error.
    ///
    /// If the type is [u8] but it is unknown whether the underlying array of
    /// bytes is a UTF-8 encoding of a string, the function will attempt to
    /// decode the array. If decoding fails, the function returns an error.
    pub fn borrow_str(&mut self, origin: Origin) -> RuntimeResult<&str> {
        match take(&mut self.0) {
            None => Err(RuntimeError::Nil {
                access_origin: origin,
            }),

            Some(chain) => {
                let data_type = chain.0.to.ty();
                let expected_type = <u8>::type_meta();

                if data_type != expected_type {
                    return Err(RuntimeError::TypeMismatch {
                        access_origin: origin,
                        data_type,
                        expected_types: Vec::from([expected_type]),
                    });
                }

                self.0 = Some(chain.value_ref(origin)?);

                match &self.0 {
                    // Safety: Just set above.
                    None => unsafe { debug_unreachable!("Nil Cell borrowing.") },

                    Some(chain) => {
                        // Safety:
                        //   1. ValueRef granted if and only if the MemoryCell is readable.
                        //   2. Item type checked above.
                        let slice = unsafe { chain.0.to.as_slice_ref::<u8>() };

                        match chain.0.to.is_unicode() {
                            true => {
                                #[cfg(debug_assertions)]
                                {
                                    match from_utf8(slice) {
                                        Ok(string) => Ok(string),

                                        Err(error) => {
                                            system_panic!(format!(
                                                "Unicode byte slice decoding failure.\n{}",
                                                error
                                            ))
                                        }
                                    }
                                }

                                #[cfg(not(debug_assertions))]
                                {
                                    // Safety: MemorySlice labeled as unicode-safe.
                                    Ok(unsafe { std::str::from_utf8_unchecked(slice) })
                                }
                            }

                            false => match from_utf8(slice) {
                                Ok(string) => Ok(string),

                                Err(error) => Err(RuntimeError::Utf8Decoding {
                                    access_origin: origin,
                                    cause: Box::new(error),
                                }),
                            },
                        }
                    }
                }
            }
        }
    }

    /// Returns a mutable reference to the Cell's data.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// from where the Cell's data was accessed.
    ///
    /// The function dereferences the first element of the array to which this
    /// Cell points. If the array consists of more than one element or zero
    /// elements (which can be checked via the [Cell::length] function), the
    /// function returns a [RuntimeError].
    ///
    /// The function also returns an error if the Script Engine cannot grant
    /// mutable access (e.g., if the data is already borrowed), or if the
    /// generic parameter `T` is not a [type](Self::is) of the Cell's data.
    ///
    /// The returned reference has the same lifetime as the Cell's instance.
    /// The borrow_mut function turns the Cell instance into an invalid state,
    /// so you cannot use this instance again. You should drop the Cell after
    /// releasing the reference. If you need to use the Cell's data after
    /// releasing the reference, you should clone the Cell before calling
    /// borrow_mut.
    #[inline]
    pub fn borrow_mut<T: ScriptType>(&mut self, origin: Origin) -> RuntimeResult<&mut T> {
        let length = self.length();

        if length != 1 {
            return Err(RuntimeError::NonSingleton {
                access_origin: origin,
                actual: length,
            });
        }

        let slice = self.borrow_slice_mut::<T>(origin)?;

        match slice.first_mut() {
            Some(singleton) => Ok(singleton),

            // Safety: Slice length checked above.
            None => unsafe { debug_unreachable!("Missing slice first item.") },
        }
    }

    /// Similar to [borrow_mut](Self::borrow_mut), but returns a mutable
    /// reference to the entire array to which this Cell points, regardless of
    /// the array's [length](Self::length).
    pub fn borrow_slice_mut<'a, T: ScriptType>(
        &'a mut self,
        origin: Origin,
    ) -> RuntimeResult<&'a mut [T]> {
        match take(&mut self.0) {
            None => Err(RuntimeError::Nil {
                access_origin: origin,
            }),

            Some(chain) => {
                let data_type = chain.0.to.ty();
                let expected_type = T::type_meta();

                if data_type != expected_type {
                    return Err(RuntimeError::TypeMismatch {
                        access_origin: origin,
                        data_type,
                        expected_types: Vec::from([expected_type]),
                    });
                }

                self.0 = Some(chain.value_mut(origin)?);

                match &self.0 {
                    // Safety: Just set above.
                    None => unsafe { debug_unreachable!("Nil Cell borrowing.") },

                    // Safety:
                    //   1. ValueMut granted if and only if the MemoryCell is writeable.
                    //   2. Item type checked above.
                    Some(chain) => Ok(unsafe { chain.0.to.as_slice_mut::<T>() }),
                }
            }
        }
    }

    /// Creates a projection of the Cell's data into another Cell that is
    /// guaranteed to point to a valid UTF-8 encoding of a string.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// from where the Cell's data has been mapped.
    ///
    /// The function's behavior is similar to calling
    /// [borrow_str](Self::borrow_str) and then storing the resulting `&str`
    /// reference in the returned Cell.
    /// The map_str function is subject to the same requirements as the
    /// borrow_str function.
    ///
    /// This function immediately puts the data of the original Cell into an
    /// immutable borrow state. This borrowing will not be released until all
    /// clones of the projected Cell instance are dropped.
    pub fn map_str(mut self, origin: Origin) -> RuntimeResult<Self> {
        let to = {
            let from = self.borrow_str(origin)?;
            let upcasted = Upcast::upcast(origin, from)?;
            let to = upcasted.into_chain(origin)?;

            to
        };

        let to = match to {
            UpcastedChain::Cell(cell) => return Ok(cell),
            UpcastedChain::Slice(slice) => slice,
        };

        let from = match to.is_owned() {
            true => Self(None),
            false => self,
        };

        Ok(Self(Some(Arc::new(Chain(ChainInner {
            from,
            to,
            grant: None,
        })))))
    }

    /// Creates a projection of the immutable reference to the Cell's data into
    /// another reference, returning a Cell that points to the projected
    /// reference.
    ///
    /// This function is useful when you want to [borrow](Self::borrow_ref) the
    /// Cell's data, then call a function that returns another reference with
    /// the same lifetime, and store this reference in a new Cell.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// from where the Cell's data has been mapped.
    ///
    /// The `map` parameter is a functor that takes a reference to the original
    /// Cell's data and returns data that can be [upcasted](Upcast) into an
    /// object with the same lifetime as the original Cell's data reference. You
    /// can use a normal Rust function as the `map` argument if it satisfies
    /// these requirements.
    ///
    /// ```
    /// # use ad_astra::{
    /// #     export,
    /// #     runtime::{Cell, Origin, RuntimeResult},
    /// # };
    /// #
    /// # #[export(include)]
    /// # #[export(package)]
    /// # #[derive(Default)]
    /// # struct Package;
    /// #
    /// #[export]
    /// pub struct Container {
    ///     value: usize,
    /// }
    ///
    /// fn map(container: &Container) -> RuntimeResult<&usize> {
    ///     Ok(&container.value)
    /// }
    ///
    /// let container = Cell::give(Origin::nil(), Container { value: 100 }).unwrap();
    ///
    /// let projection = container.map_ref(Origin::nil(), map).unwrap();
    ///
    /// assert_eq!(projection.take::<usize>(Origin::nil()).unwrap(), 100);
    /// ```
    ///
    /// In practice, you might use an [FnOnce] closure as a functor. However,
    /// this can lead to limitations in Rust's type system, where the compiler
    /// may not infer lifetime bounds correctly. To work around this, you can
    /// use a "funnel" pattern with an additional helper function:
    ///
    /// ```
    /// # use ad_astra::{
    /// #     export,
    /// #     runtime::{Cell, Origin, RuntimeResult},
    /// # };
    /// #
    /// # #[export(include)]
    /// # #[export(package)]
    /// # #[derive(Default)]
    /// # struct Package;
    /// #
    /// # #[export]
    /// # pub struct Container {
    /// #     value: usize,
    /// # }
    /// #
    /// # let container = Cell::give(Origin::nil(), Container { value: 100 }).unwrap();
    /// #
    /// fn funnel<F: FnOnce(&Container) -> RuntimeResult<&usize>>(f: F) -> F {
    ///     f
    /// }
    ///
    /// let projection = container
    ///     .map_ref(
    ///         Origin::nil(),
    ///         funnel(|container: &Container| Ok(&container.value)),
    ///     )
    ///     .unwrap();
    /// #
    /// # assert_eq!(projection.take::<usize>(Origin::nil()).unwrap(), 100);
    /// ```
    ///
    /// The map_ref function borrows the original Cell data immutably and does
    /// not release this borrowing until all clones of the projected Cell are
    /// dropped. An exception occurs if the `map` function returns owned data
    /// (data with the `'static` lifetime). In this case, the original data
    /// borrowing is not bound by the lifetime of the projection.
    ///
    /// Consequently, the function may return a [RuntimeError] if the Script
    /// Engine cannot grant immutable access to the original data (e.g., if the
    /// data is already borrowed mutably). The function also returns an error if
    /// the `From` generic type is not a [type](Self::is) of the original
    /// Cell's data, if the `map` functor returns an error, or if the original
    /// Cell's data is an array of zero or more than one element (which can be
    /// checked via the [Cell::length] function).
    pub fn map_ref<From>(
        mut self,
        origin: Origin,
        map: impl for<'a> MapRef<'a, From>,
    ) -> RuntimeResult<Self>
    where
        From: ScriptType,
    {
        let to = {
            let from = self.borrow_ref::<From>(origin)?;
            let mapped = map.map(from)?;
            let upcasted = Upcast::upcast(origin, mapped)?;
            let to = upcasted.into_chain(origin)?;

            to
        };

        let to = match to {
            UpcastedChain::Cell(cell) => return Ok(cell),
            UpcastedChain::Slice(slice) => slice,
        };

        let from = match to.is_owned() {
            true => Self(None),
            false => self,
        };

        Ok(Self(Some(Arc::new(Chain(ChainInner {
            from,
            to,
            grant: None,
        })))))
    }

    /// Similar to [map_ref](Self::map_ref), but maps a mutable reference to
    /// the Cell's data.
    ///
    /// Unlike map_ref, the map_mut function borrows the original Cell's
    /// data mutably and is subject to Rust's general exclusive dereferencing
    /// rules.
    pub fn map_mut<From>(
        mut self,
        origin: Origin,
        map: impl for<'a> MapMut<'a, From>,
    ) -> RuntimeResult<Self>
    where
        From: ScriptType,
    {
        let to = {
            let from = self.borrow_mut::<From>(origin)?;
            let mapped = map.map(from)?;
            let upcasted = Upcast::upcast(origin, mapped)?;
            let to = upcasted.into_chain(origin)?;

            to
        };

        let to = match to {
            UpcastedChain::Cell(cell) => return Ok(cell),
            UpcastedChain::Slice(slice) => slice,
        };

        let from = match to.is_owned() {
            true => Self(None),
            false => self,
        };

        Ok(Self(Some(Arc::new(Chain(ChainInner {
            from,
            to,
            grant: None,
        })))))
    }

    /// Creates a projection of the raw pointer of the Cell into another raw
    /// pointer that points to the memory allocation with the same lifetime as
    /// the original data object.
    ///
    /// This function is useful for mapping a pointer to a Rust structure into
    /// one of its fields.
    ///
    /// Unlike [map_ref](Self::map_ref) and [map_mut](Self::map_mut), the
    /// map_ptr function does not borrow the original data. The `by_ref` and
    /// `by_mut` functors should not dereference the pointers.
    ///
    /// The original data remains unborrowed until you try to access the
    /// projection's Cell data (e.g., using [Cell::borrow_ref]). At that point,
    /// both the original data and the projected data will be borrowed together.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::ptr::{addr_of, addr_of_mut};
    /// #
    /// # use ad_astra::{
    /// #     export,
    /// #     runtime::{Cell, Origin},
    /// # };
    /// #
    /// # #[export(include)]
    /// # #[export(package)]
    /// # #[derive(Default)]
    /// # struct Package;
    ///
    /// #[export]
    /// pub struct Container {
    ///     field: usize,
    /// }
    ///
    /// let container = Cell::give(Origin::nil(), Container { field: 100 }).unwrap();
    ///
    /// let mut projection = container
    ///     .map_ptr::<Container, usize>(
    ///         Origin::nil(),
    ///         // Safety: The `addr_of!` macros do not dereference the pointers,
    ///         // and the field of the structure cannot outlive its structure
    ///         // instance.
    ///         Some(|container| unsafe { addr_of!((*container).field) }),
    ///         Some(|container| unsafe { addr_of_mut!((*container).field) }),
    ///     )
    ///     .unwrap();
    ///
    /// {
    ///     let mut projection = projection.clone();
    ///
    ///     let data_mut = projection.borrow_mut::<usize>(Origin::nil()).unwrap();
    ///
    ///     *data_mut += 50;
    /// }
    ///
    /// let data_ref = projection.borrow_ref::<usize>(Origin::nil()).unwrap();
    ///
    /// assert_eq!(data_ref, &150);
    /// ```
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// where the Cell's data has been mapped.
    ///
    /// The `by_ref` and `by_mut` parameters are functors that map the original
    /// Cell's pointer to another pointer (immutably and mutably, respectively).
    ///
    /// The function returns a [RuntimeError] if the `From` generic parameter is
    /// not a [type](Self::is) of the original Cell's data, or if the `Cell`
    /// points to an array of zero or more than one element (which can be
    /// checked via the [Cell::length] function).
    ///
    /// Even though map_ptr does not borrow the original data, the raw pointer
    /// operations performed by the `by_ref` and `by_mut` functors require that
    /// the pointer's data does not have exclusive borrowings. Therefore, if the
    /// original Cell's data is borrowed mutably, this function will return an
    /// error too.
    ///
    /// The `by_ref` and `by_mut` parameters are optional. If you do not specify
    /// one of them, the projection will be either read-only or write-only
    /// (e.g., you will be able to access the data either mutably or immutably
    /// depending on which functor has been specified). If both arguments are
    /// None, the map_ptr function returns a [Nil Cell](Cell::nil).
    ///
    /// ## Safety
    ///
    /// The map_ptr function is safe by itself, but the `by_ref` and `by_mut`
    /// functors are unsafe and must adhere to the following requirements:
    ///
    /// 1. Neither of these functors should dereference the raw pointer
    ///    specified in their argument.
    /// 2. The mutability of the output pointer must match the mutability of the
    ///    input pointer.
    /// 3. The output pointer must address valid and fully initialized memory of
    ///    type `To`, and this memory must be valid for at least as long as the
    ///    input memory.
    /// 4. The memory addresses of the output pointers of both functors must
    ///    match.
    pub fn map_ptr<From, To>(
        self,
        origin: Origin,
        by_ref: Option<unsafe fn(from: *const From) -> *const To>,
        by_mut: Option<unsafe fn(from: *mut From) -> *mut To>,
    ) -> RuntimeResult<Self>
    where
        From: ScriptType,
        To: ScriptType,
    {
        let chain = match self.0 {
            Some(chain) => chain,

            None => {
                return Err(RuntimeError::Nil {
                    access_origin: origin,
                })
            }
        };

        let length = chain.0.to.length();

        if length != 1 {
            return Err(RuntimeError::NonSingleton {
                access_origin: origin,
                actual: length,
            });
        }

        let data_type = chain.0.to.ty();
        let expected_type = From::type_meta();

        if data_type != expected_type {
            return Err(RuntimeError::TypeMismatch {
                access_origin: origin,
                data_type: chain.0.to.ty(),
                expected_types: <Vec<_> as ::std::convert::From<[_; 1]>>::from([expected_type]),
            });
        }

        let by_ref = match by_ref {
            Some(by_ref) if chain.0.to.is_readable() => {
                let place = chain.clone().place_ref(origin)?;

                // Safety:
                //   1. PlaceRef granted above.
                //   2. `From` type checked above.
                //   3. Readability checked above.
                //   4. Emptiness checked above.
                let from = unsafe { place.0.to.as_ptr_ref::<From>() };

                // Safety: Upheld by the caller.
                let to = unsafe { by_ref(from) };

                drop(place);

                to
            }

            _ => null(),
        };

        let by_mut = match by_mut {
            Some(by_mut) if chain.0.to.is_writeable() => {
                let place = chain.clone().place_mut(origin)?;

                // Safety:
                //   1. PlaceMut granted above.
                //   2. `From` type checked above.
                //   3. Readability checked above.
                //   4. Emptiness checked above.
                let from = unsafe { place.0.to.as_ptr_mut::<From>() };

                // Safety: Upheld by the caller.
                let to = unsafe { by_mut(from) };

                drop(place);

                to
            }

            _ => null_mut(),
        };

        if by_ref.is_null() && by_mut.is_null() {
            return Ok(Self::default());
        }

        // Safety: Upheld by caller.
        let to = unsafe { MemorySlice::register_ptr(origin, by_ref, by_mut) }?;

        let from = match to.is_owned() {
            true => Self(None),
            false => Self(Some(chain)),
        };

        Ok(Self(Some(Arc::new(Chain(ChainInner {
            from,
            to,
            grant: None,
        })))))
    }

    /// Creates a projection of the array to which this Cell points, mapping it
    /// to a slice of the array.
    ///
    /// Using this function, you can access a single element or a range of
    /// elements within the array.
    ///
    /// The `origin` parameter specifies the range in the Rust or Script source
    /// code where the Cell's data has been mapped.
    ///
    /// The `bounds` parameter is a range of array indices that you want to map.
    /// This range must be within the [length](Cell::length) of the underlying
    /// array, unless the array consists of zero-sized (ZST) elements, in which
    /// case the index bounds can be outside the array bounds.
    ///
    /// If the `bounds` argument specifies an invalid range (e.g., `20..10`), or
    /// if the range is outside the array's bounds (unless the Cell is a ZST
    /// array), the function returns a [RuntimeError].
    ///
    /// The map_slice function does not borrow the original Cell's pointer.
    /// Instead, it creates a new pointer to a subslice of the original memory
    /// allocation (similar to the [map_ptr](Cell::map_ptr) function). When you
    /// borrow the returned projection Cell, the Script Engine borrows both the
    /// original and projection Cells together.
    ///
    /// Even though the map_slice function does not borrow the original data,
    /// if the data is currently borrowed mutably, the function will return an
    /// error.
    pub fn map_slice(self, origin: Origin, bounds: impl RangeBounds<usize>) -> RuntimeResult<Self> {
        let start_bound = match bounds.start_bound() {
            Bound::Included(bound) => *bound,

            Bound::Excluded(bound) => match bound.checked_add(1) {
                Some(bound) => bound,
                None => {
                    return Err(RuntimeError::NumericOperation {
                        invoke_origin: origin,
                        kind: NumericOperationKind::Add,
                        lhs: (<usize>::type_meta(), Arc::new(*bound)),
                        rhs: Some((<usize>::type_meta(), Arc::new(1))),
                        target: <usize>::type_meta(),
                    })
                }
            },

            Bound::Unbounded => 0,
        };

        let end_bound = match bounds.end_bound() {
            Bound::Included(bound) => match bound.checked_add(1) {
                Some(bound) => bound,
                None => {
                    return Err(RuntimeError::NumericOperation {
                        invoke_origin: origin,
                        kind: NumericOperationKind::Add,
                        lhs: (<usize>::type_meta(), Arc::new(*bound)),
                        rhs: Some((<usize>::type_meta(), Arc::new(1))),
                        target: <usize>::type_meta(),
                    })
                }
            },

            Bound::Excluded(bound) => *bound,

            Bound::Unbounded => self.length(),
        };

        if start_bound > end_bound {
            return Err(RuntimeError::MalformedRange {
                access_origin: origin,
                start_bound,
                end_bound,
            });
        }

        let chain = match self.0 {
            Some(chain) => chain,
            None => return Ok(Self::nil()),
        };

        let length = chain.0.to.length();

        if end_bound > length && chain.0.to.ty().size() > 0 {
            return Err(RuntimeError::OutOfBounds {
                access_origin: origin,
                index: end_bound.checked_sub(1).unwrap_or_default(),
                length,
            });
        }

        // 1. and 2. Corresponding place access granted based on the read/write access type.
        // 3. and 4. Bounds checked above.
        let to =
            match (chain.0.to.is_readable(), chain.0.to.is_writeable()) {
                (false, false) => return Ok(Self::nil()),

                (true, false) => unsafe {
                    chain.clone().place_ref(origin)?.0.to.subslice(
                        origin,
                        start_bound,
                        end_bound,
                    )?
                },

                (_, true) => unsafe {
                    chain.clone().place_mut(origin)?.0.to.subslice(
                        origin,
                        start_bound,
                        end_bound,
                    )?
                },
            };

        let from = match to.is_owned() {
            true => Self(None),
            false => Self(Some(chain)),
        };

        Ok(Self(Some(Arc::new(Chain(ChainInner {
            from,
            to,
            grant: None,
        })))))
    }

    #[allow(unused)]
    fn value_ref(mut self, origin: Origin) -> RuntimeResult<Self> {
        if self.0.is_none() {
            return Ok(self);
        }

        match self.0 {
            // Safety: Checked above.
            None => unsafe { debug_unreachable!("Nil Cell borrowing.") },
            Some(chain) => Ok(Self(Some(chain.value_ref(origin)?))),
        }
    }

    #[allow(unused)]
    fn value_mut(mut self, origin: Origin) -> RuntimeResult<Self> {
        if self.0.is_none() {
            return Ok(self);
        }

        match self.0 {
            // Safety: Checked above.
            None => unsafe { debug_unreachable!("Nil Cell borrowing.") },
            Some(chain) => Ok(Self(Some(chain.value_mut(origin)?))),
        }
    }

    fn place_ref(self, origin: Origin) -> RuntimeResult<Self> {
        if self.0.is_none() {
            return Ok(self);
        }

        match self.0 {
            // Safety: Checked above.
            None => unsafe { debug_unreachable!("Nil Cell borrowing.") },
            Some(chain) => Ok(Self(Some(chain.place_ref(origin)?))),
        }
    }

    fn place_mut(self, origin: Origin) -> RuntimeResult<Self> {
        if self.0.is_none() {
            return Ok(self);
        }

        match self.0 {
            // Safety: Checked above.
            None => unsafe { debug_unreachable!("Nil Cell borrowing.") },
            Some(chain) => Ok(Self(Some(chain.place_mut(origin)?))),
        }
    }
}

#[repr(transparent)]
struct Chain(ChainInner);

// Safety:
//   1. Access is atomically guarded by the BorrowTables rules.
//   2. The referred data is Send and Sync.
unsafe impl Send for Chain {}

// Safety:
//   1. Access is atomically guarded by the BorrowTables rules.
//   2. The referred data is Send and Sync.
unsafe impl Sync for Chain {}

impl Debug for Chain {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, formatter)
    }
}

impl Drop for Chain {
    fn drop(&mut self) {
        self.0.release();
    }
}

impl Chain {
    #[inline(always)]
    fn value_ref(self: Arc<Self>, origin: Origin) -> RuntimeResult<Arc<Self>> {
        match Arc::try_unwrap(self) {
            Err(this) => Ok(Arc::new(Self(this.0.value_ref(origin)?))),

            Ok(this) => {
                // Safety: into_inner releases borrow grant.
                Ok(Arc::new(Self(unsafe {
                    this.into_inner().into_value_ref(origin)?
                })))
            }
        }
    }

    #[inline(always)]
    fn value_mut(self: Arc<Self>, origin: Origin) -> RuntimeResult<Arc<Self>> {
        match Arc::try_unwrap(self) {
            Err(this) => Ok(Arc::new(Self(this.0.value_mut(origin)?))),

            Ok(this) => {
                // Safety: into_inner releases borrow grant.
                Ok(Arc::new(Self(unsafe {
                    this.into_inner().into_value_mut(origin)?
                })))
            }
        }
    }

    #[inline(always)]
    fn place_ref(self: Arc<Self>, origin: Origin) -> RuntimeResult<Arc<Self>> {
        match Arc::try_unwrap(self) {
            Err(this) => Ok(Arc::new(Self(this.0.place_ref(origin)?))),

            Ok(this) => {
                // Safety: into_inner releases borrow grant.
                Ok(Arc::new(Self(unsafe {
                    this.into_inner().into_place_ref(origin)?
                })))
            }
        }
    }

    #[inline(always)]
    fn place_mut(self: Arc<Self>, origin: Origin) -> RuntimeResult<Arc<Self>> {
        match Arc::try_unwrap(self) {
            Err(this) => Ok(Arc::new(Self(this.0.place_mut(origin)?))),

            Ok(this) => {
                // Safety: into_inner releases borrow grant.
                Ok(Arc::new(Self(unsafe {
                    this.into_inner().into_place_mut(origin)?
                })))
            }
        }
    }

    #[inline(always)]
    fn take_first<T: ScriptType>(self: Arc<Self>, origin: Origin) -> RuntimeResult<T> {
        match Arc::try_unwrap(self) {
            Err(this) => this.0.clone_inner_first(origin),

            Ok(this) => {
                // Safety: into_inner releases borrow grant.
                unsafe { this.into_inner().take_first(origin) }
            }
        }
    }

    #[inline(always)]
    fn take_vec<T: ScriptType>(self: Arc<Self>, origin: Origin) -> RuntimeResult<Vec<T>> {
        match Arc::try_unwrap(self) {
            Err(this) => Ok(this.0.clone_inner_slice(origin)?.into_vec()),

            Ok(this) => {
                // Safety: into_inner releases borrow grant.
                unsafe { this.into_inner().take_vec(origin) }
            }
        }
    }

    #[inline(always)]
    fn into_inner(self) -> ChainInner {
        // Safety: Transparent layout transmutation.
        let mut inner = unsafe { transmute::<Self, ChainInner>(self) };

        inner.release();

        inner
    }
}

struct ChainInner {
    from: Cell,
    to: Arc<MemorySlice>,
    grant: Option<Grant>,
}

impl Debug for ChainInner {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Cell")
            .field("from", &self.from)
            .field("to", &self.to)
            .field("grant", &self.grant)
            .finish()
    }
}

impl ChainInner {
    #[inline(always)]
    fn data_origin(&self) -> Origin {
        match &self.grant {
            None => *self.to.data_origin(),

            // Safety: ChainInner is well-formed object.
            Some(grant) => unsafe { self.to.grant_origin(grant) },
        }
    }

    #[inline(always)]
    fn value_ref(&self, origin: Origin) -> RuntimeResult<Self> {
        if !self.to.is_readable() {
            return Err(RuntimeError::WriteOnly {
                access_origin: origin,
                data_origin: self.data_origin(),
            });
        }

        let from = self.from.clone().place_ref(origin)?;

        let mut result = Self {
            from,
            to: self.to.clone(),
            grant: None,
        };

        // Safety: `result` is not borrowed.
        unsafe { result.grant_value_ref(origin)? };

        Ok(result)
    }

    #[inline(always)]
    fn value_mut(&self, origin: Origin) -> RuntimeResult<Self> {
        if !self.to.is_writeable() {
            return Err(RuntimeError::ReadOnly {
                access_origin: origin,
                data_origin: self.data_origin(),
            });
        }

        let from = self.from.clone().place_mut(origin)?;

        let mut result = Self {
            from,
            to: self.to.clone(),
            grant: None,
        };

        // Safety: `result` is not borrowed.
        unsafe { result.grant_value_mut(origin)? };

        Ok(result)
    }

    #[inline(always)]
    fn place_ref(&self, origin: Origin) -> RuntimeResult<Self> {
        if !self.to.is_readable() {
            return Err(RuntimeError::WriteOnly {
                access_origin: origin,
                data_origin: self.data_origin(),
            });
        }

        let from = self.from.clone().place_ref(origin)?;

        let mut result = Self {
            from,
            to: self.to.clone(),
            grant: None,
        };

        // Safety: `result` is not borrowed.
        unsafe { result.grant_place_ref(origin)? };

        Ok(result)
    }

    #[inline(always)]
    fn place_mut(&self, origin: Origin) -> RuntimeResult<Self> {
        if !self.to.is_writeable() {
            return Err(RuntimeError::ReadOnly {
                access_origin: origin,
                data_origin: self.data_origin(),
            });
        }

        let from = self.from.clone().place_mut(origin)?;

        let mut result = Self {
            from,
            to: self.to.clone(),
            grant: None,
        };

        // Safety: `result` is not borrowed.
        unsafe { result.grant_place_mut(origin)? };

        Ok(result)
    }

    #[inline(always)]
    fn clone_inner_first<T: ScriptType>(&self, origin: Origin) -> RuntimeResult<T> {
        let data_type = self.to.ty();
        let expected_type = T::type_meta();

        if data_type != expected_type {
            return Err(RuntimeError::TypeMismatch {
                access_origin: origin,
                data_type,
                expected_types: Vec::from([expected_type]),
            });
        }

        let prototype = expected_type.prototype();

        match &self.grant {
            Some(Grant::ValueRef(_)) | Some(Grant::ValueMut(_)) => {
                // Safety:
                //   1. Grant is given if and only if MemorySlice has
                //      corresponding read/write access.
                //   2. Item type checked above.
                let slice: &[T] = unsafe { self.to.as_slice_ref::<T>() };

                // Safety: Item type checked above.
                return unsafe {
                    prototype.clone_first::<T>(&origin, self.to.data_origin(), slice)
                };
            }

            _ => (),
        }

        if self.to.is_readable() {
            let grant = self.to.grant_value_ref(origin)?;

            let first_clone = {
                // Safety:
                //   1. Temporary ValueRef access granted.
                //   2. Type is checked above.
                //   3. Read-access checked above.
                let slice: &[T] = unsafe { self.to.as_slice_ref::<T>() };

                // Safety: Item type checked above.
                unsafe { prototype.clone_first::<T>(&origin, self.to.data_origin(), slice) }
            };

            // Safety: Releasing access granted above.
            unsafe { self.to.release_grant(grant) };

            return first_clone;
        }

        if self.to.is_writeable() {
            let grant = self.to.grant_value_mut(origin)?;

            let first_clone = {
                // Safety:
                //   1. Temporary ValueMut access granted.
                //   2. Type is checked above.
                //   3. Read-access checked above.
                let slice: &[T] = unsafe { self.to.as_slice_ref::<T>() };

                // Safety: Item type checked above.
                unsafe { prototype.clone_first::<T>(&origin, self.to.data_origin(), slice) }
            };

            // Safety: Releasing access granted above.
            unsafe { self.to.release_grant(grant) };

            return Ok(first_clone?);
        }

        // Safety: In the public interface non-readable and non-writeable cells
        //         represented as Nil.
        unsafe { debug_unreachable!("Chain without access.") }
    }

    #[inline(always)]
    fn clone_inner_slice<T: ScriptType>(&self, origin: Origin) -> RuntimeResult<Box<[T]>> {
        let data_type = self.to.ty();
        let expected_type = T::type_meta();

        if data_type != expected_type {
            return Err(RuntimeError::TypeMismatch {
                access_origin: origin,
                data_type,
                expected_types: Vec::from([expected_type]),
            });
        }

        let prototype = expected_type.prototype();

        match &self.grant {
            Some(Grant::ValueRef(_)) | Some(Grant::ValueMut(_)) => {
                // Safety:
                //   1. Grant is given if and only if MemorySlice has
                //      corresponding read/write access.
                //   2. Item type checked above.
                let slice: &[T] = unsafe { self.to.as_slice_ref::<T>() };

                // Safety: Item type checked above.
                return unsafe {
                    prototype.clone_slice::<T>(&origin, self.to.data_origin(), slice)
                };
            }

            _ => (),
        }

        if self.to.is_readable() {
            let grant = self.to.grant_value_ref(origin)?;

            let slice_clone = {
                // Safety:
                //   1. Temporary ValueRef access granted.
                //   2. Type is checked above.
                //   3. Read-access checked above.
                let slice: &[T] = unsafe { self.to.as_slice_ref::<T>() };

                // Safety: Item type checked above.
                unsafe { prototype.clone_slice::<T>(&origin, self.to.data_origin(), slice) }
            };

            // Safety: Releasing access granted above.
            unsafe { self.to.release_grant(grant) };

            return slice_clone;
        }

        if self.to.is_writeable() {
            let grant = self.to.grant_value_mut(origin)?;

            let slice_clone = {
                // Safety:
                //   1. Temporary ValueMut access granted.
                //   2. Type is checked above.
                //   3. Read-access checked above.
                let slice: &[T] = unsafe { self.to.as_slice_ref::<T>() };

                // Safety: Item type checked above.
                unsafe { prototype.clone_slice::<T>(&origin, self.to.data_origin(), slice) }
            };

            // Safety: Releasing access granted above.
            unsafe { self.to.release_grant(grant) };

            return slice_clone;
        }

        // Safety: In the public interface non-readable and non-writeable cells
        //         represented as Nil.
        unsafe { debug_unreachable!("Chain without access.") }
    }

    // Safety: ChainInner is not borrowed.
    #[inline(always)]
    unsafe fn into_value_ref(mut self, origin: Origin) -> RuntimeResult<Self> {
        if !self.to.is_readable() {
            return Err(RuntimeError::WriteOnly {
                access_origin: origin,
                data_origin: self.data_origin(),
            });
        }

        self.from = self.from.place_ref(origin)?;

        // Safety: Upheld by the caller.
        unsafe { self.grant_value_ref(origin)? };

        Ok(self)
    }

    // Safety: ChainInner is not borrowed.
    #[inline(always)]
    unsafe fn into_value_mut(mut self, origin: Origin) -> RuntimeResult<Self> {
        if !self.to.is_writeable() {
            return Err(RuntimeError::ReadOnly {
                access_origin: origin,
                data_origin: self.data_origin(),
            });
        }

        self.from = self.from.place_mut(origin)?;

        // Safety: Upheld by the caller.
        unsafe { self.grant_value_mut(origin)? };

        Ok(self)
    }

    // Safety: ChainInner is not borrowed.
    #[inline(always)]
    unsafe fn into_place_ref(mut self, origin: Origin) -> RuntimeResult<Self> {
        if !self.to.is_readable() {
            return Err(RuntimeError::WriteOnly {
                access_origin: origin,
                data_origin: self.data_origin(),
            });
        }

        self.from = self.from.place_ref(origin)?;

        // Safety: Upheld by the caller.
        unsafe { self.grant_place_ref(origin)? };

        Ok(self)
    }

    // Safety: ChainInner is not borrowed.
    #[inline(always)]
    unsafe fn into_place_mut(mut self, origin: Origin) -> RuntimeResult<Self> {
        if !self.to.is_writeable() {
            return Err(RuntimeError::ReadOnly {
                access_origin: origin,
                data_origin: self.data_origin(),
            });
        }

        self.from = self.from.place_mut(origin)?;

        // Safety: Upheld by the caller.
        unsafe { self.grant_place_mut(origin)? };

        Ok(self)
    }

    // Safety: ChainInner is not borrowed.
    #[inline]
    unsafe fn take_first<T: ScriptType>(mut self, origin: Origin) -> RuntimeResult<T> {
        debug_assert!(
            self.grant.is_none(),
            "An attempt to move borrowed data out of Cell.",
        );

        if self.to.is_owned() {
            self.to = match Arc::try_unwrap(self.to) {
                Err(to) => to,

                Ok(to) => {
                    let data_type = to.ty();
                    let expected_type = T::type_meta();

                    if data_type != expected_type {
                        return Err(RuntimeError::TypeMismatch {
                            access_origin: origin,
                            data_type,
                            expected_types: Vec::from([expected_type]),
                        });
                    }

                    let length = to.length();

                    if length != 1 {
                        return Err(RuntimeError::NonSingleton {
                            access_origin: origin,
                            actual: length,
                        });
                    }

                    // Safety:
                    //   1. Ownership flag checked above.
                    //   2. Item type checked above.
                    //   3. Absence of borrow grant upheld by the caller.
                    let vector = unsafe { to.into_vec() };

                    return match vector.into_iter().next() {
                        Some(first) => Ok(first),

                        // Safety: Slice length checked above.
                        None => unsafe { debug_unreachable!("Missing slice first item.") },
                    };
                }
            }
        }

        self.clone_inner_first(origin)
    }

    // Safety: ChainInner is not borrowed.
    #[inline]
    unsafe fn take_vec<T: ScriptType>(mut self, origin: Origin) -> RuntimeResult<Vec<T>> {
        debug_assert!(
            self.grant.is_none(),
            "An attempt to move borrowed data out of Cell.",
        );

        if self.to.is_owned() {
            self.to = match Arc::try_unwrap(self.to) {
                Err(to) => to,

                Ok(to) => {
                    let data_type = to.ty();
                    let expected_type = T::type_meta();

                    if data_type != expected_type {
                        return Err(RuntimeError::TypeMismatch {
                            access_origin: origin,
                            data_type,
                            expected_types: Vec::from([expected_type]),
                        });
                    }

                    // Safety:
                    //   1. Ownership flag checked above.
                    //   2. Item type checked above.
                    //   3. Absence of borrow grant upheld by the caller.
                    return Ok(unsafe { to.into_vec() });
                }
            }
        }

        Ok(self.clone_inner_slice(origin)?.into_vec())
    }

    // Safety: ChainInner is not borrowed.
    #[inline(always)]
    unsafe fn grant_value_ref(&mut self, origin: Origin) -> RuntimeResult<()> {
        let grant = self.to.grant_value_ref(origin)?;

        if replace(&mut self.grant, Some(grant)).is_some() {
            // Safety: Upheld by the caller.
            unsafe {
                debug_unreachable!("An attempt to set new borrow grant without prior release.");
            }
        }

        Ok(())
    }

    // Safety: ChainInner is not borrowed.
    #[inline(always)]
    unsafe fn grant_value_mut(&mut self, origin: Origin) -> RuntimeResult<()> {
        let grant = self.to.grant_value_mut(origin)?;

        if replace(&mut self.grant, Some(grant)).is_some() {
            // Safety: Upheld by the caller.
            unsafe {
                debug_unreachable!("An attempt to set new borrow grant without prior release.");
            }
        }

        Ok(())
    }

    // Safety: ChainInner is not borrowed.
    #[inline(always)]
    unsafe fn grant_place_ref(&mut self, origin: Origin) -> RuntimeResult<()> {
        let grant = self.to.grant_place_ref(origin)?;

        if replace(&mut self.grant, Some(grant)).is_some() {
            // Safety: Upheld by the caller.
            unsafe {
                debug_unreachable!("An attempt to set new borrow grant without prior release.");
            }
        }

        Ok(())
    }

    // Safety: ChainInner is not borrowed.
    #[inline(always)]
    unsafe fn grant_place_mut(&mut self, origin: Origin) -> RuntimeResult<()> {
        let grant = self.to.grant_place_mut(origin)?;

        if replace(&mut self.grant, Some(grant)).is_some() {
            // Safety: Upheld by the caller.
            unsafe {
                debug_unreachable!("An attempt to set new borrow grant without prior release.");
            }
        }

        Ok(())
    }

    #[inline(always)]
    fn release(&mut self) {
        if let Some(grant) = take(&mut self.grant) {
            // Safety: ChainInner is well-formed object.
            unsafe { self.to.release_grant(grant) }
        }
    }
}
