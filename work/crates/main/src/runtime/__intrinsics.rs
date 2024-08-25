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
    cmp::Ordering,
    collections::{
        hash_map::{Entry, VacantEntry},
        HashMap,
    },
    fmt::Formatter,
    hash::Hasher,
    mem::{size_of, take, transmute, transmute_copy},
    ops::Deref,
    ptr::addr_of,
    slice,
    sync::{Mutex, MutexGuard},
};

use ahash::RandomState;
pub use lady_deirdre::sync::Lazy;

use crate::{
    runtime::{
        Arg,
        Cell,
        Ident,
        InvocationMeta,
        Origin,
        RuntimeResult,
        RustIdent,
        RustOrigin,
        TypeFamily,
        TypeMeta,
    },
    type_family,
};

type_family! {
    /// Any package type.
    pub static PACKAGE_FAMILY = "Package";

    /// Any function type.
    pub static FUNCTION_FAMILY = "fn";
}

type ExporterFn = extern "C" fn();

static __AD_ASTRA_DECLARATIONS: Lazy<
    Mutex<HashMap<ExporterFn, Option<DeclarationGroup>, RandomState>>,
> = Lazy::new(|| Mutex::default());

pub struct ExportEntry {
    vacant: VacantEntry<'static, ExporterFn, Option<DeclarationGroup>>,
    #[allow(unused)]
    guard: MutexGuard<'static, HashMap<ExporterFn, Option<DeclarationGroup>, RandomState>>,
}

impl ExportEntry {
    #[inline(always)]
    pub fn get(f: ExporterFn) -> Option<Self> {
        let mut guard = __AD_ASTRA_DECLARATIONS
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        let Entry::Vacant(vacant) = guard.entry(f) else {
            return None;
        };

        let vacant = unsafe {
            transmute::<
                VacantEntry<'_, ExporterFn, Option<DeclarationGroup>>,
                VacantEntry<'static, ExporterFn, Option<DeclarationGroup>>,
            >(vacant)
        };

        Some(Self { vacant, guard })
    }

    #[inline(always)]
    pub fn export(self, group: DeclarationGroup) {
        let _ = self.vacant.insert(Some(group));
    }
}

pub struct DeclarationGroup {
    pub origin: &'static RustOrigin,
    pub packages: Vec<fn() -> PackageDeclaration>,
    pub type_metas: Vec<fn() -> TypeMetaDeclaration>,
    pub prototypes: Vec<fn() -> PrototypeDeclaration>,
}

