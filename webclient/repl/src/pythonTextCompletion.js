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
        provideCompletionItems: async function (model, position) {
            let suggestions = [];
            position.lineNumber -= repl.readOnlyLines;
            let [state_id, completions] = await repl.jedi_value.getCompletions(repl.value, position);
            for(let {name, kind} of completions){
                let label = name;
                let suggestion = makeSuggestion(label, monaco.languages.CompletionItemKind[kind_map[kind]]);
                suggestion.state_id = state_id;
                suggestion.idx = suggestions.length;
                suggestions.push(suggestion);
            }
            return {suggestions};
        },
        resolveCompletionItem : async function(model, position, item){
            let result = await repl.jedi_value.getCompletionInfo(item.state_id, item.idx);
            console.log("resolved:", result);
            let {docstring, signature} = result;
            item.detail = signature;
            item.documentation = docstring;
            return item;
        }
    };
}


export function updatePythonLanguageDefinition(monaco, executor){
    monaco.languages.setLanguageConfiguration('python', {
        indentationRules: {
            // decreaseIndentPattern: /^\s*pass\s*$/,
            increaseIndentPattern: /^.*:\s*$/
        }
    });
    monaco.languages.registerCompletionItemProvider('python', getCompletionProvider(monaco, executor));
}


