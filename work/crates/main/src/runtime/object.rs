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
    fmt::{Debug, Display, Formatter},
    hash::Hasher,
    ops::Deref,
};

use ahash::AHashMap;
use lady_deirdre::sync::Lazy;

use crate::{
    report::{debug_unreachable, system_panic},
    runtime::{
        ops::OperatorKind,
        Arg,
        Cell,
        ComponentHint,
        Ident,
        InvocationMeta,
        Origin,
        RuntimeError,
        RuntimeResult,
        ScriptType,
        TypeHint,
        TypeMeta,
        __intrinsics::{
            AddAssignOperator,
            AddOperator,
            AndOperator,
            AssignOperator,
            BindingOperator,
            BitAndAssignOperator,
            BitAndOperator,
            BitOrAssignOperator,
            BitOrOperator,
            BitXorAssignOperator,
            BitXorOperator,
            CloneOperator,
            ComponentDeclaration,
            ConcatOperator,
            DebugOperator,
            DeclarationGroup,
            DefaultOperator,
            DisplayOperator,
            DivAssignOperator,
            DivOperator,
            DynHasher,
            FieldOperator,
            HashOperator,
            InvocationOperator,
            MulAssignOperator,
            MulOperator,
            NegOperator,
            NoneOperator,
            NotOperator,
            OperatorDeclaration,
            OrOperator,
            OrdOperator,
            PartialEqOperator,
            PartialOrdOperator,
            RemAssignOperator,
            RemOperator,
            ShlAssignOperator,
            ShlOperator,
            ShrAssignOperator,
            ShrOperator,
            SubAssignOperator,
            SubOperator,
        },
    },
};

/// A wrapper for [Cell] that provides type-specific operations with the Cell
/// data.
///
/// You can construct this object using [Cell::into_object].
///
/// To discover which operations are available for this Object, you can explore
/// its [Prototype] using the [Object::prototype] function.
pub struct Object {
    receiver: Cell,
    ty: &'static TypeMeta,
    prototype: &'static Prototype,
}

