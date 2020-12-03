function openDatabase(name, version){
    const request = indexedDB.open(name, version);
    request.onupgradeneeded = (e) => {
        const db = e.target.result;

        // Delete the old datastore.
        if (db.objectStoreNames.contains(name)) {
            db.deleteObjectStore(name);
        }

        // Create a new datastore.
        const store = db.createObjectStore(name, {
            keyPath: 'key'
        });

        store.createIndex("key", "key",  { unique: true });
    };
    return new Promise((resolve, reject) => {
        request.onsuccess = (e) => resolve(e.target.result);    
        request.onerror = reject;
    });
}


export class IndexedDBStorage {
    constructor(name, version){
        this.name = name;
        this.version = version;
    }

    async open() {
        if(this.datastore){
            return;
        }
        this.datastore = await openDatabase(this.name, this.version);
    }

    readTransaction(){
        return new IndexedDBTransaction(
            this.datastore.transaction([this.name], 'readonly').objectStore(this.name), 
            false
        );
    }

    writeTransaction(){
        return new IndexedDBTransaction(
            this.datastore.transaction([this.name], 'readwrite').objectStore(this.name), 
            true
        );
    }
}

export class IndexedDBTransaction {
    constructor(objectStore, mutable){
        this.objectStore = objectStore;
        this.mutable = mutable;
    }
    
    getItem(key){
        const objectStoreRequest = this.objectStore.index("key").get(key);
        return new Promise((resolve, reject) => {
            objectStoreRequest.onsuccess = function (e) {
                resolve(objectStoreRequest.result && objectStoreRequest.result.value);
            };
            objectStoreRequest.onerror = reject;
        });
    }
    
    setItem(key, value){
        if(!this.mutable){
            throw new Error("Attempted to mutate using immutable database reference.");
        }
        const timestamp = new Date().getTime();
        const item = { key, value, timestamp };
        const request = this.objectStore.put(item);
        return new Promise((resolve, reject) => {
            request.onsuccess = function (e) {
                resolve(item);
            };    
            request.onerror = reject;
        });
    }
    
    deleteItem(key){
        const request = this.objectStore.delete(key);
            
        return new Promise((resolve, reject) => {
            request.onsuccess = function (e) {
                resolve();
            };
            request.onerror = reject;
        });
    }
}