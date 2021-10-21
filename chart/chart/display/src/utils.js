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

export function animationFrame() {
    return new Promise(resolve => window.requestAnimationFrame(resolve));
}

// Returns a function, that, when invoked, will only be triggered at most once
// during a given window of time. Normally, the throttled function will run
// as much as it can, without ever going more than once per `wait` milliseconds;
// but if you'd like to disable the execution on the leading edge, pass
// `{leading: false}`. To disable execution on the trailing edge, ditto.
export function throttle(wait, options) {
    return function helper(func){
        let context, args, result;
        let timeout = null;
        let previous = 0;
        options = options || {};
        function later() {
            previous = options.leading === false ? 0 : Date.now();
            timeout = null;
            result = func.apply(context, args);
            if (!timeout){
                context = args = null;
                wrapper.resolve();
            } 
        };
        wrapper.stoppedPromise = new Promise(resolve => resolve());
        function wrapper() {
            let now = Date.now();
            if(previous === 0){
                wrapper.stoppedPromise = new Promise(resolve => wrapper.resolve = resolve);
            }
            if (previous === 0 && options.leading === false){
                previous = now;
            } 
            let remaining = wait - (now - previous);
            context = this;
            args = arguments;
            if (remaining <= 0 || remaining > wait) {
                if (timeout) {
                    clearTimeout(timeout);
                    timeout = null;
                }
                previous = now;
                result = func.apply(context, args);
                if (!timeout){
                    context = args = null;
                }
            } else if(!timeout) {
                if(options.trailing !== false){
                    timeout = setTimeout(later, remaining);
                } else {
                    wrapper.resolve();
                }
            }
            return result;
        };
        return wrapper;
    }
};


export function uuidv4() {
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
      var r = Math.random() * 16 | 0, v = c == 'x' ? r : (r & 0x3 | 0x8);
      return v.toString(16);
    });
}