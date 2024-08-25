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

use std::{marker::PhantomData, ops::Deref};

use ahash::{AHashMap, AHashSet, RandomState};
use compact_str::CompactString;
use lady_deirdre::{
    analysis::{AnalysisResult, SemanticAccess, Semantics, TaskHandle},
    arena::{Id, Identifiable},
    lexis::{Site, SiteSpan, SourceCode, TokenRef},
    syntax::{AbstractNode, NodeRef, PolyRef, SyntaxTree, Visitor},
    units::CompilationUnit,
};

use crate::{
    analysis::{
        symbols::{EntrySymbol, PackageSymbol, VarSymbol},
        Description,
        ModuleResultEx,
    },
    report::system_panic,
    runtime::{Ident, PackageMeta, ScriptIdent, TypeHint},
    semantics::{IdentSemantics, LocalNamespace, PackageSemantics, Tag},
    syntax::{ScriptDoc, ScriptNode},
};

pub(super) static PROMPT_STRING: &'static str = "__AD_ASTRA_COMPLETION";

/// A description object for code completion.
///
/// This object is returned by the
/// [completions](crate::analysis::ModuleWrite::completions) function, which
/// performs the actual estimation
#[derive(Clone)]
pub struct Completions {
    /// A globally unique identifier for the script module
    /// (same as [ScriptModule::id](crate::analysis::ScriptModule::id)).
    pub id: Id,

    /// An absolute index of the character within the script's source code.
    ///
    /// This value is the same as the one specified in the
    /// [completions](crate::analysis::ModuleWrite::completions) function.
    pub site: Site,

    /// A range of the source code text fragment that is supposed to be replaced
    /// by the completion candidate text.
    pub place: SiteSpan,

    /// A substring of the source code text that the completion algorithm
    /// attempted to complete. In most cases, this substring corresponds to the
    /// [place](Completions::span) replacement range.
    pub pattern: String,

    /// The type of language construct that the completion candidates attempt
    /// to complete.
    pub scope: CompletionScope,

    /// The list of completion candidates.
    pub items: Vec<CompletionItem>,
}

impl Identifiable for Completions {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl Completions {
    pub(super) fn analyze<H: TaskHandle>(
        id: Id,
        site: Site,
        task: &impl SemanticAccess<ScriptNode, H, RandomState>,
    ) -> AnalysisResult<Self> {
        let doc_read = task.read_doc(id).forward()?;
        let doc = doc_read.deref();

        let from = match site > 0 {
            true => site - 1,
            false => site,
        };

        let to = match site < doc.length() {
            true => site + 1,
            false => site,
        };

        let subtree = doc.cover(from..to);

        let mut prompt_analyzer = PromptAnalyzer {
            task,
            doc,
            site,
            result: Ok(None),
            _handle: PhantomData,
        };

        doc.traverse_subtree(&subtree, &mut prompt_analyzer);

        Ok(prompt_analyzer.result?.unwrap_or_else(|| Self {
            id,
            site,
            place: site..site,
            pattern: String::new(),
            scope: CompletionScope::Unknown,
            items: Vec::new(),
        }))
    }
}

/// A completion candidate within the [Completions] code-completion description
/// object.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CompletionItem {
    /// The completion text that is supposed to replace the text at the
    /// [place](Completions#structfield.place) in the source code.
    ///
    /// You can convert this object to an actual string using the
    /// `item.label.to_string()` function.
    pub label: Ident,

    /// Additional metadata about the completed item.
    pub desc: Description,
}

/// A type of language construct targeted by the code completion.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum CompletionScope {
    /// Indicates that code completion is not available at the specified
    /// location in the source code.
    Unknown,

    /// A code completion for a package inside an import statement:
    ///
    /// ```text
    /// use foo.bar.<completion site>;
    /// ```
    Import,

    /// A code completion inside an expression:
    ///
    /// ```text
    /// foo(bar, <completion site>)
    /// ```
    Expression,

    /// A code completion inside a statement within a block:
    ///
    /// ```text
    /// {
    ///     foo;
    ///     <completion site>
    ///     bar;
    /// }
    /// ```
    Statement,

    /// A code completion for an arm of a match statement:
    ///
    /// ```text
    /// match foo {
    ///     bar => {},
    ///     <completion site>
    ///     baz => (),
    /// }
    /// ```
    MatchArm,

    /// A code completion for a field access operator in an expression:
    ///
    /// ```text
    /// foo.bar.<completion site>
    /// ```
    Field,
}

