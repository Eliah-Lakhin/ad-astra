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
    fmt::{Debug, Display, Formatter},
    ops::Deref,
};

use ahash::RandomState;
use lady_deirdre::{
    analysis::DocumentReadGuard,
    arena::{Id, Identifiable},
    syntax::SyntaxTree,
    units::Lexis,
};

use crate::{
    format::{format_script_doc, format_script_path, ScriptFormatConfig, ScriptSnippet},
    runtime::PackageMeta,
    syntax::{ScriptDoc, ScriptNode},
};

/// A view into the [ScriptModule](crate::analysis::ScriptModule) source code
/// text and lexis.
///
/// Created by the [ModuleRead::text](crate::analysis::ModuleRead::text)
/// function.
///
/// To fetch the raw text of the source code or a substring, use the
/// [ModuleText::substring](lady_deirdre::lexis::SourceCode::substring)
/// function.
///
/// To print a snippet of the source code text with syntax highlighting
/// and annotation messages to the terminal, use the [ModuleText::snippet]
/// function. The [Display] implementation of this object also prints
/// a highlighted snippet with default settings.
pub struct ModuleText<'a> {
    pub(super) package: &'static PackageMeta,
    pub(super) doc_read: DocumentReadGuard<'a, ScriptNode, RandomState>,
}

impl<'a> Identifiable for ModuleText<'a> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.doc_read.id()
    }
}

impl<'a> Lexis for ModuleText<'a> {
    type Lexis = ScriptDoc;

    #[inline(always)]
    fn lexis(&self) -> &Self::Lexis {
        self.doc_read.deref()
    }
}

impl<'a> Debug for ModuleText<'a> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_fmt(format_args!(
            "ModuleText({})",
            format_script_path(self.id(), Some(self.package))
        ))
    }
}

impl<'a> Display for ModuleText<'a> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.snippet().set_caption("module"), formatter)
    }
}

impl<'a> ModuleText<'a> {
    /// Returns an annotated snippet that prints the module's source code text with
    /// syntax highlighting and annotations for specific code fragments with
    /// string messages.
    ///
    /// ```
    /// # use ad_astra::{
    /// #     analysis::{ModuleRead, ScriptModule},
    /// #     export,
    /// #     lady_deirdre::{analysis::TriggerHandle, format::AnnotationPriority, lexis::Position},
    /// #     runtime::ScriptPackage,
    /// # };
    /// #
    /// # #[export(package)]
    /// # #[derive(Default)]
    /// # struct Package;
    /// #
    /// let module = ScriptModule::new(
    ///     Package::meta(),
    ///     r#"
    ///     let foo = 10;
    ///     let bar = foo + 20;
    /// "#,
    /// );
    ///
    /// module.rename("my_module.adastra");
    ///
    /// let handle = TriggerHandle::new();
    /// let module_read = module.read(&handle, 1).unwrap();
    /// let module_text = module_read.text();
    ///
    /// let mut snippet = module_text.snippet();
    ///
    /// snippet.annotate(
    ///     Position::new(2, 9)..Position::new(2, 12),
    ///     AnnotationPriority::Primary,
    ///     "Annotation text.",
    /// );
    ///
    /// println!("{snippet}");
    /// ```
    ///
    /// Outputs:
    /// ```text
    ///    ╭──╢ ‹doctest›.‹my_module.adastra› ╟────────────────────────────────────────╮
    ///  1 │                                                                           │
    ///  2 │     let foo = 10;                                                         │
    ///    │         ╰╴ Annotation text.                                               │
    ///  3 │     let bar = foo + 20;                                                   │
    ///  4 │                                                                           │
    ///    ╰───────────────────────────────────────────────────────────────────────────╯
    /// ```
    #[inline(always)]
    pub fn snippet(&self) -> ScriptSnippet {
        ScriptSnippet::from_doc(self.doc_read.deref())
    }

    /// Returns true if the script module does not have syntax errors.
    #[inline(always)]
    pub fn is_well_formed(&self) -> bool {
        self.doc_read.error_refs().next().is_none()
    }

    /// If the script module does not have syntax errors (i.e., the module is
    /// [well-formed](Self::is_well_formed)), returns the reformatted source
    /// code text according to the formatting rules.
    ///
    /// See [format_script_text](crate::format::format_script_text) for details.
    #[inline(always)]
    pub fn format(&self, config: ScriptFormatConfig) -> Option<String> {
        format_script_doc(config, self.doc_read.deref())
    }
}

/// An interface that provides access to script module texts by module [Id].
///
/// When displaying runtime errors (through the
/// [RuntimeError::display](crate::runtime::RuntimeError::display) function),
/// the inner algorithm needs access to the script module text of the modules
/// mentioned in the error's description. You can implement this trait on
/// your custom multi-module compiler to provide access to the module texts.
///
/// If your compiler consists of runtime-independent script modules, you can
/// use a normal [ModuleText], which implements ModuleTextResolver out of the
/// box.
pub trait ModuleTextResolver {
    /// If the underlying compiler contains a script module with `id`, returns
    /// [ModuleText] of this module. Otherwise, returns None.
    fn resolve(&self, id: Id) -> Option<&ModuleText>;
}

impl<'a> ModuleTextResolver for ModuleText<'a> {
    #[inline(always)]
    fn resolve(&self, id: Id) -> Option<&ModuleText> {
        if self.id() != id {
            return None;
        }

        Some(self)
    }
}
