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
    any::{type_name, TypeId},
    cmp::Ordering,
    collections::hash_set::Iter,
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    iter::FusedIterator,
    mem::transmute,
    ops::Deref,
    ptr::NonNull,
};

use ahash::{AHashMap, AHashSet};
use lady_deirdre::sync::Lazy;

use crate::{
    report::debug_unreachable,
    runtime::{
        ops::{
            DynamicType,
            Fn0Repr,
            Fn1Repr,
            Fn2Repr,
            Fn3Repr,
            Fn4Repr,
            Fn5Repr,
            Fn6Repr,
            Fn7Repr,
        },
        RustOrigin,
        __intrinsics::DeclarationGroup,
    },
};

/// A Rust type that has been registered with the Script Engine.
///
/// Whenever you export a Rust type using the [export](crate::export) macro, the
/// macro automatically registers the [type introspection metadata](TypeMeta)
/// for that type and implements the ScriptType trait for it.
///
/// You cannot (and should not) implement this trait manually.
pub trait ScriptType: sealed::Sealed + Send + Sync + 'static {
    /// Returns the introspection metadata of this Rust type registered in the
    /// Script Engine.
    #[inline(always)]
    fn type_meta() -> &'static TypeMeta {
        match TypeMeta::by_id(&TypeId::of::<Self>()) {
            Some(meta) => meta,

            None => {
                let name = type_name::<Self>();
                panic!("{name} type was not registered. Probably because export has been disabled for this type.")
            }
        }
    }
}

mod sealed {
    use crate::runtime::{ScriptType, __intrinsics::RegisteredType};

    pub trait Sealed {}

    impl<T: RegisteredType + ?Sized> Sealed for T {}

    impl<T: RegisteredType + ?Sized> ScriptType for T {}
}

/// An introspection metadata for the Rust type [registered](ScriptType) by
/// the Script Engine.
///
/// You cannot create this object manually; its creation is managed by the
/// Script Engine. However, you can obtain a `'static` reference to this object
/// using the [ScriptType::type_meta] function and other related API functions.
///
/// The [Display] implementation prints the user-facing name of the type (e.g.,
/// `"usize"`, `"str"`, or `"Vec<bool>"`).
///
/// This object allows you to explore the type's introspection metadata and the
/// Script operations available for this type using the [TypeMeta::prototype]
/// function.
#[derive(Clone, Copy, Debug)]
pub struct TypeMeta {
    id: TypeId,
    name: &'static str,
    origin: &'static RustOrigin,
    doc: Option<&'static str>,
    family: TypeFamilyInner,
    size: usize,
}

impl Default for &'static TypeMeta {
    #[inline(always)]
    fn default() -> Self {
        TypeMeta::nil()
    }
}

impl PartialEq for TypeMeta {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl PartialEq<TypeId> for TypeMeta {
    #[inline(always)]
    fn eq(&self, other: &TypeId) -> bool {
        self.id.eq(other)
    }
}

impl Eq for TypeMeta {}

impl Ord for TypeMeta {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for TypeMeta {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for TypeMeta {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl Display for TypeMeta {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name)
    }
}

impl TypeMeta {
    /// Returns the metadata of the [unit] `()` type. In Ad Astra terminology,
    /// the unit type is referred to as the "nil type" and represents
    /// inaccessible data.
    ///
    /// The [Nil Cell](crate::runtime::Cell::nil) corresponds to the Nil type.
    ///
    /// This function is a shortcut for `<()>::type_meta()`.
    #[inline(always)]
    pub fn nil() -> &'static Self {
        <()>::type_meta()
    }

    /// Returns the metadata of the [type placeholder](DynamicType) that cannot
    /// be analyzed at script compile-time.
    ///
    /// This function is a shortcut for `<DynamicType>::type_meta()`.
    #[inline(always)]
    pub fn dynamic() -> &'static Self {
        <DynamicType>::type_meta()
    }

