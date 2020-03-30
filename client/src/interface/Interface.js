let katex = require("katex");

applyAttributesToElement = function applyAttributesToElement(element, attributes){
    if(!element || !attributes){
        return;
    }
    for(let kv of Object.entries(attributes)){
        element.setAttribute(kv[0], kv[1]);
    }
};

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
exports.renderLaTeX = renderLatex;
exports.ensureMath = ensureMath;
exports.renderMath = x => renderLatex(ensureMath(x));

fixFormHTML = {};
fixFormHTML.radio = function(doc, field){
    let elts = doc.getElementsByName(field.name);
    let items = field.options.items;
    for(let i = 0; i < elts.length; i++){
        applyAttributesToElement(elts[i], items[i].attributes);
    }
};

class PopupForm {
    // This copies form.record into form.save_record to avoid a race condition between:
    //    the onClose writes over form.record with form.original
    //    the success code writes over form.original with form.record.
    static backupRecord(form){
        form.save_record = {};
        Object.assign(form.save_record,form.record);
    }

    // Write over form.original and form.record with form.save_record. Better have called backupRecord first!
    static saveRecord(form){
        Object.assign(form.original, form.save_record);
        Object.assign(form.record,   form.save_record);
    }

    // Write over form.record with form.original. Goes in the onClose handler.
    static restoreRecord(form){
        Object.assign(form.record, form.original);
    }

    constructor(form_options, popup_options){
        let form_obj = Object.assign({}, PopupForm.default_form_obj, form_options);
        this.form_obj = form_obj;
        let name = form_obj.name;
        form_obj.actions = {};
        form_obj.actions[this.form_obj.accept_button_name] = function() {
            // This call to ".save()" produces a logged error but seems to have the desired effect
            // of moving the current set of form fields into ".record". save is supposed to send the form data
            // to a server. There doesn't seem to be an API call to save current fields into .record, and
            // I couldn't figure out another way to access them.
            w2ui[name].save();
            PopupForm.backupRecord(w2ui[name]);
            let errs = w2ui[name].validate();
            if (errs.length > 0) {
                return;
            }
            PopupForm.saveRecord(w2ui[name]);
            w2ui[name].onSuccess();
            w2popup.close();
        };
        form_obj.actions["Cancel"] = function cancel() {
            // No special handling if the user clicks the cancel button as opposed to escape or close or click outside the box.
            w2popup.close();
        };

        $().w2form(this.form_obj);
        let form = w2ui[name];
        this.form = form;
        this.fixFormHTML(form);
        Object.assign(form.original, form_obj.record);

        this.popup_obj = Object.assign({}, PopupForm.default_popup_obj, popup_options);
        // No idea what this is for I just copied it from http://w2ui.com/web/demos/#!forms/forms-8
        this.popup_obj.onToggle = function (event) {
            $(form.box).hide();
            event.onComplete = function () {
                $(form.box).show();
                form.resize();
            }
        };
        // Pressing "Enter" is the same as clicking "open"
        this.popup_obj.onKeydown = function(event){
            if(event.originalEvent.key === "Enter"){
                if(document.getElementsByClassName("w2ui-error").length > 0){
                    return;
                }
                form.actions[form_obj.accept_button_name]();
            }
        };

        this.popup_obj.onClose = function(event){
            PopupForm.restoreRecord(form);
        };

        this.userOnOpen = this.popup_obj.onOpen;
        this.popup_obj.onOpen = (event) => {
            // There's a delay between when the popup opens and when the form is rendered into the popup.
            // It looks ugly if we let these two events happen sequentially, so we temporarily add a style
            // element to override the opacity with 0. Once the document is rendered, we remove this style element
            // to allow the form to display.
            // TODO: refactor this a bit.
            let hide_popup = document.createElement("style");
            hide_popup.innerText = '#w2ui-popup, #w2ui-lock { opacity :  0 !important }';
            document.body.appendChild(hide_popup);
            event.onComplete = () => {
                $('#w2ui-popup #form').w2render(form); // Render the form
                if(this.userOnOpen){
                    this.userOnOpen(event);
                }
                document.body.removeChild(hide_popup); // Once everything is done, remove the element.
            }
        };

        this.open = this.open.bind(this);
//        w2ui.open_sseq_form.record['sseq-file-name'] = '';
    }

    open(){
        if($('#w2ui-popup').length > 0){
            return;
        }
        $().w2popup(this.popup_obj);
    }

    fixFormHTML(){
        let doc = new DOMParser().parseFromString(this.form.formHTML, "text/html");
        for(let f of this.form.fields){
            if(f.attributes){
                applyAttributesToElement(doc.getElementsByName(f.name)[0], f.attributes);
            }
            if(fixFormHTML[f.type]){
                fixFormHTML[f.type](doc, f);
            }
        }
        this.form.formHTML = new XMLSerializer().serializeToString(doc);
    };


}

