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
    mem::{take, transmute},
    sync::Arc,
};

use crate::runtime::{
    memory::MemorySlice,
    Cell,
    Origin,
    RuntimeError,
    RuntimeResult,
    ScriptType,
    TypeHint,
    TypeMeta,
};

/// A trait that casts Script data into Rust data.
///
/// By implementing the Downcast trait on a Rust type, you make this type
/// eligible to be part of the parameter signature of exported functions.
///
/// The Script Engine uses the Downcast implementation of the Rust type to
/// transform a call containing Script data into a Rust object, passing this
/// object into the exported Rust function as a function argument.
///
/// ```
/// # use ad_astra::export;
/// #
/// # #[export(include)]
/// # #[export(package)]
/// # #[derive(Default)]
/// # struct Package;
/// #
/// #[export]
/// fn foo(_x: usize) {} // The usize type implements the Downcast trait.
/// ```
///
/// The opposite operation of transforming Rust data into Script data is
/// provided through the separate [Upcast] trait.
///
/// ## Automatic Implementation
///
/// When you use the [export](crate::export) macro to export a Rust structure
/// `Foo`, the macro automatically implements the Downcast trait for `Foo`,
/// `&Foo`, and `&mut Foo`. This makes these types usable as parameters in
/// exported functions.
///
/// ```
/// # use ad_astra::{
/// #     export,
/// #     runtime::{Cell, Downcast, Origin, Provider},
/// # };
/// #
/// # #[export(include)]
/// # #[export(package)]
/// # #[derive(Default)]
/// # struct Package;
/// #
/// #[export]
/// #[derive(Clone)]
/// pub struct Foo;
///
/// let foo = Cell::give(Origin::nil(), Foo).unwrap();
///
/// // Foo cannot be downcasted to usize.
/// assert!(<usize>::downcast(Origin::nil(), Provider::Owned(foo.clone())).is_err());
///
/// // Foo can be downcasted to Foo.
/// assert!(<Foo>::downcast(Origin::nil(), Provider::Owned(foo.clone())).is_ok());
///
/// // Foo can be downcasted to &Foo.
/// let mut foo = foo.clone();
/// assert!(<&Foo>::downcast(Origin::nil(), Provider::Borrowed(&mut foo)).is_ok());
///
/// // Foo can be downcasted to &mut Foo.
/// let mut foo = foo.clone();
/// assert!(<&mut Foo>::downcast(Origin::nil(), Provider::Borrowed(&mut foo)).is_ok());
///
/// // The Foo, &Foo, and &mut Foo types are eligible as types in the exported
/// // function signature.
/// #[export]
/// fn exported_function(foo1: Foo, foo2: &Foo, foo3: &mut Foo) {}
/// ```
///
/// ## Manual Implementation
///
/// To manually implement the Downcast trait for an exported Rust structure,
/// you should export a type alias to this structure instead of the structure
/// itself. The macro system does not automatically implement Downcast for
/// type aliases, allowing for manual implementation.
///
/// Generally, the Downcast trait can be manually implemented on any Rust type,
/// not just the exported type.
///
/// This allows for providing custom Rust representations of Script data. For
/// example, this Ad Astra crate implements Downcast for the [Option] container,
/// even though Option is not a [ScriptType].
///
/// When you downcast a Cell to an Option, the underlying implementation wraps
/// the Cell's data into [Some] if the Cell is not [nil](Cell::nil); otherwise,
/// the downcast procedure returns [None].
///
/// ```
/// # use ad_astra::{
/// #     export,
/// #     runtime::{Cell, Downcast, Origin, Provider},
/// # };
/// #
/// # #[export(include)]
/// # #[export(package)]
/// # #[derive(Default)]
/// # struct Package;
/// #
/// assert_eq!(
///     <Option<usize>>::downcast(Origin::nil(), Provider::Owned(Cell::nil())).unwrap(),
///     None,
/// );
///
/// assert_eq!(
///     <Option<usize>>::downcast(
///         Origin::nil(),
///         Provider::Owned(Cell::give(Origin::nil(), 100usize).unwrap())
///     )
///     .unwrap(),
///     Some(100usize),
/// );
///
/// assert_eq!(
///     <Option<Option<f64>>>::downcast(
///         Origin::nil(),
///         Provider::Owned(Cell::give(Origin::nil(), 10.5f64).unwrap())
///     )
///     .unwrap(),
///     Some(Some(10.5f64)),
/// );
///
/// #[export]
/// fn exported_function(_x: Option<usize>, _u: Option<Option<f64>>) {}
/// ```
///
/// ## Type Casting
///
/// Using the Downcast trait, you can perform non-trivial casting of independent
/// data types. For example, through the Downcast trait (and the [Upcast] trait),
/// the Ad Astra crate provides type casting between standard built-in primitive
/// numeric types.
///
/// ```
/// # use ad_astra::runtime::{Cell, Downcast, Origin, Provider};
/// #
/// let num = Cell::give(Origin::nil(), 100u32).unwrap();
///
/// assert_eq!(
///     <u32>::downcast(Origin::nil(), Provider::Owned(num.clone())).unwrap(),
///     100u32,
/// );
///
/// assert_eq!(
///     <f64>::downcast(Origin::nil(), Provider::Owned(num.clone())).unwrap(),
///     100.0f64,
/// );
/// ```
///
/// To implement type casting manually, you can use the [Cell::type_match]
/// function, which returns a helper [TypeMatch] object. This object allows you
/// to enumerate the possible Script types of the Cell and handle each case
/// manually, providing the corresponding type castings. If the Cell's type does
/// not match any of the expected types, you should call [TypeMatch::mismatch]
/// at the end of the implementation. This will return a descriptive
/// [RuntimeError] containing each type that you attempted to handle.
///
/// ```
/// # use ad_astra::{
/// #     export,
/// #     runtime::{Cell, Downcast, Origin, Provider, RuntimeResult, ScriptType, TypeHint},
/// # };
/// #
/// #[derive(Debug, PartialEq)]
/// struct Foo(bool);
///
/// // Exporting the type alias instead of the struct turns off automatic
/// // implementations of the Downcast trait on this struct. Therefore, you can
/// // implement the trait manually.
/// #[export]
/// type FooAlias = Foo;
///
/// impl<'a> Downcast<'a> for Foo {
///     fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
///         let cell = provider.to_owned();
///
///         let mut type_match = cell.type_match();
///
///         // If the provided Cell is "Foo", no casting is needed; we can take
///         // its data as it is.
///         if type_match.is::<Foo>() {
///             return cell.take::<Foo>(origin);
///         }
///
///         // If the provided Cell is "bool", the downcast function wraps this
///         // value into Foo.
///         if type_match.is::<bool>() {
///             let inner = cell.take::<bool>(origin)?;
///
///             return Ok(Foo(inner));
///         }
///
///         // Otherwise, return an error. The Foo object cannot be
///         // constructed from any Script type that this downcasting procedure
///         // supports.
///         Err(type_match.mismatch(origin))
///     }
///
///     fn hint() -> TypeHint {
///         Foo::type_meta().into()
///     }
/// }
///
/// assert_eq!(
///     Foo::downcast(
///         Origin::nil(),
///         Provider::Owned(Cell::give_vec(Origin::nil(), vec![Foo(true)]).unwrap())
///     )
///     .unwrap(),
///     Foo(true),
/// );
///
/// assert_eq!(
///     Foo::downcast(
///         Origin::nil(),
///         Provider::Owned(Cell::give(Origin::nil(), true).unwrap())
///     )
///     .unwrap(),
///     Foo(true),
/// );
///
/// assert!(Foo::downcast(
///     Origin::nil(),
///     Provider::Owned(Cell::give(Origin::nil(), 12345usize).unwrap())
/// )
/// .is_err());
/// ```
///
/// ## Compositions
///
/// When implementing the Downcast trait for a generic container like `Wrapper`,
/// you can require the generic parameter to also implement Downcast. This
/// allows you to downcast the generic parameter within the container's Downcast
/// implementation.
///
/// ```
/// # use ad_astra::runtime::{Cell, Downcast, Origin, Provider, RuntimeResult, TypeHint};
/// #
/// #[derive(Debug, PartialEq)]
/// struct Wrapper<T>(T);
///
/// impl<'a, T> Downcast<'a> for Wrapper<T>
/// where
///     T: Downcast<'a>,
/// {
///     fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
///         let inner = <T as Downcast<'a>>::downcast(origin, provider)?;
///
///         Ok(Wrapper(inner))
///     }
///
///     fn hint() -> TypeHint {
///         <T as Downcast<'a>>::hint()
///     }
/// }
///
/// assert_eq!(
///     <Wrapper<Wrapper<usize>>>::downcast(
///         Origin::nil(),
///         Provider::Owned(Cell::give(Origin::nil(), 100usize).unwrap()),
///     )
///     .unwrap(),
///     Wrapper(Wrapper(100usize))
/// );
/// ```
///
/// In this setup, compositions of Downcast types become Downcast as well:
/// `Wrapper<T>`, `Wrapper<Option<T>>`, `Option<Wrapper<T>>`, and other possible
/// combinations are all Downcast types.
///
/// ## Lifetime
///
/// The Downcast trait has a lifetime parameter `'a`. This parameter indicates
/// the lifetime of the target type and the lifetime of the input Cell data.
///
/// If the target type is just an owned type with the `'static` lifetime, this
/// generic parameter does not matter for the implementation. The implementation
/// is likely to fetch this owned data from the Cell using the [Cell::take]
/// and related functions.
///
/// ```
/// use ad_astra::{
///     export,
///     runtime::{Cell, Downcast, Origin, Provider, RuntimeResult, ScriptType, TypeHint},
/// };
///
/// #[derive(Debug)]
/// struct Foo;
///
/// // Exporting the type alias instead of the struct turns off automatic
/// // implementations of the Downcast trait on this struct. Therefore, you can
/// // implement the trait manually.
/// #[export]
/// type FooAlias = Foo;
///
/// impl<'a> Downcast<'a> for Foo {
///     fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
///         // Note that `to_owned` is infallible. Any provided Cell can be
///         // turned into an owned Cell instance.
///         let cell = provider.to_owned();
///
///         cell.take(origin)
///     }
///
///     fn hint() -> TypeHint {
///         Foo::type_meta().into()
///     }
/// }
///
/// // This implementation works perfectly fine with both `Provider::Owned` and
/// // `Provider::Borrowed`.
///
/// assert!(Foo::downcast(
///     Origin::nil(),
///     Provider::Owned(Cell::give_vec(Origin::nil(), vec![Foo]).unwrap()),
/// )
/// .is_ok());
///
/// let mut cell = Cell::give_vec(Origin::nil(), vec![Foo]).unwrap();
///
/// assert!(Foo::downcast(Origin::nil(), Provider::Borrowed(&mut cell),).is_ok());
/// ```
///
/// However, if the Downcast trait is implemented for `&'a T` and similar
/// referential types or their wrappers, the trait implementation will need
/// to borrow the provided Cell eventually (using the [Cell::borrow_ref],
/// [Cell::borrow_mut], and similar functions).
///
/// The lifetime of the reference received from the Cell's borrowing functions
/// must match the `'a` lifetime parameter of the Downcast trait. For this
/// reason, the [Downcast::downcast] function receives a [Provider] type instead
/// of just a Cell. Provider is a simple wrapper for a Cell that can either be
/// an owned wrapper (`Provider::Owned(cell)`) or a wrapper for a mutable
/// reference to the Cell (`Provider::Borrowed(&'a mut cell)`).
///
/// Each of these variants can be turned into an owned Cell using
/// [Provider::to_owned], but only the referential variant is eligible for
/// borrowing. The [Provider::to_borrowed] function returns a [RuntimeError]
/// if the variant is not `Borrowed`.
///
/// When implementing a Downcast trait for a referential type (e.g., `&'a T` or
/// `&'a mut T`), you should typically attempt to turn the provider into a
/// reference to the Cell using the [Provider::to_borrowed] function and then
/// borrow the underlying data of the Cell.
///
/// ```
/// use ad_astra::{
///     export,
///     runtime::{Cell, Downcast, Origin, Provider, RuntimeResult, ScriptType, TypeHint},
/// };
///
/// #[derive(Debug)]
/// struct Foo;
///
/// #[export]
/// type FooAlias = Foo;
///
/// impl<'a> Downcast<'a> for &'a Foo {
///     fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
///         // Note that `to_borrowed` can fail if the provider variant is not
///         // `Borrowed`.
///         let cell = provider.to_borrowed(&origin)?;
///
///         cell.borrow_ref(origin)
///     }
///
///     fn hint() -> TypeHint {
///         Foo::type_meta().into()
///     }
/// }
///
/// let mut cell = Cell::give_vec(Origin::nil(), vec![Foo]).unwrap();
///
/// // You can downcast `Provider::Borrowed` into `&Foo`.
/// assert!(<&Foo>::downcast(Origin::nil(), Provider::Borrowed(&mut cell)).is_ok());
///
/// let cell = Cell::give_vec(Origin::nil(), vec![Foo]).unwrap();
///
/// // But you cannot downcast `Provider::Owned` into `&Foo`.
/// assert!(<&Foo>::downcast(Origin::nil(), Provider::Owned(cell)).is_err());
/// ```
pub trait Downcast<'a>: Sized + Send + Sync + 'a {
    /// Transforms Script data into Rust data.
    ///
    /// The `origin` parameter specifies the range in the Rust or Script source
    /// code where the transformation has been requested.
    ///
    /// The `provider` parameter is a wrapper around a [Cell] that points to the
    /// Script data that needs to be transformed.
    ///
    /// The function returns a [RuntimeError] if the underlying implementation
    /// is unable to perform the transformation into the requested Rust type.
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self>;

