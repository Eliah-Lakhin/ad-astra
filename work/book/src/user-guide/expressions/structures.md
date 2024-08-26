<!------------------------------------------------------------------------------
  This file is part of "Ad Astra", an embeddable scripting programming
  language platform.

  This work is proprietary software with source-available code.

  To copy, use, distribute, or contribute to this work, you must agree to
  the terms of the General License Agreement:

  https://github.com/Eliah-Lakhin/ad-astra/blob/master/EULA.md

  The agreement grants a Basic Commercial License, allowing you to use
  this work in non-commercial and limited commercial products with a total
  gross revenue cap. To remove this commercial limit for one of your
  products, you must acquire a Full Commercial License.

  If you contribute to the source code, documentation, or related materials,
  you must grant me an exclusive license to these contributions.
  Contributions are governed by the "Contributions" section of the General
  License Agreement.

  Copying the work in parts is strictly forbidden, except as permitted
  under the General License Agreement.

  If you do not or cannot agree to the terms of this Agreement,
  do not use this work.

  This work is provided "as is", without any warranties, express or implied,
  except where such disclaimers are legally invalid.

  Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).
  All rights reserved.
------------------------------------------------------------------------------->

# Structures

Ad Astra supports a built-in syntax for creating structural data objects.

Informally, Ad Astra structures are key-value data objects that resemble
JavaScript objects, Lua tables, or Rust BTreeMaps. They serve the purpose of
object-oriented organization in script code design.

```adastra
let my_object = struct {
    field_1: 100,

    field_2: 200,
    
    some_method: fn(a, b) {
        self.field_2 = self.field_1 * (a + b);
    },
};

my_object.some_method(3, 4);

my_object.field_2 == 700;
```

A structure object is constructed using the `struct` keyword followed by a
definition body enclosed in `{...}` braces. The body consists of key-value
entries separated by commas, with an optional trailing comma.

The key of an entry can be any valid Ad Astra identifier or an unsigned integer.
The value of an entry can be any expression.

Structure values can be accessed using the field access operator:
`my_object.field_2`.

Similar to script functions, structures are anonymous objects that are typically
assigned to variables or passed directly to other expressions.

## Fields Management

The script code can add new structure entries by assigning values to new object
fields.

```adastra
let my_object = struct {
    field_1: 10,
};

my_object.field_2 = 20;
my_object.field_3 = fn() {};

my_object.field_1? == true;
my_object.field_2? == true;
my_object.field_3? == true;
my_object.field_4? == false;
```

The existence of a structure entry can be tested using the `?` nil-test operator
on the structure field: `foo.bar?`.

Ad Astra does not provide a built-in way to remove entries from structures, but
such a feature could be implemented via exported functions.

## Structure Methods

A method of a structure is an entry where the value is a script function.

Inside the method implementation, you can use the built-in special `self`
variable, which refers to the structure instance.

```adastra
let my_object = struct {
    field: 10,
    method_1: fn() self.field * 3,
};

my_object.method_2 = fn() self.field * 4;

my_object.field = 100;

my_object.method_1() == 300;
my_object.method_2() == 400;
```