    #[inline(always)]
    pub(crate) fn script_fn(arity: usize) -> Option<&'static Self> {
        match arity {
            0 => Some(Fn0Repr::type_meta()),
            1 => Some(Fn1Repr::type_meta()),
            2 => Some(Fn2Repr::type_meta()),
            3 => Some(Fn3Repr::type_meta()),
            4 => Some(Fn4Repr::type_meta()),
            5 => Some(Fn5Repr::type_meta()),
            6 => Some(Fn6Repr::type_meta()),
            7 => Some(Fn7Repr::type_meta()),
            _ => None,
        }
    }

    #[inline(always)]
    pub(super) fn enumerate() -> impl Iterator<Item = &'static TypeId> {
        let registry = TypeRegistry::get();

        registry.type_index.keys()
    }

    #[inline(always)]
    pub(super) fn by_id(id: &TypeId) -> Option<&'static Self> {
        let registry = TypeRegistry::get();

        registry.type_index.get(id)
    }

    /// Returns the [TypeId] of the original Rust type.
    #[inline(always)]
    pub fn id(&self) -> &TypeId {
        &self.id
    }

    /// Returns true if this type is a [Nil type](Self::nil), which
    /// represents a void, inaccessible object.
    #[inline(always)]
    pub fn is_nil(&self) -> bool {
        self.id.eq(&TypeId::of::<()>())
    }

    /// Returns true if the type is a [Dynamic type](DynamicType), a type
    /// placeholder that cannot be analyzed at script compile-time.
    #[inline(always)]
    pub fn is_dynamic(&self) -> bool {
        self.id.eq(&TypeId::of::<DynamicType>())
    }

    /// Returns true if this type belongs to the
    /// [family of functions](TypeFamily::fn_family).
    ///
    /// These types typically support the
    /// [invocation operator](crate::runtime::Object::invoke).
    ///
    /// This function is a shortcut for `type_meta_family().is_fn()`.
    #[inline(always)]
    pub fn is_fn(&self) -> bool {
        self.family().is_fn()
    }

    /// Returns the user-facing name of the original Rust type, such as
    /// `"usize"`, `"str"`, `"Vec<bool>"`, etc.
    #[inline(always)]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the location in the Rust source code where the Rust type was
    /// declared (or registered by the [export](crate::export) macro).
    #[inline(always)]
    pub fn origin(&self) -> &'static RustOrigin {
        self.origin
    }

    /// Returns the RustDoc documentation for the Rust type.
    ///
    /// The function returns None if the type does not have documentation or
    /// if the documentation was not recognized by the [export](crate::export)
    /// macro.
    #[inline(always)]
    pub fn doc(&self) -> Option<&'static str> {
        self.doc
    }

    #[inline(always)]
    pub(super) fn size(&self) -> usize {
        self.size
    }

    /// Returns a reference to the family of types to which this Rust type
    /// belongs.
    #[inline(always)]
    pub fn family(&self) -> &TypeFamily {
        // Safety: Transparent type transmutation.
        unsafe { transmute::<&TypeFamilyInner, &TypeFamily>(&self.family) }
    }
}

/// A set of semantically related types.
///
/// Types that can be type-casted to each other to some extent form a
/// family of types. The semantic analyzer typically treats them as a single
/// unified type, and the LSP server usually displays the type family to which
/// a specific type belongs.
///
/// This approach simplifies interoperability between Rust types in scripts.
/// For example, [usize], [f32], and other Rust built-in primitive numeric types
/// form the `number` family of types. The script engine performs automatic type
/// conversions between these types, allowing the end user to work with each
/// specific numeric type as a general "number".
///
/// When you export a type using the [export](crate::export) macro, the Script
/// Engine automatically creates a new type family containing just that type.
///
/// However, you can manually associate a type with an existing family using the
/// `#[export(family(<family_reference>))]` macro option.
///
/// To introduce a new type family, consider using the
/// [type_family](crate::type_family) declarative macro.
///
/// The TypeFamily object provides functions to explore the types associated
/// with the family.
///
/// The [IntoIterator] implementation for this object iterates over each
/// [TypeMeta] associated with this family.
///
/// The [Debug] and [Display] implementations print the name of the family, and
/// (in alternate mode) enumerate the names of all associated types.
#[repr(transparent)]
pub struct TypeFamily(TypeFamilyInner);