    /// Returns a rough estimation of the Script type (or a set of types) from
    /// which the target Rust type could be inferred.
    ///
    /// The underlying implementation makes a best effort to provide as precise
    /// type information as possible to support static semantic analysis of the
    /// script code. However, the resulting [TypeHint] object may be imprecise,
    /// up to indicating [TypeHint::dynamic], which means that the source
    /// type(s) are not known at compile time.
    fn hint() -> TypeHint;
}

/// A wrapper around a [Cell] that provides either borrowing or owning access
/// to the Cell's data.
///
/// This object is used as a parameter for the [Downcast::downcast] function.
///
/// If the Provider owns a Cell (via the `Owned` variant) or borrows it (via the
/// `Borrowed` variant), the downcast function can take ownership of the Cell's
/// data. However, the downcast function can only dereference the data of the
/// Cell if the Provider is borrowing this Cell.
///
/// For more details, see the [Downcast's Lifetime](Downcast#lifetime)
/// documentation.
pub enum Provider<'a> {
    /// The Provider only provides ownership access to the Cell's data.
    Owned(Cell),

    /// The Provider provides both ownership and dereferencing access to the
    /// Cell.
    Borrowed(&'a mut Cell),
}

impl<'a> AsRef<Cell> for Provider<'a> {
    #[inline(always)]
    fn as_ref(&self) -> &Cell {
        match self {
            Self::Owned(cell) => cell,
            Self::Borrowed(cell) => cell,
        }
    }
}

