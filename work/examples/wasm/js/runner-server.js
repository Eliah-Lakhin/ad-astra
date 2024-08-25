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
    return function RunnerServer(workerFile, wasmFile, wasmCache) {
        workerFile = workerFile || '/js/runner-worker.js';
        wasmFile = wasmFile || '/wasm-module.wasm';
        wasmCache = wasmCache || 'no-cache';

        let currentWorker;

        let onResultOk;
        let onResultErr;
        let onReport;
        let lastReportedLine = null;

        function newWorker(resolve) {
            const workerWrap = {
                worker: new Worker(workerFile),
            };

            function terminate() {
                if (!workerWrap.worker) {
                    return;
                }

                workerWrap.worker.terminate();
                console.warn('Runner worker terminated.');

                delete workerWrap.worker;

                if (!!resolve) {
                    resolve();
                    resolve = null;
                }
            }

            workerWrap.worker.onmessage = (msg) => {
                msg = msg.data;

                switch (msg.method) {
                    case 'ready':
                        resolve(workerWrap);
                        resolve = null;
                        break;

                    case 'terminate':
                        terminate();

                        break;

                    case 'report':
                        if (typeof msg.line === 'number') {
                            lastReportedLine = msg.line;
                        }

                        if (!onReport) {
                            break;
                        }

                        if (onReport(msg)) {
                            break;
                        }

                        terminate();

                        if (!!onResultErr) {
                            onResultErr('interrupt', lastReportedLine);
                        }

                        break;

                    case 'result-ok':
                        if (!!onResultOk) {
                            onResultOk(msg.result);
                            break;
                        }

                        console.warn('Evaluation result Ok.');

                        break;

                    case 'result-err':
                        if (!!onResultErr) {
                            onResultErr(msg.error);
                            break;
                        }

                        console.warn('Evaluation result Err.');

                        break;

                    default:
                        console.error(
                            'Unknown runner worker method',
                            msg.method
                        );
                        break;
                }
            };

            workerWrap.worker.postMessage({
                method: '__setup',
                wasmFile,
                wasmCache,
            });
        }

        function getWorker() {
            if (!currentWorker) {
                currentWorker = new Promise(newWorker);
            }

            return currentWorker.then((wrap) => {
                if (!wrap) {
                    console.error('Failed to setup runner worker.');
                    return;
                }

                if (!!wrap.worker) {
                    return wrap.worker;
                }

                console.warn('Restarting runner worker.');

                currentWorker = null;

                return getWorker();
            });
        }

        this.onResultOk = function (callback) {
            onResultOk = callback;
        };

        this.onResultErr = function (callback) {
            onResultErr = callback;
        };

        this.onReport = function (callback) {
            onReport = callback;
        };

        this.launch = function (uri, text) {
            return getWorker().then((worker) => {
                if (!worker) {
                    return;
                }

                lastReportedLine = null;

                worker.postMessage({ method: '__launch', uri, text });
            });
        };
    };
});
