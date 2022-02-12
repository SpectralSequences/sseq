import * as IDBKeyVal from 'idb-keyval';

const store = IDBKeyVal.createStore('pyodide-config-2', 'pyodide-config-2');
async function setWorkingDirectory(directoryHandle) {
    await IDBKeyVal.set("working_directory", directoryHandle, store);
}

async function getWorkingDirectory() {
    let result = await IDBKeyVal.get("working_directory", store);
    console.log({result});
    if (!result) {
        return;
    }
    let permission = await result.requestPermission({mode: 'readwrite'});
    if (permission === 'granted') {
        return result;
    }
}

export const nativeFSHelpers = {
    async openDirectory(){
        return await showDirectoryPicker();
    },
    async openWorkingDirectory(){
        return await getWorkingDirectory();
    },
    async setWorkingDirectory(h){
        let handle = await showDirectoryPicker();
        await setWorkingDirectory(handle);
        return handle;
    },
    async readdir(dirHandle){
        const result = [];
        for await (const name of dirHandle.keys()){
            result.push(name);
        }
        return result;
    },
    async getFileSize(fileHandle){
        const file = await fileHandle.getFile();
        return file.size;
    },
    async getFileContents(fileHandle, output_buffer){
        const file = await fileHandle.getFile();
        let contents = await file.arrayBuffer();
        output_buffer.set(new Uint8Array(contents));
    },
    async getFileTimestamp(fileHandle){
        const file = await fileHandle.getFile();
        return file.lastModified;
    },
    async writeToFile(fileHandle, position, data){
        const stream = await fileHandle.createWritable({keepExistingData: true});
        await stream.write({type : "write", position, data});
        await stream.close();
    },
    async truncate(fileHandle, length){
        const stream = await fileHandle.createWritable({keepExistingData: true});
        await stream.truncate(length);
        await stream.close();
    },
    async touch(fileHandle){
        let file = await fileHandle.getFile();
        const size = file.size;
        const stream = await fileHandle.createWritable({keepExistingData: true});
        // Truncating the file to its current size is the simplest way I found
        // to update its modification time
        await stream.truncate(size);
        await stream.close();
        // Return new timestamp
        file = await fileHandle.getFile();
        return file.lastModified;
    },
    async lookup(dirHandle, name, isDir){
        try {
            const handle = await dirHandle.getDirectoryHandle(name);
            isDir[0] = true;
            return handle;
        } catch (e) {
            if(e.name === "NotFoundError"){
                return undefined;
            }
            if(e.name !== "TypeMismatchError"){
                // An unexpected error
                throw e;
            }
        }
        // This shouldn't raise an error hopefully?
        const handle = await dirHandle.getFileHandle(name);
        isDir[0] = false;
        return handle;
    }
};