impl<'a> Provider<'a> {
    /// Provides a convenient helper interface for handling Cell data based on
    /// the [Cell type](Cell::ty).
    ///
    /// This function is similar to [Cell::type_match].
    ///
    /// For more details, see [TypeMatch].
    #[inline(always)]
    pub fn type_match(&'a self) -> TypeMatch<'a> {
        match self {
            Self::Owned(cell) => TypeMatch {
                cell,
                expected: Vec::new(),
            },

            Self::Borrowed(cell) => TypeMatch {
                cell,
                expected: Vec::new(),
            },
        }
    }

    /// Takes ownership of the Provider's Cell.
    ///
    /// This function is infallible regardless of the Provider's variant. If the
    /// variant is `Borrowed`, this function replaces the referenced instance
    /// with [Cell::nil] and returns the original Cell instance.
    #[inline(always)]
    pub fn to_owned(self) -> Cell {
        match self {
            Self::Owned(cell) => cell,
            Self::Borrowed(cell) => take(cell),
        }
    }

    /// Takes a mutable reference to the Provider's Cell.
    ///
    /// If the Provider's variant is `Borrowed`, the function returns the
    /// underlying mutable reference; otherwise, it returns a [RuntimeError].
    ///
    /// The `origin` parameter specifies the source code range in Rust or Script
    /// where the data of the Cell is about to be accessed. This parameter is
    /// used to create an error if the Provider's variant is `Owned`.
    #[inline(always)]
    pub fn to_borrowed(self, origin: &Origin) -> RuntimeResult<&'a mut Cell> {
        match self {
            Provider::Owned(_) => Err(RuntimeError::DowncastStatic {
                access_origin: *origin,
            }),
            Provider::Borrowed(cell) => Ok(cell),
        }
    }
}

