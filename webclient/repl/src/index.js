import { ReplElement } from "./repl";

// import registerServiceWorker, {
//     ServiceWorkerNoSupportError
// } from 'service-worker-loader!./service.worker';
 
navigator.serviceWorker.register("service_worker.bundle.js", { scope: '.' })
.then((registration) => {
    console.log('Service worker loaded', registration);
    // console.log("controller:", navigator.serviceWorker.controller);
}).catch((err) => {
    console.error('Error loading service worker!', err);
});