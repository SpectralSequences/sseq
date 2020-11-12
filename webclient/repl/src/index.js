import { ReplElement } from "./repl";

// import registerServiceWorker, {
//     ServiceWorkerNoSupportError
// } from 'service-worker-loader!./service.worker';
 
navigator.serviceWorker.register("/dist/service_worker.bundle.js", { scope: '/dist/' })
.then((registration) => {
    console.log('Service worker loaded', registration);
}).catch((err) => {
    // if (err instanceof ServiceWorkerNoSupportError) {
    //     console.log('Service worker is not supported.');
    // } else {
        console.error('Error loading service worker!', err);
    // }
});