/// A helper object that provides a convenient way to match on the Cell's type.
///
/// This object is created by the [Cell::type_match] or [Provider::type_match]
/// functions and helps you implement different type casting logic based on the
/// Cell's data type. It is intended for use in manual implementations of the
/// [Downcast], [Upcast], [ops](crate::runtime::ops) traits, and other scenarios
/// where you need to handle Cell data based on the [Cell type](Cell::ty).
///
/// The [TypeMatch::is] and [TypeMatch::belongs_to] matching functions return
/// true if the Cell's data corresponds to a particular Script type. Through
/// these functions, you enumerate all possible Cell data types that your
/// implementation supports. Whenever you encounter a supported type (the
/// matching function returns true), you handle the Cell accordingly and
/// return a meaningful successful result.
///
/// If no matching cases are found, you fall back to the [RuntimeError] that
/// TypeMatch generates for you by calling the [TypeMatch::mismatch] function.
/// This function returns a descriptive error indicating that the provided
/// Cell's type does not match any of the expected types enumerated by the
/// matching functions.
///
/// ```
/// # use ad_astra::{
/// #     export,
/// #     runtime::{Downcast, Origin, Provider, RuntimeResult, ScriptType, TypeHint},
/// # };
/// #
/// # #[derive(Debug, PartialEq)]
/// # struct Foo(bool);
/// #
/// # #[export]
/// # type FooAlias = Foo;
/// #
/// impl<'a> Downcast<'a> for Foo {
///     fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
///         let cell = provider.to_owned();
///
///         let mut type_match = cell.type_match();
///
///         // The Cell type is "Foo". Handling this case.
///         if type_match.is::<Foo>() {
///             return cell.take::<Foo>(origin);
///         }
///
///         // The Cell type is "bool". Handling this case.
///         if type_match.is::<bool>() {
///             let inner = cell.take::<bool>(origin)?;
///
///             return Ok(Foo(inner));
///         }
///
///         // Otherwise, returning an error indicating that the Cell type
///         // should be either "Foo" or "bool".
///         Err(type_match.mismatch(origin))
///     }
///
///     fn hint() -> TypeHint {
///         Foo::type_meta().into()
///     }
/// }
/// ```
pub struct TypeMatch<'a> {
    cell: &'a Cell,
    expected: Vec<&'static TypeMeta>,
}

