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

use std::{fs::read_to_string, path::PathBuf, str::FromStr, sync::mpsc::channel, time::Instant};

use ad_astra::{
    analysis::{ModuleRead, ModuleWrite, ScriptModule},
    export,
    lady_deirdre::{
        analysis::TriggerHandle,
        format::{Style, TerminalString},
    },
    runtime::{
        ops::{DynamicArgument, DynamicReturn, DynamicType},
        ScriptPackage,
    },
};
use clap::Parser;
use notify::{
    event::{AccessKind, AccessMode},
    Config,
    EventKind,
    RecommendedWatcher,
    RecursiveMode,
    Watcher,
};

#[export(package)]
#[derive(Default)]
struct Package;

/// Prints the provided argument and then returns it unchanged.
#[export]
fn dbg(x: DynamicArgument<DynamicType>) -> DynamicReturn<DynamicType> {
    let message = x.data.stringify(false);

    println!("{}", message.apply(Style::new().bold()));

    DynamicReturn::new(x.data)
}

/// Ad Astra Compiler
#[derive(Parser)]
#[command(about)]
struct Cli {
    /// Script file name.
    /// The default value is "./scripts/algebra.adastra".
    #[arg(short, long, default_value_t = String::from("./scripts/algebra.adastra"))]
    path: String,

    /// Continuously watches for file changes.
    /// Disabled by default.
    #[arg(short, long, default_value_t = false)]
    watch: bool,
}

fn main() {
    let cli = Cli::parse();

    let text = read_to_string(&cli.path).expect("Script file read error.");

    let module = ScriptModule::new(Package::meta(), text);
    module.rename(&cli.path);

    compile_and_run(&module);

    if !cli.watch {
        return;
    }

    let file_path = PathBuf::from_str(&cli.path).expect("Invalid file path.");

    let (tx, rx) = channel();
    let mut watcher =
        RecommendedWatcher::new(tx, Config::default()).expect("File watcher setup error.");

    watcher
        .watch(&file_path, RecursiveMode::NonRecursive)
        .expect("File watcher setup error.");

    println!("Watching for changes in the script file...");

    loop {
        let Ok(event) = rx.recv() else {
            return;
        };

        let event = event.expect("Watch error.");

        let EventKind::Access(AccessKind::Close(AccessMode::Write)) = event.kind else {
            continue;
        };

        let Some(file_path) = event.paths.first() else {
            continue;
        };

        println!("Script file modified: {}", file_path.display());

        let text = read_to_string(file_path).expect("Script file read error");

        update_module(&module, &text);
        compile_and_run(&module);
    }
}

fn compile_and_run(module: &ScriptModule) {
    let handle = TriggerHandle::new();
    let read_guard = module.read(&handle, 1).expect("Module read error.");

    for depth in 1..=3 {
        let diagnostics = read_guard
            .diagnostics(depth)
            .expect("Module analysis error.");

        if diagnostics.is_empty() {
            continue;
        }

        println!("{}", diagnostics.highlight(&read_guard.text(), !0));

        return;
    }

    println!("Compiling script module...");

    let time = Instant::now();
    let script_fn = read_guard.compile().expect("Script compilation error.");
    let time = time.elapsed();

    println!("Compilation finished in {time:?}. Script execution started...");

    let time = Instant::now();
    match script_fn.run() {
        Ok(_) => {
            let time = time.elapsed();
            println!("Script execution finished in {time:?}.");
        }
        Err(error) => println!(
            "Script execution failure:\n{}",
            error.display(&read_guard.text()),
        ),
    }
}

fn update_module(module: &ScriptModule, text: &str) {
    let handle = TriggerHandle::new();
    let mut write_guard = module.write(&handle, 1).expect("Module write error.");

    write_guard
        .edit(.., text)
        .expect("Module content synchronization error.");
}
