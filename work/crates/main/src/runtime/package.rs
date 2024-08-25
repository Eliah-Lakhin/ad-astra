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
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ops::Deref,
    sync::{RwLock, RwLockReadGuard},
};

use ahash::{AHashMap, AHashSet, RandomState};
use lady_deirdre::{
    arena::Id,
    sync::{Lazy, Table},
};
use semver::{Version, VersionReq};

use crate::{
    report::debug_unreachable,
    runtime::{
        Cell,
        RustOrigin,
        TypeMeta,
        __intrinsics::{DeclarationGroup, PackageDeclaration},
    },
};

/// A type that represents the Script Package of a crate.
///
/// This trait is automatically implemented on a struct type when you
/// export it as a crate package.
///
/// Through the [ScriptPackage::meta] function, you gain access to the
/// [PackageMeta] object. This can be used, for example, to instantiate new
/// [script modules](crate::analysis::ScriptModule) that can be analyzed in
/// accordance with the exported semantics of the crate or to run an LSP server.
///
/// ```
/// use ad_astra::{export, runtime::ScriptPackage};
///
/// #[export(package)]
/// #[derive(Default)]
/// struct Package;
///
/// assert_eq!(Package::meta().name(), "ad-astra");
/// ```
pub trait ScriptPackage {
    /// Returns a Rust source code location that points to where the package
    /// type was declared.
    ///
    /// This is a shortcut for [PackageMeta::origin].
    #[inline(always)]
    fn origin() -> &'static RustOrigin {
        Self::meta().origin()
    }

    /// Returns the name of the package's crate.
    ///
    /// This is a shortcut for [PackageMeta::name].
    #[inline(always)]
    fn name() -> &'static str {
        Self::meta().name()
    }

    /// Returns the version of the package's crate.
    ///
    /// This is a shortcut for [PackageMeta::version].
    #[inline(always)]
    fn version() -> &'static str {
        Self::meta().version()
    }

    /// Returns a reference to the full metadata object of the crate's package.
    fn meta() -> &'static PackageMeta;
}

/// Metadata for the [ScriptPackage].
///
/// You cannot instantiate this object manually; it is created automatically
/// by the Script Engine for each exported Script Package per crate. However,
/// you can obtain a static reference to the PackageMeta in several ways.
/// For instance, you can get it from the [ScriptPackage::meta] function of
/// the exported package struct. You can also manually find the reference
/// using the [PackageMeta::of] function.
///
/// ```
/// use ad_astra::{
///     export,
///     runtime::{PackageMeta, ScriptPackage},
/// };
///
/// #[export(package)]
/// #[derive(Default)]
/// struct Package;
///
/// let package_meta = Package::meta();
///
/// let same_package =
///     PackageMeta::of(package_meta.name(), &format!("={}", package_meta.version())).unwrap();
///
/// assert_eq!(package_meta, same_package);
/// ```
///
/// You can use this reference to instantiate
/// [script modules](crate::analysis::ScriptModule) or to run the LSP server.
///
/// The alternative [Debug] implementation for the package lists all script
/// modules currently associated with this Script Package. The alternative
/// [Display] implementation prints the canonical name of the package's crate:
/// `<package_name>@<package_version>`.
pub struct PackageMeta {
    origin: &'static RustOrigin,
    declaration: PackageDeclaration,
    modules: RwLock<AHashSet<Id>>,
}

impl PartialEq for PackageMeta {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.declaration.name.ne(other.declaration.name) {
            return false;
        }

        if self.declaration.version.ne(other.declaration.version) {
            return false;
        }

        true
    }
}

impl Eq for PackageMeta {}

impl Hash for PackageMeta {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.declaration.name.hash(state);
        self.declaration.version.hash(state);
    }
}

impl Ord for PackageMeta {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match self.declaration.name.cmp(other.declaration.name) {
            Ordering::Equal => self.declaration.version.cmp(other.declaration.version),
            other => other,
        }
    }
}