impl<'a> TypeMatch<'a> {
    /// Checks if the [Cell's type](Cell::is) is exactly of type `T`.
    ///
    /// If the Cell's type is not `T`, it returns false and remembers that `T`
    /// is one of the expected types.
    #[inline(always)]
    pub fn is<T: ScriptType + ?Sized>(&mut self) -> bool {
        if self.cell.is::<T>() {
            return true;
        }

        let ty = T::type_meta();

        let _ = self.expected.push(ty);

        false
    }

    /// Checks if the [Cell's type](Cell::ty) and the type `T` belong to the
    /// same [type family](crate::runtime::TypeFamily).
    ///
    /// If the Cell's type does not belong to `T`'s family, it returns false
    /// and remembers that all types from the `T` type family are the expected
    /// types.
    #[inline(always)]
    pub fn belongs_to<T: ScriptType + ?Sized>(&mut self) -> bool {
        let family = T::type_meta().family();

        if self.cell.ty().family() == family {
            return true;
        }

        for ty in family {
            let _ = self.expected.push(ty);
        }

        false
    }

    /// Returns a reference to the Cell that the TypeMatch object is
    /// matching on.
    #[inline(always)]
    pub fn cell(&self) -> &'a Cell {
        self.cell
    }

    /// Creates a [RuntimeError] indicating that the Cell's type does not match
    /// any of the expected types.
    ///
    /// This function should be called as the last statement after all matching
    /// cases ([is](Self::is) and [belongs_to](Self::belongs_to) functions) have
    /// been checked, and all of them return false.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// where the Cell was supposed to be accessed.
    #[inline(always)]
    pub fn mismatch(self, origin: Origin) -> RuntimeError {
        return RuntimeError::TypeMismatch {
            access_origin: origin,
            data_type: self.cell.ty(),
            expected_types: self.expected,
        };
    }
}

impl Cell {
    /// Provides a convenient helper interface for handling Cell data based on
    /// the [Cell type](Cell::ty).
    ///
    /// For more details, see [TypeMatch].
    #[inline(always)]
    pub fn type_match(&self) -> TypeMatch {
        TypeMatch {
            cell: self,
            expected: Vec::new(),
        }
    }
}