impl Object {
    /// Calls an assignment operator (`lhs = rhs`) on this Object as the
    /// left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the operation,
    /// including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["assign" operator](Prototype::implements_assign) or if
    /// the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn assign(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        if let Some(operator) = &self.prototype.assign {
            return (operator.invoke)(origin, self.arg(lhs), rhs);
        }

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: Some(self.receiver.origin()),
            receiver_type: self.ty,
            operator: OperatorKind::Assign,
        })
    }

    /// Returns a Cell that points to a component of the object
    /// (e.g., an object's method or a predefined field), such as `foo.bar`.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the name of the component/field. You
    /// can obtain the component's identifier using, for example, the
    /// [FieldSymbol::ident](crate::analysis::symbols::FieldSymbol::ident)
    /// function.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// have this [component](Prototype::hint_component) or if the
    /// component fetching implementation returns a RuntimeError.
    #[inline(always)]
    pub fn component(self, origin: Origin, lhs: Origin, rhs: Ident) -> RuntimeResult<Cell> {
        let key = rhs.as_ref();

        let Some(component) = self.prototype.components.get(key) else {
            return Err(RuntimeError::UnknownField {
                access_origin: origin,
                receiver_origin: self.receiver.origin(),
                receiver_type: self.ty,
                field: String::from(key),
            });
        };

        (component.constructor)(origin, self.arg(lhs))
    }

    /// Similar to [Object::component], but if the Object's type does not have a
    /// component with the specified name, it falls back to [Object::field].
    #[inline(always)]
    pub fn component_or_field(
        self,
        origin: Origin,
        lhs: Origin,
        rhs: Ident,
    ) -> RuntimeResult<Cell> {
        let key = rhs.as_ref();

        if let Some(component) = self.prototype.components.get(key) {
            return (component.constructor)(origin, self.arg(lhs));
        };

        if let Some(operator) = &self.prototype.field {
            return (operator.invoke)(origin, self.arg(lhs), rhs);
        };

        Err(RuntimeError::UnknownField {
            access_origin: origin,
            receiver_origin: self.receiver.origin(),
            receiver_type: self.ty,
            field: String::from(key),
        })
    }

    /// Returns a Cell that points to a field resolved at runtime, such as
    /// `foo.bar`.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the name of the field. You can obtain
    /// the field's identifier using, for example, the
    /// [FieldSymbol::ident](crate::analysis::symbols::FieldSymbol::ident)
    /// function.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// implement a [runtime field resolver](Prototype::implements_field) or if
    /// the resolver's implementation returns a RuntimeError.
    #[inline(always)]
    pub fn field(self, origin: Origin, lhs: Origin, rhs: Ident) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.field else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Field,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a Clone operator (`*foo`) on this Object, creating a clone of
    /// the underlying Cell's data.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `rhs` parameter specifies the Rust or Script source code range
    /// that spans the operand (this Object).
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["clone" operator](Prototype::implements_clone) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn clone(self, origin: Origin, rhs: Origin) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.clone else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(rhs),
                receiver_type: self.ty,
                operator: OperatorKind::Clone,
            });
        };

        (operator.invoke)(origin, self.arg(rhs))
    }

    /// Calls a Debug operator on this Object to format the underlying
    /// Cell's data for debugging purposes.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the operand (this Object).
    ///
    /// The `formatter` parameter specifies the Rust [Formatter] that will be
    /// passed to the [Debug::fmt] function.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["debug" operator](Prototype::implements_debug) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn debug(
        self,
        origin: Origin,
        lhs: Origin,
        formatter: &mut Formatter<'_>,
    ) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.debug else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Debug,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), formatter)
    }

    /// Calls a Display operator on this Object to format the underlying
    /// Cell's data for display purposes.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the operand (this Object).
    ///
    /// The `formatter` parameter specifies the Rust [Formatter] that will be
    /// passed to the [Display::fmt] function.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["display" operator](Prototype::implements_display) or if
    /// the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn display(
        self,
        origin: Origin,
        lhs: Origin,
        formatter: &mut Formatter<'_>,
    ) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.display else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Display,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), formatter)
    }

    /// Calls an equality operator (`lhs == rhs`) on this Object as the
    /// left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the [equality operator](Prototype::implements_partial_eq) or if
    /// the operator's implementation returns a RuntimeError.
    ///
    /// Note that in the current model of script interpretation, partial
    /// equality ([PartialEq]) also serves the purpose of full equality ([Eq]).
    #[inline]
    pub fn partial_eq(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<bool> {
        let Some(operator) = &self.prototype.partial_eq else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::PartialEq,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a partial ordering operator (`lhs >= rhs`, `lhs < rhs`, etc.) on
    /// this Object as the left-hand side (LHS) of the operation.
    ///
    /// Returns the result of the objects' comparison via [PartialOrd].
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the operation,
    /// including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the
    /// [partial ordering operator](Prototype::implements_partial_ord) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn partial_ord(
        self,
        origin: Origin,
        lhs: Origin,
        rhs: Arg,
    ) -> RuntimeResult<Option<Ordering>> {
        let Some(operator) = &self.prototype.partial_ord else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::PartialOrd,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a full ordering operator (`lhs >= rhs`, `lhs < rhs`, etc.) on
    /// this Object as the left-hand side (LHS) of the operation.
    ///
    /// Returns the result of the objects' comparison via [Ord].
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the operation,
    /// including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the [full ordering operator](Prototype::implements_ord) or if
    /// the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn ord(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Ordering> {
        let Some(operator) = &self.prototype.ord else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Ord,
            });
        };

        return (operator.invoke)(origin, self.arg(lhs), rhs);
    }

    /// Similar to [Object::ord], but if the Object's type does not support the
    /// [full ordering operator](Prototype::implements_ord), it falls back to
    /// [Object::partial_ord]. If the partial ordering returns None, this
    /// function returns a [RuntimeError] indicating that the
    /// ["ord" operator](OperatorKind::Ord) is not supported.
    #[inline]
    pub fn ord_fallback(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Ordering> {
        if let Some(operator) = &self.prototype.ord {
            return (operator.invoke)(origin, self.arg(lhs), rhs);
        };

        let receiver_type = self.ty;
        let receiver_origin = self.receiver.origin();

        if let Some(operator) = &self.prototype.partial_ord {
            if let Some(ordering) = (operator.invoke)(origin, self.arg(lhs), rhs)? {
                return Ok(ordering);
            }
        };

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: Some(receiver_origin),
            receiver_type,
            operator: OperatorKind::Ord,
        })
    }

    /// Feeds this Object's content into the specified `hasher`.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the hashing operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the hashing operand (this Object).
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["hash" operator](Prototype::implements_hash) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn hash(self, origin: Origin, lhs: Origin, hasher: &mut impl Hasher) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.hash else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Hash,
            });
        };

        let mut hasher = DynHasher::new(hasher);

        (operator.invoke)(origin, self.arg(lhs), &mut hasher)
    }

    /// Calls an invocation operator (`func(arg1, arg2, arg3)`) on this Object
    /// as the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the operation,
    /// including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["invoke" operator](Prototype::implements_invoke) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn invoke(self, origin: Origin, lhs: Origin, arguments: &mut [Arg]) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.invocation else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Invocation,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), arguments)
    }

    /// Sets the invocation context of the object.
    ///
    /// The meaning of the binding operator is type-dependent, but usually it
    /// sets the context in which this object will be [invoked](Self::invoke).
    ///
    /// For example, in the `foo.bar()` code, the interpreter would first bind
    /// "bar" to "foo," assuming "foo" is the
    /// [receiver-parameter](InvocationMeta::receiver) of the "bar" function.
    /// Then, the interpreter would call the invocation operator on the bound
    /// "bar" Object.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the binding context
    /// (the source code range and the data of the context Cell).
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["binding" operator](Prototype::implements_binding) or if
    /// the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn bind(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.binding else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Binding,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls an addition operator (`lhs + rhs`) on this Object as the
    /// left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["add" operator](Prototype::implements_add) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn add(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.add else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Add,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls an addition-assignment operator (`lhs += rhs`) on this Object as
    /// the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["add-assign" operator](Prototype::implements_add_assign) or
    /// if the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn add_assign(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.add_assign else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::AddAssign,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Similar to [Object::add_assign], but if the Object's type does not
    /// support the ["add-assign" operator](Prototype::implements_add_assign),
    /// it falls back to [Object::add], and then to [Object::assign].
    ///
    /// If the fallback operators are also not supported, this function returns
    /// a [RuntimeError] indicating that the
    /// ["add-assign" operator](OperatorKind::AddAssign) is not supported.
    #[inline]
    pub fn add_assign_fallback(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        if let Some(operator) = &self.prototype.add_assign {
            return (operator.invoke)(origin, self.arg(lhs), rhs);
        };

        if let (Some(assign_op), Some(fb_op)) = (&self.prototype.assign, &self.prototype.add) {
            let lhs = self.arg(lhs);

            let rhs_origin = rhs.origin;
            let rhs_cell = (fb_op.invoke)(origin, lhs.clone(), rhs)?;

            let rhs = Arg {
                origin: rhs_origin,
                data: rhs_cell,
            };

            return (assign_op.invoke)(origin, lhs, rhs);
        }

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: Some(self.receiver.origin()),
            receiver_type: self.ty,
            operator: OperatorKind::AddAssign,
        })
    }

    /// Calls a subtraction operator (`lhs - rhs`) on this Object as the
    /// left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the operation,
    /// including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["sub" operator](Prototype::implements_sub) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn sub(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.sub else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Sub,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a subtraction-assignment operator (`lhs -= rhs`) on this Object
    /// as the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the operation,
    /// including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["sub-assign" operator](Prototype::implements_sub_assign) or
    /// if the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn sub_assign(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.sub_assign else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::SubAssign,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Similar to [Object::sub_assign], but if the Object's type does not
    /// support the ["sub-assign" operator](Prototype::implements_sub_assign),
    /// it falls back to [Object::sub], and then to [Object::assign].
    ///
    /// If the fallback operators are also not supported, this function returns
    /// a [RuntimeError] indicating that the
    /// ["sub-assign" operator](OperatorKind::SubAssign) is not supported.
    #[inline]
    pub fn sub_assign_fallback(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        if let Some(operator) = &self.prototype.sub_assign {
            return (operator.invoke)(origin, self.arg(lhs), rhs);
        };

        if let (Some(assign_op), Some(fb_op)) = (&self.prototype.assign, &self.prototype.sub) {
            let lhs = self.arg(lhs);

            let rhs_origin = rhs.origin;
            let rhs_cell = (fb_op.invoke)(origin, lhs.clone(), rhs)?;

            let rhs = Arg {
                origin: rhs_origin,
                data: rhs_cell,
            };

            return (assign_op.invoke)(origin, lhs, rhs);
        }

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: Some(self.receiver.origin()),
            receiver_type: self.ty,
            operator: OperatorKind::SubAssign,
        })
    }

    /// Calls a multiplication operator (`lhs * rhs`) on this Object as the
    /// left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["mul" operator](Prototype::implements_mul) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn mul(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.mul else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Mul,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a multiplication-assignment operator (`lhs *= rhs`) on this
    /// Object as the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the operation,
    /// including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["mul-assign" operator](Prototype::implements_mul_assign) or
    /// if the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn mul_assign(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.mul_assign else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::MulAssign,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Similar to [Object::mul_assign], but if the Object's type does not
    /// support the ["mul-assign" operator](Prototype::implements_mul_assign),
    /// it falls back to [Object::mul], and then to [Object::assign].
    ///
    /// If the fallback operators are also not supported, this function returns
    /// a [RuntimeError] indicating that the
    /// ["mul-assign" operator](OperatorKind::MulAssign) is not supported.
    #[inline]
    pub fn mul_assign_fallback(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        if let Some(operator) = &self.prototype.mul_assign {
            return (operator.invoke)(origin, self.arg(lhs), rhs);
        };

        if let (Some(assign_op), Some(fb_op)) = (&self.prototype.assign, &self.prototype.mul) {
            let lhs = self.arg(lhs);

            let rhs_origin = rhs.origin;
            let rhs_cell = (fb_op.invoke)(origin, lhs.clone(), rhs)?;

            let rhs = Arg {
                origin: rhs_origin,
                data: rhs_cell,
            };

            return (assign_op.invoke)(origin, lhs, rhs);
        }

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: Some(self.receiver.origin()),
            receiver_type: self.ty,
            operator: OperatorKind::MulAssign,
        })
    }

    /// Calls a division operator (`lhs / rhs`) on this Object as the
    /// left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["div" operator](Prototype::implements_div) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn div(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.div else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Div,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a division-assignment operator (`lhs /= rhs`) on this
    /// Object as the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["div-assign" operator](Prototype::implements_div_assign) or
    /// if the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn div_assign(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.div_assign else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::DivAssign,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Similar to [Object::div_assign], but if the Object's type does not
    /// support the ["div-assign" operator](Prototype::implements_div_assign),
    /// it falls back to [Object::div], and then to [Object::assign].
    ///
    /// If the fallback operators are also not supported, this function returns
    /// a [RuntimeError] indicating that the
    /// ["div-assign" operator](OperatorKind::DivAssign) is not supported.
    #[inline]
    pub fn div_assign_fallback(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        if let Some(operator) = &self.prototype.div_assign {
            return (operator.invoke)(origin, self.arg(lhs), rhs);
        };

        if let (Some(assign_op), Some(fb_op)) = (&self.prototype.assign, &self.prototype.div) {
            let lhs = self.arg(lhs);

            let rhs_origin = rhs.origin;
            let rhs_cell = (fb_op.invoke)(origin, lhs.clone(), rhs)?;

            let rhs = Arg {
                origin: rhs_origin,
                data: rhs_cell,
            };

            return (assign_op.invoke)(origin, lhs, rhs);
        }

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: Some(self.receiver.origin()),
            receiver_type: self.ty,
            operator: OperatorKind::DivAssign,
        })
    }

    /// Calls a logical conjunction operator (`lhs && rhs`) on this Object as
    /// the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["and" operator](Prototype::implements_and) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn and(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.and else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::And,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a logical disjunction operator (`lhs || rhs`) on this Object as
    /// the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["or" operator](Prototype::implements_or) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn or(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.or else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Or,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a logical negation operator (`!rhs`) on this Object.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `rhs` parameter specifies the Rust or Script source code range
    /// that spans the operand (this Object).
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["not" operator](Prototype::implements_not) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn not(self, origin: Origin, rhs: Origin) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.not else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Not,
            });
        };

        (operator.invoke)(origin, self.arg(rhs))
    }

    /// Calls a numeric negation operator (`-rhs`) on this Object.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `rhs` parameter specifies the Rust or Script source code range
    /// that spans the operand (this Object).
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["neg" operator](Prototype::implements_neg) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn neg(self, origin: Origin, rhs: Origin) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.neg else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Neg,
            });
        };

        (operator.invoke)(origin, self.arg(rhs))
    }

    /// Calls a bitwise conjunction operator (`lhs & rhs`) on this Object as the
    /// left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["bit-and" operator](Prototype::implements_bit_and) or if
    /// the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn bit_and(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.bit_and else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::BitAnd,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a bitwise conjunction and assignment operator (`lhs &= rhs`) on
    /// this Object as the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the
    /// ["bit-and-assign" operator](Prototype::implements_bit_and_assign) or if
    /// the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn bit_and_assign(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.bit_and_assign else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::BitAndAssign,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Similar to [Object::bit_and_assign], but if the Object's type does not
    /// support the
    /// ["bit-and-assign" operator](Prototype::implements_bit_and_assign), it
    /// falls back to [Object::bit_and], and then to [Object::assign].
    ///
    /// If the fallback operators are also not supported, this function returns
    /// a [RuntimeError] indicating that the
    /// ["bit-and-assign" operator](OperatorKind::BitAndAssign) is not
    /// supported.
    #[inline]
    pub fn bit_and_assign_fallback(
        self,
        origin: Origin,
        lhs: Origin,
        rhs: Arg,
    ) -> RuntimeResult<()> {
        if let Some(operator) = &self.prototype.bit_and_assign {
            return (operator.invoke)(origin, self.arg(lhs), rhs);
        };

        if let (Some(assign_op), Some(fb_op)) = (&self.prototype.assign, &self.prototype.bit_and) {
            let lhs = self.arg(lhs);

            let rhs_origin = rhs.origin;
            let rhs_cell = (fb_op.invoke)(origin, lhs.clone(), rhs)?;

            let rhs = Arg {
                origin: rhs_origin,
                data: rhs_cell,
            };

            return (assign_op.invoke)(origin, lhs, rhs);
        }

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: Some(self.receiver.origin()),
            receiver_type: self.ty,
            operator: OperatorKind::BitAndAssign,
        })
    }

    /// Calls a bitwise disjunction operator (`lhs | rhs`) on this Object as the
    /// left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["bit-or" operator](Prototype::implements_bit_or) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn bit_or(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.bit_or else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::BitOr,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a bitwise disjunction and assignment operator (`lhs |= rhs`) on
    /// this Object as the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the
    /// ["bit-or-assign" operator](Prototype::implements_bit_or_assign) or if
    /// the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn bit_or_assign(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.bit_or_assign else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::BitOrAssign,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Similar to [Object::bit_or_assign], but if the Object's type does not
    /// support the
    /// ["bit-or-assign" operator](Prototype::implements_bit_or_assign), it
    /// falls back to [Object::bit_or], and then to [Object::assign].
    ///
    /// If the fallback operators are also not supported, this function returns
    /// a [RuntimeError] indicating that the
    /// ["bit-or-assign" operator](OperatorKind::BitOrAssign) is not supported.
    #[inline]
    pub fn bit_or_assign_fallback(
        self,
        origin: Origin,
        lhs: Origin,
        rhs: Arg,
    ) -> RuntimeResult<()> {
        if let Some(operator) = &self.prototype.bit_or_assign {
            return (operator.invoke)(origin, self.arg(lhs), rhs);
        };

        if let (Some(assign_op), Some(fb_op)) = (&self.prototype.assign, &self.prototype.bit_or) {
            let lhs = self.arg(lhs);

            let rhs_origin = rhs.origin;
            let rhs_cell = (fb_op.invoke)(origin, lhs.clone(), rhs)?;

            let rhs = Arg {
                origin: rhs_origin,
                data: rhs_cell,
            };

            return (assign_op.invoke)(origin, lhs, rhs);
        }

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: Some(self.receiver.origin()),
            receiver_type: self.ty,
            operator: OperatorKind::BitOrAssign,
        })
    }

    /// Calls a bitwise exclusive disjunction operator (`lhs ^ rhs`) on this
    /// Object as the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["bit-xor" operator](Prototype::implements_bit_xor) or if
    /// the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn bit_xor(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.bit_xor else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::BitXor,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a bitwise exclusive disjunction and assignment operator
    /// (`lhs ^= rhs`) on this Object as the left-hand side (LHS) of the
    /// operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the operation,
    /// including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the
    /// ["bit-xor-assign" operator](Prototype::implements_bit_xor_assign) or if
    /// the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn bit_xor_assign(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.bit_xor_assign else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::BitXorAssign,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Similar to [Object::bit_xor_assign], but if the Object's type does not
    /// support the
    /// ["bit-xor-assign" operator](Prototype::implements_bit_xor_assign), it
    /// falls back to [Object::bit_xor], and then to [Object::assign].
    ///
    /// If the fallback operators are also not supported, this function returns
    /// a [RuntimeError] indicating that the
    /// ["bit-xor-assign" operator](OperatorKind::BitXorAssign) is not
    /// supported.
    #[inline]
    pub fn bit_xor_assign_fallback(
        self,
        origin: Origin,
        lhs: Origin,
        rhs: Arg,
    ) -> RuntimeResult<()> {
        if let Some(operator) = &self.prototype.bit_xor_assign {
            return (operator.invoke)(origin, self.arg(lhs), rhs);
        };

        if let (Some(assign_op), Some(fb_op)) = (&self.prototype.assign, &self.prototype.bit_xor) {
            let lhs = self.arg(lhs);

            let rhs_origin = rhs.origin;
            let rhs_cell = (fb_op.invoke)(origin, lhs.clone(), rhs)?;

            let rhs = Arg {
                origin: rhs_origin,
                data: rhs_cell,
            };

            return (assign_op.invoke)(origin, lhs, rhs);
        }

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: Some(self.receiver.origin()),
            receiver_type: self.ty,
            operator: OperatorKind::BitXorAssign,
        })
    }

    /// Calls a bitwise left shift operator (`lhs << rhs`) on this
    /// Object as the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["shl" operator](Prototype::implements_shl) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn shl(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.shl else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Shl,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a bitwise left shift and assignment operator (`lhs <<= rhs`) on
    /// this Object as the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["shl-assign" operator](Prototype::implements_shl_assign) or
    /// if the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn shl_assign(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.shl_assign else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::ShlAssign,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Similar to [Object::shl_assign], but if the Object's type does not
    /// support the ["shl-assign" operator](Prototype::implements_shl_assign),
    /// it falls back to [Object::shl], and then to [Object::assign].
    ///
    /// If the fallback operators are also not supported, this function returns
    /// a [RuntimeError] indicating that the
    /// ["shl-assign" operator](OperatorKind::ShlAssign) is not supported.
    #[inline]
    pub fn shl_assign_fallback(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        if let Some(operator) = &self.prototype.shl_assign {
            return (operator.invoke)(origin, self.arg(lhs), rhs);
        };

        if let (Some(assign_op), Some(fb_op)) = (&self.prototype.assign, &self.prototype.shl) {
            let lhs = self.arg(lhs);

            let rhs_origin = rhs.origin;
            let rhs_cell = (fb_op.invoke)(origin, lhs.clone(), rhs)?;

            let rhs = Arg {
                origin: rhs_origin,
                data: rhs_cell,
            };

            return (assign_op.invoke)(origin, lhs, rhs);
        }

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: Some(self.receiver.origin()),
            receiver_type: self.ty,
            operator: OperatorKind::ShlAssign,
        })
    }

    /// Calls a bitwise right shift operator (`lhs >> rhs`) on this
    /// Object as the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["shr" operator](Prototype::implements_shr) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn shr(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.shr else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Shr,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a bitwise right shift and assignment operator (`lhs >>= rhs`) on
    /// this Object as the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the
    /// operation, including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["shr-assign" operator](Prototype::implements_shr_assign) or
    /// if the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn shr_assign(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.shr_assign else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::ShrAssign,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Same as [Object::shr_assign], but if the Object's type does not
    /// support an ["shr-assign" operator](Prototype::implements_shr_assign),
    /// falls back to the [Object::shr], and then to the [Object::assign].
    ///
    /// If the falling-back operators are not supported as well, this
    /// function returns [RuntimeError] indicating that the
    /// ["shr-assign" operator](OperatorKind::ShrAssign) is not supported.
    #[inline]
    pub fn shr_assign_fallback(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        if let Some(operator) = &self.prototype.shr_assign {
            return (operator.invoke)(origin, self.arg(lhs), rhs);
        };

        if let (Some(assign_op), Some(fb_op)) = (&self.prototype.assign, &self.prototype.shr) {
            let lhs = self.arg(lhs);

            let rhs_origin = rhs.origin;
            let rhs_cell = (fb_op.invoke)(origin, lhs.clone(), rhs)?;

            let rhs = Arg {
                origin: rhs_origin,
                data: rhs_cell,
            };

            return (assign_op.invoke)(origin, lhs, rhs);
        }

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: Some(self.receiver.origin()),
            receiver_type: self.ty,
            operator: OperatorKind::ShrAssign,
        })
    }

    /// Calls a remainder of division operator (`lhs % rhs`) on this Object as
    /// the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the operation,
    /// including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["rem" operator](Prototype::implements_rem) or if the
    /// operator's implementation returns a RuntimeError.
    #[inline]
    pub fn rem(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<Cell> {
        let Some(operator) = &self.prototype.rem else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::Rem,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Calls a remainder of division and assignment operator (`lhs %= rhs`) on
    /// this Object as the left-hand side (LHS) of the operation.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator.
    ///
    /// The `lhs` parameter specifies the Rust or Script source code range
    /// that spans the left-hand operand (this Object).
    ///
    /// The `rhs` parameter specifies the right-hand side (RHS) of the operation,
    /// including the source code range and the data of the RHS.
    ///
    /// The function returns a [RuntimeError] if the Object's type does not
    /// support the ["rem-assign" operator](Prototype::implements_rem_assign) or
    /// if the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn rem_assign(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        let Some(operator) = &self.prototype.rem_assign else {
            return Err(RuntimeError::UndefinedOperator {
                access_origin: origin,
                receiver_origin: Some(self.receiver.origin()),
                receiver_type: self.ty,
                operator: OperatorKind::RemAssign,
            });
        };

        (operator.invoke)(origin, self.arg(lhs), rhs)
    }

    /// Similar to [Object::rem_assign], but if the Object's type does not
    /// support the ["rem-assign" operator](Prototype::implements_rem_assign),
    /// it falls back to [Object::rem], and then to [Object::assign].
    ///
    /// If the fallback operators are also not supported, this
    /// function returns a [RuntimeError] indicating that the
    /// ["rem-assign" operator](OperatorKind::RemAssign) is not supported.
    #[inline]
    pub fn rem_assign_fallback(self, origin: Origin, lhs: Origin, rhs: Arg) -> RuntimeResult<()> {
        if let Some(operator) = &self.prototype.rem_assign {
            return (operator.invoke)(origin, self.arg(lhs), rhs);
        };

        if let (Some(assign_op), Some(fb_op)) = (&self.prototype.assign, &self.prototype.rem) {
            let lhs = self.arg(lhs);

            let rhs_origin = rhs.origin;
            let rhs_cell = (fb_op.invoke)(origin, lhs.clone(), rhs)?;

            let rhs = Arg {
                origin: rhs_origin,
                data: rhs_cell,
            };

            return (assign_op.invoke)(origin, lhs, rhs);
        }

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: Some(self.receiver.origin()),
            receiver_type: self.ty,
            operator: OperatorKind::RemAssign,
        })
    }

    /// Returns a reference to the underlying [Cell] of this Object.
    #[inline(always)]
    pub fn cell(&self) -> &Cell {
        &self.receiver
    }

    /// Returns the metadata of the Object's type (similar to [Cell::ty]).
    #[inline(always)]
    pub fn ty(&self) -> &'static TypeMeta {
        self.ty
    }

    /// Returns a [Prototype] of the Object's type, which describes the
    /// operations available for this type.
    #[inline(always)]
    pub fn prototype(&self) -> &'static Prototype {
        self.prototype
    }

    /// Converts this object back into the underlying Cell.
    ///
    /// This operation is the reverse of [Cell::into_object].
    #[inline(always)]
    pub fn into_cell(self) -> Cell {
        self.receiver
    }

    #[inline(always)]
    fn arg(self, origin: Origin) -> Arg {
        Arg {
            origin,
            data: self.receiver,
        }
    }
}