struct PromptAnalyzer<'a, H: TaskHandle, T: SemanticAccess<ScriptNode, H, RandomState>> {
    task: &'a T,
    doc: &'a ScriptDoc,
    site: Site,
    result: AnalysisResult<Option<Completions>>,
    _handle: PhantomData<H>,
}

impl<'a, H: TaskHandle, T: SemanticAccess<ScriptNode, H, RandomState>> Visitor
    for PromptAnalyzer<'a, H, T>
{
    fn visit_token(&mut self, _token_ref: &TokenRef) {}

    fn enter_node(&mut self, node_ref: &NodeRef) -> bool {
        let Ok(None) = &self.result else {
            return false;
        };

        let Some(script_node) = node_ref.deref(self.doc) else {
            return true;
        };

        self.result = match script_node {
            ScriptNode::Package {
                token, semantics, ..
            } => self.analyze_package(token, semantics),

            ScriptNode::Ident {
                parent,
                token,
                semantics,
                ..
            } => self.analyze_ident(parent, token, semantics),

            ScriptNode::Field { parent, token, .. } => self.analyze_field(parent, token),
            _ => return true,
        };

        match &self.result {
            Ok(None) => {
                let Some(span) = script_node.span(self.doc) else {
                    return true;
                };

                if span.end < self.site || span.start > self.site {
                    return false;
                }

                true
            }
            _ => false,
        }
    }

    fn leave_node(&mut self, _node_ref: &NodeRef) {}
}

impl<'a, H: TaskHandle, T: SemanticAccess<ScriptNode, H, RandomState>> PromptAnalyzer<'a, H, T> {
    fn analyze_package(
        &self,
        token: &TokenRef,
        semantics: &Semantics<PackageSemantics>,
    ) -> AnalysisResult<Option<Completions>> {
        let Some(chunk) = token.chunk(self.doc) else {
            return Ok(None);
        };

        let Some(byte_index) = chunk.string.find(PROMPT_STRING) else {
            return Ok(None);
        };

        let pattern = &chunk.string[0..byte_index];

        let package_semantics = semantics.get().forward()?;

        let (_, resolution) = package_semantics
            .package_resolution
            .snapshot(self.task)
            .forward()?;

        let mut items = Vec::new();

        if let Some(parent) = resolution.parent {
            let parent_prototype = parent.ty().prototype();

            for component in parent_prototype.hint_all_components() {
                if !component.ty.is_package() {
                    continue;
                }

                items.push(CompletionItem {
                    label: Ident::Rust(component.name),
                    desc: Description::from_component(&component),
                });
            }
        };

        Ok(Some(Completions {
            id: self.doc.id(),
            site: self.site,
            place: chunk.start()..(chunk.end() - PROMPT_STRING.len()),
            pattern: String::from(pattern),
            scope: CompletionScope::Import,
            items,
        }))
    }

