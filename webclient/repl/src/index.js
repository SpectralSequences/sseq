import { ReplElement } from "./repl";


navigator.serviceWorker.register("service_worker.bundle.js", { scope: '.' })
.then((registration) => {
    window.loadingWidget.addLoadingMessage("Downloading Monaco");
    console.log('Service worker loaded', registration);
}).catch((err) => {
    console.error('Error loading service worker!', err);
});