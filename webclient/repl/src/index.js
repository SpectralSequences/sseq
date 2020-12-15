import { PythonExecutor } from "./pythonExecutor";
import { ReplElement } from "./repl";
import { sleep } from "./utils";

async function main(){
    let registration;
    try {
        registration = await navigator.serviceWorker.register("service_worker.bundle.js", { scope: '.' });
    } catch(e){
        console.error('Error loading service worker!', err);
        return;
    }

    try {
        console.log('Service worker loaded', registration);
        if(navigator.serviceWorker.controller === null){
            window.loadingWidget.addLoadingMessage("Waiting for service worker (try refreshing page if it hangs here).");
            await new Promise(resolve => navigator.serviceWorker.oncontrollerchange = resolve );
            window.loadingWidget.addLoadingMessage("Service worker ready.");
        }
        let executor  = new PythonExecutor();
        await sleep(100);
        await document.querySelector("repl-terminal").start(executor);
		window.loadingWidget.ready = true;
    } catch(e) {
        window.loadingWidget.loadingFailed(e);
        throw e;
    }
}
main();