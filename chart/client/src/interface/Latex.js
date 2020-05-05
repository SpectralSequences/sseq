let katex = require("katex");

function ensureMath(str){
    if(str.startsWith("\\(") || str.startsWith("$")){
        return str;
    }
    if(!str){
        return "";
    }
    return "$" + str + "$";
}

function renderLatex(html) {
    html = html.replace(/\n/g, "\n<hr>\n")
    let html_list = html.split(/(?:\\\[)|(?:\\\()|(?:\\\))|(?:\\\])|(?:\$)/);
    for(let i = 1; i < html_list.length; i+=2){
        html_list[i] = katex.renderToString(html_list[i]);
    }
    return html_list.join("\n")
}
exports.renderLatex = renderLatex;
exports.ensureMath = ensureMath;
exports.renderMath = x => renderLatex(ensureMath(x));