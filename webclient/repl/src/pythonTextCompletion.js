

let keywords = [
    'and', 'not', 'or', 
    'as',   
    'del',   
    'for', 'from',  
    'in', 'is', 'lambda',   'self', 
]

let colonKeywords = [
    'finally',
    'try',
    'else'
]

let initialKeywords = [
    'def',
    'global',
    'assert',
    'def',
    'pass',
    'class', 'continue',
    'break',
    'if', 
    'elif',
    'except',
    'import',
    'print', 'raise', 'return', 
     'while',
    'with', 'yield',
];

let types = [
    'int', 'float', 'long', 'complex', 'hex',
];

let builtins = [
    'abs', 'all', 'any', 'apply', 'basestring',
    'bin', 'bool', 'buffer', 'bytearray',
    'callable', 'chr', 'classmethod', 'cmp',
    'coerce', 'compile', 'complex', 'delattr',
    'dict', 'dir', 'divmod', 'enumerate',
    'eval', 'exec', 'execfile', 'file', 'filter',
    'format', 'frozenset', 'getattr', 'globals',
    'hasattr', 'hash', 'help', 'id', 'input', 
    'intern', 'isinstance', 'issubclass', 'iter', 
    'len', 'locals', 'list', 'map', 'max', 'memoryview',
    'min', 'next', 'object', 'oct', 'open',
    'ord', 'pow', 'print', 'property', 'reversed',
    'range', 'raw_input', 'reduce', 'reload',
    'repr', 'reversed', 'round', 'set', 'setattr',
    'slice', 'sorted', 'staticmethod', 'str',
    'sum', 'super', 'tuple', 'type', 'unichr',
    'unicode', 'vars', 'xrange', 'zip',
];

let constants = [ 'True', 'False', 'None' ];

let fields = [
    '__dict__', '__methods__', '__members__',
    '__class__', '__bases__',  '__name__', 
    '__mro__', '__subclasses__', 
    '__import__'
];

let methods = [
    '__init__',
]

function makeSuggestion(text, kind){
    return {
        insertText : text,
        kind,
        label : text
    }
}

function getNormalCompletions(module, position, upToPosition){
    let suggestions = [];
    if(/^\s*[a-zA-Z]*$/.test(upToPosition)){ // If this is the first non whitespace on line
        for(let cmd of initialKeywords){
            suggestions.push(makeSuggestion(cmd, monaco.languages.CompletionItemKind.Keyword));
        }
        for(let cmd of colonKeywords){
            let suggestion = makeSuggestion(cmd, monaco.languages.CompletionItemKind.Keyword);
            suggestion.insertText += ":";
            suggestions.push(suggestion);
        }
    }
    for(let cmd of keywords){
        suggestions.push(makeSuggestion(cmd, monaco.languages.CompletionItemKind.Keyword));
    }
    for(let cmd of types){
        suggestions.push(makeSuggestion(cmd, monaco.languages.CompletionItemKind.TypeParameter))
    }
    for(let cmd of builtins){
        suggestions.push(makeSuggestion(cmd, monaco.languages.CompletionItemKind.Function))
    }
    for(let cmd of constants){
        suggestions.push(makeSuggestion(cmd, monaco.languages.CompletionItemKind.Constant))
    }
    return {
        suggestions
    }
}

function getAttrCompletions(model, position, upToPosition){
    let suggestions = [];
    for(let cmd of fields){
        suggestions.push(makeSuggestion(cmd, monaco.languages.CompletionItemKind.Field))
    }
    for(let cmd of methods){
        suggestions.push(makeSuggestion(cmd, monaco.languages.CompletionItemKind.Method));
    }
    return {
        suggestions
    }
}

function getCompletionProvider(monaco) {
    return {
        provideCompletionItems: function (model, position) {
            let {lineNumber : line, column : col} = position;
            let upToPosition = model.getValueInRange(new monaco.Range(line, 0, line, col));
            let getattrExpression = /\.([a-zA-Z_])*$/.test(upToPosition);
            if(getattrExpression){
                return getAttrCompletions(model, position, upToPosition)
            }
            return getNormalCompletions(model, position, upToPosition);
        }
    };
}



export function updatePythonLanguageDefinition(monaco){
    monaco.languages.setLanguageConfiguration('python', {
        indentationRules: {
            // decreaseIndentPattern: /^\s*pass\s*$/,
            increaseIndentPattern: /^.*:\s*$/
        }
    });
    monaco.languages.registerCompletionItemProvider('python', getCompletionProvider(monaco));
    console.log("updateLanguageDefinition");
}


