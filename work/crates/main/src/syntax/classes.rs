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

use std::collections::HashSet;

use compact_str::CompactString;
use lady_deirdre::{
    analysis::{Classifier, Grammar},
    sync::SyncBuildHasher,
    syntax::NodeRef,
};

use crate::syntax::{ScriptDoc, ScriptNode};

pub struct ScriptClassifier;

impl Classifier for ScriptClassifier {
    type Node = ScriptNode;
    type Class = ScriptClass;

    #[inline(always)]
    fn classify<S: SyncBuildHasher>(
        doc: &ScriptDoc,
        node_ref: &NodeRef,
    ) -> HashSet<Self::Class, S> {
        let mut classes = HashSet::default();

        let Some(node) = node_ref.deref(doc) else {
            return classes;
        };

        if node.is_scope() {
            let _ = classes.insert(ScriptClass::AllScopes);
        }

        match node {
            ScriptNode::Ident { token, .. } => {
                if let Some(string) = token.string(doc) {
                    let _ = classes.insert(ScriptClass::AllIdents);
                    let _ = classes.insert(ScriptClass::Ident(CompactString::from(string)));
                }
            }

            ScriptNode::This { .. } => {
                let _ = classes.insert(ScriptClass::AllThese);
            }

            ScriptNode::Crate { .. } => {
                let _ = classes.insert(ScriptClass::AllCrates);
            }

            ScriptNode::Field { token, .. } => {
                if let Some(string) = token.string(doc) {
                    let _ = classes.insert(ScriptClass::AllFields);
                    let _ = classes.insert(ScriptClass::Field(CompactString::from(string)));
                }
            }

            _ => (),
        }

        classes
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ScriptClass {
    AllScopes,
    AllIdents,
    AllThese,
    AllFields,
    AllCrates,
    Ident(CompactString),
    Field(CompactString),
}
