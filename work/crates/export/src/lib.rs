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

//TODO check warnings regularly
#![allow(warnings)]

//! # Ad Astra Macros Crate
//!
//! This is a helper crate for the [main crate](https://docs.rs/ad-astra/latest/ad_astra/)
//! of Ad Astra, an embeddable scripting programming language platform.
//!
//! The [export] attribute macro in this crate performs introspection of Rust
//! module items and exports the introspected metadata into the Ad Astra
//! script engine, allowing script users to interact with Rust APIs from their
//! scripts.
//!
//! ## Quick Links
//!
//! - [GitHub Repository](https://github.com/Eliah-Lakhin/ad-astra)
//! - [API Documentation](https://docs.rs/ad-astra)
//! - [Main Crate](https://crates.io/crates/ad-astra)
//! - [Guide Book](https://ad-astra.lakhin.com)
//! - [Examples](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples)
//! - [Playground](https://ad-astra.lakhin.com/playground.html)
//!
//! ## Copyright
//!
//! This work is proprietary software with source-available code.
//!
//! To copy, use, distribute, or contribute to this work, you must agree to the
//! terms and conditions of the
//! [General License Agreement](https://github.com/Eliah-Lakhin/ad-astra/blob/master/EULA.md).
//!
//! For an explanation of the licensing terms, see the
//! [F.A.Q.](https://github.com/Eliah-Lakhin/ad-astra/tree/master/FAQ.md)
//!
//! Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин). All rights reserved.

mod export;
mod utils;

use proc_macro::TokenStream;
use quote::quote_spanned;
use syn::{parse_macro_input, spanned::Spanned};

use crate::export::ExportItem;

//todo prevent export of non-Rust reprs.

