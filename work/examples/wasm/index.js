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

const CLIENT_NAME = 'Ad Astra in the Browser';

const CLIENT_CAPABILITIES = {
    textDocument: {
        publishDiagnostics: {
            versionSupport: true,
        },
        inlayHint: {},
        formatting: {},
        completion: {
            completionItem: {
                snippetSupport: true,
                documentationFormat: ['plaintext', 'markdown'],
            },
        },
        hover: {
            contentFormat: ['plaintext', 'markdown'],
        },
        definition: {},
        documentHighlight: {},
        implementation: {},
        codeAction: {},
        signatureHelp: {
            signatureInformation: {
                documentationFormat: ['plaintext', 'markdown'],
            },
        },
        rename: { prepareSupport: true },
    },
};

const LANGUAGE_ID = 'adastra';

const EXAMPLE_PATH = 'example.adastra';

const MONACO_OPTIONS = {
    automaticLayout: true,
};

require.config({
    paths: {
        vs: 'https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.47.0/min/vs',
        js: 'js',
    },
});

require([
    'js/lsp-server',
    'js/lsp-client',
    'js/runner-server',
    'js/theme',
], function (LspServer, LspClient, RunnerServer, defineTheme) {
    defineTheme();

    const CONTAINER_ELEMENT = document.getElementById('container');
    const LAUNCH_ELEMENT = document.getElementById('launch');
    const STOP_ELEMENT = document.getElementById('stop');
    const LSP_SERVER = new LspServer(CLIENT_NAME, CLIENT_CAPABILITIES);
    const LSP_CLIENT = new LspClient(
        CLIENT_CAPABILITIES,
        LANGUAGE_ID,
        CONTAINER_ELEMENT,
        MONACO_OPTIONS
    );
    const RUNNER_SERVER = new RunnerServer();

    LSP_SERVER.onNotification('window/logMessage', (params) => {
        switch (params.type) {
            case 1:
                console.error(params.message);
                break;
            case 2:
                console.warn(params.message);
                break;
            case 3:
                console.info(params.message);
                break;
            case 4:
                console.log(params.message);
                break;
        }
    });

    const serverInitialized = LSP_SERVER.initializeServer();

    const modelLoaded = LSP_CLIENT.loadModel(EXAMPLE_PATH);

    const languageInitialized = serverInitialized.then(() => {
        LSP_CLIENT.createLanguage(LSP_SERVER);
        LSP_SERVER.clientInitialized();
    });

    Promise.all([languageInitialized, modelLoaded]).then(() => {
        LSP_CLIENT.syncModel();
        LSP_CLIENT.unlockModel();
    });

    let interruptFlag = false;
    let lastRender = Date.now();

    RUNNER_SERVER.onResultOk((result) => {
        LSP_CLIENT.renderInlayHints();
        console.log('ok', result);
    });

    RUNNER_SERVER.onResultErr((result, line) => {
        if (result === 'interrupt') {
            console.log('interrupted on line', line);
            LSP_CLIENT.setLineMessage(line, {
                label: ' ❗ script evaluation interrupted',
            });
            LSP_CLIENT.renderInlayHints();
            return;
        }

        console.log('err\n', result);
    });

    RUNNER_SERVER.onReport((report) => {
        switch (report.kind) {
            case 1:
            case 2:
                return !interruptFlag;

            case 3:
                LSP_CLIENT.setLineMessage(report.line, {
                    label: ' ≈ ' + report.label,
                    tooltip: report.tooltip,
                });

                const now = Date.now();

                if (now - lastRender > 250) {
                    lastRender = now;
                    LSP_CLIENT.renderInlayHints();
                }
                break;
        }

        return true;
    });

    LAUNCH_ELEMENT.addEventListener('click', () => {
        const uri = LSP_CLIENT.modelUri();
        const text = LSP_CLIENT.modelText();

        LSP_CLIENT.clearLineMessages();
        LSP_CLIENT.renderInlayHints();

        interruptFlag = false;
        lastRender = Date.now();

        RUNNER_SERVER.launch(uri, text);
    });

    STOP_ELEMENT.addEventListener('click', () => {
        interruptFlag = true;
    });
});
