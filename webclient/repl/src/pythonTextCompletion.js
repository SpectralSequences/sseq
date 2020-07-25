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
            repl.jedi_value.setCode(repl.value);
            let completions = await repl.jedi_value.getCompletions();
            for(let {name, kind} of completions){
                let label = name;
                let suggestion = makeSuggestion(label, monaco.languages.CompletionItemKind[kind_map[kind]]);
                suggestion.idx = suggestions.length;
                suggestions.push(suggestion);
            }
            return {suggestions};
        },
        resolveCompletionItem : async function(model, position, item){
            let result = await repl.jedi_value.getCompletionInfo(item.idx);
            let {docstring, signature} = result;
            // console.log(result);
            // console.log("resolved", docstring, signature);
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