impl PartialOrd for PackageMeta {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Debug for PackageMeta {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let alternate = formatter.alternate();

        let mut debug_struct = formatter.debug_struct("PackageMeta");

        if alternate {
            debug_struct.field("origin", self.origin());
        }

        debug_struct
            .field("name", &self.name())
            .field("version", &self.version());

        if alternate {
            if let Ok(modules) = self.modules.try_read() {
                struct ListModules<'a> {
                    modules: RwLockReadGuard<'a, AHashSet<Id>>,
                }

                impl<'a> Debug for ListModules<'a> {
                    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                        let mut list = formatter.debug_list();

                        for id in self.modules.iter() {
                            let name = id.name();

                            match name.is_empty() {
                                true => list.entry(&format_args!("‹#{}›", id.into_inner())),
                                false => list.entry(&format_args!("‹{}›", name)),
                            };
                        }

                        list.finish()
                    }
                }

                let print_modules = ListModules { modules };

                debug_struct.field("modules", &print_modules);
            }

            let prototype = self.declaration.instance.ty().prototype();

            debug_struct.field("prototype", prototype);
        }

        debug_struct.finish()
    }
}

impl Display for PackageMeta {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())?;

        if formatter.alternate() {
            formatter.write_fmt(format_args!("@{}", self.version()))?;
        }

        Ok(())
    }
}

impl PackageMeta {
    #[inline(always)]
    fn new(origin: &'static RustOrigin, declaration: PackageDeclaration) -> Self {
        Self {
            origin,
            declaration,
            modules: RwLock::new(AHashSet::new()),
        }
    }

    /// Looks up the `PackageMeta` by the package's crate `name` and `version`.
    ///
    /// The `name` should match the exact crate name specified in the crate's
    /// `Cargo.toml` configuration.
    ///
    /// The `version` specifies the crate version requirement. There can be
    /// multiple crates with the same name but different versions in the crate
    /// dependency graph. The format of the `version` string is the same as used
    /// in the `[dependencies]` section of `Cargo.toml`. For example, you can
    /// specify the version as `3.0` to match the latest minor version, or use
    /// the equality sign `=2.1.5` to match a specific version exactly.
    ///
    /// The function returns None if there are no crates with the specified
    /// name and version requirements or if the crates do not have an exported
    /// Script Package.
    pub fn of(name: &str, version: &str) -> Option<&'static Self> {
        let registry = PackageRegistry::get();

        let version_set = registry.index.get(name)?;

        let requirement = VersionReq::parse(version).ok()?;

        let mut candidate: Option<(&Version, &PackageMeta)> = None;

        for (version, variant) in version_set {
            if !requirement.matches(version) {
                continue;
            }

            match candidate {
                Some((previous, _)) if previous > version => (),
                _ => candidate = Some((version, variant)),
            }
        }

        let (_, meta) = candidate?;

