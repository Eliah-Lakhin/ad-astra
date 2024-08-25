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
    collections::BTreeMap,
    fmt::{Debug, Display, Formatter},
    mem::transmute,
};

use crate::{
    export,
    exports::utils::Stringifier,
    runtime::{
        ops::{DynamicType, ScriptAssign, ScriptField, ScriptNone, ScriptPartialEq},
        Arg,
        Cell,
        Downcast,
        Ident,
        Origin,
        Provider,
        RuntimeError,
        RuntimeResult,
        ScriptType,
        TypeHint,
        Upcast,
    },
};

/// A key-value table type.
///
/// ```text
/// let st = struct {
///     foo: 10,
///     bar: 20,
///     124: "hello",
///     func: fn() {},
/// };
///
/// st.bar == 20;
/// st.func();
///
/// st.new_field = 367;
///
/// st.foo? == true;
/// st.124? == true;
/// st.new_field? == true;
/// st.unknown_field? == false;
/// ```
#[export(include)]
#[export(name "struct")]
#[derive(Clone, Default)]
#[repr(transparent)]
pub struct Struct {
    pub(crate) map: BTreeMap<Ident, Cell>,
}

impl<'a> Downcast<'a> for BTreeMap<Ident, Cell> {
    #[inline(always)]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        Ok(provider.to_owned().take::<Struct>(origin)?.map)
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(Struct::type_meta())
    }
}

impl<'a> Downcast<'a> for &'a BTreeMap<Ident, Cell> {
    #[inline(always)]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let structure = provider
            .to_borrowed(&origin)?
            .borrow_ref::<Struct>(origin)?;

        // Safety: Transparent layout transmutation.
        Ok(unsafe { transmute::<&Struct, Self>(structure) })
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(Struct::type_meta())
    }
}

impl<'a> Downcast<'a> for &'a mut BTreeMap<Ident, Cell> {
    #[inline(always)]
    fn downcast(origin: Origin, provider: Provider<'a>) -> RuntimeResult<Self> {
        let structure = provider
            .to_borrowed(&origin)?
            .borrow_mut::<Struct>(origin)?;

        // Safety: Transparent layout transmutation.
        Ok(unsafe { transmute::<&mut Struct, Self>(structure) })
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(Struct::type_meta())
    }
}

impl<'a> Upcast<'a> for BTreeMap<Ident, Cell> {
    type Output = Box<Struct>;

    #[inline(always)]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(Box::new(Struct { map: this }))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(Struct::type_meta())
    }
}

impl<'a> Upcast<'a> for &'a BTreeMap<Ident, Cell> {
    type Output = &'a Struct;

    #[inline(always)]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        // Safety: Transparent layout transmutation.
        Ok(unsafe { transmute::<Self, &Struct>(this) })
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(Struct::type_meta())
    }
}

impl<'a> Upcast<'a> for &'a mut BTreeMap<Ident, Cell> {
    type Output = &'a mut Struct;

    #[inline(always)]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        // Safety: Transparent layout transmutation.
        Ok(unsafe { transmute::<Self, &mut Struct>(this) })
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(Struct::type_meta())
    }
}

#[export(include)]
impl Debug for Struct {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, formatter)
    }
}

#[export(include)]
impl Display for Struct {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let mut debug_map = formatter.debug_map();

        for (key, cell) in &self.map {
            debug_map.key(&key);

            if cell.is::<str>() {
                let mut cell = cell.clone();

                match cell.borrow_str(Origin::default()) {
                    Ok(string) => debug_map.value(&string),
                    Err(_) => debug_map.value(&"<str>"),
                };

                continue;
            }

            let stringifier = Stringifier {
                origin: Origin::default(),
                cell,
                error: RefCell::new(None),
                fallback_to_type: true,
            };

            debug_map.value(&stringifier);
        }

        debug_map.finish()
    }
}

#[export(include)]
impl ScriptPartialEq for Struct {
    type RHS = Self;

    fn script_eq(_origin: Origin, mut lhs: Arg, mut rhs: Arg) -> RuntimeResult<bool> {
        let lhs = <&Self as Downcast>::downcast(lhs.origin, lhs.provider())?;
        let rhs = <&Self as Downcast>::downcast(rhs.origin, rhs.provider())?;

        if lhs.map.len() != rhs.map.len() {
            return Ok(false);
        }

        for (lhs, rhs) in lhs.map.iter().zip(rhs.map.iter()) {
            if lhs.0 != rhs.0 {
                return Ok(false);
            }

            let lhs_type = lhs.1.ty();
            let rhs_type = rhs.1.ty();

            if lhs_type != rhs_type {
                return Ok(false);
            }

            if !lhs_type.prototype().implements_partial_eq() {
                return Ok(false);
            }

            let lhs = lhs.1.clone().into_object();

            let equal = lhs
                .partial_eq(
                    Origin::default(),
                    Origin::default(),
                    Arg {
                        origin: Origin::default(),
                        data: rhs.1.clone(),
                    },
                )
                .unwrap_or(false);

            if !equal {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[export(include)]
impl ScriptField for Struct {
    type Result = DynamicType;

    fn script_field(origin: Origin, lhs: Arg, rhs: Ident) -> RuntimeResult<Cell> {
        {
            let mut lhs_data = lhs.data.clone();

            let structure = lhs_data.borrow_ref::<Self>(lhs.origin)?;

            if let Some(entry) = structure.map.get(&rhs) {
                return Ok(entry.clone());
            }
        }

        Cell::give(
            origin,
            Vacant {
                structure: lhs.data,
                key: rhs,
            },
        )
    }
}

#[export(include)]
type VacantType = Vacant;

pub struct Vacant {
    structure: Cell,
    key: Ident,
}

impl<'a> Upcast<'a> for Vacant {
    type Output = Box<Self>;

    #[inline(always)]
    fn upcast(_origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
        Ok(Box::new(this))
    }

    #[inline(always)]
    fn hint() -> TypeHint {
        TypeHint::Type(Self::type_meta())
    }
}

#[export(include)]
impl Display for Vacant {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("struct.")?;
        Display::fmt(&self.key, formatter)
    }
}

#[export(include)]
impl Debug for Vacant {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, formatter)
    }
}

#[export(include)]
impl ScriptAssign for Vacant {
    type RHS = DynamicType;

    fn script_assign(origin: Origin, lhs: Arg, rhs: Arg) -> RuntimeResult<()> {
        if rhs.data.clone().ty().prototype().implements_none() {
            return Err(RuntimeError::Nil {
                access_origin: rhs.origin,
            });
        }

        let mut vacant = lhs.data.take::<Self>(lhs.origin)?;

        if rhs.data.ty().prototype().implements_binding() {
            let rhs = rhs.clone();

            rhs.data.into_object().bind(
                rhs.origin,
                rhs.origin,
                Arg {
                    origin: lhs.origin,
                    data: vacant.structure.clone(),
                },
            )?;
        }

        let structure = vacant.structure.borrow_mut::<Struct>(origin)?;

        let _ = structure.map.insert(vacant.key, rhs.data);

        Ok(())
    }
}

#[export(include)]
impl ScriptNone for Vacant {}
