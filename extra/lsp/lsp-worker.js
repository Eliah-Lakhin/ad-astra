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

let module;

self.onmessage = (msg) => {
    switch (msg.data.method) {
        case '__setup':
            setup(msg.data);
            break;
        default:
            handleMessage(msg.data);
            break;
    }
};

function setup({ wasmFile, wasmCache }) {
    const IMPORTS = {
        worker: {
            terminate: () => {
                self.postMessage('terminate');
            },
        },
        logger: {
            log: (head) => {
                console.log(importString(head));
            },
            info: (head) => {
                console.info(importString(head));
            },
            warn: (head) => {
                console.warn(importString(head));
            },
            debug: (head) => {
                console.debug(importString(head));
            },
            error: (head) => {
                console.error(importString(head));
            },
        },
        runner: {
            report: () => {
                console.error('Calling report function in LSP worker');
                self.postMessage('terminate');
            },
        },
    };

    console.log('Preparing LSP worker...');

    const assembly = fetch(wasmFile, { cache: wasmCache })
        .then((response) => {
            const reader = response.body.getReader();

            let loaded = 0;

            const source = {
                start: (controller) => {
                    function enqueue(next) {
                        return next.then(({ done, value }) => {
                            if (done) {
                                console.log('LSP worker assembly loaded.');
                                controller.close();
                                return Promise.resolve();
                            }

                            controller.enqueue(value);

                            loaded += value.byteLength;

                            self.postMessage(JSON.stringify({
                                __action: 'loading',
                                loaded,
                            }));

                            return enqueue(reader.read());
                        });
                    }

                    return enqueue(reader.read());
                },
            };

            const strategy = {
                "status" : response.status,
                "statusText" : response.statusText,
            };

            const newResponse = new Response(new ReadableStream(source, strategy));

            for (let entry of response.headers.entries()) {
                newResponse.headers.set(entry[0], entry[1]);
            }

            return newResponse;
        });

    WebAssembly.instantiateStreaming(assembly, IMPORTS).then(({ instance }) => {
        console.log('LSP worker assembly instantiated.');

        module = instance.exports;

        let exports = 0;

        for (const property in module) {
            if (property.startsWith('__ADASTRA_EXPORT_')) {
                module[property]();
                exports += 1;
            }
        }

        console.log('LSP worker exports triggered: ', exports);

        module.server_init();

        console.log('LSP worker is ready.');

        self.postMessage('ready');
    });
}

function handleMessage(message) {
    exportString(JSON.stringify(message));
    module.server_input();

    while (true) {
        let head = module.server_output();

        if (head === 0) {
            console.log('LSP worker outgoing channel closed.');
            return;
        }

        let string = importString(head);

        if (string.length === 0) {
            break;
        }

        self.postMessage(string);
    }
}

function importString(head) {
    const len = module.buffer_len();
    const vec = new Uint8Array(module.memory.buffer, head, len);
    const decoded = new TextDecoder('utf-8').decode(vec);

    module.buffer_free(head);

    return decoded;
}

function exportString(string) {
    const encoded = new TextEncoder().encode(string);

    const head = module.buffer_alloc(encoded.length);
    const target = new Uint8Array(module.memory.buffer, head, encoded.length);

    target.set(encoded);
}