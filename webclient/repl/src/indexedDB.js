export function openDatabase(name, version){
    const request = indexedDB.open(name, version);
    request.onupgradeneeded = (e) => {
        const db = e.target.result;

        // // Delete the old datastore.
        // if (db.objectStoreNames.contains(name)) {
        //     db.deleteObjectStore(name);
        // }

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


export function getItemRequest(objStore, key){
    const objectStoreRequest = objStore.index("key").get(key);
    return new Promise((resolve, reject) => {
        objectStoreRequest.onsuccess = function (e) {
            resolve(objectStoreRequest.result && objectStoreRequest.result.value);
        };
        objectStoreRequest.onerror = reject;
    });
}

export function setItemRequest(objStore, key, value){
    const timestamp = new Date().getTime();
    const item = { key, value, timestamp };
    console.log("setItemRequest", item);
    const request = objStore.put(item);
    return new Promise((resolve, reject) => {
        request.onsuccess = function (e) {
            resolve(item);
        };    
        request.onerror = reject;
    });
}

export function deleteItemRequest(objectStore, key){
    const request = objStore.delete(key);
        
    return new Promise((resolve, reject) => {
        request.onsuccess = function (e) {
            resolve();
        };
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

    async pushArray(array){
        const objStore = this.datastore.transaction([this.name], 'readwrite').objectStore(this.name);
        let promises = array.map((element, idx) => setItemRequest(objStore, idx, element));
        promises.push(setItemRequest(objStore, "listLength", array.length))
        await Promise.all(promises);
    }

    async setItem(key, value) {
        const objStore = this.datastore.transaction([this.name], 'readwrite').objectStore(this.name);
        return await setItemRequest(objStore, key, value);
    }

    async getItem(key){
        const objStore = this.datastore.transaction([this.name], 'readonly').objectStore(this.name);
        return await getItemRequest(objStore, key);
    }


    async removeItem(key){
        const objStore = this.datastore.transaction([this.name], 'readwrite').objectStore(this.name);
        return await deleteItemRequest(objectStore, key);
    }

    hasItem(key){

    }

}


export class IndexedDBArray {
    constructor(name, version){
        this.name = name;
        this.version = version;
        return new Proxy(this, {
            get : (obj, key) => {
                if(!Number.isInteger(Number(key))){
                    return Reflect.get(obj, key);
                }
                return this.getItem(Number(key));
            }
        })
    }

    async open() {
        if(this.datastore){
            return;
        }
        this.datastore = await openDatabase(this.name, this.version);
    }

    async pushArray(array, originalLength){
        let offset = (await this.length) - originalLength;
        const objStore = this.datastore.transaction([this.name], 'readwrite').objectStore(this.name);
        let promises = array.map((element, idx) => setItemRequest(objStore, idx + offset, element));
        promises.push(setItemRequest(objStore, "listLength", array.length + offset))
        await Promise.all(promises);
    }

    get length(){
        return (async () => await this.getItem("listLength") || 0)();
    }


    async getItem(key){
        const objStore = this.datastore.transaction([this.name], 'readonly').objectStore(this.name);
        return await getItemRequest(objStore, key);
    }

}