/// A trait that casts Rust data into Script data.
///
/// By implementing the Upcast trait for a Rust type, you make this type
/// eligible for returning values from exported functions.
///
/// The Script Engine uses the Upcast implementation of a Rust type to
/// transfer data returned by a Rust function back to the Script Engine.
///
/// ```
/// # use ad_astra::export;
/// #
/// # #[export(include)]
/// # #[export(package)]
/// # #[derive(Default)]
/// # struct Package;
/// #
/// #[export]
/// fn foo() -> usize { 123 } // The usize type implements the Upcast trait.
/// ```
///
/// The opposite operation of transferring Script data into Rust is
/// provided by the separate [Downcast] trait.
///
/// ## Automatic Implementation
///
/// When you use the [export](crate::export) attribute on a Rust structure
/// `Foo`, the export macro automatically implements the Upcast trait for `Foo`,
/// `&Foo`, and `&mut Foo`. This allows these types to be used as return types
/// for exported functions.
///
/// ```
/// # use ad_astra::{
/// #     export,
/// #     runtime::{Cell, Origin},
/// # };
/// #
/// # #[export(include)]
/// # #[export(package)]
/// # #[derive(Default)]
/// # struct Package;
/// #
/// #[export]
/// #[derive(Clone)]
/// pub struct Foo;
///
/// // Cell::give requires that the data parameter implements Upcast.
/// let _ = Cell::give(Origin::nil(), Foo).unwrap();
///
/// #[export]
/// impl Foo {
///     // Foo implements Upcast, so you can return Foo.
///     fn exported_method_1(self) -> Foo { self }
///
///     // &Foo implements Upcast, so you can return &Foo.
///     fn exported_method_2(&self) -> &Foo { self }
///
///     // &mut Foo implements Upcast, so you can return &mut Foo.
///     fn exported_method_3(&mut self) -> &mut Foo { self }
/// }
/// ```
///
/// ## Manual Implementation
///
/// To manually implement the Upcast trait for an exported Rust structure, you
/// should export a type alias for this structure instead of the structure
/// itself. The macro system does not automatically implement Upcast for type
/// aliases, allowing for manual implementation.
///
/// Generally, the Upcast trait can be manually implemented for any Rust type,
/// not necessarily an exported type. This flexibility enables custom Rust
/// representations of Script data. For example, the Ad Astra crate implements
/// Upcast for the [Option] container, even though Option is not a [ScriptType].
///
/// When upcasting an Option to Cell, the underlying implementation returns
/// [Cell::nil] if the Option is [None]; otherwise, it unwraps the inner object
/// and upcasts it.
///
/// ```
/// # use ad_astra::{
/// #     export,
/// #     runtime::{Cell, Origin},
/// # };
/// #
/// # #[export(include)]
/// # #[export(package)]
/// # #[derive(Default)]
/// # struct Package;
/// #
/// let cell = Cell::give(Origin::nil(), Some(100usize)).unwrap();
///
/// assert_eq!(cell.take::<usize>(Origin::nil()).unwrap(), 100);
///
/// let cell = Cell::give(Origin::nil(), Some(Some(100usize))).unwrap();
///
/// assert_eq!(cell.take::<usize>(Origin::nil()).unwrap(), 100);
///
/// let cell = Cell::give(Origin::nil(), Option::<usize>::None).unwrap();
///
/// assert!(cell.is_nil());
///
/// #[export]
/// fn exported_function_1() -> Option<usize> { Some(100) }
///
/// #[export]
/// fn exported_function_2() -> Option<Option<usize>> { Some(Some(100)) }
/// ```
///
/// When manually implementing the Upcast trait, you must specify an
/// [Upcast::Output] associated type that denotes the result of the upcast.
/// This type is limited to a certain set of possible types. To upcast the type
/// to a script-registered type `T`, set the Output to `Box<T>`, and create a
/// box containing the input data (or a transformed version of it) within the
/// upcast implementation, thereby transferring the data to the heap.
///
/// ```
/// # use ad_astra::{
/// #     export,
/// #     runtime::{Cell, Origin, RuntimeResult, ScriptType, TypeHint, Upcast},
/// # };
/// #
/// #[derive(Debug, PartialEq)]
/// struct Foo;
///
/// // Exporting a type alias instead of the struct disables automatic
/// // implementations of Upcast for this struct, allowing for manual trait
/// // implementation.
/// #[export]
/// type FooAlias = Foo;
///
/// impl<'a> Upcast<'a> for Foo {
///     type Output = Box<Foo>;
///
///     fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
///         Ok(Box::new(this))
///     }
///
///     fn hint() -> TypeHint {
///         Foo::type_meta().into()
///     }
/// }
///
/// let cell = Cell::give(Origin::nil(), Foo).unwrap();
///
/// assert_eq!(cell.take::<Foo>(Origin::nil()).unwrap(), Foo);
/// ```
///
/// ## Compositions
///
/// When implementing the Upcast trait for a generic container, such as
/// `Wrapper`, you can require that the generic parameter also implements
/// Upcast, and then upcast the generic parameter within the container's Upcast
/// implementation.
///
/// ```
/// # use ad_astra::runtime::{Cell, Origin, RuntimeResult, TypeHint, Upcast};
/// #
/// struct Wrapper<T>(T);
///
/// impl<'a, T> Upcast<'a> for Wrapper<T>
/// where
///     T: Upcast<'a>,
/// {
///     type Output = <T as Upcast<'a>>::Output;
///
///     #[inline(always)]
///     fn upcast(origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
///         <T as Upcast<'a>>::upcast(origin, this.0)
///     }
///
///     #[inline(always)]
///     fn hint() -> TypeHint {
///         <T as Upcast<'a>>::hint()
///     }
/// }
///
/// let cell = Cell::give(Origin::nil(), Wrapper(100usize)).unwrap();
///
/// assert_eq!(cell.take::<usize>(Origin::nil()).unwrap(), 100);
/// ```
///
/// This approach allows compositions of Upcast types to also be Upcast:
/// `Wrapper<T>`, `Wrapper<Option<T>>`, `Option<Wrapper<T>>`, and other
/// possible combinations are all Upcast types.
///
/// ## Lifetime
///
/// The Upcast trait has a lifetime parameter `'a`. This parameter indicates
/// the lifetime of the input type and the output Cell data.
///
/// If the target type is an owned type with the `'static` lifetime, this
/// generic parameter is not significant for the implementation. Typically, the
/// implementation will simply wrap the input value in a Box and return it.
///
/// However, if the Upcast trait is implemented for `&'a T` or similar
/// referential types or their wrappers, this lifetime must be included in the
/// [Upcast::Output] type specification.
///
/// ```
/// # use ad_astra::{
/// #     export,
/// #     runtime::{Cell, Origin, RuntimeResult, ScriptType, TypeHint, Upcast},
/// # };
/// #
/// #[derive(Debug, PartialEq)]
/// struct Foo;
///
/// #[export]
/// type FooAlias = Foo;
///
/// impl<'a> Upcast<'a> for Foo {
///     type Output = Box<Foo>;
///
///     #[inline(always)]
///     fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
///         Ok(Box::new(this))
///     }
///
///     #[inline(always)]
///     fn hint() -> TypeHint {
///         Foo::type_meta().into()
///     }
/// }
///
/// impl<'a> Upcast<'a> for &'a Foo {
///     type Output = &'a Foo;
///
///     #[inline(always)]
///     fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
///         Ok(this)
///     }
///
///     #[inline(always)]
///     fn hint() -> TypeHint {
///         Foo::type_meta().into()
///     }
/// }
///
/// let cell = Cell::give(Origin::nil(), Foo).unwrap();
///
/// // The `Cell::map_ref` function requires that the result type implement
/// // Upcast on the functor's returned reference.
/// let mut mapped_cell = cell.map_ref::<Foo>(Origin::nil(), ref_to_ref).unwrap();
///
/// assert_eq!(mapped_cell.borrow_ref::<Foo>(Origin::nil()).unwrap(), &Foo);
///
/// fn ref_to_ref(foo: &Foo) -> RuntimeResult<&Foo> {
///     Ok(foo)
/// }
/// ```
///
/// Note that if you provide an implementation for `&T` or `&mut T`, the
/// Upcast trait will automatically be implemented for all possible combinations
/// of references, such as `&&T`, `&&mut T`, etc. This makes Upcast
/// referentially transparent out of the box, similar to Rust's referential
/// transparency.
pub trait Upcast<'a>: Sized + 'a {
    /// A type into which the input type will be upcasted.
    ///
    /// This upcasted type is limited to one of the following options:
    ///
    /// - `()`: Corresponds to the [Nil Cell](Cell::nil). Use this type if the
    ///   upcast implementation always returns a unit value `()`.
    ///
    /// - `Box<T>`: Where `T` is a [script-registered type](ScriptType).
    ///   Use this option to create a Cell with exactly one owned element with
    ///   `'static` lifetime.
    ///
    /// - `Vec<T>`: Where `T` is a script-registered type. Use this option to
    ///   create a Cell with an array of owned elements with `'static` lifetime.
    ///
    /// - `&'a T`, `&'a mut T`, `&'a [T]`, `'a mut [T]`: Where `T` is a
    ///   script-registered type. Use these options to create a Cell that serves
    ///   as a projection of another memory allocation.
    ///
    /// - `&'a str`: Use this option if the Cell should be a projection of a
    ///   Unicode string.
    ///
    /// - `String`: Use this option if the Cell should own a Unicode string.
    ///
    /// - `Cell`: Use this option to manually construct the Cell inside the
    ///   upcast function implementation.
    ///
    /// - [Either<A, B>](Either): Use this option if the resulting Cell could be
    ///   of type `A` or `B`, where `A` and `B` are any of the upcasting options
    ///   listed above (including other Either types). To return Cell data from
    ///   the upcast function, use the [Either::Left] or [Either::Right]
    ///   variants corresponding to `A` and `B`, respectively.
    type Output: Upcasted + 'a;