        Some(meta)
    }

    #[inline(always)]
    pub(crate) fn by_id(id: Id) -> Option<&'static Self> {
        let registry = ModuleRegistry::get();

        Some(*registry.index.get(&id)?)
    }

    /// Returns the Rust source code location that points to where the
    /// package type was declared.
    #[inline(always)]
    pub fn origin(&self) -> &'static RustOrigin {
        self.origin
    }

    /// Returns the name of the crate for this package, as specified in the
    /// crate's `Cargo.toml`.
    #[inline(always)]
    pub fn name(&self) -> &'static str {
        self.declaration.name
    }

    /// Returns the version of the crate for this package, as specified in the
    /// crate's `Cargo.toml`.
    #[inline(always)]
    pub fn version(&self) -> &'static str {
        self.declaration.version
    }

    /// Returns the documentation URL of the crate for this package, as
    /// specified in the crate's `Cargo.toml`.
    #[inline(always)]
    pub fn doc(&self) -> Option<&'static str> {
        self.declaration.doc
    }

    /// Returns the type metadata of the Rust struct that has been exported
    /// as a [ScriptPackage].
    #[inline(always)]
    pub fn ty(&self) -> &'static TypeMeta {
        self.declaration.instance.deref().ty()
    }

    /// Returns a smart pointer to the instance of the Rust struct that
    /// represents the [ScriptPackage].
    ///
    /// The Script Engine automatically instantiates each package struct type
    /// during initialization, using the [Default] constructor of the Rust
    /// struct.
    ///
    /// Through this instance, you can access the exported fields and methods of
    /// the struct. The crate's exported global functions and statics are also
    /// available as [components](crate::runtime::Object::component) of this
    /// type. Additionally, dependency crates (that have exported ScriptPackage)
    /// become components of this instance.
    ///
    /// The script code can access this instance using the `crate` script
    /// keyword or by referencing dependent crates
    /// (`my_crate.dep_crate` or `crate.dep_crate`).
    #[inline(always)]
    pub fn instance(&self) -> Cell {
        self.declaration.instance.deref().clone()
    }

    // Safety: `id` is not registered anywhere.
    #[inline(always)]
    pub(crate) unsafe fn attach_module(&'static self, id: Id) {
        let mut modules = self
            .modules
            .write()
            .unwrap_or_else(|poison| poison.into_inner());

        if !modules.insert(id) {
            // Safety: Upheld by the caller.
            unsafe { debug_unreachable!("Duplicate module.") }
        }

        let registry = ModuleRegistry::get();

        if registry.index.insert(id, self).is_some() {
            // Safety: Upheld by the caller.
            unsafe { debug_unreachable!("Duplicate module.") }
        }
    }

    // Safety: `id` exists in this Package.
    #[inline(always)]
    pub(crate) unsafe fn detach_module(&'static self, id: Id) {
        let mut modules = self
            .modules
            .write()
            .unwrap_or_else(|poison| poison.into_inner());

        if !modules.remove(&id) {
            // Safety: Upheld by the caller.
            unsafe { debug_unreachable!("Missing module.") }
        }

        let registry = ModuleRegistry::get();

        if registry.index.remove(&id).is_none() {
            // Safety: Upheld by the caller.
            unsafe { debug_unreachable!("Missing module.") }
        }
    }
}

struct PackageRegistry {
    index: AHashMap<&'static str, AHashMap<Version, PackageMeta>>,
}

impl PackageRegistry {
    #[inline(always)]
    fn get() -> &'static Self {
        static REGISTRY: Lazy<PackageRegistry> = Lazy::new(|| {
            let mut index = AHashMap::<&'static str, AHashMap<Version, PackageMeta>>::new();

            for group in DeclarationGroup::enumerate() {
                let origin = group.origin;

                for declaration in &group.packages {
                    let declaration = declaration();

                    let version_set = index.entry(declaration.name).or_default();

                    let version = match Version::parse(declaration.version) {
                        Ok(version) => version,

                        Err(error) => {
                            let name = declaration.name;
                            let version = &declaration.version;

                            origin.blame(&format!(
                                "Package {name}@{version} version parse error. {error}",
                            ))
                        }
                    };

                    if let Some(previous) = version_set.get(&version) {
                        let name = declaration.name;
                        let version = &declaration.version;
                        let previous = previous.origin;

                        origin.blame(&format!(
                            "Package {name}@{version} already declared in {previous}.",
                        ))
                    }

                    let meta = PackageMeta::new(origin, declaration);

                    if let Some(_) = version_set.insert(version, meta) {
                        // Safety: Uniqueness checked above.
                        unsafe { debug_unreachable!("Duplicate package entry.") }
                    }
                }
            }

            PackageRegistry { index }
        });

        REGISTRY.deref()
    }
}

struct ModuleRegistry {
    index: Table<Id, &'static PackageMeta, RandomState>,
}

impl ModuleRegistry {
    fn get() -> &'static Self {
        static REGISTRY: Lazy<ModuleRegistry> = Lazy::new(|| ModuleRegistry {
            index: Table::new(),
        });

        &REGISTRY
    }
}
