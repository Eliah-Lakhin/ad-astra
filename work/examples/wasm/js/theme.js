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

define(['vs/editor/editor.main'], function (monaco) {
    const DEFAULTS = {
        FG: '#f2f4f4',
        BG: '#424949',
        SELECTION: '#616a6b',
        LINE_NUMBER: '#7b7d7d',
        CURSOR:  '#eb984e',
        HIGHLIGHT_DEFINITION:  '#17a589',
        HIGHLIGHT_WRITE:  '#117864',
        HIGHLIGHT_READ:  '#0e6251',
        LINK:  '#abebc6',
        HINT:  '#85c1e9',
        BRACKETS:  '#2980b9',
        SCROLLBAR:  '#ccd1d1',
        SCROLLBAR_INACTIVE: '#ccd1d1',
        HOVER:  '#626567',
        INTERFACE_BG:  '#7b7d7d',
        INTERFACE_BORDER:  '#616a6b',
        INTERFACE_SELECTION:  '#34495e',
        INTERFACE_HIGHLIGHT:  '#566573',
        INTERFACE_SEARCH:  '#f8c471',

        IDENT:  '#fdfefe',
        KEYWORD:  '#fad7a0',
        LITERAL:  '#abebc6',
        COMMENT:  '#b3b6b7',
    };

    return function defineTheme(colors) {
        colors = Object.assign({}, DEFAULTS, colors || {});

        monaco.editor.defineTheme('adastra', {
            base: 'vs',
            inherit: true,
            rules: [
                {
                    token: 'identifier',
                    foreground: colors.IDENT,
                },
                {
                    token: 'keyword',
                    foreground: colors.KEYWORD,
                    fontStyle: 'bold',
                },
                {
                    token: 'constant',
                    foreground: colors.LITERAL,
                },
                {
                    token: 'string',
                    foreground: colors.LITERAL,
                },
                {
                    token: 'comment',
                    foreground: colors.COMMENT,
                },
            ],
            colors: {
                'editor.foreground': colors.FG,
                'editor.background': colors.BG,
                'editor.selectionBackground': colors.SELECTION,
                'editorLineNumber.foreground': colors.LINE_NUMBER,
                'editorLineNumber.activeForeground': colors.LINE_NUMBER,
                'editorCursor.foreground': colors.CURSOR,
                'editor.wordHighlightBackground': colors.HIGHLIGHT_READ,
                'editor.wordHighlightStrongBackground': colors.HIGHLIGHT_WRITE,
                'editor.wordHighlightTextBorder': colors.HIGHLIGHT_DEFINITION,
                'editor.wordHighlightTextBackground': colors.BG,
                'editor.lineHighlightBackground': colors.BG,
                'editorInlayHint.background': '#00000000',
                'editorInlayHint.foreground': colors.HINT,
                'editorBracketMatch.background': colors.BRACKETS,
                'editorBracketMatch.border': colors.BG,
                'scrollbarSlider.background': colors.SCROLLBAR_INACTIVE,
                'scrollbarSlider.activeBackground': colors.SCROLLBAR,
                'scrollbarSlider.hoverBackground': colors.SCROLLBAR,
                'editorLink.activeForeground': colors.LINK,
                'editor.hoverHighlightBackground': colors.HOVER,
                'dropdown.background': colors.INTERFACE_BG,
                'dropdown.foreground': colors.FG,
                'dropdown.border': colors.INTERFACE_BORDER,
                'editorWidget.background': colors.INTERFACE_BG,
                'editorWidget.foreground': colors.FG,
                'editorWidget.border': colors.INTERFACE_BORDER,
                'list.activeSelectionBackground': colors.INTERFACE_SELECTION,
                'list.hoverBackground': colors.INTERFACE_HIGHLIGHT,
                'list.highlightForeground': colors.INTERFACE_SEARCH,
            },
        });
    };
});
