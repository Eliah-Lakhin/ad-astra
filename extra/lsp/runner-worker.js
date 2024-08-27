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
        case '__launch':
            launch(msg.data);
            break;
        default:
            console.error('Unknown runner server method', msg.method);
            break;
    }
};

function setup({ wasmFile, wasmCache }) {
    const IMPORTS = {
        worker: {
            terminate: () => {
                self.postMessage({ method: 'terminate' });
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
            report: (head) => {
                let result = JSON.parse(importString(head));

                result.method = 'report';

                self.postMessage(result);
            },
        },
    };

    console.log('Preparing runner worker...');

    const assembly = fetch(wasmFile, { cache: wasmCache });

    console.log('Runner worker assembly loaded.');

    WebAssembly.instantiateStreaming(assembly, IMPORTS).then(({ instance }) => {
        console.log('Runner worker assembly instantiated.');

        module = instance.exports;

        let exports = 0;

        for (const property in module) {
            if (property.startsWith('__ADASTRA_EXPORT_')) {
                module[property]();
                exports += 1;
            }
        }

        console.log('Runner worker exports triggered: ', exports);

        module.runner_init();

        console.log('Runner worker is ready.');

        self.postMessage({ method: 'ready' });
    });
}

function launch({ uri, text }) {
    {
        console.time('Module ' + uri + ' load time');

        exportString(uri + ']' + text);
        module.runner_load_module();

        console.timeEnd('Module ' + uri + ' load time');
    }

    {
        console.time('Module ' + uri + ' compile time');

        module.runner_compile_module();

        console.timeEnd('Module ' + uri + ' compile time');
    }

    console.log('Module', uri, 'launched.');

    let result = importString(module.runner_launch());

    switch (result.startsWith('ok]')) {
        case true:
            self.postMessage({
                method: 'result-ok',
                result: result.substring(3),
            });
            break;
        case false:
            self.postMessage({
                method: 'result-err',
                error: result.substring(3),
            });
            break;
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
