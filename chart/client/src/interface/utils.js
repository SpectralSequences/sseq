export function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

export function promiseFromDomEvent(eventTarget, eventName){
    return new Promise(resolve => {
        eventTarget.addEventListener(eventName, function handler(e) {
            resolve(e);
            eventTarget.removeEventListener(eventName, handler);
        });
    });
}
