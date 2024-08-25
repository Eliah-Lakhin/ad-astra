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

use std::net::SocketAddr;

use ad_astra::{
    export,
    runtime::{
        ops::{DynamicArgument, DynamicReturn, DynamicType},
        ScriptPackage,
    },
    server::{
        inlay_hint,
        LspLoggerConfig,
        LspLoggerServerConfig,
        LspServer,
        LspServerConfig,
        LspTransportConfig,
    },
};
use clap::Parser;

#[export(package)]
#[derive(Default)]
struct Package;

/// Prints the provided argument and then returns it unchanged.
#[export]
pub fn dbg(x: DynamicArgument<DynamicType>) -> DynamicReturn<DynamicType> {
    let message = x.data.stringify(false);
    let tooltip = x.data.stringify(true);

    // debug!("{}", tooltip);

    let tooltip = match message == tooltip {
        true => String::new(),
        false => format!("```\n{tooltip}\n```"),
    };

    inlay_hint(x.origin, message, tooltip);

    DynamicReturn::new(x.data)
}

/// Ad Astra LSP Server.
#[derive(Parser)]
#[command(about)]
struct Cli {
    /// Uses STD-IO as the communication channel.
    /// This is the default option.
    #[arg(long, default_value_t = false)]
    stdio: bool,

    /// Uses TCP as the communication channel.
    /// The LSP server creates the TCP server, and the client connects to the
    /// server's socket.
    #[arg(long)]
    tcp: Option<SocketAddr>,
}

fn main() {
    let cli = Cli::parse();

    let server_config = LspServerConfig::new();

    let mut logger_config = LspLoggerConfig::new();

    let mut transport_config = LspTransportConfig::Stdio;

    if let Some(addr) = cli.tcp {
        transport_config = LspTransportConfig::TcpServer(addr);
    }

    if let LspTransportConfig::Stdio = &transport_config {
        logger_config.server = LspLoggerServerConfig::Off;
    }

    LspServer::startup(
        server_config,
        logger_config,
        transport_config,
        Package::meta(),
    );
}
