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

use std::fmt::{Debug, Formatter};

use ahash::RandomState;
use lady_deirdre::{
    analysis::{AnalysisResult, SemanticAccess, TaskHandle},
    sync::Shared,
    syntax::NodeRef,
};

use crate::{
    analysis::ModuleResultEx,
    interpret::{ScriptFn, Subroutines},
    report::system_panic,
    runtime::Cell,
    syntax::{ScriptDoc, ScriptNode},
};

impl Debug for ScriptFn {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.assembly
            .as_ref()
            .debug(formatter, 0, self.subroutines.as_ref())
    }
}

impl ScriptFn {
    pub(crate) fn compile<H: TaskHandle>(
        task: &impl SemanticAccess<ScriptNode, H, RandomState>,
        doc: &ScriptDoc,
        routine_ref: &NodeRef,
    ) -> AnalysisResult<Self> {
        let Some(routine_node) = routine_ref.deref(doc) else {
            return Ok(Self::default());
        };

        let compilation = routine_node.compilation().forward()?;

        let (_, assembly) = compilation.assembly.snapshot(task).forward()?;

        let mut closures = Vec::with_capacity(assembly.as_ref().closures);

        for _ in 0..assembly.as_ref().closures {
            closures.push(Cell::nil());
        }

        let Subroutines::Refs(subroutines) = &assembly.as_ref().subroutines else {
            system_panic!("Cannot compile static assembly.");
        };

        let mut compiled_subroutines = Vec::with_capacity(assembly.as_ref().subroutines.len());

        for subroutine_ref in subroutines {
            compiled_subroutines.push(Self::compile(task, doc, subroutine_ref)?);
        }

        Ok(Self {
            assembly,
            closures,
            subroutines: Shared::new(compiled_subroutines),
        })
    }
}