impl Cell {
    /// Converts a low-level Cell API into an Object, a higher-level Cell
    /// wrapper. This allows you to work with the Cell data as if it were an
    /// object of the [ScriptType].
    #[inline(always)]
    pub fn into_object(self) -> Object {
        let ty = self.ty();
        let prototype = ty.prototype();

        Object {
            receiver: self,
            ty,
            prototype,
        }
    }
}

/// A metadata about the available operations on the [type](TypeMeta).
///
/// The type's operations are exposed using the [export](crate::export) macro.
///
/// You can obtain this object from various API functions, including the
/// [TypeMeta::prototype] function.
///
/// The [Display] implementation for this object returns a map of all available
/// components (i.e., methods and fields) of the type.
#[derive(Default)]
pub struct Prototype {
    components: AHashMap<&'static str, ComponentDeclaration>,
    assign: Option<AssignOperator>,
    concat: Option<ConcatOperator>,
    field: Option<FieldOperator>,
    clone: Option<CloneOperator>,
    debug: Option<DebugOperator>,
    display: Option<DisplayOperator>,
    partial_eq: Option<PartialEqOperator>,
    default: Option<DefaultOperator>,
    partial_ord: Option<PartialOrdOperator>,
    ord: Option<OrdOperator>,
    hash: Option<HashOperator>,
    invocation: Option<InvocationOperator>,
    binding: Option<BindingOperator>,
    add: Option<AddOperator>,
    add_assign: Option<AddAssignOperator>,
    sub: Option<SubOperator>,
    sub_assign: Option<SubAssignOperator>,
    mul: Option<MulOperator>,
    mul_assign: Option<MulAssignOperator>,
    div: Option<DivOperator>,
    div_assign: Option<DivAssignOperator>,
    and: Option<AndOperator>,
    or: Option<OrOperator>,
    not: Option<NotOperator>,
    neg: Option<NegOperator>,
    bit_and: Option<BitAndOperator>,
    bit_and_assign: Option<BitAndAssignOperator>,
    bit_or: Option<BitOrOperator>,
    bit_or_assign: Option<BitOrAssignOperator>,
    bit_xor: Option<BitXorOperator>,
    bit_xor_assign: Option<BitXorAssignOperator>,
    shl: Option<ShlOperator>,
    shl_assign: Option<ShlAssignOperator>,
    shr: Option<ShrOperator>,
    shr_assign: Option<ShrAssignOperator>,
    rem: Option<RemOperator>,
    rem_assign: Option<RemAssignOperator>,
    none: Option<NoneOperator>,
}

