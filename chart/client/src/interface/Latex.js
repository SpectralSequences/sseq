import { renderToString } from "katex";

export function ensureMath(str){
    if(str.startsWith("\\(") || str.startsWith("$")){
        return str;
    }
    if(!str){
        return "";
    }
    return "$" + str + "$";
}

export function renderLatex(html) {
    // html = html.replace(/\n/g, "\n<hr>\n");
    let html_list = html.split(/(?:\\\[)|(?:\\\()|(?:\\\))|(?:\\\])|(?:\$)/);
    for(let i = 1; i < html_list.length; i+=2){
        html_list[i] = renderToString(html_list[i]);
    }
    return html_list.join("\n")
}
export function renderMath(x) {
    return renderLatex(ensureMath(x));
} 