    /// Creates Script data from Rust data.
    ///
    /// The `origin` parameter specifies the source code range (either Rust or
    /// Script) where the data object was requested for creation.
    ///
    /// The `this` parameter is the original data object that is to be upcasted
    /// into the resulting [Cell].
    ///
    /// The function returns a [RuntimeError] if the underlying implementation
    /// is unable to perform the upcasting of the Rust object.
    fn upcast(origin: Origin, this: Self) -> RuntimeResult<Self::Output>;

    /// Returns a rough estimation of the Script type (or a set of types) into
    /// which the source Rust type could be upcasted.
    ///
    /// The underlying implementation makes the best effort to provide as
    /// precise type information as possible to support the static semantic
    /// analysis of the script code. However, the resulting [TypeHint] object
    /// may be imprecise, up to the [TypeHint::dynamic] value, which indicates
    /// that the source type(s) is not known at compile time.
    fn hint() -> TypeHint;
}

impl<'a, T> Upcast<'a> for &'a &'a T
where
    &'a T: Upcast<'a>,
{
    type Output = <&'a T as Upcast<'a>>::Output;

    #[inline(always)]
    fn upcast(origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        <&'a T as Upcast>::upcast(origin, this)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        <&'a T as Upcast>::hint()
    }
}

impl<'a, T> Upcast<'a> for &'a &'a mut T
where
    &'a T: Upcast<'a>,
{
    type Output = <&'a T as Upcast<'a>>::Output;

    #[inline(always)]
    fn upcast(origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        <&'a T as Upcast>::upcast(origin, this)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        <&'a T as Upcast>::hint()
    }
}

impl<'a, T> Upcast<'a> for &'a mut &'a T
where
    &'a T: Upcast<'a>,
{
    type Output = <&'a T as Upcast<'a>>::Output;

    #[inline(always)]
    fn upcast(origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        <&'a T as Upcast>::upcast(origin, this)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        <&'a T as Upcast>::hint()
    }
}

impl<'a, T> Upcast<'a> for &'a mut &'a mut T
where
    &'a mut T: Upcast<'a>,
{
    type Output = <&'a mut T as Upcast<'a>>::Output;

    #[inline(always)]
    fn upcast(origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        <&'a mut T as Upcast>::upcast(origin, this)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        <&'a mut T as Upcast>::hint()
    }
}

pub trait Upcasted {
    fn into_chain(self, origin: Origin) -> RuntimeResult<UpcastedChain>;
}