impl PartialEq for TypeFamily {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let this_ptr = match &self.0 {
            TypeFamilyInner::Singleton { id: this_id } => {
                if let TypeFamilyInner::Singleton { id: other_id } = &other.0 {
                    return this_id.eq(other_id);
                }

                return false;
            }

            // Safety: Discriminant is checked.
            TypeFamilyInner::Group { .. } => unsafe { self.ptr() },

            TypeFamilyInner::Reference { ptr } => *ptr,
        };

        let other_ptr = match &other.0 {
            TypeFamilyInner::Singleton { .. } => return false,

            // Safety: Discriminant is checked.
            TypeFamilyInner::Group { .. } => unsafe { other.ptr() },

            TypeFamilyInner::Reference { ptr } => *ptr,
        };

        this_ptr.eq(&other_ptr)
    }
}

impl Eq for TypeFamily {}

impl Hash for TypeFamily {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        let reference = match &self.0 {
            TypeFamilyInner::Singleton { id } => return id.hash(state),

            // Safety: Discriminant is checked.
            TypeFamilyInner::Group { .. } => unsafe { self.ptr() },

            TypeFamilyInner::Reference { ptr } => *ptr,
        };

        reference.hash(state);
    }
}

impl Display for TypeFamily {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        formatter.write_str(name)?;

        if !formatter.alternate() {
            return Ok(());
        }

        formatter.write_str("(")?;

        let mut first = true;
        for ty in self {
            match first {
                true => first = false,
                false => formatter.write_str(", ")?,
            }

            Display::fmt(ty.name, formatter)?;
        }

        formatter.write_str(")")?;

        Ok(())
    }
}

impl Debug for TypeFamily {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, formatter)
    }
}

impl<'a> IntoIterator for &'a TypeFamily {
    type Item = &'static TypeMeta;
    type IntoIter = TypeFamilyIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        let reference = match &self.0 {
            TypeFamilyInner::Singleton { id } => return TypeFamilyIter::Singleton(*id),

            // Safety: Discriminant is checked.
            TypeFamilyInner::Group { .. } => unsafe { self.ptr() },

            TypeFamilyInner::Reference { ptr } => *ptr,
        };

        let registry = TypeRegistry::get();

        match registry.family_index.get(&reference) {
            None => TypeFamilyIter::Ended,
            Some(set) => TypeFamilyIter::Group(set.iter()),
        }
    }
}

impl TypeFamily {
    /// Creates a new TypeFamily with the specified `name` and without
    /// documentation.
    ///
    /// Instances of type families should typically be stored in statics:
    ///
    /// ```
    /// # use ad_astra::runtime::TypeFamily;
    /// #
    /// static FOO_FAMILY: TypeFamily = TypeFamily::new("foo");
    /// ```
    ///
    /// It is recommended to use the [type_family](crate::type_family)
    /// declarative macro to declare type families instead.
    #[inline(always)]
    pub const fn new(name: &'static str) -> Self {
        Self(TypeFamilyInner::Group { name, doc: None })
    }

    /// Similar to the [new](Self::new) constructor, but allows specifying
    /// RustDoc documentation for the type family through the `doc` parameter.
    ///
    /// The `doc` string is expected to be raw Markdown documentation text.
    #[inline(always)]
    pub const fn with_doc(name: &'static str, doc: &'static str) -> Self {
        Self(TypeFamilyInner::Group {
            name,
            doc: Some(doc),
        })
    }