impl DeclarationGroup {
    #[inline(always)]
    pub(crate) fn enumerate() -> impl Iterator<Item = &'static DeclarationGroup> {
        static ENUMERATION: Lazy<Vec<DeclarationGroup>> = Lazy::new(|| {
            for exporter in DeclarationGroup::exporters() {
                exporter();
            }

            let mut declarations = __AD_ASTRA_DECLARATIONS
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());

            let mut vector = Vec::with_capacity(declarations.len());

            for (_, declaration) in declarations.iter_mut() {
                let Some(declaration) = take(declaration) else {
                    continue;
                };

                vector.push(declaration);
            }

            vector
        });

        ENUMERATION.deref().iter()
    }

    #[cfg(any(
        target_os = "none",
        target_os = "linux",
        target_os = "android",
        target_os = "fuchsia",
        target_os = "psp",
        target_os = "freebsd"
    ))]
    fn exporters() -> &'static [ExporterFn] {
        extern "Rust" {
            #[link_name = "__start_adastrexpr"]
            static START: ExporterFn;
            #[link_name = "__stop_adastrexpr"]
            static STOP: ExporterFn;
        }

        #[used]
        #[link_section = "adastrexpr"]
        static mut EMPTY: [ExporterFn; 0] = [];

        let start = unsafe { addr_of!(START) };
        let stop = unsafe { addr_of!(STOP) };

        let len = ((stop as usize) - (start as usize)) / size_of::<ExporterFn>();

        unsafe { slice::from_raw_parts::<'static, ExporterFn>(start, len) }
    }

    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "tvos"))]
    fn exporters() -> &'static [ExporterFn] {
        extern "Rust" {
            #[link_name = "\x01section$start$__DATA$__adastrexpr"]
            static START: ExporterFn;
            #[link_name = "\x01section$end$__DATA$__adastrexpr"]
            static STOP: ExporterFn;
        }

        let start = unsafe { addr_of!(START) };
        let stop = unsafe { addr_of!(STOP) };

        let len = ((stop as usize) - (start as usize)) / size_of::<ExporterFn>();

        unsafe { slice::from_raw_parts::<'static, ExporterFn>(start, len) }
    }

    #[cfg(any(target_os = "illumos"))]
    fn exporters() -> &'static [ExporterFn] {
        extern "Rust" {
            #[link_name = "__start_set_adastrexpr"]
            static START: ExporterFn;
            #[link_name = "__stop_set_adastrexpr"]
            static STOP: ExporterFn;
        }

        #[used]
        #[link_section = "set_adastrexpr"]
        static mut EMPTY: [ExporterFn; 0] = [];

        let start = unsafe { addr_of!(START) };
        let stop = unsafe { addr_of!(STOP) };

        let len = ((stop as usize) - (start as usize)) / size_of::<ExporterFn>();

        unsafe { slice::from_raw_parts::<'static, ExporterFn>(start, len) }
    }

    #[cfg(target_os = "windows")]
    fn exporters() -> &'static [ExporterFn] {
        extern "Rust" {
            #[link_name = ".adastrexpr$a"]
            static START: [ExporterFn; 0];
            #[link_name = ".adastrexpr$c"]
            static STOP: [ExporterFn; 0];
        }

        let start = unsafe { addr_of!(START) } as *const ExporterFn;
        let stop = unsafe { addr_of!(STOP) } as *const ExporterFn;

        let len = ((stop as usize) - (start as usize)) / size_of::<ExporterFn>();

        let start = hint::black_box(start);

        unsafe { slice::from_raw_parts::<'static, ExporterFn>(start, len) }
    }

    #[cfg(not(any(
        target_os = "none",
        target_os = "linux",
        target_os = "android",
        target_os = "fuchsia",
        target_os = "psp",
        target_os = "freebsd",
        target_os = "macos",
        target_os = "ios",
        target_os = "tvos",
        target_os = "illumos",
        target_os = "windows",
    )))]
    fn exporters() -> &'static [ExporterFn] {
        &[]
    }
}

pub struct PackageDeclaration {
    pub name: &'static str,
    pub version: &'static str,
    pub doc: Option<&'static str>,
    pub instance: Lazy<Cell>,
}

pub struct TypeMetaDeclaration {
    pub name: &'static str,
    pub doc: Option<&'static str>,
    pub id: TypeId,
    pub family: Option<&'static TypeFamily>,
    pub size: usize,
}

pub struct PrototypeDeclaration {
    pub receiver: TypeId,
    pub components: Vec<ComponentDeclaration>,
    pub operators: Vec<OperatorDeclaration>,
}

pub struct ComponentDeclaration {
    pub name: &'static RustIdent,
    pub constructor: fn(origin: Origin, lhs: Arg) -> RuntimeResult<Cell>,
    pub hint: &'static TypeMeta,
    pub doc: Option<&'static str>,
}

pub enum OperatorDeclaration {
    Assign(AssignOperator),
    Concat(ConcatOperator),
    Field(FieldOperator),
    Clone(CloneOperator),
    Debug(DebugOperator),
    Display(DisplayOperator),
    PartialEq(PartialEqOperator),
    Default(DefaultOperator),
    PartialOrd(PartialOrdOperator),
    Ord(OrdOperator),
    Hash(HashOperator),
    Invocation(InvocationOperator),
    Binding(BindingOperator),
    Add(AddOperator),
    AddAssign(AddAssignOperator),
    Sub(SubOperator),
    SubAssign(SubAssignOperator),
    Mul(MulOperator),
    MulAssign(MulAssignOperator),
    Div(DivOperator),
    DivAssign(DivAssignOperator),
    And(AndOperator),
    Or(OrOperator),
    Not(NotOperator),
    Neg(NegOperator),
    BitAnd(BitAndOperator),
    BitAndAssign(BitAndAssignOperator),
    BitOr(BitOrOperator),
    BitOrAssign(BitOrAssignOperator),
    BitXor(BitXorOperator),
    BitXorAssign(BitXorAssignOperator),
    Shl(ShlOperator),
    ShlAssign(ShlAssignOperator),
    Shr(ShrOperator),
    ShrAssign(ShrAssignOperator),
    Rem(RemOperator),
    RemAssign(RemAssignOperator),
    None(NoneOperator),
}