impl Debug for Prototype {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, formatter)
    }
}

impl Display for Prototype {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let mut debug_map = formatter.debug_map();

        for component in self.components.values() {
            debug_map.entry(&component.name.string, &format_args!("{}", component.hint));
        }

        debug_map.finish()
    }
}

impl Prototype {
    /// Returns true if the underlying type has a component (a method or a
    /// field) with the specified `name`.
    ///
    /// If this function returns true, the [Object::component] supports
    /// this component `name`.
    ///
    /// The components are exposed using the `#[export(component)]` macro
    /// attribute.
    #[inline(always)]
    pub fn implements_component(&self, name: &str) -> bool {
        self.components.contains_key(name)
    }

    /// Returns true if the underlying type has a dynamic field resolver.
    ///
    /// If this function returns true, the [Object::field] operator is
    /// generally supported.
    ///
    /// Dynamic field resolvers are exposed using the
    /// [ScriptField](crate::runtime::ops::ScriptField) trait.
    #[inline(always)]
    pub fn implements_field(&self) -> bool {
        self.field.is_some()
    }

    /// Returns true if the underlying type supports the assignment operator:
    /// `lhs = rhs`.
    ///
    /// If this function returns true, the [Object::assign] operator is
    /// generally supported.
    ///
    /// Assignment operators are exposed using the
    /// [ScriptAssign](crate::runtime::ops::ScriptAssign) trait.
    #[inline(always)]
    pub fn implements_assign(&self) -> bool {
        self.assign.is_some()
    }

    /// Returns true if the underlying type supports array constructors:
    /// `[x, y, z]`.
    ///
    /// If this function returns true, the [TypeMeta::concat] constructor is
    /// generally supported.
    ///
    /// Concatenation operators are exposed using the
    /// [ScriptConcat](crate::runtime::ops::ScriptConcat) trait.
    #[inline(always)]
    pub fn implements_concat(&self) -> bool {
        self.concat.is_some()
    }

    /// Returns true if the underlying type supports cloning: `*rhs`.
    ///
    /// If this function returns true, the [Object::clone] operator is
    /// generally supported.
    ///
    /// Cloning operators are exposed using the
    /// [ScriptClone](crate::runtime::ops::ScriptClone) trait.
    #[inline(always)]
    pub fn implements_clone(&self) -> bool {
        self.clone.is_some()
    }

    /// Returns true if the underlying type supports debug formatting.
    ///
    /// If this function returns true, the [Object::debug] operator is
    /// generally supported.
    ///
    /// Debug formatting operators are exposed using the
    /// [ScriptDebug](crate::runtime::ops::ScriptDebug) trait.
    #[inline(always)]
    pub fn implements_debug(&self) -> bool {
        self.debug.is_some()
    }

    /// Returns true if the underlying type supports display formatting.
    ///
    /// If this function returns true, the [Object::display] operator is
    /// generally supported.
    ///
    /// Display formatting operators are exposed using the
    /// [ScriptDisplay](crate::runtime::ops::ScriptDisplay) trait.
    #[inline(always)]
    pub fn implements_display(&self) -> bool {
        self.display.is_some()
    }

    /// Returns true if the underlying type supports an equality operator:
    /// `lhs == rhs`.
    ///
    /// If this function returns true, the [Object::partial_eq] operator is
    /// generally supported.
    ///
    /// The assignment operators are exposed using the
    /// [ScriptPartialEq](crate::runtime::ops::ScriptPartialEq) trait.
    #[inline(always)]
    pub fn implements_partial_eq(&self) -> bool {
        self.partial_eq.is_some()
    }

    /// Returns true if the underlying type supports default constructor.
    ///
    /// If this function returns true, the [TypeMeta::instantiate] constructor
    /// is generally supported.
    ///
    /// The default operators are exposed using the
    /// [ScriptDefault](crate::runtime::ops::ScriptDefault) trait.
    #[inline(always)]
    pub fn implements_default(&self) -> bool {
        self.default.is_some()
    }

    /// Returns true if the underlying type supports a partial ordering
    /// operator: `lhs >= rhs`, `lhs < rhs`, etc.
    ///
    /// If this function returns true, the [Object::partial_ord] operator is
    /// generally supported.
    ///
    /// The partial ordering operators are exposed using the
    /// [ScriptPartialOrd](crate::runtime::ops::ScriptPartialOrd) trait.
    #[inline(always)]
    pub fn implements_partial_ord(&self) -> bool {
        self.partial_ord.is_some()
    }

    /// Returns true if the underlying type supports a full ordering operator:
    /// `lhs >= rhs`, `lhs < rhs`, etc.
    ///
    /// If this function returns true, the [Object::ord] operator is
    /// generally supported.
    ///
    /// The full ordering operators are exposed using the
    /// [ScriptOrd](crate::runtime::ops::ScriptOrd) trait.
    #[inline(always)]
    pub fn implements_ord(&self) -> bool {
        self.ord.is_some()
    }

    /// Returns true if the underlying type supports data hashing.
    ///
    /// If this function returns true, the [Object::hash] operator is
    /// generally supported.
    ///
    /// The hashing operators are exposed using the
    /// [ScriptHash](crate::runtime::ops::ScriptHash) trait.
    #[inline(always)]
    pub fn implements_hash(&self) -> bool {
        self.hash.is_some()
    }

    /// Returns true if the underlying type supports an invocation operator:
    /// `foo(a, b, c)`.
    ///
    /// If this function returns true, the [Object::invoke] operator is
    /// generally supported.
    ///
    /// The invocation operators are exposed using the
    /// [ScriptInvocation](crate::runtime::ops::ScriptInvocation) trait.
    #[inline(always)]
    pub fn implements_invocation(&self) -> bool {
        self.invocation.is_some()
    }

    /// Returns true if the underlying type supports context binding.
    ///
    /// If this function returns true, the [Object::bind] operator is
    /// generally supported.
    ///
    /// The binging operators are exposed using the
    /// [ScriptBinding](crate::runtime::ops::ScriptBinding) trait.
    #[inline(always)]
    pub fn implements_binding(&self) -> bool {
        self.binding.is_some()
    }

    /// Returns true if the underlying type supports an addition operator:
    /// `lhs + rhs`.
    ///
    /// If this function returns true, the [Object::add] operator is
    /// generally supported.
    ///
    /// The addition operators are exposed using the
    /// [ScriptAdd](crate::runtime::ops::ScriptAdd) trait.
    #[inline(always)]
    pub fn implements_add(&self) -> bool {
        self.add.is_some()
    }

    /// Returns true if the underlying type supports an addition and assignment
    /// operator: `lhs += rhs`.
    ///
    /// If this function returns true, the [Object::add_assign] operator is
    /// generally supported.
    ///
    /// The addition and assignment operators are exposed using the
    /// [ScriptAddAssign](crate::runtime::ops::ScriptAddAssign) trait.
    #[inline(always)]
    pub fn implements_add_assign(&self) -> bool {
        self.add_assign.is_some()
    }

    /// Returns true if the underlying type supports a subtraction operator:
    /// `lhs - rhs`.
    ///
    /// If this function returns true, the [Object::sub] operator is
    /// generally supported.
    ///
    /// The subtraction operators are exposed using the
    /// [ScriptSub](crate::runtime::ops::ScriptSub) trait.
    #[inline(always)]
    pub fn implements_sub(&self) -> bool {
        self.sub.is_some()
    }

    /// Returns true if the underlying type supports a subtraction and
    /// assignment operator: `lhs -= rhs`.
    ///
    /// If this function returns true, the [Object::sub_assign] operator is
    /// generally supported.
    ///
    /// The subtraction and assignment operators are exposed using the
    /// [ScriptSubAssign](crate::runtime::ops::ScriptSubAssign) trait.
    #[inline(always)]
    pub fn implements_sub_assign(&self) -> bool {
        self.sub_assign.is_some()
    }

