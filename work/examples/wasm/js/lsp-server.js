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

define(function () {
    const METHOD_NOT_FOUND = -32601;

    return function LspServer(
        clientName,
        clientCapabilities,
        workerFile,
        wasmFile,
        wasmCache
    ) {
        workerFile = workerFile || '/js/lsp-worker.js';
        wasmFile = wasmFile || '/wasm-module.wasm';
        wasmCache = wasmCache || 'no-cache';

        let stage = 0;

        let serverWorker;
        let serverInitialized;

        let clientRequestId = 0;
        let notificationHandlers = {};
        let requestHandlers = {};
        let responseHandlers = {};

        function lspInitialize(serverCapabilities) {
            const params = {
                clientInfo: {
                    name: clientName,
                },

                capabilities: clientCapabilities,
            };

            sendRequest('initialize', params).then((result) => {
                Object.assign(serverCapabilities, result.capabilities);
                stage = 1;

                console.log(
                    'LSP server',
                    result.serverInfo.name,
                    'capabilities',
                    serverCapabilities
                );

                serverInitialized();
            });
        }

        function handleServerMessage(msg) {
            const message = JSON.parse(msg);

            if (message.jsonrpc !== '2.0') {
                console.error('Missing "jsonrpc" in the server message.');
                return;
            }

            if (!message.id) {
                handleNotification(message.method, message.params);
                return;
            }

            if (
                typeof message.result !== 'undefined' ||
                typeof message.error !== 'undefined'
            ) {
                handleResponse(message.id, message.result, message.error);
                return;
            }

            handleRequest(message.id, message.method, message.params);
        }

        function handleNotification(method, params) {
            const handler = notificationHandlers[method];

            if (!handler) {
                console.error('Unknown server notification', method);
                return;
            }

            handler(params);
        }

        function handleRequest(id, method, params) {
            const handler = requestHandlers[method];

            if (!handler) {
                console.error('Unknown server request', method);
                sendErrResponse(id, METHOD_NOT_FOUND);
                return;
            }

            let result = handler(params);

            if (result instanceof Promise) {
                result.then(
                    (body) => {
                        if (typeof result === 'number') {
                            sendErrResponse(id, result);
                            return;
                        }

                        if (result === null || typeof result === 'undefined') {
                            sendOkResponse(id, null);
                            return;
                        }

                        sendOkResponse(id, body);
                    },
                    (code) => {
                        sendErrResponse(id, code);
                    }
                );
                return;
            }

            if (typeof result === 'number') {
                sendErrResponse(id, result);
                return;
            }

            if (result === null || typeof result === 'undefined') {
                sendOkResponse(id, null);
                return;
            }

            sendOkResponse(id, result);
        }

        function handleResponse(id, result, error) {
            if (!responseHandlers[id]) {
                console.error('Missing response handler for id: ', id);
            }

            if (!!error) {
                responseHandlers[id].reject(error);
                return;
            }

            responseHandlers[id].resolve(result);
        }

        function sendRequest(method, params) {
            const id = ++clientRequestId;
            const payload = {
                jsonrpc: '2.0',
                method,
                params,
                id,
            };

            serverWorker.postMessage(payload);

            let resolve;
            let reject;

            const promise = new Promise((promiseResolve, promiseReject) => {
                resolve = promiseResolve;
                reject = promiseReject;
            });

            responseHandlers[id] = { resolve, reject };

            return promise;
        }

        const sendOkResponse = (id, result) => {
            const payload = {
                jsonrpc: '2.0',
                id,
                result,
            };

            serverWorker.postMessage(payload);
        };

        const sendErrResponse = (id, code) => {
            const payload = {
                jsonrpc: '2.0',
                id,
                error: {
                    code,
                    message: '',
                },
            };

            serverWorker.postMessage(payload);
        };

        const sendNotification = (method, params) => {
            const payload = {
                jsonrpc: '2.0',
                method,
                params,
            };

            serverWorker.postMessage(payload);
        };

        this.onNotification = function (method, handler) {
            notificationHandlers[method] = handler;
        };

        this.onRequest = function (method, handler) {
            requestHandlers[method] = handler;
        };

        this.capabilities = {};

        this.initializeServer = function () {
            if (stage !== 0) {
                console.error('Server already initialized.');
                return;
            }

            stage = 1;

            const onServerInitialized = new Promise((resolve) => {
                serverInitialized = resolve;
            });

            serverWorker = new Worker(workerFile);

            const serverCapabilities = this.capabilities;
            serverWorker.onmessage = (msg) => {
                switch (msg.data) {
                    case 'ready':
                        lspInitialize(serverCapabilities);
                        break;
                    case 'terminate':
                        serverWorker.terminate();
                        console.warn('LSP server worker terminated.');
                        break;
                    default:
                        handleServerMessage(msg.data);
                        break;
                }
            };

            serverWorker.postMessage({
                method: '__setup',
                wasmFile,
                wasmCache,
            });

            return onServerInitialized;
        };

        this.clientInitialized = function () {
            if (stage !== 1) {
                console.error('Client already initialized.');
                return;
            }

            stage = 2;

            sendNotification('initialized');
        };

        this.notify = function (method, params) {
            if (stage < 2) {
                console.error('Client is not initialized.');
                return;
            }

            sendNotification(method, params);
        };

        this.request = function (method, params) {
            if (stage < 2) {
                console.error('Client is not initialized.');
                return;
            }

            return sendRequest(method, params);
        };
    };
});
