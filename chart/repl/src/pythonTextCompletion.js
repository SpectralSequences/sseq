import css_colors from './css-color-names.json';
let color_list_regex = new RegExp(
    '"(' + Object.keys(css_colors).join('|') + ')"',
    'g',
);

const kind_map = {
    module: 'Module',
    class: 'Class',
    instance: 'Value',
    function: 'Function',
    param: 'TypeParameter',
    path: 'Folder',
    keyword: 'Keyword',
    property: 'Property',
    statement: 'Constant',
};

function makeSuggestion(label, kind) {
    return {
        insertText: label,
        kind,
        label,
        sortText: label.replace(/^_/, '~'),
    };
}

function getCompletionProvider(monaco, repl) {
    return {
        triggerCharacters: ['.'],
        provideCompletionItems: async function (
            model,
            position,
            context /*: CompletionContext*/,
            token /*: CancellationToken*/,
        ) {
            let suggestions = [];
            let { lineNumber, column } = position;
            lineNumber -= repl.readOnlyLines;
            column--;
            position = { lineNumber, column };
            let { state_id, completions } =
                await repl.jedi_value.getCompletions(
                    repl.value,
                    position,
                    token,
                );
            for (let { name, kind } of completions) {
                let label = name;
                let suggestion = makeSuggestion(
                    label,
                    monaco.languages.CompletionItemKind[kind_map[kind]],
                );
                suggestion.state_id = state_id;
                suggestion.idx = suggestions.length;
                suggestions.push(suggestion);
            }
            return { suggestions };
        },
        resolveCompletionItem: async function (model, position, item, token) {
            let result = await repl.jedi_value.getCompletionInfo(
                item.state_id,
                item.idx,
                token,
            );
            let { docstring, signature, full_name, root } = result;
            item.detail = signature;
            if (full_name) {
                let doclink = `apidocs/_autosummary/${root}.html#${full_name}`;
                docstring += `\n\n[API Docs](${doclink})`;
            }
            item.documentation = {
                isTrusted: true,
                value: docstring,
            };
            return item;
        },
    };
}

function getSignatureHelpProvider(monaco, repl) {
    return {
        triggerCharacters: ['(', ','],
        provideSignatureHelp: async function (model, position, token) {
            let { lineNumber, column } = position;
            lineNumber -= repl.readOnlyLines;
            column--;
            position = { lineNumber, column };
            let { signatures, full_name, root } =
                await repl.jedi_value.getSignatures(
                    repl.value,
                    position,
                    token,
                );
            if (!signatures) {
                return;
            }
            if (full_name) {
                let doclink = `apidocs/_autosummary/${root}.html#${full_name}`;
                let sig = signatures.signatures[0];
                let documentation = sig.documentation;
                documentation += `\n\n[API Docs](${doclink})`;
                sig.documentation = {
                    isTrusted: true,
                    value: documentation,
                };
            }
            return {
                value: signatures,
                dispose: () => false,
            };
        },
    };
}

function getColorProvider(monaco, repl) {
    return {
        provideColorPresentations: (model, colorInfo, cancellationToken) => {
            let { red, green, blue, alpha } = colorInfo.color;
            let colorStrs = [red, green, blue, alpha].map(e =>
                e.toFixed(3).replace(/0*$/, '').replace(/\.$/, ''),
            );
            return [
                {
                    label: `Color(${colorStrs})`,
                    textEdit: {
                        range: colorInfo.range,
                        text: `Color(${colorStrs})`,
                    },
                },
            ];
        },

        provideDocumentColors: (model, cancellationToken) => {
            if (!repl.editor) {
                return [];
            }
            let startLine = repl.startOfInputPosition.lineNumber;
            let endLine = repl.endOfInputPosition.lineNumber;
            let result = [];
            function hex_string_to_color(s) {
                let hexs = [s.slice(1, 3), s.slice(3, 5), s.slice(5, 7)];
                return hexs.map(s => Number.parseInt(s, 16) / 255);
            }
            function get_color(red, green, blue, alpha) {
                red = red || 0;
                green = green || 0;
                blue = blue || 0;
                alpha = alpha || 1;
                return { red, blue, green, alpha };
            }
            function get_range(line, startColumn, endColumn) {
                return {
                    startLineNumber: line,
                    startColumn,
                    endLineNumber: line,
                    endColumn,
                };
            }

            for (let line = startLine; line <= endLine; line++) {
                let value = model.getLineContent(line);
                for (let match of value.matchAll(color_list_regex)) {
                    let startColumn = match.index;
                    let endColumn = startColumn + match[0].length + 2;
                    let [red, green, blue] = hex_string_to_color(
                        css_colors[match[1]],
                    );
                    result.push({
                        color: get_color(red, green, blue),
                        range: get_range(line, startColumn, endColumn),
                    });
                }
                for (let match of value.matchAll(/"(#[0-9A-Fa-f]{6})"/g)) {
                    let startColumn = match.index + 1;
                    let endColumn = startColumn + match[0].length + 1;
                    let [red, green, blue] = hex_string_to_color(match[1]);
                    result.push({
                        color: get_color(red, green, blue),
                        range: get_range(line, startColumn, endColumn),
                    });
                }
                for (let match of value.matchAll(/Color\(([^)]*)\)/g)) {
                    let startColumn = match.index + 1;
                    let endColumn = startColumn + match[0].length + 1;
                    let [red, green, blue, alpha] = match[1]
                        .split(',')
                        .map(s => Number.parseFloat(s));
                    result.push({
                        color: get_color(red, green, blue, alpha),
                        range: get_range(line, startColumn, endColumn),
                    });
                }
            }
            return result;
        },
    };
}

export function updatePythonLanguageDefinition(monaco, repl) {
    monaco.languages.setLanguageConfiguration('python', {
        indentationRules: {
            // decreaseIndentPattern: /^\s*pass\s*$/,
            increaseIndentPattern: /^.*:\s*$/,
        },
    });
    monaco.languages.registerCompletionItemProvider(
        'python',
        getCompletionProvider(monaco, repl),
    );
    monaco.languages.registerSignatureHelpProvider(
        'python',
        getSignatureHelpProvider(monaco, repl),
    );
    monaco.languages.registerColorProvider(
        'python',
        getColorProvider(monaco, repl),
    );
}
