let worker = new Worker("./steenrod_calculator_worker.js");//
worker.addEventListener("message", ev => {
    let m = ev.data;
    if(!(m.cmd in message_handlers)){
        console.error(`Unknown command '${m.cmd}'`);
        return;
    }
    message_handlers[m.cmd](m);
});

function katexMathInDelims(string){
    let html_list = string.split(/(?:\\\[)|(?:\\\()|(?:\\\))|(?:\\\])|(?:\$)/);
    for(let i = 1; i < html_list.length; i+=2) {
        html_list[i] = katex.renderToString(html_list[i]);
    }
    return html_list.join("");
}

let description_element = document.getElementById("description-div");
description_element.innerHTML = katexMathInDelims(description_element.innerHTML);

let adem_reln_element = document.getElementById("adem-relation");
adem_reln_element.innerHTML = katexMathInDelims(adem_reln_element.innerHTML);


let prime_input = document.getElementById("calculator-prime");
let generic_input = document.getElementById("calculator-generic");
let calculator_input = document.getElementById("calculator-input");

window.runAdem = function runAdem(){
    let p = Number.parseInt(prime_input.value);
    let result_element = document.getElementById("adem-result");
    let result_simple_element = document.getElementById("adem-result-simple");
    result_simple_element.innerText = "";
    result_element.innerHTML = "";
    var radios = document.getElementsByName('basis');
    let basis;
    for(let i = 0, length = radios.length; i < length; i++){
        if (radios[i].checked){
            basis = radios[i].value;
            break;
        }
    }
    worker.postMessage({
        "cmd" : "calculate",
        "basis" : basis,
        "prime" : p,
        "input" : calculator_input.value
    });
}

let message_handlers = {};
function handleMessageFromWorker(message){
    handlers[message.data.cmd](message.data);
}

message_handlers.extending_basis = function(data){
    let result_simple_element = document.getElementById("adem-result");
    result_simple_element.innerHTML = `<br>Extending basis from degree ${data.old_max} to degree ${data.new_max}...`;
}

message_handlers.extending_basis_done = function(data){
    let result_simple_element = document.getElementById("adem-result");
    if(!result_simple_element.innerText.endsWith("Done.")){
        result_simple_element.innerText += " Done.";
    }
}

message_handlers.result = function(data){
    let result_element = document.getElementById("adem-result");
    let result_simple_element = document.getElementById("adem-result-simple");
    result_element.innerHTML = katexMathInDelims(`$${data.latex_input}=${data.latex_result}$ (click to copy latex)`);
    result_element.title = `$${data.latex_input}=${data.latex_result}$` // stow the raw latex in title for copying.
    result_simple_element.innerText = data.simple_result;
}

message_handlers.parse_error = function(data){
    let result_element = document.getElementById("adem-result");
    let result_simple_element = document.getElementById("adem-result-simple");
    result_element.innerHTML = `Bad input ${data.error_str} at index ${data.position}.`;
}

message_handlers.error = function(data){
    let result_element = document.getElementById("adem-result");
    let result_simple_element = document.getElementById("adem-result-simple");
    result_element.innerHTML = data.error;
}

window.copyToClipboard = function copyToClipboard(text) {
    if (window.clipboardData && window.clipboardData.setData) {
        // IE specific code path to prevent textarea being shown while dialog is visible.
        return clipboardData.setData("Text", text);

    } else if (document.queryCommandSupported && document.queryCommandSupported("copy")) {
        var textarea = document.createElement("textarea");
        textarea.textContent = text;
        textarea.style.position = "fixed";  // Prevent scrolling to bottom of page in MS Edge.
        document.body.appendChild(textarea);
        textarea.select();
        try {
            return document.execCommand("copy");  // Security exception may be thrown by some browsers.
        } catch (ex) {
            console.warn("Copy to clipboard failed.", ex);
            return false;
        } finally {
            document.body.removeChild(textarea);
        }
    }
}

window.copyAdemResultToClipboard = function copyAdemResultToClipboard(){
    let result_element = document.getElementById("adem-result");
    copyToClipboard(result_element.title);
}