    /// Returns a reference to the type family of the [Nil](TypeMeta::nil) type.
    #[inline(always)]
    pub fn nil() -> &'static Self {
        TypeMeta::nil().family()
    }

    /// Returns a reference to the type family of the [dynamic](DynamicType)
    /// type.
    #[inline(always)]
    pub fn dynamic() -> &'static Self {
        TypeMeta::dynamic().family()
    }

    /// Returns a reference to the type family of function-like objects,
    /// which are objects that have
    /// [invocation](crate::runtime::Prototype::implements_invocation)
    /// capabilities.
    #[inline(always)]
    pub fn fn_family() -> &'static Self {
        &crate::runtime::__intrinsics::FUNCTION_FAMILY
    }

    /// Returns a reference to the type family of
    /// [ScriptPackage](crate::runtime::ScriptPackage) objects.
    #[inline(always)]
    pub fn package() -> &'static Self {
        &crate::runtime::__intrinsics::PACKAGE_FAMILY
    }

    /// Returns a reference to the type family of numeric objects.
    ///
    /// The [usize], [f32], and other Rust built-in numeric types belong to this
    /// family.
    #[inline(always)]
    pub fn number() -> &'static Self {
        &NUMBER_FAMILY
    }

    /// Returns true if this family is the [Nil Family](Self::nil).
    #[inline(always)]
    pub fn is_nil(&self) -> bool {
        self == Self::nil()
    }

    /// Returns true if this family is the [Dynamic Family](Self::dynamic).
    #[inline(always)]
    pub fn is_dynamic(&self) -> bool {
        self == Self::dynamic()
    }

    /// Returns true if this family is the [Functions Family](Self::fn_family).
    #[inline(always)]
    pub fn is_fn(&self) -> bool {
        self == Self::fn_family()
    }

    /// Returns true if this family is the [Packages Family](Self::package).
    #[inline(always)]
    pub fn is_package(&self) -> bool {
        self == Self::package()
    }

    /// Returns true if this family is the [Numeric Family](Self::number).
    #[inline(always)]
    pub fn is_number(&self) -> bool {
        self == Self::number()
    }

    /// Returns the number of types associated with this family.
    #[inline(always)]
    pub fn len(&self) -> usize {
        let reference = match &self.0 {
            TypeFamilyInner::Singleton { .. } => return 1,

            // Safety: Discriminant is checked.
            TypeFamilyInner::Group { .. } => unsafe { self.ptr() },

            TypeFamilyInner::Reference { ptr } => *ptr,
        };

        let registry = TypeRegistry::get();

        let Some(set) = registry.family_index.get(&reference) else {
            return 0;
        };

        set.len()
    }

    /// Returns the user-facing name of this family.
    #[inline(always)]
    pub fn name(&self) -> &'static str {
        match &self.0 {
            TypeFamilyInner::Singleton { id } => {
                let registry = TypeRegistry::get();

                match registry.type_index.get(id) {
                    Some(meta) => meta.name,

                    // Safety: Singletons always refer existing TypeMeta entries.
                    None => unsafe { debug_unreachable!("Missing singleton type family entry.") },
                }
            }

            TypeFamilyInner::Group { name, .. } => *name,

            TypeFamilyInner::Reference { ptr } => {
                // Safety: References always point to existing static data.
                let family = unsafe { ptr.as_ref() };

                match family.0 {
                    TypeFamilyInner::Group { name, .. } => name,

                    // Safety: References always point to Groups.
                    _ => unsafe { debug_unreachable!("TypeFamily broken reference.") },
                }
            }
        }
    }

    /// Returns the RustDoc documentation for this type family. Returns None if
    /// the family does not have specified documentation.
    #[inline(always)]
    pub fn doc(&self) -> Option<&'static str> {
        match &self.0 {
            TypeFamilyInner::Singleton { id } => {
                let registry = TypeRegistry::get();

                match registry.type_index.get(id) {
                    Some(meta) => meta.doc,

                    // Safety: Singletons always refer existing TypeMeta entries.
                    None => unsafe { debug_unreachable!("Missing singleton type family entry.") },
                }
            }

            TypeFamilyInner::Group { doc, .. } => *doc,

            TypeFamilyInner::Reference { ptr } => {
                // Safety: References always point to existing static data.
                let family = unsafe { ptr.as_ref() };

                match family.0 {
                    TypeFamilyInner::Group { doc, .. } => doc,

                    // Safety: References always point to Groups.
                    _ => unsafe { debug_unreachable!("TypeFamily broken reference.") },
                }
            }
        }
    }

    /// Returns true if this type family contains a Rust type with the
    /// specified [TypeId].
    #[inline(always)]
    pub fn includes(&self, ty: &TypeId) -> bool {
        let ptr = match &self.0 {
            TypeFamilyInner::Singleton { id } => return id.eq(ty),

            // Safety: Discriminant is checked.
            TypeFamilyInner::Group { .. } => unsafe { self.ptr() },

            TypeFamilyInner::Reference { ptr } => *ptr,
        };

        let registry = TypeRegistry::get();

        let set = match registry.family_index.get(&ptr) {
            None => return false,
            Some(set) => set,
        };

        set.contains(ty)
    }

    // Safety: Inner discriminant is Group.
    unsafe fn ptr(&self) -> NonNull<TypeFamily> {
        match &self.0 {
            TypeFamilyInner::Group { .. } => unsafe {
                NonNull::new_unchecked(self as *const TypeFamily as *mut TypeFamily)
            },

            // Safety: Upheld by the caller.
            _ => unsafe {
                debug_unreachable!("An attempt to crate pointer from non-Group TypeFamily.")
            },
        }
    }
}