PopupForm.default_form_obj = {style: 'border: 0px; background-color: transparent;'};
PopupForm.default_popup_obj = {
    body    : '<div id="form" style="width: 100%; height: 100%;"></div>',
    style   : 'padding: 15px 0px 0px 0px opacity: 0',
    width   : 500,
    height  : 220
};

exports.PopupForm = PopupForm;


class Undo {
    constructor(sseq){
        this.sseq = sseq;
        this.undoStack = [];
        this.undoObjStack = [];
        this.redoStack = [];
        this.redoObjStack = [];
        this.undo = this.undo.bind(this);
        this.redo = this.redo.bind(this);
    };

    startMutationTracking(){
        this.mutationMap = new Map();
    }

    addMutationsToUndoStack(event_obj){
        this.add(this.mutationMap, event_obj);
        this.mutationMap = undefined;
    }

    addMutation(obj, pre, post){
        if(!this.mutationMap){
            return;
        }
        if(this.mutationMap.get(obj)){
            pre = this.mutationMap.get(obj).before;
        }
        this.mutationMap.set(obj, {obj: obj, before: pre, after : post});
    }

    add(mutations, event_obj) {
        this.undoStack.push({type:"normal",  mutations: mutations});
        this.undoObjStack.push(event_obj);
        this.redoStack = [];
        this.redoObjStack = [];
    }

    addValueChange(target, prop, before, after, callback) {
        let e = {type:"value", target: target, prop: prop, before: before, after: after, callback: callback};
        this.undoStack.push(e);
        this.undoObjStack.push(e);
        this.redoStack = [];
        this.redoObjStack = [];
    }
    addManual(e, e_obj) {
        this.undoStack.push(e);
        this.undoObjStack.push(e_obj);
        this.redoStack = [];
        this.redoObjStack = [];
    }

    clear(){
        this.undoStack = [];
        this.redoStack = [];
    };

    undo() {
        if (this.undoStack.length === 0) {
            return;
        }
        let e = this.undoStack.pop();
        this.redoStack.push(e);
        let obj = this.undoObjStack.pop();
        this.redoObjStack.push(obj);
        switch (e.type) {
            case "normal":
                this.undoNormal(e);
                break;
            case "value":
                e.target[e.prop] = e.before;
                if (e.callback) e.callback();
                break;
        }
        this.sseq.emit("update");
    };

    undoNormal(obj){
        let mutations = obj.mutations;
        for(let m of mutations.values()){
            if(m.obj.undoFromMemento){
                m.obj.undoFromMemento(m.before);
            } else {
                m.obj.restoreFromMemento(m.before);
            }
        }
    }

    redo() {
        if (this.redoStack.length === 0) {
            return;
        }
        let e = this.redoStack.pop();
        this.undoStack.push(e);
        let obj = this.redoObjStack.pop();
        this.undoObjStack.push(obj);
        switch (e.type) {
            case "normal":
                this.redoNormal(e);
                break;
            case "value":
                e.target[e.prop] = e.after;
                if (e.callback) e.callback();
                break;
        }
        this.sseq.emit("update");
    };

    redoNormal(obj){
        let mutations = obj.mutations;
        for(let m of mutations.values()){
            if(m.obj.redoFromMemento){
                m.obj.redoFromMemento(m.after);
            } else {
                m.obj.restoreFromMemento(m.after);
            }
        }
    }

    addLock(msg){
        let d = new Date();
        if(msg === undefined){
            msg = `Undo events before save at ${d.getFullYear()}-${d.getMonth()}-${d.getDay()} ${d.getHours()}:${d.getMinutes().toString().padStart(2,"0")}?`;
        }
        this.undoStack.push({
            type : "lock",
            msg : msg,
            date : d,
            undoFunction : lockFunction.bind(this)
        })
    }

    getEventObjects() {
        return this.undoObjStack;
    }

    toJSON(){
        return this.undoStack.map(function(e) {
            if(e.type === "normal"){
                return {
                    "type" : "normal",
                    "mutations" : Array.from(e.mutations.entries()).map(([k,v]) => [k.recid, v.before])
                };
            } else {
                return e;
            }
        });
    }
}

Undo.undoFunctions = {};
Undo.redoFunctions = {};
Undo.undoFunctions["lock"] = lockFunction;
Undo.redoFunctions["lock"] = function() {};


function lockFunction(obj){
    w2confirm(obj.msg)
        .yes(() => {
            this.redoStack.pop();
        })
        .no(() => {
            let e = this.redoStack.pop();
            this.undoStack.push(e);
        });
}

Undo.defaultLockMessage = "Undo events before loaded page?";

exports.Undo = Undo;
