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

import {workspace, ExtensionContext} from "vscode";
import {window} from "vscode";
import * as net from "net";

import {
    LanguageClient,
    LanguageClientOptions,
    Executable,
    TransportKind,
    StreamInfo,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(_context: ExtensionContext) {
    promptConfig().then((config) => {
        const clientOptions: LanguageClientOptions = {
            documentSelector: [{scheme: "file", language: "adastra"}],
            synchronize: {
                fileEvents: workspace.createFileSystemWatcher("**/.adastra"),
            },
        };

        switch (config.lspServerMode) {
            case "IO":
                client = new LanguageClient(
                    "adastra",
                    "Ad Astra Example",
                    ioServer(config.lspServerPath),
                    clientOptions,
                );
                break;

            case "TCP":
                client = new LanguageClient(
                    "adastra",
                    "Ad Astra Example",
                    tcpServer(config.lspServerPort),
                    clientOptions,
                );
                break;
        }

        client.registerProposedFeatures();
        client.start();
    });
}

export function deactivate() {
    if (!client) {
        return;
    }

    return client.stop();
}

function ioServer(cwd: string): Executable {
    let env = process.env;

    env["RUSTFLAGS"] = (env["RUSTFLAGS"] || "") + " -Zlinker-features=-lld";
    env["RUST_BACKTRACE"] = "1";

    return {
        command: "cargo",
        transport: TransportKind.stdio,
        args: ["run", "--quiet", "--bin", "lsp-server", "--"],
        options: {
            cwd,
            env,
        },
    };
}

function tcpServer(port: number): () => Promise<StreamInfo> {
    return () => {
        const socket = net.connect({
            port,
        });

        return Promise.resolve({
            writer: socket,
            reader: socket,
        });
    };
}

async function promptConfig(): Promise<Config> {
    const config = workspace.getConfiguration("adastra");

    let lspServerPath = config.get("lspServerPath");
    let lspServerMode = config.get("lspServerMode");
    let lspServerPort = config.get("lspServerPort");

    if (lspServerMode !== "IO" && lspServerMode !== "TCP") {
        lspServerMode = await window.showQuickPick(
            [
                "Stdio transport. The LSP server process will be spawned by the client.",
                "TCP transport. The LSP server must be spawned manually.",
            ],
            {
                title: 'Missing "adastra.lspServerMode" setting. Select communication mode:',
            },
        );
    }

    switch (lspServerMode) {
        case "TCP transport. The LSP server must be spawned manually.":
        case "TCP":
            lspServerMode = "TCP";
            break;

        default:
        case "Stdio transport. The LSP server process will be spawned by the client.":
        case "IO":
            lspServerMode = "IO";
            break;
    }

    if (!lspServerPath && lspServerMode == "IO") {
        lspServerPath = await window.showInputBox({
            title: 'Missing "adastra.lspServerPath" setting',
            prompt: "Provide a path to the Cargo.toml file of the LSP server.",
            value: ".",
        });
    }

    if (!lspServerPath) {
        lspServerPath = ".";
    }

    if (!lspServerPort && lspServerMode == "TCP") {
        lspServerPort = await window.showInputBox({
            title: 'Missing "adastra.lspServerPort" setting',
            prompt: "Provide a path to the Cargo.toml file of the LSP server.",
            value: ".",
        });
    }

    switch (typeof lspServerPort) {
        case "number":
            break;

        case "string":
            let port = parseInt(lspServerPort);

            if (port <= 0) {
                port = 8081;
            }

            lspServerPort = port;
            break;

        default:
            lspServerPort = 8081;
            break;
    }

    return {
        lspServerMode: lspServerMode as any as LspMode,
        lspServerPath: lspServerPath as any as string,
        lspServerPort: lspServerPort as number,
    };
}

type LspMode = "IO" | "TCP";

interface Config {
    lspServerMode: LspMode;
    lspServerPath: string;
    lspServerPort: number;
}