pub enum TypeFamilyIter {
    Ended,
    Singleton(TypeId),
    Group(Iter<'static, TypeId>),
}

impl Iterator for TypeFamilyIter {
    type Item = &'static TypeMeta;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Ended => None,

            Self::Singleton(id) => {
                let id = *id;

                *self = Self::Ended;

                match TypeMeta::by_id(&id) {
                    None => unsafe { debug_unreachable!("Invalid TypeFamily singleton.") },

                    Some(meta) => Some(meta),
                }
            }

            Self::Group(iterator) => match iterator.next() {
                None => None,

                Some(id) => match TypeMeta::by_id(&id) {
                    None => unsafe { debug_unreachable!("Invalid TypeFamily group.") },

                    Some(meta) => Some(meta),
                },
            },
        }
    }
}

impl FusedIterator for TypeFamilyIter {}

#[derive(Copy)]
enum TypeFamilyInner {
    Singleton {
        id: TypeId,
    },
    Group {
        name: &'static str,
        doc: Option<&'static str>,
    },
    Reference {
        ptr: NonNull<TypeFamily>,
    },
}

impl Debug for TypeFamilyInner {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        // Safety: Transparent type transmutation.
        let this = unsafe { transmute::<&TypeFamilyInner, &TypeFamily>(self) };

        Debug::fmt(this, formatter)
    }
}

// Safety: The inner pointer refers static data which is Send+Sync.
unsafe impl Send for TypeFamilyInner {}

// Safety: The inner pointer refers static data which is Send+Sync.
unsafe impl Sync for TypeFamilyInner {}

impl Clone for TypeFamilyInner {
    fn clone(&self) -> Self {
        match self {
            Self::Singleton { id } => Self::Singleton { id: *id },
            Self::Reference { ptr } => Self::Reference { ptr: *ptr },

            // Safety: This variant is never exposed to the public clone-able interface.
            Self::Group { .. } => unsafe {
                debug_unreachable!("An attempt to clone Group TypeFamily")
            },
        }
    }
}