    fn analyze_ident(
        &self,
        parent: &NodeRef,
        token: &TokenRef,
        semantics: &Semantics<IdentSemantics>,
    ) -> AnalysisResult<Option<Completions>> {
        let Some(chunk) = token.chunk(self.doc) else {
            return Ok(None);
        };

        let Some(byte_index) = chunk.string.find(PROMPT_STRING) else {
            return Ok(None);
        };

        let pattern = &chunk.string[0..byte_index];

        let mut scope_attr = semantics.scope_attr().forward()?;

        let ident_semantics = semantics.get().forward()?;

        let (_, local_namespace) = ident_semantics.namespace.snapshot(self.task).forward()?;

        let mut accumulator = AHashMap::new();

        self.prompt_namespace(local_namespace.as_ref(), &mut accumulator)?;

        loop {
            let scope_ref = scope_attr.snapshot(self.task).forward()?.1.scope_ref;

            let Some(ScriptNode::Fn { semantics, .. }) = scope_ref.deref(self.doc) else {
                break;
            };

            scope_attr = semantics.scope_attr().forward()?;

            let fn_semantics = semantics.get().forward()?;

            let (_, fn_namespace) = fn_semantics.namespace.snapshot(self.task).forward()?;

            self.prompt_namespace(fn_namespace.as_ref(), &mut accumulator)?;
        }

        let mut scope = CompletionScope::Expression;

        if let Some(ScriptNode::Expr { parent, node, .. }) = parent.deref(self.doc) {
            match parent.deref(self.doc) {
                Some(ScriptNode::Clause { end, .. }) if end.is_nil() => {
                    scope = CompletionScope::Statement;
                }

                Some(ScriptNode::MatchArm { case, handler, .. })
                    if case == node && pattern.is_empty() && handler.is_nil() =>
                {
                    scope = CompletionScope::MatchArm;
                }

                _ => (),
            }
        }

        Ok(Some(Completions {
            id: self.doc.id(),
            site: self.site,
            place: chunk.start()..(chunk.end() - PROMPT_STRING.len()),
            pattern: String::from(pattern),
            scope,
            items: accumulator.into_values().collect(),
        }))
    }

    fn analyze_field(
        &self,
        parent: &NodeRef,
        token: &TokenRef,
    ) -> AnalysisResult<Option<Completions>> {
        let Some(chunk) = token.chunk(self.doc) else {
            return Ok(None);
        };

        let Some(byte_index) = chunk.string.find(PROMPT_STRING) else {
            return Ok(None);
        };

        let pattern = &chunk.string[0..byte_index];

        let mut items = Vec::new();

        loop {
            let Some(ScriptNode::Binary { left, .. }) = parent.deref(self.doc) else {
                break;
            };

            let Some(left_node) = left.deref(self.doc) else {
                break;
            };

            let (_, left_type_resolution) = left_node
                .type_resolution()
                .forward()?
                .snapshot(self.task)
                .forward()?;

            items = match &left_type_resolution.tag {
                Tag::Struct(struct_ref) => self.prompt_struct(&struct_ref)?,
                other => self.prompt_type(other)?,
            };

            break;
        }

        let place = chunk.start()..(chunk.end() - PROMPT_STRING.len());

        Ok(Some(Completions {
            id: self.doc.id(),
            site: self.site,
            place,
            pattern: String::from(pattern),
            scope: CompletionScope::Field,
            items,
        }))
    }

