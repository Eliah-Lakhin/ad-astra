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

define({
    configuration: {
        comments: {
            lineComment: '//',
            blockComment: ['/*', '*/'],
        },
        brackets: [
            ['{', '}'],
            ['[', ']'],
            ['(', ')'],
        ],
        autoClosingPairs: [
            { open: '{', close: '}' },
            { open: '[', close: ']' },
            { open: '(', close: ')' },
            { open: '"', close: '"', notIn: ['string', 'comment'] },
            { open: '/*', close: ' */', notIn: ['string', 'comment'] },
        ],
    },

    monarch: {
        defaultToken: '',
        tokenPostfix: '.adastra',

        keywords: [
            'fn',
            'let',
            'struct',
            'use',
            'for',
            'in',
            'loop',
            'break',
            'continue',
            'return',
            'if',
            'else',
            'match',
            'true',
            'false',
            'crate',
            'self',
            'max',
            'len',
        ],

        tokenizer: {
            root: [
                [
                    /[a-zA-Z][a-zA-Z0-9]*/,
                    {
                        cases: {
                            '@keywords': 'keyword',
                            '@default': 'identifier',
                        },
                    },
                ],

                [/[0-9]+/, 'constant.numeric'],

                [/"((\\.)|[^"])*"/, 'string'],

                { include: '@whitespace' },
            ],

            whitespace: [
                [/[ \t\r\n]+/, ''],
                [/\/\*/, 'comment', '@comment'],
                [/\/\/.*$/, 'comment'],
            ],

            comment: [
                [/[^\/*]+/, 'comment.block'],
                [/\*\//, 'comment.block', '@pop'],
                [/[\/*]/, 'comment.block'],
            ],
        },
    },
});
