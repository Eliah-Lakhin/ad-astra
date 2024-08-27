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

const OK_MESSAGE_COLOR = '#196f3d';
const ERR_MESSAGE_COLOR = '#7b241c';
const INFO_MESSAGE_COLOR = '#1a5276';

const MONACO_OPTIONS = {
    minimap: { enabled: false },
    fontFamily: '"JetBrains Mono", monospace',
    fontSize: '14',
    cursorBlinking: 'smooth',
    lineNumbersMinChars: 4,
    padding: { top: 10, bottom: 10 },
    theme: 'adastra',
    'bracketPairColorization.enabled': false,
    guides: {
        bracketPairs: false,
        bracketPairsHorizontal: false,
        highlightActiveBracketPair: false,
        highlightActiveIndentation: false,
        indentation: false,
    },
    scrollBeyondLastLine: false,
    showFoldingControls: 'never',
    stickyScroll: { enabled: false },
    scrollbar: {
        horizontalScrollbarSize: 8,
        verticalScrollbarSize: 8,
    },
    hideCursorInOverviewRuler: true,
    overviewRulerLanes: 0,
    overviewRulerBorder: false,
    automaticLayout: true,
};

const IS_LOCAL = window.location.hostname === 'localhost';

console.log('LOCAL MODE:', IS_LOCAL);

let LSP_WORKER_PATH = '/extra/lsp/lsp-worker.js';
let RUNNER_WORKER_PATH = '/extra/lsp/runner-worker.js';
let WASM_MODULE_PATH = '/extra/lsp/wasm-module.wasm';
let WASM_MODULE_CACHE = 'default';
let WASM_MODULE_SIZE = 11337581;
let EXAMPLE_PATH = '/examples/name.adastra';
let EXAMPLE_CACHE = 'default';
let EXAMPLE_SWITCH_TIMEOUT = 0;

switch (IS_LOCAL) {
    case true:
        require.config({
            paths: {
                'vs/editor': '/extra/libs',
                js: '/extra/lsp',
            },
        });
        break;

    case false:
        const GH_PAGES_CDN = 'https://cdn.jsdelivr.net/gh/Eliah-Lakhin/ad-astra@gh-pages';

        require.config({
            paths: {
                vs: 'https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.47.0/min/vs',
                js: `${GH_PAGES_CDN}/extra/lsp`,
            },
        });

        WASM_MODULE_PATH = `${GH_PAGES_CDN}${WASM_MODULE_PATH}`;
        EXAMPLE_PATH = `${GH_PAGES_CDN}${EXAMPLE_PATH}`;
        break;
}