/// A macro that declares new [type families](TypeFamily).
///
/// Using this macro, you can declare new type families in statics,
/// with RustDoc documentation using [TypeFamily::with_doc] or without
/// it using [TypeFamily::new].
///
/// ```
/// use ad_astra::type_family;
///
/// type_family!(
///     /// Documentation line 1.
///     /// Documentation line 2.
///     pub static FOO_FAMILY = "foo";
///
///     static BAR_FAMILY = "bar";
/// );
///
/// assert_eq!(FOO_FAMILY.name(), "foo");
/// assert_eq!(FOO_FAMILY.doc(), Some(" Documentation line 1.\n Documentation line 2.\n"));
///
/// assert_eq!(BAR_FAMILY.name(), "bar");
/// assert_eq!(BAR_FAMILY.doc(), None);
/// ```
#[macro_export]
macro_rules! type_family {
    (
        $vis:vis static $ident:ident = $name:expr;
    ) => {
        $vis static $ident: $crate::runtime::TypeFamily = $crate::runtime::TypeFamily::new($name);
    };

    (
        $(#[doc = $doc:expr])+
        $vis:vis static $ident:ident = $name:expr;
    ) => {
        $(#[doc = $doc])+
        $vis static $ident: $crate::runtime::TypeFamily = $crate::runtime::TypeFamily::with_doc(
            $name, ::std::concat!($($doc, "\n"),+)
        );
    };

    {
        $(
            $(#[doc = $doc:expr])*
            $vis:vis static $ident:ident = $name:expr;
        )*
    } => {
        $(
            $crate::type_family!{
                $(#[doc = $doc])*
                $vis static $ident = $name;
            }
        )*
    };
}

struct TypeRegistry {
    type_index: AHashMap<TypeId, TypeMeta>,
    family_index: AHashMap<NonNull<TypeFamily>, AHashSet<TypeId>>,
}

// Safety: The inner pointer refers static data which is Send+Sync.
unsafe impl Send for TypeRegistry {}

// Safety: The inner pointer refers static data which is Send+Sync.
unsafe impl Sync for TypeRegistry {}

impl TypeRegistry {
    #[inline(always)]
    fn get() -> &'static Self {
        static REGISTRY: Lazy<TypeRegistry> = Lazy::new(|| {
            let mut type_index = AHashMap::<TypeId, TypeMeta>::new();
            let mut family_index = AHashMap::<NonNull<TypeFamily>, AHashSet<TypeId>>::new();

            for group in DeclarationGroup::enumerate() {
                let origin = group.origin;

                for declaration in &group.type_metas {
                    let declaration = declaration();

                    if let Some(previous) = type_index.get(&declaration.id) {
                        origin.blame(&format!(
                            "Type {} already declared in {} as {}.",
                            declaration.name, previous.origin, previous.name,
                        ))
                    }

                    let family = match declaration.family {
                        None => TypeFamilyInner::Singleton { id: declaration.id },

                        Some(group) => {
                            // Safety: By the time of the Registry creation
                            //         there are instances of the Group TypeFamilies
                            //         only available in the external context.
                            let ptr = unsafe { group.ptr() };

                            let set = family_index.entry(ptr).or_default();

                            if !set.insert(declaration.id) {
                                // Safety: Uniqueness checked above.
                                unsafe { debug_unreachable!("Duplicate type family entry.") }
                            }

                            TypeFamilyInner::Reference { ptr }
                        }
                    };

                    let meta = TypeMeta {
                        id: declaration.id,
                        name: declaration.name,
                        origin,
                        doc: declaration.doc,
                        family,
                        size: declaration.size,
                    };

                    if let Some(_) = type_index.insert(declaration.id, meta) {
                        // Safety: Uniqueness checked above.
                        unsafe { debug_unreachable!("Duplicate type meta entry.") }
                    }
                }
            }

            TypeRegistry {
                type_index,
                family_index,
            }
        });

        REGISTRY.deref()
    }
}

use crate::exports::NUMBER_FAMILY;