    fn prompt_namespace(
        &self,
        namespace: &LocalNamespace,
        accumulator: &mut AHashMap<CompactString, CompletionItem>,
    ) -> AnalysisResult<()> {
        for (key, value) in &namespace.map {
            if accumulator.contains_key(key) {
                continue;
            }

            let Some(decl_node) = value.as_ref().decl.deref(self.doc) else {
                continue;
            };

            let item = match decl_node {
                ScriptNode::Root { .. } => {
                    let Some(package) = PackageMeta::by_id(self.doc.id()) else {
                        system_panic!("Missing package.");
                    };

                    let Some(component) = package.ty().prototype().hint_component(key.as_str())
                    else {
                        continue;
                    };

                    CompletionItem {
                        label: Ident::Rust(component.name),
                        desc: Description::from_component(&component),
                    }
                }

                ScriptNode::Use { .. } => {
                    let Some(package_ref) = value.as_ref().defs.iter().next() else {
                        continue;
                    };

                    let Some(ScriptNode::Package { semantics, .. }) = package_ref.deref(self.doc)
                    else {
                        continue;
                    };

                    let package_semantics = semantics.get().forward()?;

                    let (_, package_resolution) = package_semantics
                        .package_resolution
                        .snapshot(self.task)
                        .forward()?;

                    let Some(package) = package_resolution.package else {
                        continue;
                    };

                    let Some(component) = package.ty().prototype().hint_component(key.as_str())
                    else {
                        continue;
                    };

                    CompletionItem {
                        label: Ident::Rust(component.name),
                        desc: Description {
                            type_hint: component.ty,
                            impl_symbol: PackageSymbol::from_package_ref(package_ref),
                            doc: component.doc.or(component.ty.doc()),
                        },
                    }
                }

                ScriptNode::For { .. } | ScriptNode::FnParams { .. } => {
                    let Some(var_ref) = value.as_ref().defs.iter().next() else {
                        continue;
                    };

                    let Some(ScriptNode::Var {
                        token, semantics, ..
                    }) = var_ref.deref(self.doc)
                    else {
                        continue;
                    };

                    let var_semantics = semantics.get().forward()?;

                    let (_, var_type_resolution) = var_semantics
                        .type_resolution
                        .snapshot(self.task)
                        .forward()?;

                    let ty = var_type_resolution.tag.type_hint();

                    CompletionItem {
                        label: Ident::Script(ScriptIdent::from_string(*token, key.clone())),
                        desc: Description {
                            type_hint: ty,
                            impl_symbol: VarSymbol::from_var_ref(var_ref),
                            doc: ty.doc(),
                        },
                    }
                }

                ScriptNode::Let { name, .. } => {
                    let Some(ScriptNode::Var { token, .. }) = name.deref(self.doc) else {
                        continue;
                    };

                    let mut resolution = Tag::Unset;

                    for def_ref in &value.as_ref().defs {
                        let Some(def_node) = def_ref.deref(self.doc) else {
                            continue;
                        };

                        let (_, def_type_resolution) = def_node
                            .type_resolution()
                            .forward()?
                            .snapshot(self.task)
                            .forward()?;

                        resolution.merge(def_type_resolution.tag);
                    }

                    if let Tag::Unset = &resolution {
                        resolution = Tag::dynamic();
                    }

                    let ty = resolution.type_hint();

                    CompletionItem {
                        label: Ident::Script(ScriptIdent::from_string(*token, key.clone())),
                        desc: Description {
                            type_hint: ty,
                            impl_symbol: VarSymbol::from_var_ref(name),
                            doc: ty.doc(),
                        },
                    }
                }

                _ => continue,
            };

            let _ = accumulator.insert(key.clone(), item);
        }

        Ok(())
    }

    fn prompt_struct(&self, struct_ref: &NodeRef) -> AnalysisResult<Vec<CompletionItem>> {
        let mut result = Vec::new();

        let Some(ScriptNode::Struct { semantics, .. }) = struct_ref.deref(self.doc) else {
            return Ok(result);
        };

        let struct_semantics = semantics.get().forward()?;

        let (_, struct_entries_vec) = struct_semantics
            .struct_entries_vec_syntax
            .snapshot(self.task)
            .forward()?;

        let capacity = struct_entries_vec.as_ref().vec.len();

        let mut visited = AHashSet::with_capacity(capacity);

        result.reserve(capacity);

        for (key, key_ref, value_ref) in &struct_entries_vec.as_ref().vec {
            if visited.contains(key) {
                continue;
            }

            let Some(ScriptNode::StructEntryKey { token, .. }) = key_ref.deref(self.doc) else {
                continue;
            };

            let mut ty = TypeHint::dynamic();

            if let Some(value_node) = value_ref.deref(self.doc) {
                let (_, value_type_resolution) = value_node
                    .type_resolution()
                    .forward()?
                    .snapshot(self.task)
                    .forward()?;

                ty = value_type_resolution.tag.type_hint();
            }

            result.push(CompletionItem {
                label: Ident::Script(ScriptIdent::from_string(*token, key.clone())),
                desc: Description {
                    type_hint: ty,
                    impl_symbol: EntrySymbol::from_struct_entry_key_ref(key_ref),
                    doc: ty.doc(),
                },
            });

            let _ = visited.insert(key);
        }

        Ok(result)
    }

    fn prompt_type(&self, tag: &Tag) -> AnalysisResult<Vec<CompletionItem>> {
        let mut result = Vec::new();

        let Some(ty) = tag.type_meta() else {
            return Ok(result);
        };

        result.reserve(ty.prototype().components_len());

        for component in ty.prototype().hint_all_components() {
            result.push(CompletionItem {
                label: Ident::Rust(component.name),
                desc: Description::from_component(&component),
            });
        }

        Ok(result)
    }
}