pub struct AssignOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct ConcatOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, items: &mut [Arg]) -> RuntimeResult<Cell>,
    pub hint_result: &'static TypeMeta,
}

pub struct FieldOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Ident) -> RuntimeResult<Cell>,
    pub hint_result: &'static TypeMeta,
}

pub struct CloneOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg) -> RuntimeResult<Cell>,
    pub clone_fn: CloneFn,
}

pub struct DebugOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, formatter: &mut Formatter<'_>) -> RuntimeResult<()>,
}

pub struct DisplayOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, formatter: &mut Formatter<'_>) -> RuntimeResult<()>,
}

pub struct PartialEqOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<bool>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct DefaultOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin) -> RuntimeResult<Cell>,
}

pub struct PartialOrdOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Option<Ordering>>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct OrdOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Ordering>,
}

pub struct HashOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, hasher: &mut DynHasher<'_>) -> RuntimeResult<()>,
}

pub struct InvocationOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, arguments: &mut [Arg]) -> RuntimeResult<Cell>,
    pub hint: fn() -> Option<&'static InvocationMeta>,
}

pub struct BindingOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct AddOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>,
    pub hint_rhs: &'static TypeMeta,
    pub hint_result: &'static TypeMeta,
}

pub struct AddAssignOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct SubOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>,
    pub hint_rhs: &'static TypeMeta,
    pub hint_result: &'static TypeMeta,
}

pub struct SubAssignOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct MulOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>,
    pub hint_rhs: &'static TypeMeta,
    pub hint_result: &'static TypeMeta,
}

pub struct MulAssignOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct DivOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>,
    pub hint_rhs: &'static TypeMeta,
    pub hint_result: &'static TypeMeta,
}

pub struct DivAssignOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct AndOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>,
    pub hint_rhs: &'static TypeMeta,
    pub hint_result: &'static TypeMeta,
}

pub struct OrOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>,
    pub hint_rhs: &'static TypeMeta,
    pub hint_result: &'static TypeMeta,
}

pub struct NotOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg) -> RuntimeResult<Cell>,
    pub hint_result: &'static TypeMeta,
}

pub struct NegOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg) -> RuntimeResult<Cell>,
    pub hint_result: &'static TypeMeta,
}

pub struct BitAndOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>,
    pub hint_rhs: &'static TypeMeta,
    pub hint_result: &'static TypeMeta,
}

pub struct BitAndAssignOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct BitOrOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>,
    pub hint_rhs: &'static TypeMeta,
    pub hint_result: &'static TypeMeta,
}

pub struct BitOrAssignOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct BitXorOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>,
    pub hint_rhs: &'static TypeMeta,
    pub hint_result: &'static TypeMeta,
}

pub struct BitXorAssignOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct ShlOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>,
    pub hint_rhs: &'static TypeMeta,
    pub hint_result: &'static TypeMeta,
}

pub struct ShlAssignOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct ShrOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>,
    pub hint_rhs: &'static TypeMeta,
    pub hint_result: &'static TypeMeta,
}

pub struct ShrAssignOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct RemOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<Cell>,
    pub hint_rhs: &'static TypeMeta,
    pub hint_result: &'static TypeMeta,
}

pub struct RemAssignOperator {
    pub origin: &'static RustOrigin,
    pub invoke: fn(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()>,
    pub hint_rhs: &'static TypeMeta,
}

pub struct NoneOperator {
    pub origin: &'static RustOrigin,
}

pub trait RegisteredType: Send + Sync + 'static {}

pub trait SizeOf {
    const SIZE: usize;
}

impl<T: Sized> SizeOf for T {
    const SIZE: usize = size_of::<T>();
}

impl<T: Sized> SizeOf for [T] {
    const SIZE: usize = T::SIZE;
}

impl SizeOf for str {
    const SIZE: usize = 1;
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct CloneFn {
    ptr: *const (),
}

// Safety: CloneFn can only be created from Send function.
unsafe impl Send for CloneFn {}

// Safety: CloneFn can only be created from Sync function.
unsafe impl Sync for CloneFn {}

impl CloneFn {
    #[inline(always)]
    pub fn from_clone<T: Clone + Send + Sync + 'static>() -> Self {
        Self::from_function(<T as Clone>::clone)
    }

