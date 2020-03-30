function setStatus(html){
    if(window.status_div_timer){
        clearTimeout(window.status_div_timer);
    }
    document.getElementById("status").innerHTML = html;
}

function delayedSetStatus(html, delay){
    window.status_div_timer = setTimeout(() => setStatus(html), delay);
}

macros = {
    "\\toda" : ["\\langle #1\\rangle",1],
    "\\tmf" : "\\mathit{tmf}",
    "\\HF" : "H\\F",
    "\\HZ" : "H\\Z",
    "\\semidirect" : "\\rtimes",
    "\\F" : "\\mathbb{F}",
    "\\Z" : "\\mathbb{Z}",
    "\\Zbb" : "\\mathbb{Z}",
    "\\CP" : "\\mathbb{CP}"
}

function katexMathInDelims(string){
    html_list = string.split(/(?:\\\[)|(?:\\\()|(?:\\\))|(?:\\\])|(?:\$)/);
    for(let i = 1; i < html_list.length; i+=2){
        html_list[i] = katex.renderToString(html_list[i], {macros : macros});
    }
    return html_list.join("");
}


url = new URL(document.location)
jsFile = url.searchParams.get("sseq");
function addLoadingMessage(message){
    let msg_div = document.getElementById('loading');
    if(msg_div == null){
        msg_div = document.createElement("div");
        msg_div.id = "loading";
        msg_div.style.position = "absolute";
        msg_div.style.top = "10pt";
        msg_div.style.left = "10pt";
        document.body.appendChild(msg_div);
    }
    if(typeof display === "undefined"){
        msg_div.innerHTML += `<p>${message}</p>`;
    }
    console.log(message);
}

let script = document.createElement('script');
script.src = jsFile;
document.body.appendChild(script);
