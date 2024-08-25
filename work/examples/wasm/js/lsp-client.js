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

define(['vs/editor/editor.main', 'js/grammar'], function (monaco, grammar) {
    const FROM_LSP_SEVERITY = {
        1: monaco.MarkerSeverity.Error,
        2: monaco.MarkerSeverity.Warning,
        3: monaco.MarkerSeverity.Information,
        4: monaco.MarkerSeverity.Hint,
    };

    const FROM_LSP_COMPLETION_ITEM_KIND = {
        1: monaco.languages.CompletionItemKind.Text,
        2: monaco.languages.CompletionItemKind.Method,
        3: monaco.languages.CompletionItemKind.Function,
        4: monaco.languages.CompletionItemKind.Constructor,
        5: monaco.languages.CompletionItemKind.Field,
        6: monaco.languages.CompletionItemKind.Variable,
        7: monaco.languages.CompletionItemKind.Class,
        8: monaco.languages.CompletionItemKind.Interface,
        9: monaco.languages.CompletionItemKind.Module,
        10: monaco.languages.CompletionItemKind.Property,
        11: monaco.languages.CompletionItemKind.Unit,
        12: monaco.languages.CompletionItemKind.Value,
        13: monaco.languages.CompletionItemKind.Enum,
        14: monaco.languages.CompletionItemKind.Keyword,
        15: monaco.languages.CompletionItemKind.Snippet,
        16: monaco.languages.CompletionItemKind.Color,
        17: monaco.languages.CompletionItemKind.File,
        18: monaco.languages.CompletionItemKind.Reference,
        19: monaco.languages.CompletionItemKind.Folder,
        20: monaco.languages.CompletionItemKind.EnumMember,
        21: monaco.languages.CompletionItemKind.Constant,
        22: monaco.languages.CompletionItemKind.Struct,
        23: monaco.languages.CompletionItemKind.Event,
        24: monaco.languages.CompletionItemKind.Operator,
        25: monaco.languages.CompletionItemKind.TypeParameter,
    };

    const FROM_LSP_DOCUMENT_HIGHLIGHT_KIND = {
        1: monaco.languages.DocumentHighlightKind.Text,
        2: monaco.languages.DocumentHighlightKind.Read,
        3: monaco.languages.DocumentHighlightKind.Write,
    };

    function toLspRange(monacoRange) {
        return {
            start: {
                line: monacoRange.startLineNumber - 1,
                character: monacoRange.startColumn - 1,
            },
            end: {
                line: monacoRange.endLineNumber - 1,
                character: monacoRange.endColumn - 1,
            },
        };
    }

    function fromLspRange(lspRange) {
        return {
            startLineNumber: lspRange.start.line + 1,
            startColumn: lspRange.start.character + 1,
            endLineNumber: lspRange.end.line + 1,
            endColumn: lspRange.end.character + 1,
        };
    }

    function fromLspPosition(lspPosition) {
        return {
            column: lspPosition.character + 1,
            lineNumber: lspPosition.line + 1,
        };
    }

    function toLspPosition(monacoPosition) {
        return {
            character: monacoPosition.column - 1,
            line: monacoPosition.lineNumber - 1,
        };
    }

    function fromLspMarkup(markup) {
        switch (typeof markup) {
            case 'string':
                return markup;
            case 'object':
                switch (markup.kind) {
                    case 'plaintext':
                        return markup.value;
                    case 'markdown':
                        return {
                            value: markup.value,
                        };
                }
        }

        return markup;
    }

    function fromLspWorkspaceEdit(monaco, model, edit) {
        if (!edit) {
            return [];
        }

        if (!edit.changes) {
            return [];
        }

        if (!!edit.documentChanges) {
            return [];
        }

        if (!!edit.changeAnnotations) {
            return [];
        }

        const modelUri = model.uri.toString();
        const versionId = model.getVersionId();

        const edits = [];

        for (uri in edit.changes) {
            if (uri !== modelUri) {
                continue;
            }

            const changes = edit.changes[uri];
            const resource = monaco.Uri.parse(uri);

            for (const changeIndex in changes) {
                const change = changes[changeIndex];

                edits.push({
                    resource,
                    versionId,
                    textEdit: {
                        range: fromLspRange(change.range),
                        text: change.newText,
                    },
                });
            }
        }

        return edits;
    }

    function undisposable() {}

    function markerKey(marker) {
        const key = [
            marker.code,
            marker.startLineNumber,
            marker.startColumn,
            marker.endLineNumber,
            marker.endColumn,
        ];

        return JSON.stringify(key);
    }

    function positionKey(monacoPosition) {
        const key = [monacoPosition.lineNumber, monacoPosition.column];

        return JSON.stringify(key);
    }

    function configureDefaults(monaco, editor) {
        monaco.editor.addKeybindingRule({
            command: 'editor.action.formatDocument',
            keybinding: monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS,
        });

        const suggestController = editor.getContribution(
            'editor.contrib.suggestController'
        );

        if (suggestController?.widget) {
            const widget = suggestController.widget.value;

            if (widget._setDetailsVisible) {
                widget._setDetailsVisible(true);
            }

            if (widget._persistedSize) {
                widget._persistedSize.store({ width: 250, height: 200 });
            }

            widget._isAuto = true;
        }
    }

    return function LspClient(capabilities, languageId, domElement, options) {
        options = Object.assign({}, options || {}, {
            domReadOnly: true,
            readOnly: true,
            value: 'Loading script example...',
            language: 'plaintext',
            wordBasedSuggestions: 'off',
        });

        const editor = monaco.editor.create(domElement, options);

        configureDefaults(monaco, editor);

        let server;
        let model;
        let diagnostics = {};
        let lineMessages = {};
        let allowedInlayHints = {
            types: true,
            parameters: true,
            messages: true,
        };
        let triggerInlayHints;

        this.focus = function() {
            editor.focus();
            editor.setPosition({
                column: 0,
                lineNumber: 10000,
            });
        };

        this.loadModel = function (path, cache) {
            cache = cache || 'no-cache';

            const modelUri = 'inmemory://' + path;

            return fetch(path, { cache })
                .then((response) => response.text())
                .then((text) => {
                    model = monaco.editor.createModel(
                        text,
                        'plaintext',
                        monaco.Uri.parse(modelUri)
                    );

                    const previousModel = editor.getModel();

                    if (
                        previousModel.getLanguageId() === languageId &&
                        server?.capabilities.textDocumentSync?.openClose
                    ) {
                        server.notify('textDocument/didClose', {
                            textDocument: {
                                uri: previousModel.uri.toString(),
                            },
                        });
                    }

                    diagnostics = {};
                    lineMessages = {};

                    editor.setModel(model);

                    previousModel.dispose();
                });
        };

        this.modelUri = function () {
            if (!model) {
                console.error('Model is not loaded.');
                return;
            }

            return model.uri.toString();
        };

        this.modelVersion = function () {
            if (!model) {
                console.error('Model is not loaded.');
                return;
            }

            return model.getVersionId();
        };

        this.modelText = function () {
            if (!model) {
                console.error('Model is not loaded.');
                return;
            }

            return model.getValue();
        };

        this.syncModel = function () {
            if (!server) {
                console.error('Language was not created.');
                return;
            }

            if (!model) {
                console.error('Model is not loaded.');
                return;
            }

            if (server.capabilities.textDocumentSync?.openClose) {
                server.notify('textDocument/didOpen', {
                    textDocument: {
                        uri: model.uri.toString(),
                        languageId,
                        version: model.getVersionId(),
                        text: model.getValue(),
                    },
                });
            }

            if (server.capabilities.textDocumentSync?.change > 0) {
                model.onDidChangeContent((event) => {
                    const textDocument = {
                        uri: model.uri.toString(),
                        version: event.versionId,
                    };

                    let contentChanges;

                    switch (server.capabilities.textDocumentSync.change) {
                        case 1:
                            contentChanges = model.getValue();
                            break;

                        case 2:
                            contentChanges = event.changes.map((change) => {
                                return {
                                    range: toLspRange(change.range),
                                    rangeLength: change.rangeLength,
                                    text: change.text,
                                };
                            });

                            break;
                    }

                    server.notify('textDocument/didChange', {
                        textDocument,
                        contentChanges,
                    });
                });
            }
        };

        this.lockModel = function () {
            if (!server) {
                return;
            }

            if (!model) {
                return;
            }

            editor.updateOptions({
                domReadOnly: true,
                readOnly: true,
            });
        };

        this.unlockModel = function () {
            if (!server) {
                console.error('Language was not created.');
                return;
            }

            if (!model) {
                console.error('Model is not loaded.');
                return;
            }

            if (model.getLanguageId() !== languageId) {
                monaco.editor.setModelLanguage(model, languageId);
            }

            editor.updateOptions({
                domReadOnly: false,
                readOnly: false,
                lightbulb: { enabled: 'on' },
            });
        };

        this.setLineMessage = function (line, message) {
            if (!server) {
                console.error('Language was not created.');
                return;
            }

            if (!model) {
                console.error('Model is not loaded.');
                return;
            }

            if (!!lineMessages[line]) {
                delete lineMessages[line];
            }

            if (!!message) {
                lineMessages[line] = message;
            }
        };

        this.clearLineMessages = function () {
            lineMessages = {};
        };

        this.renderInlayHints = function (flags) {
            if (!!flags) {
                Object.assign(allowedInlayHints, flags);
            }

            if (!server) {
                return;
            }

            if (!model) {
                return;
            }

            if (!!triggerInlayHints) {
                triggerInlayHints();
            }
        };

        this.createLanguage = function (languageServer) {
            if (!!server) {
                console.error('Language already created.');
                return;
            }

            server = languageServer;

            monaco.languages.register({
                id: languageId,
            });

            monaco.languages.setLanguageConfiguration(
                languageId,
                grammar.configuration
            );

            monaco.languages.setMonarchTokensProvider(
                languageId,
                grammar.monarch
            );

            if (!!capabilities.textDocument?.publishDiagnostics) {
                server.onNotification(
                    'textDocument/publishDiagnostics',
                    publishDiagnostics
                );
            }

            if (
                !!capabilities.textDocument?.inlayHint &&
                !!server.capabilities.inlayHintProvider
            ) {
                monaco.languages.registerInlayHintsProvider(languageId, {
                    onDidChangeInlayHints: (trigger) => {
                        triggerInlayHints = trigger;
                    },
                    provideInlayHints,
                });
            }

            if (
                !!capabilities.textDocument?.formatting &&
                !!server.capabilities.documentFormattingProvider
            ) {
                monaco.languages.registerDocumentFormattingEditProvider(
                    languageId,
                    {
                        provideDocumentFormattingEdits,
                    }
                );
            }

            if (
                !!capabilities.textDocument?.completion &&
                !!server.capabilities.completionProvider
            ) {
                monaco.languages.registerCompletionItemProvider(languageId, {
                    provideCompletionItems,
                });
            }

            if (
                !!capabilities.textDocument?.hover &&
                !!server.capabilities.hoverProvider
            ) {
                monaco.languages.registerHoverProvider(languageId, {
                    provideHover,
                });
            }

            if (
                !!capabilities.textDocument?.definition &&
                !!server.capabilities.definitionProvider
            ) {
                monaco.languages.registerDefinitionProvider(languageId, {
                    provideDefinition,
                });
            }

            if (
                !!capabilities.textDocument?.documentHighlight &&
                !!server.capabilities.documentHighlightProvider
            ) {
                monaco.languages.registerDocumentHighlightProvider(languageId, {
                    provideDocumentHighlights,
                });
            }

            if (
                !!capabilities.textDocument?.implementation &&
                !!server.capabilities.implementationProvider
            ) {
                monaco.languages.registerImplementationProvider(languageId, {
                    provideImplementation,
                });
            }

            if (
                !!capabilities.textDocument?.codeAction &&
                !!server.capabilities.codeActionProvider
            ) {
                let meta;

                if (server.capabilities.codeActionProvider.codeActionKinds) {
                    meta = {
                        providedCodeActionKinds:
                            server.capabilities.codeActionProvider
                                .codeActionKinds,
                    };
                }

                monaco.languages.registerCodeActionProvider(
                    languageId,
                    {
                        provideCodeActions,
                    },
                    meta
                );
            }

            if (
                !!capabilities.textDocument?.signatureHelp &&
                !!server.capabilities.signatureHelpProvider
            ) {
                let signatureHelpTriggerCharacters = ['('];
                let signatureHelpRetriggerCharacters = [];

                if (
                    server.capabilities.signatureHelpProvider.triggerCharacters
                ) {
                    signatureHelpTriggerCharacters =
                        server.capabilities.signatureHelpProvider
                            .triggerCharacters;
                }

                if (
                    server.capabilities.signatureHelpProvider
                        .retriggerCharacters
                ) {
                    signatureHelpRetriggerCharacters =
                        server.capabilities.signatureHelpProvider
                            .retriggerCharacters;
                }

                monaco.languages.registerSignatureHelpProvider(languageId, {
                    signatureHelpTriggerCharacters,
                    signatureHelpRetriggerCharacters,
                    provideSignatureHelp,
                });
            }

            if (
                !!capabilities.textDocument?.rename &&
                !!server.capabilities.renameProvider
            ) {
                monaco.languages.registerRenameProvider(languageId, {
                    provideRenameEdits,

                    resolveRenameLocation:
                        capabilities.textDocument.rename.prepareSupport &&
                        server.capabilities.renameProvider.prepareProvider
                            ? resolveRenameLocation
                            : undefined,
                });
            }
        };

        function publishDiagnostics(params) {
            if (!model) {
                console.error('Model is not loaded.');
                return;
            }

            if (params.uri !== model.uri.toString()) {
                return;
            }

            if (!capabilities.textDocument?.publishDiagnostics) {
                return;
            }

            if (
                capabilities.textDocument?.publishDiagnostics.versionSupport &&
                !!params.version
            ) {
                if (model.getVersionId() > params.version) {
                    return;
                }
            }

            const markers = [];
            diagnostics = {};

            (params.diagnostics || []).forEach((diagnostic) => {
                let marker = fromLspRange(diagnostic.range);

                if (!!diagnostic.code) {
                    marker.code = diagnostic.code.toString();
                }

                marker.severity =
                    FROM_LSP_SEVERITY[diagnostic.severity] ||
                    monaco.MarkerSeverity.Hint;

                marker.message = diagnostic.message;

                diagnostics[markerKey(marker)] = diagnostic;

                markers.push(marker);
            });

            monaco.editor.setModelMarkers(model, languageId, markers);
        }

        function provideInlayHints(model, range) {
            return server
                .request('textDocument/inlayHint', {
                    textDocument: { uri: model.uri.toString() },
                    range: toLspRange(range),
                })
                .then((serverHints) => {
                    const occupations = {};
                    const allHints = [];

                    if (!!allowedInlayHints.messages) {
                        for (const key in lineMessages) {
                            const message = lineMessages[key];
                            const lineNumber = Number(key);

                            const position = {
                                lineNumber,
                                column: model.getLineMaxColumn(key),
                            };

                            occupations[positionKey(position)] = true;

                            let tooltip;

                            if (!!message.tooltip) {
                                tooltip = {
                                    value: message.tooltip,
                                };
                            }

                            allHints.push({
                                label: message.label,
                                position,
                                tooltip,
                            });
                        }
                    }

                    if (!!serverHints) {
                        for (const serverHintIndex in serverHints) {
                            const hint = serverHints[serverHintIndex];

                            const position = fromLspPosition(hint.position);
                            const key = positionKey(position);

                            if (!!occupations[key]) {
                                continue;
                            }

                            let include = true;

                            switch (hint.kind) {
                                case monaco.languages.InlayHintKind.Type:
                                    if (!allowedInlayHints.types) {
                                        include = false;
                                    }
                                    break;

                                case monaco.languages.InlayHintKind.Parameter:
                                    if (!allowedInlayHints.parameters) {
                                        include = false;
                                    }
                                    break;
                            }

                            if (!include) {
                                continue;
                            }

                            occupations[key] = true;

                            allHints.push({
                                kind: hint.kind,
                                label: hint.label.toString(),
                                paddingLeft: hint.paddingLeft,
                                paddingRight: hint.paddingRight,
                                position,
                                tooltip: fromLspMarkup(hint.tooltip),
                            });
                        }
                    }

                    return {
                        dispose: undisposable,
                        hints: allHints,
                    };
                });
        }

        function provideDocumentFormattingEdits(model, options) {
            return server
                .request('textDocument/formatting', {
                    textDocument: { uri: model.uri.toString() },
                    options,
                })
                .then((edits) =>
                    (edits || []).map((edit) => ({
                        range: fromLspRange(edit.range),
                        text: edit.newText,
                    }))
                );
        }

        function provideCompletionItems(model, position, context) {
            return server
                .request('textDocument/completion', {
                    textDocument: { uri: model.uri.toString() },
                    position: toLspPosition(position),
                    context: {
                        triggerKind: context.triggerKind + 1,
                        triggerCharacter: context.triggerCharacter,
                    },
                })
                .then((result) => {
                    let list;

                    if (Array.isArray(result)) {
                        list = result;
                    } else if (!list) {
                        list = [];
                    } else {
                        list = result.items;
                    }

                    return {
                        suggestions: list.map((item) => ({
                            label: {
                                label: item.label,
                                detail: item.labelDetails?.detail,
                                description: item.labelDetails?.description,
                            },
                            kind:
                                FROM_LSP_COMPLETION_ITEM_KIND[item.kind] ||
                                monaco.languages.CompletionItemKind.Text,
                            detail: item.detail,
                            documentation: fromLspMarkup(item.documentation),
                            insertText: item.insertText || item.label,
                            insertTextRules:
                                item.insertTextFormat === 2
                                    ? monaco.languages
                                          .CompletionItemInsertTextRule
                                          .InsertAsSnippet
                                    : undefined,
                        })),
                    };
                });
        }

        function provideHover(model, position) {
            return server
                .request('textDocument/hover', {
                    textDocument: { uri: model.uri.toString() },
                    position: toLspPosition(position),
                })
                .then((result) => {
                    let range;
                    let list = [];

                    if (!!result) {
                        if (!!result.range) {
                            range = fromLspRange(result.range);
                        }

                        if (Array.isArray(result.contents)) {
                            list = result.contents;
                        } else {
                            list = [result.contents];
                        }
                    }

                    return {
                        range,

                        contents: list.map((item) => {
                            if (typeof item === 'string') {
                                return { value: item };
                            }

                            if (!!item.language) {
                                return {
                                    value: [
                                        '```' + item.language,
                                        item.value,
                                        '```',
                                    ].join('\n'),
                                };
                            }

                            return { value: item.value };
                        }),
                    };
                });
        }

        function provideDefinition(model, position) {
            return server
                .request('textDocument/definition', {
                    textDocument: { uri: model.uri.toString() },
                    position: toLspPosition(position),
                })
                .then((result) => {
                    let list = [];

                    if (!!result) {
                        if (Array.isArray(result)) {
                            list = result;
                        } else {
                            list = [result];
                        }
                    }

                    return list.map((item) => ({
                        range: fromLspRange(item.range),
                        uri: monaco.Uri.parse(item.uri),
                    }));
                });
        }

        function provideDocumentHighlights(model, position) {
            return server
                .request('textDocument/documentHighlight', {
                    textDocument: { uri: model.uri.toString() },
                    position: toLspPosition(position),
                })
                .then((result) =>
                    (result || []).map((item) => {
                        let kind;

                        if (item.kind) {
                            kind = FROM_LSP_DOCUMENT_HIGHLIGHT_KIND[item.kind];
                        }

                        return {
                            kind,
                            range: fromLspRange(item.range),
                        };
                    })
                );
        }

        function provideImplementation(model, position) {
            return server
                .request('textDocument/implementation', {
                    textDocument: { uri: model.uri.toString() },
                    position: toLspPosition(position),
                })
                .then((result) => {
                    let list = [];

                    if (!!result) {
                        if (Array.isArray(result)) {
                            list = result;
                        } else {
                            list = [result];
                        }
                    }

                    return list.map((item) => ({
                        range: fromLspRange(item.range),
                        uri: monaco.Uri.parse(item.uri),
                    }));
                });
        }

        function provideCodeActions(model, range, context) {
            const lspContext = {
                triggerKind: context.trigger,
            };

            if (!!context.only) {
                lspContext.only = [context.only];
            }

            lspContext.diagnostics = context.markers
                .map((marker) => diagnostics[markerKey(marker)])
                .filter((diagnostic) => !!diagnostic);

            return server
                .request('textDocument/codeAction', {
                    textDocument: { uri: model.uri.toString() },
                    range: toLspRange(range),
                    context: lspContext,
                })
                .then((result) => {
                    if (!result) {
                        return {
                            dispose: undisposable,
                            actions: [],
                        };
                    }

                    let actions = [];

                    for (const itemIndex in result) {
                        const item = result[itemIndex];

                        if (!!item.command) {
                            continue;
                        }

                        if (!!item.diagnostics) {
                            continue;
                        }

                        const edits = fromLspWorkspaceEdit(
                            monaco,
                            model,
                            item.edit
                        );

                        if (edits.length === 0) {
                            continue;
                        }

                        let disabled;

                        if (item.disabled) {
                            disabled = item.disabled.reason;
                        }

                        actions.push({
                            title: item.title,
                            kind: item.kind,
                            isPreferred: item.isPreferred,
                            disabled,
                            edit: { edits },
                        });
                    }

                    return {
                        dispose: undisposable,
                        actions,
                    };
                });
        }

        function provideSignatureHelp(model, position) {
            return server
                .request('textDocument/signatureHelp', {
                    textDocument: { uri: model.uri.toString() },
                    position: toLspPosition(position),
                })
                .then((result) => {
                    let activeSignature = 0;
                    let activeParameter = 0;

                    if (!result) {
                        return {
                            dispose: undisposable,
                            value: {
                                signatures: [],
                                activeSignature,
                                activeParameter,
                            },
                        };
                    }

                    if (!!result.activeSignature) {
                        activeSignature = result.activeSignature;
                    }

                    if (!!result.activeParameter) {
                        activeParameter = result.activeParameter;
                    }

                    const signatures = result.signatures.map((signature) => {
                        let parameters = [];

                        if (!!signature.parameters) {
                            parameters = signature.parameters.map((param) => ({
                                label: param.label,
                                documentation: fromLspMarkup(
                                    param.documentation
                                ),
                            }));
                        }

                        return {
                            label: signature.label,
                            documentation: fromLspMarkup(
                                signature.documentation
                            ),
                            parameters,
                            activeParameter: signature.activeParameter,
                        };
                    });

                    fromLspMarkup();

                    return {
                        dispose: undisposable,
                        value: {
                            signatures,
                            activeSignature,
                            activeParameter,
                        },
                    };
                });
        }

        function provideRenameEdits(model, position, newName) {
            return server
                .request('textDocument/rename', {
                    textDocument: { uri: model.uri.toString() },
                    position: toLspPosition(position),
                    newName,
                })
                .then(
                    (result) => {
                        if (!result) {
                            return { edits: [] };
                        }

                        const edits = fromLspWorkspaceEdit(
                            monaco,
                            model,
                            result
                        );

                        return {
                            edits,
                        };
                    },
                    (error) => ({
                        edits: [],
                        rejectReason: error.message,
                    })
                );
        }

        function resolveRenameLocation(model, position) {
            const lspPosition = toLspPosition(position);

            return server
                .request('textDocument/prepareRename', {
                    textDocument: { uri: model.uri.toString() },
                    position: lspPosition,
                })
                .then(
                    (result) => {
                        if (!result) {
                            return {
                                range: fromLspRange({
                                    start: lspPosition,
                                    end: lspPosition,
                                }),
                                text: '',
                                rejectReason:
                                    'nothing to rename in this location',
                            };
                        }

                        if (!!result.defaultBehavior) {
                            const wordAtPosition =
                                model.getWordAtPosition(position);

                            if (!wordAtPosition) {
                                return {
                                    range: fromLspRange({
                                        start: lspPosition,
                                        end: lspPosition,
                                    }),
                                    text: '',
                                    rejectReason:
                                        'nothing to rename in this location',
                                };
                            }

                            return {
                                range: {
                                    startLineNumber: position.lineNumber,
                                    startColumn: wordAtPosition.startColumn,
                                    endLineNumber: position.lineNumber,
                                    endColumn: wordAtPosition.endColumn,
                                },
                                text: wordAtPosition.word,
                            };
                        }

                        if (!!result.range) {
                            return {
                                range: fromLspRange(result.range),
                                text: result.placeholder,
                            };
                        }

                        const range = fromLspRange(result);

                        return {
                            range,
                            text: model.getValueInRange(range),
                        };
                    },
                    (error) => ({
                        range: fromLspRange({
                            start: lspPosition,
                            end: lspPosition,
                        }),
                        text: '',
                        rejectReason: error.message,
                    })
                );
        }
    };
});
