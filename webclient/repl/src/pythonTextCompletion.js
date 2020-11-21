const kind_map = {
    "module" : "Module",
    "class" : "Class",
    "instance" : "Value",
    "function" : "Function",
    "param" : "TypeParameter",
    "path" : "Folder",
    "keyword" : "Keyword",
    "property" : "Property",
    "statement" : "Constant"
}


function makeSuggestion(label, kind){
    return {
        insertText : label,
        kind,
        label,
        sortText : label.replace(/^_/,"~")
    }
}



function getCompletionProvider(monaco, repl) {
    return {
        triggerCharacters: ['.'],
        provideCompletionItems: async function (model, position, context/*: CompletionContext*/, token/*: CancellationToken*/) {
            let suggestions = [];
            let {lineNumber, column} = position;
            lineNumber -= repl.readOnlyLines;
            column--;
            position = {lineNumber, column};
            // TODO: If SAB is present, use CancellationToken
            let [state_id, completions] = await repl.jedi_value.getCompletions(repl.value, position, token);
            for(let {name, kind} of completions){
                let label = name;
                let suggestion = makeSuggestion(label, monaco.languages.CompletionItemKind[kind_map[kind]]);
                suggestion.state_id = state_id;
                suggestion.idx = suggestions.length;
                suggestions.push(suggestion);
            }
            return {suggestions};
        },
        resolveCompletionItem : async function(model, position, item, token){
            let result = await repl.jedi_value.getCompletionInfo(item.state_id, item.idx, token);
            let {docstring, signature, full_name, root} = result;
            console.log("full_name:", full_name, "root:", root);
            item.detail = signature;
            if(full_name){
                let doclink = `apidocs/_autosummary/${root}.html#${full_name}`;
                docstring += `\n\n[API Docs](${doclink})`;
            }

            item.documentation = {
                isTrusted : true,
                value : docstring,
                uris : [{
                    authority : "www.google.com",
                    query : "",
                    fragment : "",
                    path : "",
                    scheme : "https",
                }]
            };
            return item;
        }
    };
}


function getSignatureHelpProvider(monaco, repl) {
    return {
        triggerCharacters: ['(', ","],
        provideSignatureHelp: async function (model, position, token) {
            let {lineNumber, column} = position;
            lineNumber -= repl.readOnlyLines;
            column--;
            position = {lineNumber, column};
            let signatures = await repl.jedi_value.getSignatures(repl.value, position, token);
            if(!signatures){
                return;
            }
            return { 
                value : signatures,
                dispose : () => false
            }            
        }
    };
}

function getColorProvider(monaco, repl){
    return {
        provideColorPresentations: (model, colorInfo, cancellationToken) => {
            console.log(colorInfo);
            return [
                {
                    label: JSON.stringify(colorInfo.color)
                }
            ];
        },

        provideDocumentColors: (model, cancellationToken) => {
            return [
                // {
                //     color: { red: 255, blue: 0, green: 0, },
                //     range:{
                //         startLineNumber: 1,
                //         startColumn: 0,
                //         endLineNumber: 1,
                //         endColumn: 0
                //     }
                // },
                // {
                //     color: { red: 0, blue: 255, green: 0, },
                //     range:{
                //         startLineNumber: 2,
                //         startColumn: 0,
                //         endLineNumber: 2,
                //         endColumn: 0
                //     }
                // },
                // {
                //     color: { red: 0, blue: 0, green: 255, },
                //     range:{
                //         startLineNumber: 3,
                //         startColumn: 0,
                //         endLineNumber: 3,
                //         endColumn: 0
                //     }
                // }
            ]
        }
    }
}

export function updatePythonLanguageDefinition(monaco, repl){
    monaco.languages.setLanguageConfiguration('python', {
        indentationRules: {
            // decreaseIndentPattern: /^\s*pass\s*$/,
            increaseIndentPattern: /^.*:\s*$/
        }
    });
    monaco.languages.registerCompletionItemProvider('python', getCompletionProvider(monaco, repl));
    monaco.languages.registerSignatureHelpProvider('python', getSignatureHelpProvider(monaco, repl));
    monaco.languages.registerColorProvider('python', getColorProvider(monaco, repl));
}


