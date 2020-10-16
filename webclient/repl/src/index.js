import { ReplElement } from "./repl";

import registerServiceWorker, {
    ServiceWorkerNoSupportError
} from 'service-worker-loader!./service.worker';
 
registerServiceWorker({ scope: '/dist/' }).then((registration) => {
    console.log('Service worker loaded');
}).catch((err) => {
 
    if (err instanceof ServiceWorkerNoSupportError) {
        console.log('Service worker is not supported.');
    } else {
        console.error('Error loading service worker!', err);
    }
});