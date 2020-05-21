export function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

export function promiseFromDomEvent(eventTarget, eventName){
    return new Promise(resolve => {
        eventTarget.addEventListener(eventName, function handler(e) {
            eventTarget.removeEventListener(eventName, handler);
            resolve(e);
        });
    });
}


export function findAncestorElement(elt, nodeName){
    let ancestor = elt;
    let ucNodeName = nodeName.toUpperCase();
    while(ancestor && ancestor.nodeName !== ucNodeName){
        ancestor = ancestor.parentElement;
    }
    if(!ancestor){
        throw Error(`Element must be a descendant of ${nodeName}.`);
    }
    return ancestor;
}