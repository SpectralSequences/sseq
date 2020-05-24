export function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

export function promiseFromDomEvent(eventTarget, eventName, filter){
    return new Promise(resolve => {
        eventTarget.addEventListener(eventName, function handler(e) {
            if(filter === undefined || filter(e)) {
                eventTarget.removeEventListener(eventName, handler);
                resolve(e);
            }
        });
    });
}



export function findAncestorElement(elt, selector){
    let ancestor = elt;
    while(ancestor && !ancestor.matches(selector)){
        ancestor = ancestor.parentElement;
    }
    if(!ancestor){
        throw Error(`Element must be a descendant of ${nodeName}.`);
    }
    return ancestor;
}