require([
    'js/lsp-server',
    'js/lsp-client',
    'js/runner-server',
    'js/theme',
], function (LspServer, LspClient, RunnerServer, defineTheme) {
    defineTheme({
        HINT: '#7b7d7d',
        BRACKETS: '#626567',
        HIGHLIGHT_DEFINITION: '#3498db',
        HIGHLIGHT_WRITE: '#3498db',
        HIGHLIGHT_READ: '#3498db',
        SCROLLBAR_INACTIVE: '#7b7d7d',
        INTERFACE_BORDER: '#b3b6b7',
        INTERFACE_SELECTION: '#424949',
        INTERFACE_HIGHLIGHT: '#616a6b',
    });

    const EDITOR_ELEMENT = document.getElementById('editor');
    const LOADING_ELEMENT = document.getElementById('loading');
    const LOADING_CLIENT_ELEMENT = document.getElementById('loading-client');
    const LOADING_SERVER_ELEMENT = document.getElementById('loading-server');
    const LOADING_SERVER_PROGRESS_ELEMENT = document.getElementById('loading-server-progress');
    const LOADING_EXAMPLE_ELEMENT = document.getElementById('loading-example');
    const LOADING_EXAMPLE_PROGRESS_ELEMENT = document.getElementById('loading-example-progress');
    const LAUNCH_ELEMENT = document.getElementById('editor-launch-btn');
    const CLEANUP_ELEMENT = document.getElementById('editor-cleanup-btn');
    const STOP_ELEMENT = document.getElementById('editor-stop-btn');
    const HINTS_ELEMENT = document.getElementById('editor-hints-btn');
    const EDITOR_CONSOLE_ELEMENT = document.getElementById('editor-console');
    const EXAMPLE_SELECT_ELEMENT = document.getElementById('example-select');

    const LSP_SERVER = new LspServer(
        CLIENT_NAME,
        CLIENT_CAPABILITIES,
        LSP_WORKER_PATH,
        WASM_MODULE_PATH,
        WASM_MODULE_CACHE,
        (progress) => {
            updateUIState({ server: progress.loaded });
        }
    );
    const LSP_CLIENT = new LspClient(
        CLIENT_CAPABILITIES,
        LANGUAGE_ID,
        EDITOR_ELEMENT,
        MONACO_OPTIONS
    );
    const RUNNER_SERVER = new RunnerServer(
        RUNNER_WORKER_PATH,
        WASM_MODULE_PATH,
        WASM_MODULE_CACHE,
    );

    let uiState = {
        loading: false,
        client: false,
        server: false,
        example: false,
        launched: false,
        messages: false,
        interrupt: false,
        hints: false,
    };

    updateUIState({ client: true });

    function updateUIState(state) {
        Object.assign(uiState, state || {});

        let loadingSteps = 0;

        switch (uiState.client) {
            case true:
                LOADING_CLIENT_ELEMENT.style.visibility = 'visible';
                loadingSteps += 1;
                break;
            case false:
                LOADING_CLIENT_ELEMENT.style.visibility = 'hidden';
                break;
        }

        if (uiState.server === true) {
            LOADING_SERVER_ELEMENT.style.visibility = 'visible';
            LOADING_SERVER_PROGRESS_ELEMENT.innerHTML = '';
            loadingSteps += 1;
        } else {
            switch (typeof uiState.server) {
                case 'number':
                    const percents =
                        Math.round(100 * uiState.server / WASM_MODULE_SIZE);

                    LOADING_SERVER_PROGRESS_ELEMENT.innerHTML = `(${percents}%)`;
                    break;
                default:
                    LOADING_SERVER_PROGRESS_ELEMENT.innerHTML = '';
                    break;
            }

            LOADING_SERVER_ELEMENT.style.visibility = 'hidden';
        }

        if (uiState.example === true) {
            LOADING_EXAMPLE_ELEMENT.style.visibility = 'visible';
            LOADING_EXAMPLE_PROGRESS_ELEMENT.innerHTML = '';
            loadingSteps += 1;
        } else {
            switch (typeof uiState.example) {
                case 'number':
                    LOADING_EXAMPLE_PROGRESS_ELEMENT.innerHTML =
                        `(${uiState.example} Bytes)`;
                    break;
                default:
                    LOADING_EXAMPLE_PROGRESS_ELEMENT.innerHTML = '';
                    break;
            }

            LOADING_EXAMPLE_ELEMENT.style.visibility = 'hidden';
        }

        uiState.loading = loadingSteps < 3;

        switch (uiState.loading) {
            case true:
                LOADING_ELEMENT.className = 'loading-visible';
                break;
            case false:
                LOADING_ELEMENT.className = 'loading-hidden';
                break;
        }

        switch (uiState.loading || uiState.launched) {
            case true:
                EXAMPLE_SELECT_ELEMENT.setAttribute('disabled', 'disabled');
                break;

            case false:
                EXAMPLE_SELECT_ELEMENT.removeAttribute('disabled');
                break;
        }

        switch (uiState.launched) {
            case true:
                LAUNCH_ELEMENT.style.display = 'none';
                STOP_ELEMENT.style.display = '';
                break;
            case false:
                LAUNCH_ELEMENT.style.display = '';
                STOP_ELEMENT.style.display = 'none';
                break;
        }

        switch (uiState.messages) {
            case true:
                CLEANUP_ELEMENT.style.display = '';
                break;
            case false:
                CLEANUP_ELEMENT.style.display = 'none';
                break;
        }

        switch (uiState.hints) {
            case true:
                HINTS_ELEMENT.title = 'Hide extra hints';
                HINTS_ELEMENT.style.color = '#7b7d7d';
                break;

            case false:
                HINTS_ELEMENT.title = 'Show extra hints';
                HINTS_ELEMENT.style.color = '#b3b6b7';
                break;
        }
    }

    function printToConsole(message, color) {
        if (!message || message.length === 0) {
            return;
        }

        message = message
            .replace(/&/g, "&amp;")
            .replace(/>/g, "&gt;")
            .replace(/</g, "&lt;")
            .replace(/"/g, "&quot;")
            .replace(/ /g, "&nbsp;")
            .replace(/\n/g, "<br/>");

        let tail = '<br/>' + message;

        if (!!color) {
            tail = `<span style="color: ${color};">${tail}</span>`
        }

        EDITOR_CONSOLE_ELEMENT.innerHTML += tail;
        EDITOR_CONSOLE_ELEMENT.scrollTop = EDITOR_CONSOLE_ELEMENT.scrollHeight;
    }

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

        printToConsole(params.message);
    });

    const serverInitialized = LSP_SERVER.initializeServer();

    let modelLoaded;

    const languageInitialized = serverInitialized.then(() => {
        updateUIState({ server: true });

        LSP_CLIENT.createLanguage(LSP_SERVER);
        LSP_CLIENT.renderInlayHints({ parameters: uiState.hints });
        LSP_SERVER.clientInitialized();
    });

    function syncExample() {
        updateUIState({
            example: false,
            messages: false,
            launched: false,
            interrupt: true,
        });

        let name = EXAMPLE_SELECT_ELEMENT.value;

        LSP_CLIENT.clearLineMessages();
        LSP_CLIENT.renderInlayHints();
        LSP_CLIENT.lockModel();

        EXAMPLE_SELECT_ELEMENT.setAttribute('disabled', 'disabled');

        modelLoaded = LSP_CLIENT
            .loadModel(
                EXAMPLE_PATH.replace(/name/g, name),
                EXAMPLE_CACHE,
                `inmemory://${name}.adastra`,
                ({ loaded }) => {
                    updateUIState({ example: loaded });
                }
            )
            .then(() => {
                return new Promise((resolve) => {
                    setTimeout(() => {
                        resolve();
                        updateUIState({ example: true });
                    }, EXAMPLE_SWITCH_TIMEOUT)
                });
            });

        Promise.all([languageInitialized, modelLoaded]).then(() => {
            updateUIState({ interrupt: false });

            LSP_CLIENT.syncModel();
            LSP_CLIENT.unlockModel();

            EXAMPLE_SELECT_ELEMENT.removeAttribute('disabled');

            LSP_CLIENT.focus();
        });
    }

    syncExample();

    let lastRender = Date.now();

    RUNNER_SERVER.onResultOk((result) => {
        updateUIState({ launched: false });
        LSP_CLIENT.renderInlayHints();
        console.log('ok', result);
        printToConsole(
            `Evaluation finished. Result is ${result}.`,
            OK_MESSAGE_COLOR,
        );
    });

    RUNNER_SERVER.onResultErr((result, line) => {
        if (result === 'interrupt') {
            updateUIState({
                launched: false,
                messages: true,
                interrupt: false,
            });

            console.log('interrupted on line', line);
            LSP_CLIENT.setLineMessage(line, {
                label: ' ❗ interrupted',
            });
            LSP_CLIENT.renderInlayHints();
            return;
        }

        updateUIState({ launched: false });

        console.log('err\n', result);

        printToConsole(
            `Evaluation error.\n${result}`,
            ERR_MESSAGE_COLOR,
        );
    });

    RUNNER_SERVER.onReport((report) => {
        switch (report.kind) {
            case 1:
            case 2:
                return !uiState.interrupt;

            case 3:
                printToConsole(
                    `Debug[${report.line}]: ${report.label}`,
                    INFO_MESSAGE_COLOR,
                );

                LSP_CLIENT.setLineMessage(report.line, {
                    label: ` ≈ ${report.label}`,
                    tooltip: report.tooltip,
                });

                updateUIState({ messages: true });

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
        if (uiState.loading) {
            return;
        }

        const uri = LSP_CLIENT.modelUri();
        const text = LSP_CLIENT.modelText();

        LSP_CLIENT.clearLineMessages();
        LSP_CLIENT.renderInlayHints();

        updateUIState({
            messages: false,
            interrupt: false,
            launched: true,
        });

        lastRender = Date.now();

        printToConsole('Evaluation started.');

        RUNNER_SERVER.launch(uri, text);
    });

    STOP_ELEMENT.addEventListener('click', () => {
        updateUIState({
            interrupt: true,
            launched: false,
        });

        printToConsole('Evaluation interrupted.', ERR_MESSAGE_COLOR);
    });

    CLEANUP_ELEMENT.addEventListener('click', () => {
        LSP_CLIENT.clearLineMessages();
        LSP_CLIENT.renderInlayHints();

        updateUIState({ messages: false });
    });

    HINTS_ELEMENT.addEventListener('click', () => {
        updateUIState({ hints: !uiState.hints });

        languageInitialized.then(() => {
            LSP_CLIENT.renderInlayHints({ parameters: uiState.hints });
        });
    });

    EXAMPLE_SELECT_ELEMENT.addEventListener('change', () => {
        syncExample();
    });
});
