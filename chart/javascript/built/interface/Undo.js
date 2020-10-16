"use strict";
class Undo {
    constructor(sseq) {
        this.sseq = sseq;
        this.undoStack = [];
        this.undoObjStack = [];
        this.redoStack = [];
        this.redoObjStack = [];
        this.undo = this.undo.bind(this);
        this.redo = this.redo.bind(this);
    }
    ;
    startMutationTracking() {
        this.mutationMap = new Map();
    }
    addMutationsToUndoStack(event_obj) {
        this.add(this.mutationMap, event_obj);
        this.mutationMap = undefined;
    }
    addMutation(obj, pre, post) {
        if (!this.mutationMap) {
            return;
        }
        if (this.mutationMap.get(obj)) {
            pre = this.mutationMap.get(obj).before;
        }
        this.mutationMap.set(obj, { obj: obj, before: pre, after: post });
    }
    add(mutations, event_obj) {
        this.undoStack.push({ type: "normal", mutations: mutations });
        this.undoObjStack.push(event_obj);
        this.redoStack = [];
        this.redoObjStack = [];
    }
    addValueChange(target, prop, before, after, callback) {
        let e = { type: "value", target: target, prop: prop, before: before, after: after, callback: callback };
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
    clear() {
        this.undoStack = [];
        this.redoStack = [];
    }
    ;
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
                if (e.callback)
                    e.callback();
                break;
        }
        this.sseq.emit("update");
    }
    ;
    undoNormal(obj) {
        let mutations = obj.mutations;
        for (let m of mutations.values()) {
            if (m.obj.undoFromMemento) {
                m.obj.undoFromMemento(m.before);
            }
            else {
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
                if (e.callback)
                    e.callback();
                break;
        }
        this.sseq.emit("update");
    }
    ;
    redoNormal(obj) {
        let mutations = obj.mutations;
        for (let m of mutations.values()) {
            if (m.obj.redoFromMemento) {
                m.obj.redoFromMemento(m.after);
            }
            else {
                m.obj.restoreFromMemento(m.after);
            }
        }
    }
    addLock(msg) {
        let d = new Date();
        if (msg === undefined) {
            msg = `Undo events before save at ${d.getFullYear()}-${d.getMonth()}-${d.getDay()} ${d.getHours()}:${d.getMinutes().toString().padStart(2, "0")}?`;
        }
        this.undoStack.push({
            type: "lock",
            msg: msg,
            date: d,
            undoFunction: lockFunction.bind(this)
        });
    }
    getEventObjects() {
        return this.undoObjStack;
    }
    toJSON() {
        return this.undoStack.map(function (e) {
            if (e.type === "normal") {
                return {
                    "type": "normal",
                    "mutations": Array.from(e.mutations.entries()).map(([k, v]) => [k.recid, v.before])
                };
            }
            else {
                return e;
            }
        });
    }
}
Undo.undoFunctions = {};
Undo.redoFunctions = {};
Undo.undoFunctions["lock"] = lockFunction;
Undo.redoFunctions["lock"] = function () { };
function lockFunction(obj) {
    console.error("This function (lock) probably doesn't work!");
    confirm(obj.msg)
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