    /// Returns true if the underlying type supports a multiplication operator:
    /// `lhs * rhs`.
    ///
    /// If this function returns true, the [Object::mul] operator is
    /// generally supported.
    ///
    /// The multiplication operators are exposed using the
    /// [ScriptMul](crate::runtime::ops::ScriptMul) trait.
    #[inline(always)]
    pub fn implements_mul(&self) -> bool {
        self.mul.is_some()
    }

    /// Returns true if the underlying type supports a multiplication and
    /// assignment operator: `lhs *= rhs`.
    ///
    /// If this function returns true, the [Object::sub_assign] operator is
    /// generally supported.
    ///
    /// The multiplication and assignment operators are exposed using the
    /// [ScriptMulAssign](crate::runtime::ops::ScriptMulAssign) trait.
    #[inline(always)]
    pub fn implements_mul_assign(&self) -> bool {
        self.mul_assign.is_some()
    }

    /// Returns true if the underlying type supports a division operator:
    /// `lhs / rhs`.
    ///
    /// If this function returns true, the [Object::div] operator is
    /// generally supported.
    ///
    /// The division operators are exposed using the
    /// [ScriptDiv](crate::runtime::ops::ScriptDiv) trait.
    #[inline(always)]
    pub fn implements_div(&self) -> bool {
        self.div.is_some()
    }

    /// Returns true if the underlying type supports a division and
    /// assignment operator: `lhs /= rhs`.
    ///
    /// If this function returns true, the [Object::div_assign] operator is
    /// generally supported.
    ///
    /// The division and assignment operators are exposed using the
    /// [ScriptDivAssign](crate::runtime::ops::ScriptDivAssign) trait.
    #[inline(always)]
    pub fn implements_div_assign(&self) -> bool {
        self.div_assign.is_some()
    }

    /// Returns true if the underlying type supports a logical conjunction
    /// operator: `lhs && rhs`.
    ///
    /// If this function returns true, the [Object::and] operator is
    /// generally supported.
    ///
    /// The logical conjunction operators are exposed using the
    /// [ScriptAnd](crate::runtime::ops::ScriptAnd) trait.
    #[inline(always)]
    pub fn implements_and(&self) -> bool {
        self.and.is_some()
    }

    /// Returns true if the underlying type supports a logical disjunction
    /// operator: `lhs || rhs`.
    ///
    /// If this function returns true, the [Object::or] operator is
    /// generally supported.
    ///
    /// The logical disjunction operators are exposed using the
    /// [ScriptOr](crate::runtime::ops::ScriptOr) trait.
    #[inline(always)]
    pub fn implements_or(&self) -> bool {
        self.or.is_some()
    }

    /// Returns true if the underlying type supports a logical negation
    /// operator: `!foo`.
    ///
    /// If this function returns true, the [Object::not] operator is
    /// generally supported.
    ///
    /// The logical negation operators are exposed using the
    /// [ScriptNot](crate::runtime::ops::ScriptNot) trait.
    #[inline(always)]
    pub fn implements_not(&self) -> bool {
        self.not.is_some()
    }

    /// Returns true if the underlying type supports a numeric negation
    /// operator: `-foo`.
    ///
    /// If this function returns true, the [Object::neg] operator is
    /// generally supported.
    ///
    /// The numeric negation operators are exposed using the
    /// [ScriptNeg](crate::runtime::ops::ScriptNeg) trait.
    #[inline(always)]
    pub fn implements_neg(&self) -> bool {
        self.neg.is_some()
    }

    /// Returns true if the underlying type supports a bitwise conjunction
    /// operator: `lhs & rhs`.
    ///
    /// If this function returns true, the [Object::bit_and] operator is
    /// generally supported.
    ///
    /// The bitwise conjunction operators are exposed using the
    /// [ScriptBitAnd](crate::runtime::ops::ScriptBitAnd) trait.
    #[inline(always)]
    pub fn implements_bit_and(&self) -> bool {
        self.bit_and.is_some()
    }

    /// Returns true if the underlying type supports a bitwise conjunction and
    /// assignment operator: `lhs &= rhs`.
    ///
    /// If this function returns true, the [Object::bit_and_assign] operator is
    /// generally supported.
    ///
    /// The bitwise conjunction and assignment operators are exposed using the
    /// [ScriptBitAndAssign](crate::runtime::ops::ScriptBitAndAssign) trait.
    #[inline(always)]
    pub fn implements_bit_and_assign(&self) -> bool {
        self.bit_and_assign.is_some()
    }

    /// Returns true if the underlying type supports a bitwise disjunction
    /// operator: `lhs | rhs`.
    ///
    /// If this function returns true, the [Object::bit_or] operator is
    /// generally supported.
    ///
    /// The bitwise disjunction operators are exposed using the
    /// [ScriptBitOr](crate::runtime::ops::ScriptBitOr) trait.
    #[inline(always)]
    pub fn implements_bit_or(&self) -> bool {
        self.bit_or.is_some()
    }

    /// Returns true if the underlying type supports a bitwise disjunction and
    /// assignment operator: `lhs |= rhs`.
    ///
    /// If this function returns true, the [Object::bit_or_assign] operator is
    /// generally supported.
    ///
    /// The bitwise disjunction and assignment operators are exposed using the
    /// [ScriptBitOrAssign](crate::runtime::ops::ScriptBitOrAssign) trait.
    #[inline(always)]
    pub fn implements_bit_or_assign(&self) -> bool {
        self.bit_or_assign.is_some()
    }

    /// Returns true if the underlying type supports a bitwise exclusive
    /// disjunction operator: `lhs ^ rhs`.
    ///
    /// If this function returns true, the [Object::bit_xor] operator is
    /// generally supported.
    ///
    /// The bitwise exclusive disjunction operators are exposed using the
    /// [ScriptBitXor](crate::runtime::ops::ScriptBitXor) trait.
    #[inline(always)]
    pub fn implements_bit_xor(&self) -> bool {
        self.bit_xor.is_some()
    }

    /// Returns true if the underlying type supports a bitwise exclusive
    /// disjunction and assignment operator: `lhs ^= rhs`.
    ///
    /// If this function returns true, the [Object::bit_xor_assign] operator is
    /// generally supported.
    ///
    /// The bitwise exclusive disjunction and assignment operators are exposed
    /// using the [ScriptBitXorAssign](crate::runtime::ops::ScriptBitXorAssign)
    /// trait.
    #[inline(always)]
    pub fn implements_bit_xor_assign(&self) -> bool {
        self.bit_xor_assign.is_some()
    }

    /// Returns true if the underlying type supports a bitwise left shift
    /// operator: `lhs << rhs`.
    ///
    /// If this function returns true, the [Object::shl] operator is
    /// generally supported.
    ///
    /// The bitwise left shift operators are exposed using the
    /// [ScriptShl](crate::runtime::ops::ScriptShl) trait.
    #[inline(always)]
    pub fn implements_shl(&self) -> bool {
        self.shl.is_some()
    }

    /// Returns true if the underlying type supports a bitwise left shift
    /// and assignment operator: `lhs <<= rhs`.
    ///
    /// If this function returns true, the [Object::shl_assign] operator is
    /// generally supported.
    ///
    /// The bitwise left shift and assignment operators are exposed
    /// using the [ScriptShlAssign](crate::runtime::ops::ScriptShlAssign)
    /// trait.
    #[inline(always)]
    pub fn implements_shl_assign(&self) -> bool {
        self.shl_assign.is_some()
    }

    /// Returns true if the underlying type supports a bitwise right shift
    /// operator: `lhs >> rhs`.
    ///
    /// If this function returns true, the [Object::shr] operator is
    /// generally supported.
    ///
    /// The bitwise right shift operators are exposed using the
    /// [ScriptShr](crate::runtime::ops::ScriptShr) trait.
    #[inline(always)]
    pub fn implements_shr(&self) -> bool {
        self.shr.is_some()
    }

    /// Returns true if the underlying type supports a bitwise right shift
    /// and assignment operator: `lhs >>= rhs`.
    ///
    /// If this function returns true, the [Object::shr_assign] operator is
    /// generally supported.
    ///
    /// The bitwise right shift and assignment operators are exposed
    /// using the [ScriptShrAssign](crate::runtime::ops::ScriptShrAssign)
    /// trait.
    #[inline(always)]
    pub fn implements_shr_assign(&self) -> bool {
        self.shr_assign.is_some()
    }

    /// Returns true if the underlying type supports a reminder of division
    /// operator: `lhs % rhs`.
    ///
    /// If this function returns true, the [Object::rem] operator is
    /// generally supported.
    ///
    /// The reminder of division operators are exposed using the
    /// [ScriptRem](crate::runtime::ops::ScriptRem) trait.
    #[inline(always)]
    pub fn implements_rem(&self) -> bool {
        self.rem.is_some()
    }

    /// Returns true if the underlying type supports a reminder of division
    /// and assignment operator: `lhs %= rhs`.
    ///
    /// If this function returns true, the [Object::rem_assign] operator is
    /// generally supported.
    ///
    /// The reminder of division and assignment operators are exposed
    /// using the [ScriptRemAssign](crate::runtime::ops::ScriptRemAssign)
    /// trait.
    #[inline(always)]
    pub fn implements_rem_assign(&self) -> bool {
        self.rem_assign.is_some()
    }

    /// Returns true if this type represents void data. The query script
    /// operator `foo?` tests if the underlying object has "none"-type.
    ///
    /// In particular the [Nil](TypeMeta::nil) is a "none"-type.
    ///
    /// The none markers are exposed using the
    /// [ScriptNone](crate::runtime::ops::ScriptNone) trait.
    #[inline(always)]
    pub fn implements_none(&self) -> bool {
        self.none.is_some()
    }

