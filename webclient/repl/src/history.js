import { IndexedDBArray } from "./indexedDB";

export class History {
    constructor(){ 
        this.store = new IndexedDBArray("sseq-repl-history", 1);
        this.databaseReady = this.openDatabase();
        this.historyStrings = [];
        this.modifiedStrings = [];
        this.stringsFromLocalStorage = [];
        return new Proxy(this, {
            get : (obj, key) => {
                if(!Number.isInteger(Number(key))){
                    return Reflect.get(obj, key);
                }
                return (async () => {
                    await this.databaseReady;
                    if(key in this.modifiedStrings){
                        return this.modifiedStrings[key];
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
                })();
            },
            set : (obj, key, value) => {
                if(!Number.isInteger(Number(key))){
                    return Reflect.set(obj, key, value);
                }
                this.modifiedStrings[key] = value;
                return true;
            }
        });
    }

    async openDatabase(){
        await this.store.open();
        this.storedHistoryLength = await this.store.length;
    }

    get length(){
        // await this.databaseReady;
        return this.historyStrings.length || this.storedHistoryLength;
    }

    async push(value){
        await this.databaseReady;
        this.historyStrings[this.length] = value;
    }

    clearModifiedStrings(){
        this.modifiedStrings = [];
    }

    async writeToLocalStorage(){
        await this.databaseReady;
        console.log(this.storedHistoryLength);
        await this.store.pushArray(this.historyStrings, this.storedHistoryLength);
    }
}

window.History = History;