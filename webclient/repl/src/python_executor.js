// import Worker from './pyodide.worker.js';
import { v4 as uuid4 } from "uuid";

export class PythonExecutor {
    constructor(){
        this.worker = new Worker("worker.bundle.js");
        this.worker.addEventListener("message", (event) =>{
            this.promiseResolves[event.data.id](event.data);
        })
        this.promiseResolves = {};
    }

    async execute(code){
        let id = uuid4();
        let promise = new Promise((resolve, reject) => {
            this.promiseResolves[id] = resolve;
        });
        this.worker.postMessage({cmd : "execute", python : code, id : id});
        return await promise;
    }

    async validate(code){
        let id = uuid4();
        let promise = new Promise((resolve, reject) => {
            this.promiseResolves[id] = resolve;
        });
        this.worker.postMessage({cmd : "validate", python : code, id : id});
        return await promise;
    }
}