    /// Returns the right-hand side type of the assignment operator:
    /// `lhs = rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptAssign::RHS](crate::runtime::ops::ScriptAssign::RHS) associated
    /// type.
    ///
    /// If the assignment operator is not [supported](Self::implements_assign),
    /// the function returns None.
    #[inline(always)]
    pub fn hint_assign_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.assign {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the type of the result of objects concatenations:
    /// `[a, b, c]`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptConcat::Result](crate::runtime::ops::ScriptConcat::Result)
    /// associated type.
    ///
    /// If the concatenation operator is not
    /// [supported](Self::implements_concat), the function returns None.
    #[inline(always)]
    pub fn hint_concat_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.concat {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the description of the type's component (e.g., a Rust struct
    /// method or field).
    ///
    /// The `name` parameter specifies the component's name (e.g., `foo.<name>`).
    ///
    /// If this type [does not have](Self::implements_component) the specified
    /// component, the function returns None.
    #[inline(always)]
    pub fn hint_component(&self, name: impl AsRef<str>) -> Option<ComponentHint> {
        if let Some(component) = self.components.get(name.as_ref()) {
            return Some(ComponentHint {
                name: component.name,
                ty: TypeHint::Type(component.hint),
                doc: component.doc,
            });
        }

        None
    }

    /// Enumerates all exported components of this type (e.g., all Rust struct
    /// methods and fields). The iterator yields descriptions for each
    /// component.
    #[inline(always)]
    pub fn hint_all_components(&self) -> impl Iterator<Item = ComponentHint> + '_ {
        self.components.values().map(|component| ComponentHint {
            name: component.name,
            ty: TypeHint::Type(component.hint),
            doc: component.doc,
        })
    }

    /// Returns the number of all known exported components of this type (e.g.,
    /// the number of all Rust struct methods and fields).
    #[inline(always)]
    pub fn components_len(&self) -> usize {
        self.components.len()
    }

    /// Returns the type of the result of objects concatenations:
    /// `[a, b, c]`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptConcat::Result](crate::runtime::ops::ScriptConcat::Result)
    /// associated type.
    ///
    /// If the concatenation operator is not
    /// [supported](Self::implements_concat), the function returns None.
    #[inline(always)]
    pub fn hint_field(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.field {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the right-hand side type of the quality operator:
    /// `lhs == rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptPartialEq::RHS](crate::runtime::ops::ScriptPartialEq::RHS)
    /// associated type.
    ///
    /// If the equality operator is not [supported](Self::implements_partial_eq),
    /// the function returns None.
    #[inline(always)]
    pub fn hint_partial_eq_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.partial_eq {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the right-hand side type of the partial ordering operator:
    /// `lhs >= rhs`, `lhs < rhs`, etc.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptPartialOrd::RHS](crate::runtime::ops::ScriptPartialOrd::RHS)
    /// associated type.
    ///
    /// If the partial ordering operator is not
    /// [supported](Self::implements_partial_ord), the function returns None.
    #[inline(always)]
    pub fn hint_partial_ord_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.partial_ord {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the invocation operator description for the type:
    /// `foo(a, b, c)`.
    ///
    /// If the invocation operator is not
    /// [supported](Self::implements_invocation), the function returns None.
    #[inline(always)]
    pub fn hint_invocation(&self) -> Option<&'static InvocationMeta> {
        if let Some(operator) = &self.invocation {
            return (operator.hint)();
        }

        None
    }

    /// Returns the right-hand side type of the addition operator:
    /// `lhs + rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptAdd::RHS](crate::runtime::ops::ScriptAdd::RHS) associated
    /// type.
    ///
    /// If the addition operator is not [supported](Self::implements_add),
    /// the function returns None.
    #[inline(always)]
    pub fn hint_add_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.add {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the result type of the addition operator:
    /// `lhs + rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptAdd::Result](crate::runtime::ops::ScriptAdd::Result) associated
    /// type.
    ///
    /// If the addition operator is not [supported](Self::implements_add),
    /// the function returns None.
    #[inline(always)]
    pub fn hint_add_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.add {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the right-hand side type of the addition and assignment
    /// operator: `lhs += rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptAddAssign::RHS](crate::runtime::ops::ScriptAddAssign::RHS)
    /// associated type.
    ///
    /// If the addition and assignment operator is not
    /// [supported](Self::implements_add_assign), the function returns None.
    #[inline(always)]
    pub fn hint_add_assign_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.add_assign {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the right-hand side type of the subtraction operator:
    /// `lhs - rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptSub::RHS](crate::runtime::ops::ScriptSub::RHS) associated
    /// type.
    ///
    /// If the subtraction operator is not [supported](Self::implements_sub),
    /// the function returns None.
    #[inline(always)]
    pub fn hint_sub_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.sub {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the result type of the subtraction operator:
    /// `lhs - rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptSub::Result](crate::runtime::ops::ScriptSub::Result) associated
    /// type.
    ///
    /// If the subtraction operator is not [supported](Self::implements_sub),
    /// the function returns None.
    #[inline(always)]
    pub fn hint_sub_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.sub {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the right-hand side type of the subtraction and assignment
    /// operator: `lhs -= rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptSubAssign::RHS](crate::runtime::ops::ScriptSubAssign::RHS)
    /// associated type.
    ///
    /// If the subtraction and assignment operator is not
    /// [supported](Self::implements_sub_assign), the function returns None.
    #[inline(always)]
    pub fn hint_sub_assign_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.sub_assign {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the right-hand side type of the multiplication operator:
    /// `lhs * rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptMul::RHS](crate::runtime::ops::ScriptMul::RHS) associated
    /// type.
    ///
    /// If the multiplication operator is not [supported](Self::implements_mul),
    /// the function returns None.
    #[inline(always)]
    pub fn hint_mul_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.mul {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the result type of the multiplication operator:
    /// `lhs * rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptMul::Result](crate::runtime::ops::ScriptMul::Result) associated
    /// type.
    ///
    /// If the multiplication operator is not [supported](Self::implements_mul),
    /// the function returns None.
    #[inline(always)]
    pub fn hint_mul_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.mul {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the right-hand side type of the multiplication and assignment
    /// operator: `lhs *= rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptMulAssign::RHS](crate::runtime::ops::ScriptMulAssign::RHS)
    /// associated type.
    ///
    /// If the multiplication and assignment operator is not
    /// [supported](Self::implements_mul_assign), the function returns None.
    #[inline(always)]
    pub fn hint_mul_assign_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.mul_assign {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the right-hand side type of the division operator:
    /// `lhs / rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptDiv::RHS](crate::runtime::ops::ScriptDiv::RHS) associated
    /// type.
    ///
    /// If the division operator is not [supported](Self::implements_div),
    /// the function returns None.
    #[inline(always)]
    pub fn hint_div_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.div {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the result type of the division operator:
    /// `lhs / rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptDiv::Result](crate::runtime::ops::ScriptDiv::Result) associated
    /// type.
    ///
    /// If the division operator is not [supported](Self::implements_div),
    /// the function returns None.
    #[inline(always)]
    pub fn hint_div_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.div {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the right-hand side type of the division and assignment
    /// operator: `lhs /= rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptDivAssign::RHS](crate::runtime::ops::ScriptDivAssign::RHS)
    /// associated type.
    ///
    /// If the division and assignment operator is not
    /// [supported](Self::implements_div_assign), the function returns None.
    #[inline(always)]
    pub fn hint_div_assign_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.div_assign {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the right-hand side type of the logical conjunction operator:
    /// `lhs && rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptAnd::RHS](crate::runtime::ops::ScriptAnd::RHS) associated
    /// type.
    ///
    /// If the logical conjunction operator is not
    /// [supported](Self::implements_and), the function returns None.
    #[inline(always)]
    pub fn hint_and_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.and {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the result type of the logical conjunction operator:
    /// `lhs && rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptAnd::Result](crate::runtime::ops::ScriptAnd::Result) associated
    /// type.
    ///
    /// If the logical conjunction operator is not
    /// [supported](Self::implements_and), the function returns None.
    #[inline(always)]
    pub fn hint_and_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.and {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the right-hand side type of the logical disjunction operator:
    /// `lhs || rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptOr::RHS](crate::runtime::ops::ScriptOr::RHS) associated
    /// type.
    ///
    /// If the logical disjunction operator is not
    /// [supported](Self::implements_or), the function returns None.
    #[inline(always)]
    pub fn hint_or_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.or {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the result type of the logical disjunction operator:
    /// `lhs || rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptOr::Result](crate::runtime::ops::ScriptOr::Result) associated
    /// type.
    ///
    /// If the logical disjunction operator is not
    /// [supported](Self::implements_or), the function returns None.
    #[inline(always)]
    pub fn hint_or_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.or {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the result type of the logical negation operator:
    /// `!foo`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptNot::Result](crate::runtime::ops::ScriptNot::Result) associated
    /// type.
    ///
    /// If the logical negation operator is not
    /// [supported](Self::implements_not), the function returns None.
    #[inline(always)]
    pub fn hint_not_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.not {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the result type of the numeric negation operator:
    /// `-foo`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptNeg::Result](crate::runtime::ops::ScriptNeg::Result) associated
    /// type.
    ///
    /// If the numeric negation operator is not
    /// [supported](Self::implements_neg), the function returns None.
    #[inline(always)]
    pub fn hint_neg_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.neg {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the right-hand side type of the bitwise conjunction operator:
    /// `lhs & rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptBitAnd::RHS](crate::runtime::ops::ScriptBitAnd::RHS) associated
    /// type.
    ///
    /// If the bitwise conjunction operator is not
    /// [supported](Self::implements_bit_and), the function returns None.
    #[inline(always)]
    pub fn hint_bit_and_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.bit_and {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the result type of the bitwise conjunction operator:
    /// `lhs & rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptBitAnd::Result](crate::runtime::ops::ScriptBitAnd::Result)
    /// associated type.
    ///
    /// If the bitwise conjunction operator is not
    /// [supported](Self::implements_bit_and), the function returns None.
    #[inline(always)]
    pub fn hint_bit_and_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.bit_and {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the right-hand side type of the bitwise conjunction and
    /// assignment operator: `lhs &= rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptBitAndAssign::RHS](crate::runtime::ops::ScriptBitAndAssign::RHS)
    /// associated type.
    ///
    /// If the bitwise conjunction and assignment operator is not
    /// [supported](Self::implements_bit_and_assign), the function returns None.
    #[inline(always)]
    pub fn hint_bit_and_assign_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.bit_and_assign {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the right-hand side type of the bitwise disjunction operator:
    /// `lhs | rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptBitOr::RHS](crate::runtime::ops::ScriptBitOr::RHS) associated
    /// type.
    ///
    /// If the bitwise disjunction operator is not
    /// [supported](Self::implements_bit_or), the function returns None.
    #[inline(always)]
    pub fn hint_bit_or_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.bit_or {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the result type of the bitwise disjunction operator:
    /// `lhs | rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptBitOr::Result](crate::runtime::ops::ScriptBitOr::Result)
    /// associated type.
    ///
    /// If the bitwise disjunction operator is not
    /// [supported](Self::implements_bit_or), the function returns None.
    #[inline(always)]
    pub fn hint_bit_or_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.bit_or {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the right-hand side type of the bitwise disjunction and
    /// assignment operator: `lhs |= rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptBitOrAssign::RHS](crate::runtime::ops::ScriptBitOrAssign::RHS)
    /// associated type.
    ///
    /// If the bitwise disjunction and assignment operator is not
    /// [supported](Self::implements_bit_or_assign), the function returns None.
    #[inline(always)]
    pub fn hint_bit_or_assign_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.bit_or_assign {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the right-hand side type of the bitwise exclusive disjunction
    /// operator: `lhs ^ rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptBitXor::RHS](crate::runtime::ops::ScriptBitXor::RHS) associated
    /// type.
    ///
    /// If the bitwise exclusive disjunction operator is not
    /// [supported](Self::implements_bit_xor), the function returns None.
    #[inline(always)]
    pub fn hint_bit_xor_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.bit_xor {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the result type of the bitwise exclusive disjunction operator:
    /// `lhs ^ rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptBitXor::Result](crate::runtime::ops::ScriptBitXor::Result)
    /// associated type.
    ///
    /// If the bitwise exclusive disjunction operator is not
    /// [supported](Self::implements_bit_xor), the function returns None.
    #[inline(always)]
    pub fn hint_bit_xor_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.bit_xor {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the right-hand side type of the bitwise exclusive disjunction
    /// and assignment operator: `lhs ^= rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptBitXorAssign::RHS](crate::runtime::ops::ScriptBitXorAssign::RHS)
    /// associated type.
    ///
    /// If the bitwise exclusive disjunction and assignment operator is not
    /// [supported](Self::implements_bit_xor_assign), the function returns None.
    #[inline(always)]
    pub fn hint_bit_xor_assign_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.bit_xor_assign {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the right-hand side type of the bitwise left shift
    /// operator: `lhs << rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptShl::RHS](crate::runtime::ops::ScriptShl::RHS) associated
    /// type.
    ///
    /// If the bitwise left shift operator is not
    /// [supported](Self::implements_shl), the function returns None.
    #[inline(always)]
    pub fn hint_shl_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.shl {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the result type of the bitwise left shift operator:
    /// `lhs << rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptShl::Result](crate::runtime::ops::ScriptShl::Result)
    /// associated type.
    ///
    /// If the bitwise left shift operator is not
    /// [supported](Self::implements_shl), the function returns None.
    #[inline(always)]
    pub fn hint_shl_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.shl {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the right-hand side type of the left shift
    /// and assignment operator: `lhs <<= rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptShlAssign::RHS](crate::runtime::ops::ScriptShlAssign::RHS)
    /// associated type.
    ///
    /// If the left shift and assignment operator is not
    /// [supported](Self::implements_shl_assign), the function returns None.
    #[inline(always)]
    pub fn hint_shl_assign_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.shl_assign {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the right-hand side type of the bitwise right shift
    /// operator: `lhs >> rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptShr::RHS](crate::runtime::ops::ScriptShr::RHS) associated
    /// type.
    ///
    /// If the bitwise right shift operator is not
    /// [supported](Self::implements_shr), the function returns None.
    #[inline(always)]
    pub fn hint_shr_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.shr {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the result type of the bitwise right shift operator:
    /// `lhs >> rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptShr::Result](crate::runtime::ops::ScriptShr::Result)
    /// associated type.
    ///
    /// If the bitwise right shift operator is not
    /// [supported](Self::implements_shr), the function returns None.
    #[inline(always)]
    pub fn hint_shr_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.shr {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the right-hand side type of the right shift
    /// and assignment operator: `lhs >>= rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptShrAssign::RHS](crate::runtime::ops::ScriptShrAssign::RHS)
    /// associated type.
    ///
    /// If the right shift and assignment operator is not
    /// [supported](Self::implements_shr_assign), the function returns None.
    #[inline(always)]
    pub fn hint_shr_assign_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.shr_assign {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the right-hand side type of the reminder of division
    /// operator: `lhs % rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptRem::RHS](crate::runtime::ops::ScriptRem::RHS) associated
    /// type.
    ///
    /// If the reminder of division operator is not
    /// [supported](Self::implements_rem), the function returns None.
    #[inline(always)]
    pub fn hint_rem_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.rem {
            return Some(operator.hint_rhs);
        }

        None
    }

    /// Returns the result type of the reminder of division operator:
    /// `lhs % rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptRem::Result](crate::runtime::ops::ScriptRem::Result)
    /// associated type.
    ///
    /// If the reminder of division operator is not
    /// [supported](Self::implements_rem), the function returns None.
    #[inline(always)]
    pub fn hint_rem_result(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.rem {
            return Some(operator.hint_result);
        }

        None
    }

    /// Returns the right-hand side type of the reminder of division
    /// and assignment operator: `lhs %= rhs`.
    ///
    /// The returned type metadata corresponds to the
    /// [ScriptRemAssign::RHS](crate::runtime::ops::ScriptRemAssign::RHS)
    /// associated type.
    ///
    /// If the reminder of division and assignment operator is not
    /// [supported](Self::implements_rem_assign), the function returns None.
    #[inline(always)]
    pub fn hint_rem_assign_rhs(&self) -> Option<&'static TypeMeta> {
        if let Some(operator) = &self.rem_assign {
            return Some(operator.hint_rhs);
        }

        None
    }

    // Safety: The prototype describes type `T`.
    #[inline]
    pub(super) unsafe fn clone_first<T: ScriptType>(
        &self,
        access_origin: &Origin,
        slice_origin: &Origin,
        slice: &[T],
    ) -> RuntimeResult<T> {
        let clone_fn = match &self.clone {
            // Safety: Upheld by the caller.
            Some(operator) => unsafe { operator.clone_fn.into_fn::<T>() },

            None => {
                return Err(RuntimeError::UndefinedOperator {
                    access_origin: *access_origin,
                    receiver_origin: Some(*slice_origin),
                    receiver_type: T::type_meta(),
                    operator: OperatorKind::Clone,
                });
            }
        };

        let length = slice.len();

        if slice.len() != 1 {
            return Err(RuntimeError::NonSingleton {
                access_origin: *access_origin,
                actual: length,
            });
        }

        let first = match slice.first() {
            Some(first) => first,

            // Safety: Slice length checked above.
            None => unsafe { debug_unreachable!("Missing slice first item.") },
        };

        return Ok(clone_fn(first));
    }

    // Safety: The prototype describes type `T`.
    #[inline]
    pub(super) unsafe fn clone_slice<T: ScriptType>(
        &self,
        access_origin: &Origin,
        slice_origin: &Origin,
        slice: &[T],
    ) -> RuntimeResult<Box<[T]>> {
        let clone_fn = match &self.clone {
            // Safety: Upheld by the caller.
            Some(operator) => unsafe { operator.clone_fn.into_fn::<T>() },

            None => {
                return Err(RuntimeError::UndefinedOperator {
                    access_origin: *access_origin,
                    receiver_origin: Some(*slice_origin),
                    receiver_type: T::type_meta(),
                    operator: OperatorKind::Clone,
                });
            }
        };

        return Ok(slice
            .iter()
            .map(clone_fn)
            .collect::<Vec<_>>()
            .into_boxed_slice());
    }
}

impl TypeMeta {
    /// Returns a [Prototype] of the Rust type that describes the script
    /// operations available for this type.
    #[inline(always)]
    pub fn prototype(&self) -> &'static Prototype {
        let registry = PrototypeRegistry::get();

        match registry.prototypes.get(self.id()) {
            Some(prototype) => prototype,

            // Safety: Each TypeMeta has corresponding Prototype.
            None => unsafe { debug_unreachable!("TypeMeta without Prototype.") },
        }
    }

    /// Creates an instance of this type using
    /// the [default constructor](Prototype::implements_default).
    ///
    /// The function returns [RuntimeError] if the default constructor is not
    /// exported for this type, or if the constructor's implementation returns
    /// a RuntimeError.
    #[inline]
    pub fn instantiate(&'static self, origin: Origin) -> RuntimeResult<Cell> {
        if let Some(operator) = &self.prototype().default {
            return (operator.invoke)(origin);
        }

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: None,
            receiver_type: self,
            operator: OperatorKind::Default,
        })
    }

    /// Creates an array of objects by concatenating the data in `items` into
    /// a single slice in the resulting `Cell`.
    ///
    /// The `items` source array does not necessarily have to be an array of
    /// objects of the same type. The underlying implementation may attempt
    /// to cast them into other types. The source array may also contain "gaps"
    /// ([Nil Cells](Cell::nil)) and Cells with arrays. In this case, the
    /// canonical implementation of this operator typically flattens these
    /// sub-arrays. Additionally, the canonical implementation usually takes
    /// [Arg] objects from the `items` source slice.
    ///
    /// The `origin` parameter specifies the Rust or Script source code range
    /// that spans the operator (e.g., the script array declaration).
    ///
    /// The function returns a [RuntimeError] if the type does not
    /// support the ["concat" operator](Prototype::implements_concat), or if
    /// the operator's implementation returns a RuntimeError.
    #[inline]
    pub fn concat(&'static self, origin: Origin, items: &mut [Arg]) -> RuntimeResult<Cell> {
        if let Some(operator) = &self.prototype().concat {
            return (operator.invoke)(origin, items);
        }

        Err(RuntimeError::UndefinedOperator {
            access_origin: origin,
            receiver_origin: None,
            receiver_type: self,
            operator: OperatorKind::Concat,
        })
    }
}

struct PrototypeRegistry {
    prototypes: AHashMap<TypeId, Prototype>,
}

impl PrototypeRegistry {
    #[inline(always)]
    fn get() -> &'static Self {
        static REGISTRY: Lazy<PrototypeRegistry> = Lazy::new(|| {
            let mut prototypes = TypeMeta::enumerate()
                .map(|id| (*id, Prototype::default()))
                .collect::<AHashMap<TypeId, _>>();

            for group in DeclarationGroup::enumerate() {
                let origin = group.origin;

                for declaration in &group.prototypes {
                    let declaration = declaration();

                    let type_meta = match TypeMeta::by_id(&declaration.receiver) {
                        Some(meta) => meta,

                        None => origin.blame("Unregistered TypeMeta."),
                    };

                    let type_meta_package = type_meta.origin().package;

                    let origin_package = match origin.package {
                        Some(package) => package,
                        None => {
                            system_panic!("DeclarationGroup origin without package.")
                        }
                    };

                    match type_meta_package {
                        Some(type_meta_package) => {
                            if type_meta_package != origin_package {
                                origin.blame(&format!(
                                    "Type {} declared in the package \
                                    {}@{}. Type semantics can not be extended \
                                    from the foreign crate {}@{}.",
                                    type_meta,
                                    type_meta_package.0,
                                    type_meta_package.1,
                                    origin_package.0,
                                    origin_package.1,
                                ))
                            }
                        }

                        None => origin.blame(&format!(
                            "Built-in type {} can not be extended from the \
                                foreign crate {}@{}.",
                            type_meta, origin_package.0, origin_package.1,
                        )),
                    }

                    let prototype = match prototypes.get_mut(&declaration.receiver) {
                        Some(prototype) => prototype,

                        None => {
                            // Safety:
                            //   1. TypeMeta existence checked above.
                            //   2. Each TypeMeta has corresponding Prototype.
                            unsafe { debug_unreachable!("Missing Prototype for registered type.") }
                        }
                    };

                    for component in declaration.components {
                        let name = component.name.string;

                        if let Some(previous) = prototype.components.get(name) {
                            let previous = previous.name.origin;

                            component.name.origin.blame(&format!(
                                "Duplicate \"{type_meta}.{name}\" component \
                                declaration. The same component already \
                                declared in {previous}.",
                            ))
                        }

                        if prototype.components.insert(name, component).is_some() {
                            // Safety: Uniqueness checked above.
                            unsafe { debug_unreachable!("Duplicate component entry.") };
                        }
                    }

                    for operator in declaration.operators {
                        match operator {
                            OperatorDeclaration::Assign(operator) => {
                                if let Some(previous) = &prototype.assign {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Assign operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.assign = Some(operator);
                            }

                            OperatorDeclaration::Concat(operator) => {
                                if let Some(previous) = &prototype.concat {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Concat operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.concat = Some(operator);
                            }

                            OperatorDeclaration::Field(operator) => {
                                if let Some(previous) = &prototype.field {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Field operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.field = Some(operator);
                            }

                            OperatorDeclaration::Clone(operator) => {
                                if let Some(previous) = &prototype.clone {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Clone operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.clone = Some(operator);
                            }

                            OperatorDeclaration::Debug(operator) => {
                                if let Some(previous) = &prototype.debug {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Debug operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.debug = Some(operator);
                            }

                            OperatorDeclaration::Display(operator) => {
                                if let Some(previous) = &prototype.display {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Display operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.display = Some(operator);
                            }

                            OperatorDeclaration::PartialEq(operator) => {
                                if let Some(previous) = &prototype.partial_eq {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        PartialEq operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.partial_eq = Some(operator);
                            }

                            OperatorDeclaration::Default(operator) => {
                                if let Some(previous) = &prototype.default {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Default operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.default = Some(operator);
                            }

                            OperatorDeclaration::PartialOrd(operator) => {
                                if let Some(previous) = &prototype.partial_ord {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        PartialOrd operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.partial_ord = Some(operator);
                            }

                            OperatorDeclaration::Ord(operator) => {
                                if let Some(previous) = &prototype.ord {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Ord operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.ord = Some(operator);
                            }

                            OperatorDeclaration::Hash(operator) => {
                                if let Some(previous) = &prototype.hash {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Hash operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.hash = Some(operator);
                            }

                            OperatorDeclaration::Invocation(operator) => {
                                if let Some(previous) = &prototype.invocation {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Invocation operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.invocation = Some(operator);
                            }

                            OperatorDeclaration::Binding(operator) => {
                                if let Some(previous) = &prototype.binding {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Binding operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.binding = Some(operator);
                            }

                            OperatorDeclaration::Add(operator) => {
                                if let Some(previous) = &prototype.add {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Add operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.add = Some(operator);
                            }

                            OperatorDeclaration::AddAssign(operator) => {
                                if let Some(previous) = &prototype.add_assign {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        AddAssign operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.add_assign = Some(operator);
                            }

                            OperatorDeclaration::Sub(operator) => {
                                if let Some(previous) = &prototype.sub {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Sub operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.sub = Some(operator);
                            }

                            OperatorDeclaration::SubAssign(operator) => {
                                if let Some(previous) = &prototype.sub_assign {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        SubAssign operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.sub_assign = Some(operator);
                            }

                            OperatorDeclaration::Mul(operator) => {
                                if let Some(previous) = &prototype.mul {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Mul operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.mul = Some(operator);
                            }

                            OperatorDeclaration::MulAssign(operator) => {
                                if let Some(previous) = &prototype.mul_assign {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        MulAssign operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.mul_assign = Some(operator);
                            }

                            OperatorDeclaration::Div(operator) => {
                                if let Some(previous) = &prototype.div {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Div operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.div = Some(operator);
                            }

                            OperatorDeclaration::DivAssign(operator) => {
                                if let Some(previous) = &prototype.div_assign {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        DivAssign operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.div_assign = Some(operator);
                            }

                            OperatorDeclaration::And(operator) => {
                                if let Some(previous) = &prototype.and {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        And operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.and = Some(operator);
                            }

                            OperatorDeclaration::Or(operator) => {
                                if let Some(previous) = &prototype.or {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Or operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.or = Some(operator);
                            }

                            OperatorDeclaration::Not(operator) => {
                                if let Some(previous) = &prototype.not {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Not operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.not = Some(operator);
                            }

                            OperatorDeclaration::Neg(operator) => {
                                if let Some(previous) = &prototype.neg {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Neg operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.neg = Some(operator);
                            }

                            OperatorDeclaration::BitAnd(operator) => {
                                if let Some(previous) = &prototype.bit_and {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        BitAnd operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.bit_and = Some(operator);
                            }

                            OperatorDeclaration::BitAndAssign(operator) => {
                                if let Some(previous) = &prototype.bit_and_assign {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        BitAndAssign operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.bit_and_assign = Some(operator);
                            }

                            OperatorDeclaration::BitOr(operator) => {
                                if let Some(previous) = &prototype.bit_or {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        BitOr operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.bit_or = Some(operator);
                            }

                            OperatorDeclaration::BitOrAssign(operator) => {
                                if let Some(previous) = &prototype.bit_or_assign {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        BitOrAssign operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.bit_or_assign = Some(operator);
                            }

                            OperatorDeclaration::BitXor(operator) => {
                                if let Some(previous) = &prototype.bit_xor {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        BitXor operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.bit_xor = Some(operator);
                            }

                            OperatorDeclaration::BitXorAssign(operator) => {
                                if let Some(previous) = &prototype.bit_xor_assign {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        BitXorAssign operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.bit_xor_assign = Some(operator);
                            }

                            OperatorDeclaration::Shl(operator) => {
                                if let Some(previous) = &prototype.shl {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Shl operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.shl = Some(operator);
                            }

                            OperatorDeclaration::ShlAssign(operator) => {
                                if let Some(previous) = &prototype.shl_assign {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        ShlAssign operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.shl_assign = Some(operator);
                            }

                            OperatorDeclaration::Shr(operator) => {
                                if let Some(previous) = &prototype.shr {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Shr operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.shr = Some(operator);
                            }

                            OperatorDeclaration::ShrAssign(operator) => {
                                if let Some(previous) = &prototype.shr_assign {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        ShrAssign operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.shr_assign = Some(operator);
                            }

                            OperatorDeclaration::Rem(operator) => {
                                if let Some(previous) = &prototype.rem {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        Rem operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.rem = Some(operator);
                            }

                            OperatorDeclaration::RemAssign(operator) => {
                                if let Some(previous) = &prototype.rem_assign {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        RemAssign operator declaration. The same \
                                        operator already declared in {previous}.",
                                    ))
                                }

                                prototype.rem_assign = Some(operator);
                            }

                            OperatorDeclaration::None(operator) => {
                                if let Some(previous) = &prototype.none {
                                    let previous = previous.origin;

                                    operator.origin.blame(&format!(
                                        "Duplicate {type_meta} \
                                        None marker declaration. The same \
                                        marker already declared in {previous}.",
                                    ))
                                }

                                prototype.none = Some(operator);
                            }
                        }
                    }
                }
            }

            PrototypeRegistry { prototypes }
        });

        REGISTRY.deref()
    }
}