impl Upcasted for () {
    #[inline(always)]
    fn into_chain(self, _origin: Origin) -> RuntimeResult<UpcastedChain> {
        Ok(UpcastedChain::Cell(Cell::nil()))
    }
}

impl Upcasted for Cell {
    #[inline(always)]
    fn into_chain(self, _origin: Origin) -> RuntimeResult<UpcastedChain> {
        Ok(UpcastedChain::Cell(self))
    }
}

impl Upcasted for String {
    #[inline(always)]
    fn into_chain(self, origin: Origin) -> RuntimeResult<UpcastedChain> {
        Ok(UpcastedChain::Slice(MemorySlice::register_string(
            origin, self,
        )?))
    }
}

impl<T: ScriptType> Upcasted for Box<T> {
    #[inline(always)]
    fn into_chain(self, origin: Origin) -> RuntimeResult<UpcastedChain> {
        if TypeId::of::<T>() == TypeId::of::<()>() {
            return Ok(UpcastedChain::Cell(Cell::nil()));
        }

        let this = Box::into_raw(self);

        // Safety: Vector data originated from Box data that represents owned singleton slice.
        let vector = unsafe { Vec::from_raw_parts(this, 1, 1) };

        Ok(UpcastedChain::Slice(MemorySlice::register_vec(
            origin, vector,
        )?))
    }
}

impl<T: ScriptType> Upcasted for Vec<T> {
    #[inline(always)]
    fn into_chain(self, origin: Origin) -> RuntimeResult<UpcastedChain> {
        if TypeId::of::<T>() == TypeId::of::<()>() {
            return Ok(UpcastedChain::Cell(Cell::nil()));
        }

        Ok(UpcastedChain::Slice(MemorySlice::register_vec(
            origin, self,
        )?))
    }
}

impl<'a, T: ScriptType> Upcasted for &'a T {
    #[inline(always)]
    fn into_chain(self, origin: Origin) -> RuntimeResult<UpcastedChain> {
        if TypeId::of::<T>() == TypeId::of::<()>() {
            return Ok(UpcastedChain::Cell(Cell::nil()));
        }

        // Safety: Transparent layout transmutation.
        let slice = unsafe { transmute::<&T, &[T; 1]>(self) } as &[T];

        Ok(UpcastedChain::Slice(MemorySlice::register_slice_ref(
            origin, slice,
        )?))
    }
}

impl<'a, T: ScriptType> Upcasted for &'a mut T {
    #[inline(always)]
    fn into_chain(self, origin: Origin) -> RuntimeResult<UpcastedChain> {
        if TypeId::of::<T>() == TypeId::of::<()>() {
            return Ok(UpcastedChain::Cell(Cell::nil()));
        }

        // Safety: Transparent layout transmutation.
        let slice = unsafe { transmute::<&mut T, &mut [T; 1]>(self) } as &mut [T];

        Ok(UpcastedChain::Slice(MemorySlice::register_slice_mut(
            origin, slice,
        )?))
    }
}

impl<'a> Upcasted for &'a str {
    #[inline(always)]
    fn into_chain(self, origin: Origin) -> RuntimeResult<UpcastedChain> {
        Ok(UpcastedChain::Slice(MemorySlice::register_str(
            origin, self,
        )?))
    }
}

impl<'a, T: ScriptType> Upcasted for &'a [T] {
    #[inline(always)]
    fn into_chain(self, origin: Origin) -> RuntimeResult<UpcastedChain> {
        if TypeId::of::<T>() == TypeId::of::<()>() {
            return Ok(UpcastedChain::Cell(Cell::nil()));
        }

        Ok(UpcastedChain::Slice(MemorySlice::register_slice_ref(
            origin, self,
        )?))
    }
}

impl<'a, T: ScriptType> Upcasted for &'a mut [T] {
    #[inline(always)]
    fn into_chain(self, origin: Origin) -> RuntimeResult<UpcastedChain> {
        if TypeId::of::<T>() == TypeId::of::<()>() {
            return Ok(UpcastedChain::Cell(Cell::nil()));
        }

        Ok(UpcastedChain::Slice(MemorySlice::register_slice_mut(
            origin, self,
        )?))
    }
}

/// An alternative between two upcasted types.
///
/// See [Upcast::Output] for details.
pub enum Either<L: Upcasted, R: Upcasted> {
    /// The first alternative of the upcasted type.
    Left(L),

    /// The second alternative of the upcasted type.
    Right(R),
}

impl<L: Upcasted, R: Upcasted> Upcasted for Either<L, R> {
    #[inline(always)]
    fn into_chain(self, origin: Origin) -> RuntimeResult<UpcastedChain> {
        match self {
            Self::Left(left) => left.into_chain(origin),
            Self::Right(right) => right.into_chain(origin),
        }
    }
}

pub enum UpcastedChain {
    Slice(Arc<MemorySlice>),
    Cell(Cell),
}
