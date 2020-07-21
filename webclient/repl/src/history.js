import { IndexedDBArray } from "./indexedDB";

export class History {
    constructor(){ 
        this.store = new IndexedDBArray("sseq-repl-history", 1);
        this.databaseReady = this.openDatabase();
        this.historyStrings = [];
        this.temporaryValues = [];
        this.stringsFromLocalStorage = [];
    }

    async getItem(key){
        await this.databaseReady;
        if(key in this.temporaryValues){
            return this.temporaryValues[key];
        } 
        if(key in this.historyStrings){
            return this.historyStrings[key];
        } 
        if(key in this.stringsFromLocalStorage){
            return this.stringsFromLocalStorage[key];
        }
        await this.store.open();
        this.stringsFromLocalStorage[key] = await this.store[key];
        return this.stringsFromLocalStorage[key];
    }

    async openDatabase(){
        await this.store.open();
        this.storedHistoryLength = await this.store.length;
        this.idx = this.storedHistoryLength;
    }

    get length(){
        // await this.databaseReady;
        return this.historyStrings.length || this.storedHistoryLength;
    }

    async push(value){
        await this.databaseReady;
        this.historyStrings[this.length] = value;
        this.temporaryValues = [];
        this.undoStack = [];
        this.redoStack = [];
        this.idx = this.length;
    }

    step(didx) {
		let oldIdx = this.idx;
		this.idx = Math.min(Math.max(this.idx + didx, 0), this.length);
        console.log(oldIdx, this.idx, didx);
        if(this.idx === oldIdx){
			return false;
		}
		this.redoStack = [];
        this.undoStack.push(didx);
        return true;
    }
    
    get value(){
        return this.getItem(this.idx);
    }

    setTemporaryValue(value){
        this.temporaryValues[this.idx] = value;
    }

    undoStep(){
        let didx = this.undoStack.pop();
        this.idx -= didx;
        this.redoStack.push(didx);
    }

    redoStep(){
        let didx = this.redoStack.pop();
        this.historyIdx += didx;
        this.undoStack.push(didx);
    }

    async writeToLocalStorage(){
        await this.databaseReady;
        console.log(this.storedHistoryLength);
        await this.store.pushArray(this.historyStrings, this.storedHistoryLength);
    }
}

window.History = History;