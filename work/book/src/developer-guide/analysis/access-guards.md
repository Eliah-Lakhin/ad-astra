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

# Access Guards

Script modules are designed for script semantic analysis in multi-threaded
applications. Even though you can use this interface perfectly well in a
single-threaded application, we need to discuss its API a little further.

The design of the ScriptModule is somewhat similar to a read-write lock: it is
an object that can be shared between threads (e.g., by wrapping it in
`Arc<ScriptModule>`), and the threads access the underlying data through a
system of read and write guards.

The read guards provide read-only operations on the module content. This kind of
access is non-exclusive, allowing as many simultaneous read guards across
independent threads as needed, provided there is no active write guard.

The write guard provides both read and write operations. This kind of access is
exclusive, meaning that when write access is granted, no other read or write
guards can be active.

```rust,ignore
let handle = TriggerHandle::new();
let read_guard = module.read(&handle, 1).expect("Module read error.");
```

The [ScriptModule::read](https://docs.rs/ad-astra/1.0.0/ad_astra/analysis/struct.ScriptModule.html#method.read)
and [ScriptModule::write](https://docs.rs/ad-astra/1.0.0/ad_astra/analysis/struct.ScriptModule.html#method.write)
functions request read and write access, respectively.

Both functions require a handle argument
([`TriggerHandle`](https://docs.rs/lady-deirdre/latest/lady_deirdre/analysis/struct.TriggerHandle.html))
and a guard priority (`1`).

The handle is an object that allows you to manually revoke the access grant.
For instance, you can clone this object, transfer it to another thread, and
[trigger](https://docs.rs/lady-deirdre/latest/lady_deirdre/analysis/struct.TriggerHandle.html#method.trigger)
it to revoke access.

Typically, you should use a unique instance of the handle for each read and
write access request.

The second argument, the priority number, determines the priority of the
operations you intend to perform with this guard.

For example, if there are active read guards with a priority of 2, and another
thread attempts to request a write guard with a priority of 3, the read guards
will be revoked. However, if the write access priority is 1, the request will
block the current thread until all read guards with a priority of 2 are released.

## Multi-Threaded Applications

In multi-threaded applications, where threads may simultaneously request
different types of access operations with distinct priorities, any function
returning a
[ModuleResult](https://docs.rs/ad-astra/1.0.0/ad_astra/analysis/type.ModuleResult.html)
may result in a
[ModuleError::Interrupted](https://docs.rs/ad-astra/1.0.0/ad_astra/analysis/enum.ModuleError.html#variant.Interrupted)
error.

This result indicates that the access grant has been revoked. In this event,
you should typically drop the access guard (if any), put the current thread on
hold for a short amount of time to allow another thread to obtain access with
higher priority, and then retry the operation.

```rust,ignore
loop {
    let handle = TriggerHandle::new();

    let read_guard = match module.read(&handle, 5) {
        Ok(guard) => guard,
        Err(ModuleError::Interrupted(_)) => {
            sleep(Duration::from_millis(100));
            continue;
        }
        Err(other) => todo!("{other}"),
    };

    let diagnostics = match read_guard.diagnostics(2) {
        Ok(diagnostics) => diagnostics,
        Err(ModuleError::Interrupted(_)) => {
            sleep(Duration::from_millis(100));
            continue;
        }
        Err(other) => todo!("{other}"),
    };

    return diagnostics;
}
```

## Single-Threaded Applications

In a single-threaded application, or in a multi-threaded application where each
script module is managed exclusively by a dedicated thread, the situation of
simultaneous access is practically impossible, and the
`ModuleError::Interrupted` error should never occur.

Therefore, in practice, you can more confidently unwrap the results of the
analysis API functions, which simplifies the overall design.

For instance, the
[Runner Example](https://github.com/Eliah-Lakhin/ad-astra/tree/master/work/examples/runner)
application is a single-threaded application that unwraps module results.

```rust,ignore
let handle = TriggerHandle::new();

let read_guard = module.read(&handle, 5).expect("Module read error.");

let diagnostics = read_guard.diagnostics(2).expect("Module analysis error.");

return diagnostics;
```