/// Exports Rust code to the Script Runtime.
///
/// The `Export` macro inspects the Rust code of the item it is applied to and
/// exports this metadata into the Script Engine, making the Rust API accessible
/// from the script environment.
///
/// The macro offers numerous configuration options but often works without
/// additional setup.
///
/// ```
/// # use ad_astra_export::export;
/// #
/// #[export]
/// #[derive(Clone, Debug)]
/// pub struct Vector {
///     pub x: f32,
///     pub y: f32,
/// }
///
/// #[export]
/// impl Vector {
///     pub fn new(x: f32, y: f32) -> Self {
///         Self { x, y }
///     }
///
///     pub fn length(&self) -> f32 {
///         (self.x * self.x + self.y * self.y).sqrt()
///     }
///
///     pub fn normalize(&mut self) -> &mut Self {
///         let len = self.length();
///
///         self.x /= len;
///         self.y /= len;
///
///         self
///     }
/// }
/// ```
///
/// ## Exportable Items
///
/// The macro can be applied to the following Rust source code items:
///
/// - Struct declarations: `struct Foo {}`.
/// - Static and constant values: `static FOO: usize = 10;`.
/// - Crate functions: `fn foo() {}`.
/// - Implementation blocks for types: `impl Foo {}`.
/// - Implementation blocks for traits: `impl Trait for Foo {}`.
/// - Trait declarations: `trait Foo {}`.
///
/// ## Export Options
///
/// The macro has default exporting rules that can be reconfigured using the
/// macro parameter and additional `#[export(...)]` attributes within the
/// introspected Rust code.
///
/// ```
/// # use ad_astra_export::export;
/// #
/// #[export]
/// pub struct Vector {
///     pub x: f32,
///
///     // By default, the macro exports only public fields, but you can enforce
///     // the export of private fields as well.
///     #[export]
///     y: f32,
/// }
///
/// // By default, the macro exports a function using its original Rust name,
/// // but you can rename it so that this function will be referred to as
/// // `num_sum` in scripts.
/// #[export(name "num_sum")]
/// pub fn sum(a: usize, b: usize) -> usize {
///     a + b
/// }
///
/// // You can specify several configuration options per introspected item.
/// // The order of the `#[export(...)]` attributes does not matter.
/// #[export(shallow)]
/// #[export(name "Bar")]
/// struct Foo;
/// ```
///
/// ## Structs Exporting
///
/// When applied to the `struct Foo {}` item, the macro exports the type, making
/// its public fields accessible from scripts for both reading and writing.
///
/// - If a field is not public, you can enforce its export using the `#[export]`
///   or `#[export(include)]` attributes.
/// - You can exclude a public field from exporting using the
///   `#[export(exclude)]` attribute.
/// - You can rename the struct and any of its fields using the
///   `#[export(name <name expression>)]` attribute.
/// - You can restrict access to a field by specifying the `#[export(readonly)]`
///   or `#[export(writeonly)]` attributes.
/// - By default, the macro also exports the `#[derive(...)]` specification of
///   the type. Therefore, the derive attribute must be placed after the
///   `#[export]` attribute for the macro introspection system to recognize the
///   derives.
///
/// ```
/// # use ad_astra_export::export;
/// #
/// #[export]
/// // The export system can export standard Rust derives.
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// pub struct Vector {
///     // This field will be available only for reading from scripts
///     #[export(readonly)]
///     pub x: u8,
///
///     // Enforces exporting of a private field.
///     #[export]
///     y: u8,
///
///     // This field will be accessible from scripts as `foo.z_field`.
///     #[export(name "z_field")]
///     z: u8,
///
///     // This field will not be accessible from scripts.
///     #[export(exclude)]
///     pub w: u8,
/// }
/// ```
///
/// Note that the macro exports Rust items regardless of their access level.
/// In the example above, the `Vector` type will be exported even if it is a
/// private type.
///
/// ## Exported Types
///
/// Every type exported from Rust to the script environment, regardless of the
/// crate from which it is exported, is considered a Script Type.
///
/// The struct fields that you export must also be Script Types.
///
/// Out of the box, Ad Astra exports a small subset of Rust standard types,
/// including all numeric primitive types, the `[str]` type, the `[bool]` type,
/// and the `[unit]` `()` type.
///
/// Overall, the system of Script Types defines the structure of the script
/// environment's domain.
///
/// ## Type Casting
///
/// Ad Astra provides a mechanism for casting between Script Types.
///
/// Downcasting is the process of converting script data into Rust data.
/// Upcasting is the reverse process of converting Rust data into script data.
///
/// Specifically, through this mechanism, Ad Astra automatically casts between
/// primitive numeric types, allowing a script user to pass a `u32` value to a
/// function that accepts an `f64` argument.
///
/// Type casting is an advanced topic beyond typical exporting use cases. You
/// can find more information on this topic in the `runtime` module
/// documentation of the main crate.
///
/// For the purpose of exporting, you should know that the export macro
/// automatically makes all exported struct types transparently downcasted
/// and upcasted (meaning that the struct data is used in scripts as-is, without
/// any type casting).
///
/// Ad Astra also automatically supports upcasting and downcasting for some
/// standard Rust types, including [Option], [Result],
/// [BTreeMap](std::collections::BTreeMap), [Box], [Cow](std::borrow::Cow),
/// [String], standard range types, tuples, static arrays, and slices.
/// Primitive types (numbers, boolean, unit, and str) are also casted types.
///
/// When exporting a struct, all of its exported fields must be Script Types.
/// Therefore, you cannot export a field like `field: Option<i32>`, because
/// Option is a casted type, but it is not a Script Type by itself.
///
/// However, when exporting a function or method, the parameter and result type
/// must be casted types, but they do not need to be Script Types. For example,
/// you can use `Option<i32>` as a function's parameter or return type.
///
/// ## Package Exporting
///
/// To export Rust items for access in scripts from a crate, you need to declare
/// and export a Script Package.
///
/// Script Packages are regular Rust structs that must implement the [Default]
/// constructor.
///
/// You can declare only one Script Package per crate. Typically, you would
/// declare the package in the `lib.rs` or `main.rs` file of your crate.
///
/// ```
/// # use ad_astra_export::export;
/// #
/// #[export(package)]
/// #[derive(Default)]
/// pub struct Package;
/// ```
///
/// To declare a package, use the `#[export(package)]` attribute.
///
/// The name and access level of the struct do not matter for the exporting
/// system. Packages are just regular structs that may have exported fields,
/// methods, and additional trait and operator implementations. The only
/// requirements are that there must be no more than one Script Package
/// declaration per crate, and the struct type must implement the [Default] trait.
///
/// The Script Engine interprets script source code semantics on behalf of
/// the Package.
///
/// ## Functions Exporting
///
/// You can export crate-global functions using the Export macro. All such
/// functions must have unique exported names within the crate, regardless of
/// the Rust module from which they are exported. In scripts, these functions
/// are accessed by their exported names.
///
/// ```
/// # use ad_astra_export::export;
/// #
/// // In scripts, this function can be called as `foo()`.
/// #[export]
/// fn foo() {}
///
/// mod bar {
/// #    use ad_astra_export::export;
/// #
///     // In scripts, this function can be called as `foo_from_bar()`.
///     #[export(name "foo_from_bar")]
///     fn foo() {}
/// }
/// ```
///
/// An exported function may have parameters and a return type. The parameter
/// types must be downcasted types, and the return type must be an upcasted
/// type.
///
/// ```
/// # use ad_astra_export::export;
/// #
/// #[export]
/// fn foo(
///     arg_1: &[usize],
///     arg_2: Option<f64>,
///     arg_3: &bool,
///     arg_4: &str,
///     arg_5: (i32, i64, String),
/// ) -> Box<Option<i16>> {
///     todo!()
/// }
/// ```
///
/// The export system supports reference types (e.g., `&str` or `&mut f64`) in
/// parameter and return positions, with elided lifetimes. Explicit lifetimes
/// are not supported.
///
/// You can also use callback functions as parameters and return types.
///
/// Callback functions must be boxed functions with up to 7 upcasted arguments
/// that return a `RuntimeResult` containing a downcasted value.
///
/// ```ignore
/// #[export]
/// fn foo(
///     arg_1: usize,
///     arg_2: Box<dyn Fn(usize, f32) -> RuntimeResult<String> + Send + Sync>,
/// ) -> RuntimeResult<String> {
///     arg_2(arg_1, 10.5)
/// }
/// ```
///
/// These requirements arise because the exported function may be called from
/// scripts, which might pass a callback that is typically a script-defined
/// function.
///
/// Script functions are subject to runtime errors, which should not be ignored
/// by the exported Rust code.
///
/// To simplify callback signatures, you can use one of the helper aliases from
/// the `runtime::ops` module of the main crate.
///
/// The above code could be rewritten as follows:
///
/// ```ignore
/// use ad_astra::runtime::ops::Fn2;
///
/// #[export]
/// fn foo(arg_1: usize, arg_2: Fn2<usize, f32, String>) -> RuntimeResult<String> {
///     arg_2(arg_1, 10.5)
/// }
/// ```
///
/// ## Implementation Blocks Exporting
///
/// When you apply the Export macro to a type's implementation block (`impl`),
/// the macro exports all associated constants and functions without a receiver
/// as global items. Therefore, these names must be unique across the crate,
/// and you may need to rename them accordingly.
///
/// Associated functions with a receiver (methods) are bound to the underlying
/// type. Their names only need to be unique within the type's namespace.
///
/// By default, the macro exports only public members. If you want to export a
/// private member, you must explicitly annotate it with the `#[export(...)]`
/// attribute.
///
/// ```
/// # use ad_astra_export::export;
/// #
/// #[export]
/// struct Foo;
///
/// #[export]
/// impl Foo {
///     // A private field. To export it, specify an `#[export]` annotation.
///     //
///     // An associated constant will be exported into the global namespace of
///     // the Script Package, so you should rename this constant if necessary.
///     #[export(name "FOO_X")]
///     const X: usize = 10;
///
///     // A function without a receiver will also be exported into the namespace
///     // of the Script Package. As such, you should rename it if needed.
///     //
///     // In scripts, this function will be available as `new_foo()`.
///     #[export(name "new_foo")]
///     pub fn new() -> Self {
///         Self
///     }
///
///     // A function with a receiver (a method of the Foo object).
///     //
///     // These functions will be associated with the Foo namespace and usually
///     // don't need to be renamed.
///     //
///     // In scripts, this function will be available as `foo.get()`.
///     //
///     // Note, however, that the struct's namespace includes field names.
///     // Therefore, if the "Foo" struct has a "get" field, you should rename
///     // either that field or this method.
///     pub fn get(&self) -> &Self {
///         self
///     }
/// }
/// ```
///
/// ## Components Exporting
///
/// Type components are the members associated with a type. For structs, these
/// can include struct fields and methods (functions with a "self" receiver).
///
/// The `export` macro allows you to define new type components that don't have
/// a direct representation in the Rust API but will be treated as type
/// components in scripts.
///
/// ```ignore
/// #[export]
/// struct Foo;
///
/// #[export]
/// impl Foo {
///     #[export(component usize)]
///     fn num_ten(origin: Origin, arg: Arg) -> RuntimeResult<Cell> {
///         Cell::give(origin, 10)
///     }
/// }
/// ```
///
/// In scripts, the expression `foo.num_ten` would return the number `10`.
///
/// With custom component implementations, you have full control over what
/// happens when the script's code attempts to access a field with the specified
/// name on an instance of the type.
///
/// Custom component implementation is an advanced exporting topic that requires
/// a deep understanding of the `runtime` module's API in the main crate.
///
/// Specifically, the type specified in the `#[export(component <type>)]`
/// attribute describes the field's type. The purpose of this specification is
/// to assist the static script code analyzer in determining the script type of
/// the field. However, the actual data returned from the function during script
/// evaluation does not necessarily need to be of this type (because scripts are
/// dynamically typed).
///
/// The function's return value is a `Cell` instance representing the script
/// data returned to the script. The function can also return a runtime error.
///
/// The `origin` parameter specifies a script source code range that points to
/// the accessed field in the script code. The `arg` parameter specifies an
/// instance of the "Foo" object from which this field has been accessed
/// (essentially, the "self" receiver).
///
/// ## Parametric Polymorphism
///
/// In general, all Script Types are monomorphic concrete Rust types. The Script
/// Engine does not have a concept of "type generics".
///
/// However, the macro provides support for exporting generic types and
/// functions through the automatic monomorphism of generic types. This feature
/// is recommended for limited use, as the monomorphization process complicates
/// both Rust compilation and script interpretation.
///
/// In Rust code, when you have a generic parameter type or a constant generic
/// parameter, you should manually enumerate each possible variant of the
/// parameter intended for exporting.
///
/// ```
/// # use ad_astra_export::export;
/// #
/// // The macro will export `Foo<usize>` and `Foo<f32>` structs.
/// #[export]
/// struct Foo<#[export(type usize, f32)] T> {
///     pub field: T,
/// }
///
/// // The macro will export `bar::<1, bool>`, `bar::<2, bool>`, and
/// // `bar::<3, bool>` functions.
/// #[export]
/// fn bar<#[export(const 1..=3)] const N: usize, #[export(type bool)] T>(x: [T; N]) {}
/// ```
///
/// ## Advanced Renaming
///
/// When exporting Rust code with generics, such as the `fn bar` function in the
/// example above, the macro exports each function with the same base name,
/// like "bar". To avoid conflicts in script names, these exports must be
/// renamed using the `#[export(name <name_expr>)]` attribute.
///
/// The `<name_expr>` format consists of components separated by spaces.
/// The final name string is a concatenation of these component
/// stringifications.
///
/// For instance, `#[export(name "foo_" Expr:Lower[N] "_" Type:Upper[T])]` is a
/// concatenation of the string "foo_", the lower-cased expression "N", the
/// string "_", and the upper-cased type "T". Expressions and types will be
/// interpreted according to the monomorphized specializations of the item.
///
/// ```
/// # use ad_astra_export::export;
/// #
/// # #[export]
/// # struct Foo<#[export(type usize, f32)] T> {
/// #     pub field: T,
/// # }
/// #
/// // `bar::<1, bool>` will be exported as "foo_1_BOOL".
/// // `bar::<2, bool>` will be exported as "foo_2_BOOL".
/// // `bar::<3, bool>` will be exported as "foo_3_BOOL".
/// #[export(name "foo_" Expr:Lower[N] "_" Type:Upper[T])]
/// fn bar<#[export(const 1..=3)] const N: usize, #[export(type bool)] T>(x: [T; N]) {}
/// ```
///
/// The following components can be used in a name format:
///
/// - Any Rust literal, such as a string literal (`"foo_"`), number, boolean
///   value, etc. These values will be stringified as they are.
/// - `Expr:<case>[<rust expression>]`: const expression component. This can be
///   a const generic.
/// - `Type:<case>[<rust type>]`: type component. This can include any Rust
///   type, including generics and the `Self` type in `impl` blocks (with "Self"
///   expanded to the fully specialized implementation type).
/// - `Arg:<case>[<arg name>]`: type of a function argument, where `<arg name>`
///   is the name of any function argument. This component is applicable only to
///   named functions.
/// - `Ret:<case>`: type of the function return value. This component is
///   applicable only to named functions.
///
/// The `<case>` part of the component specifies the final string transformation
/// and can be one of the following:
///
/// - `Upper`: "FOO BAR". Uppercase transformation
/// - `Lower`: "foo bar". Lowercase transformation
/// - `UpperCamel`: "FooBar". All words concatenated in lower case with the
///   leading character of each word in uppercase.
/// - `Camel`: "fooBar". Same as UpperCamel, but the first word is in lowercase.
/// - `Snake`: "foo_bar". All words are lowercased and separated by the "_"
///   character.
/// - `UpperSnake`: "FOO_BAR". All words are uppercased and separated by the "_"
///   character.
/// - `Kebab`: "foo-bar". All words are lowercased and separated by the "-"
///   character.
/// - `UpperKebab`: "FOO-BAR". All words are uppercased and separated by the "-"
///   character.
/// - `Train`: "Foo-Bar". Similar to Kebab, but each leading character of the
///   word is uppercase.
/// - `Flat`: "foobar". Concatenates all words together in lowercase without a
///   separator.
/// - `UpperFlat`: "FOOBAR". Similar to Flat, but the entire string is in
///   uppercase.
/// - `Title`: "Foo to Bar". All words are in lowercase with the leading
///   character uppercase. The capitalization of common prepositions and
///   conjunctions will not be affected. The words are separated by spaces.
///
/// Note that Script Type names cannot be directly referred to in scripts.
/// The Script Engine identifies Script Types by their
/// [TypeId](std::any::TypeId), and their names serve as end-user-facing strings
/// to help users visually distinguish between types.
///
/// As such, the macro allows any strings for Script Type names. However,
/// exported functions, constants, and similar objects are referable in scripts
/// and must conform to Ad Astra's identifier rules, consisting only of ASCII
/// alphabetic, numeric, and underscore `_` characters. The macro sanitizes the
/// name format components accordingly: characters that do not meet these
/// requirements are replaced by an underscore.
///
/// For example, `Type::Lower[Foo<Bar, Baz>]` will be sanitized to
/// "foo_bar_baz".
///
/// ## Traits Exporting
///
/// The Script Engine does not have a concept of traits. When you export a
/// trait, the macro exports all of its members (e.g., functions) as if they
/// were exported on behalf of the types that implement this trait.
///
/// Therefore, when exporting a trait, you must enumerate all the corresponding
/// Rust types that implement this trait. The general exporting rules applicable
/// to `impl` blocks also apply to trait members, except that the Export macro
/// assumes these members are public by default (and thus will be exported
/// unless you exclude a member using the `#[export(exclude)]` attribute).
///
/// ```
/// # use ad_astra_export::export;
/// #
/// #[export]
/// struct Foo;
///
/// // The trait members will be exported for the type `Foo` that implements this trait.
/// // You can enumerate more types separated by commas: `#[export(type Foo, Bar)]`.
/// #[export(type Foo)]
/// trait MyTrait {
///     // This method will be exported for each enumerated type.
///     // The macro considers each trait member exportable by default
///     // (in contrast to normal `impl` blocks, where only public members
///     // are exported by default).
///     fn method_1(&self);
///
///     // This method will also be exported as "method_2_of_foo".
///     // Trait methods can be renamed as usual.
///     #[export(name "method_2_of_" Type:Snake[Self])]
///     fn method_2(&self) {
///         todo!()
///     }
///
///     // This method will not be exported as it is explicitly excluded.
///     #[export(exclude)]
///     fn method_3(&self);
/// }
///
/// impl MyTrait for Foo {
///     fn method_1(&self) {
///         todo!()
///     }
///
///     fn method_3(&self) {
///         todo!()
///     }
/// }
/// ```
///
/// ## Trait Implementations Exporting
///
/// Alternatively, you can export an implementation of a trait for a type.
///
/// ```
/// # use ad_astra_export::export;
/// #
/// #[export]
/// struct Foo;
///
/// trait MyTrait {
///     fn method_1(&self);
///
///     fn method_2(&self) {
///         todo!()
///     }
///
///     fn method_3(&self);
/// }
///
/// #[export]
/// impl MyTrait for Foo {
///     fn method_1(&self) {
///         todo!()
///     }
///
///     #[export(exclude)]
///     fn method_3(&self) {
///         todo!()
///     }
/// }
/// ```
///
/// Note that in the above code, only the `<Foo as MyTrait>::method_1` function
/// will be exported.
///
/// The macro will ignore "method_3" because it is explicitly excluded.
/// Additionally, the macro will ignore "method_2" because the introspection
/// system cannot recognize default members of the trait.
///
/// ## Operators Exporting
///
/// When exporting trait implementation blocks, the macro has special handling
/// for some built-in traits associated with Script operators.
///
/// Specifically, the macro supports most traits from the [std::ops] module,
/// such as [std::ops::Add] and [std::ops::Shl], as implementations of script
/// operators ("+" and "<<" operators in these cases). Additionally, all
/// standard Rust derivable traits will also be treated as Script Type
/// operators, as well as special low-level Script Engine exporting traits from
/// the `runtime::ops` module of the main crate.
///
/// ```
/// # use ad_astra_export::export;
/// #
/// use std::ops::Add;
///
/// #[export]
/// struct Foo;
///
/// // Exports the script-cloning operator: `*foo`.
/// #[export]
/// impl Clone for Foo {
///     fn clone(&self) -> Self {
///         todo!()
///     }
/// }
///
/// // Exports the script-equality operator: `foo_1 == foo_2`.
/// #[export]
/// impl PartialEq for Foo {
///     fn eq(&self, other: &Self) -> bool {
///         todo!()
///     }
/// }
///
/// // Exports the script-addition operator: `foo_1 + foo_2`.
/// #[export]
/// impl Add for Foo {
///     type Output = Foo;
///
///     fn add(self, rhs: Self) -> Self::Output {
///         todo!()
///     }
/// }
/// ```
///
/// ## Type Aliases Exporting
///
/// When you export a Rust struct type, the export system registers this type as
/// a Script Type. Additionally, the macro implements type casting for this
/// type, exports struct fields, and implements other useful operators
/// (e.g., a script-assignment operator).
///
/// If you want to bypass all automatic exports and implementations, or if you
/// want to export a type that is not a Rust struct type, you can export a type
/// alias item instead. In this case, the export system will only register the
/// aliased type as a Script Type, and you will have the opportunity to manually
/// implement all necessary features of this Script Type.
///
/// ```ignore
/// #[derive(Clone)]
/// enum Foo {
///     Var1,
///     Var2,
/// }
///
/// #[export]
/// type FooAlias = Foo;
///
/// #[export]
/// impl ScriptClone for Foo {}
///
/// impl<'a> Upcast<'a> for Foo {
///     type Output = ();
///
///     fn upcast(origin: Origin, this: Self) -> RuntimeResult<Self::Output> {
///         todo!()
///     }
///
///     fn hint() -> TypeHint {
///         todo!()
///     }
/// }
/// ```
///
/// Manual exporting is an advanced topic. For more information, refer to the
/// `runtime` and `runtime::ops` module documentation of the main crate.
///
/// ## Type Families
///
/// When exporting a type (either a struct type or a type alias), you can
/// specify a family of types to which this type belongs using the
/// `#[export(family <family reference>)]` attribute.
///
/// Types within the same family are considered castable to each other.
/// The Script Engine often treats distinct instances of the same family as
/// instances of the same type during the preliminary static analysis of
/// scripts.
///
/// For example, all Rust primitive numeric types belong to the same "number"
/// family.
///
/// ```ignore
/// type_family! {
///     pub static FOO_AND_BAR_FAMILY = "foo_bar";
/// }
///
/// #[export(family &FOO_AND_BAR_FAMILY)]
/// struct Foo;
///
/// #[export(family &FOO_AND_BAR_FAMILY)]
/// struct Bar;
/// ```
///
/// For types with families, you would typically implement the Downcast and
/// Upcast type-casting traits manually. This feature is most useful for
/// exported type aliases.
///
/// If you do not specify a type family, the type will belong to a unique,
/// dedicated family consisting solely of that type.
///
/// ## Comments Exporting
///
/// When an exported type, function, constant, or implementation method has a
/// RustDoc comment, this comment will be associated with the corresponding
/// exported Script Engine object. The LSP server will then display these
/// exported RustDoc comments in the code editor.
///
/// ```
/// # use ad_astra_export::export;
/// #
/// /// This documentation will be shown to the editor's end-user when they
/// /// interact with the type (e.g., by hovering the cursor over an inlay hint
/// /// with this type).
/// #[export]
/// struct Foo;
///
/// #[export]
/// impl Foo {
///     /// This documentation will appear in the code-completion menu for the user.
///     pub fn some_method(&self) {}
/// }
/// ```
///
/// ## Export Disabling and Debugging
///
/// The main Ad Astra crate has a feature called `export`. When this feature is
/// enabled (which it is by default), the `Export` macro exports introspected
/// code normally. If you disable this feature flag, all exportable items will
/// be introspected but not exported.
///
/// An exception to this rule is the `#[export(include)]` attribute.
/// By specifying inclusivity, you enforce exporting regardless of the crate's
/// feature flags.
///
/// In practice, mass disabling of exporting will prevent the automatic
/// implementation of Script Engine traits such as `ScriptType`, `Upcast`,
/// `Downcast`, and others, which might be undesirable in certain situations.
///
/// To bypass this problem, the main crate has a `shallow` feature. This feature
/// flag is disabled by default, but if you enable Shallow Mode, the macro will
/// provide dummy implementations for items without actual semantics
/// introspection metadata. This mode is recommended for debugging/development
/// purposes only. In Shallow Mode, the output generated by the macro will be
/// much shorter, simplifying general crate compilation.
///
/// Finally, using the `#[export(dump)]` attribute, the macro will show the
/// pretty-printed output of the macro using `panic`. If you encounter a bug in
/// the macro's behavior, you can report this bug with the macro output dump.
#[proc_macro_attribute]
pub fn export(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = proc_macro2::TokenStream::from(attr);
    let attr_span = attr.span();

    let input = TokenStream::from_iter(
        TokenStream::from(quote_spanned!(attr_span=> #[export(#attr)]))
            .into_iter()
            .chain(item),
    );

    let output = parse_macro_input!(input as ExportItem);
    output.into()
}
