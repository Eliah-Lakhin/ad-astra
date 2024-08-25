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

use std::{cell::UnsafeCell, mem::replace};

use crate::runtime::Cell;

const LIMIT: usize = 10_000;
const CAPACITY: usize = 1_000;

thread_local! {
    static STACK: UnsafeCell<Stack> = const { UnsafeCell::new(Stack::new()) };
}

pub(crate) type StackDepth = usize;

pub(super) struct Stack {
    cells: Vec<Cell>,
    max_depth: usize,
    min_capacity: usize,
    max_capacity: usize,
}

impl Stack {
    #[inline(always)]
    const fn new() -> Self {
        Self {
            cells: Vec::new(),
            max_depth: LIMIT,
            min_capacity: CAPACITY / 2,
            max_capacity: CAPACITY,
        }
    }

    #[inline(always)]
    pub(super) fn enter_frame(frame: StackDepth, arity: StackDepth) -> Option<StackDepth> {
        STACK.with(|stack| {
            // Safety: Access is localized.
            let stack = unsafe { &mut *stack.get() };

            let diff = frame.checked_sub(arity).unwrap_or_default();

            let depth = stack.cells.len();

            if depth + diff > stack.max_depth {
                return None;
            }

            stack.cells.reserve(diff);

            Some(depth.checked_sub(arity).unwrap_or_default())
        })
    }

    #[inline(always)]
    pub(super) fn leave_frame(begin: StackDepth) {
        STACK.with(|stack| {
            // Safety: Access is localized.
            let stack = unsafe { &mut *stack.get() };

            stack.cells.truncate(begin);

            if stack.cells.capacity() > stack.max_capacity {
                stack.cells.shrink_to(stack.min_capacity);
            }
        })
    }

    #[inline(always)]
    pub(super) fn push_nil() {
        STACK.with(move |stack| {
            // Safety: Access is localized.
            let stack = unsafe { &mut *stack.get() };

            stack.cells.push(Cell::nil());
        });
    }

    #[inline(always)]
    pub(super) fn push(cell: Cell) {
        STACK.with(move |stack| {
            // Safety: Access is localized.
            let stack = unsafe { &mut *stack.get() };

            stack.cells.push(cell);
        });
    }

    #[inline(always)]
    pub(super) fn pop_1(frame_begin: StackDepth) -> Cell {
        STACK.with(move |stack| {
            // Safety: Access is localized.
            let stack = unsafe { &mut *stack.get() };

            if stack.cells.len() <= frame_begin {
                return Cell::nil();
            }

            stack.cells.pop().unwrap_or(Cell::nil())
        })
    }

    #[inline(always)]
    pub(super) fn pop_2(frame_begin: StackDepth) -> (Cell, Cell) {
        STACK.with(move |stack| {
            // Safety: Access is localized.
            let stack = unsafe { &mut *stack.get() };

            let cell_2 = match stack.cells.len() <= frame_begin {
                true => Cell::nil(),
                false => stack.cells.pop().unwrap_or(Cell::nil()),
            };

            let cell_1 = match stack.cells.len() <= frame_begin {
                true => Cell::nil(),
                false => stack.cells.pop().unwrap_or(Cell::nil()),
            };

            (cell_1, cell_2)
        })
    }

    #[inline(always)]
    pub(super) fn pop_many(frame_begin: StackDepth, length: StackDepth) -> Vec<Cell> {
        STACK.with(move |stack| {
            // Safety: Access is localized.
            let stack = unsafe { &mut *stack.get() };

            let mut at = stack.cells.len().checked_sub(length).unwrap_or_default();

            if at < frame_begin {
                at = frame_begin;
            }

            if at > stack.cells.len() {
                return Vec::new();
            }

            stack.cells.split_off(at)
        })
    }

    #[inline(always)]
    pub(super) fn peek_1(frame_begin: StackDepth) -> Cell {
        STACK.with(move |stack| {
            // Safety: Access is localized.
            let stack = unsafe { &mut *stack.get() };

            if stack.cells.len() <= frame_begin {
                return Cell::nil();
            }

            let Some(last) = stack.cells.last() else {
                return Cell::nil();
            };

            last.clone()
        })
    }

    #[inline(always)]
    pub(super) fn lift(frame_begin: StackDepth, mut depth: StackDepth) {
        depth += frame_begin;

        STACK.with(move |stack| {
            // Safety: Access is localized.
            let stack = unsafe { &mut *stack.get() };

            let cell = match stack.cells.get_mut(depth) {
                Some(cell) => replace(cell, Cell::nil()),
                None => Cell::nil(),
            };

            stack.cells.push(cell);
        })
    }

    #[inline(always)]
    pub(super) fn swap(frame_begin: StackDepth, mut depth: StackDepth) {
        depth += frame_begin;

        STACK.with(move |stack| {
            // Safety: Access is localized.
            let stack = unsafe { &mut *stack.get() };

            if depth >= stack.cells.len() {
                return;
            }

            let last = stack.cells.len() - 1;

            stack.cells.swap(depth, last);
        })
    }

    #[inline(always)]
    pub(super) fn dup(frame_begin: StackDepth, mut depth: StackDepth) {
        depth += frame_begin;

        STACK.with(move |stack| {
            // Safety: Access is localized.
            let stack = unsafe { &mut *stack.get() };

            let cell = match stack.cells.get(depth) {
                Some(cell) => cell.clone(),
                None => Cell::nil(),
            };

            stack.cells.push(cell);
        })
    }

    #[inline(always)]
    pub(super) fn shrink(frame_begin: StackDepth, mut depth: StackDepth) {
        depth += frame_begin;

        STACK.with(move |stack| {
            // Safety: Access is localized.
            let stack = unsafe { &mut *stack.get() };

            stack.cells.truncate(depth)
        })
    }
}
