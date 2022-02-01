import { ReplElement } from './repl';

navigator.serviceWorker
    .register('service_worker.bundle.js', { scope: '.' })
    .then(registration => {
        console.log('Service worker loaded', registration);
    })
    .catch(err => {
        console.error('Error loading service worker!', err);
    });