    #[inline(always)]
    pub fn from_function<T: Send + Sync + 'static>(
        clone_fn: for<'a> fn(receiver: &'a T) -> T,
    ) -> Self {
        Self {
            ptr: clone_fn as *const (),
        }
    }

    // Safety: The CloneFn instance originated from specified type T.
    pub(super) unsafe fn into_fn<T: Send + Sync + 'static>(
        self,
    ) -> for<'a> fn(receiver: &'a T) -> T {
        // Safety: self.ptr originated from the target transmutation type as long
        //         as the `T` type matches which is upheld by the caller.
        unsafe { transmute_copy::<*const (), for<'a> fn(receiver: &'a T) -> T>(&self.ptr) }
    }
}

#[repr(transparent)]
pub struct DynHasher<'a> {
    hasher: &'a mut dyn Hasher,
}

impl<'a> DynHasher<'a> {
    #[inline(always)]
    pub(super) fn new(hasher: &'a mut impl Hasher) -> Self {
        Self { hasher }
    }
}

impl<'a> Hasher for DynHasher<'a> {
    #[inline(always)]
    fn finish(&self) -> u64 {
        self.hasher.finish()
    }

    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) {
        self.hasher.write(bytes)
    }

    #[inline(always)]
    fn write_u8(&mut self, i: u8) {
        self.hasher.write_u8(i)
    }

    #[inline(always)]
    fn write_u16(&mut self, i: u16) {
        self.hasher.write_u16(i)
    }

    #[inline(always)]
    fn write_u32(&mut self, i: u32) {
        self.hasher.write_u32(i)
    }

    #[inline(always)]
    fn write_u64(&mut self, i: u64) {
        self.hasher.write_u64(i)
    }

    #[inline(always)]
    fn write_u128(&mut self, i: u128) {
        self.hasher.write_u128(i)
    }

    #[inline(always)]
    fn write_usize(&mut self, i: usize) {
        self.hasher.write_usize(i)
    }

    #[inline(always)]
    fn write_i8(&mut self, i: i8) {
        self.hasher.write_i8(i)
    }

    #[inline(always)]
    fn write_i16(&mut self, i: i16) {
        self.hasher.write_i16(i)
    }

    #[inline(always)]
    fn write_i32(&mut self, i: i32) {
        self.hasher.write_i32(i)
    }

    #[inline(always)]
    fn write_i64(&mut self, i: i64) {
        self.hasher.write_i64(i)
    }

    #[inline(always)]
    fn write_i128(&mut self, i: i128) {
        self.hasher.write_i128(i)
    }

    #[inline(always)]
    fn write_isize(&mut self, i: isize) {
        self.hasher.write_isize(i)
    }
}

pub mod canonicals {
    use std::mem::take;

    use crate::runtime::{Arg, Cell, Origin, RuntimeResult, ScriptType};

    #[inline(always)]
    pub fn script_assign<T: ScriptType>(mut lhs: Arg, rhs: Arg) -> RuntimeResult<()> {
        let rhs = rhs.data.take::<T>(rhs.origin)?;
        let lhs = lhs.data.borrow_mut::<T>(lhs.origin)?;

        *lhs = rhs;

        Ok(())
    }

    #[inline(always)]
    pub fn script_concat<T: ScriptType>(origin: Origin, items: &mut [Arg]) -> RuntimeResult<Cell> {
        let mut result = Vec::<T>::new();

        for item in items {
            if item.data.is_nil() {
                continue;
            }

            let mut item_slice = take(&mut item.data).take_vec::<T>(item.origin)?;

            result.append(&mut item_slice);
        }

        if result.is_empty() {
            return Ok(Cell::nil());
        };

        Cell::give_vec(origin, result)
    }
}
