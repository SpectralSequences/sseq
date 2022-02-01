'use strict';

function stdCatToString(x: any): string {
    if (x === undefined) {
        throw TypeError('Argument is undefined.');
    }
    if (x.getStringifyingMapKey !== undefined) {
        return x.getStringifyingMapKey();
    } else {
        return x.toString();
    }
}

export default class StringifyingMap<K, V> {
    catToString: (arg: K) => string;
    m: Map<string, V>;
    key_string_to_key_object: Map<string, K>;
    constructor(catToString?: (arg: K) => string) {
        if (catToString === undefined) {
            catToString = stdCatToString;
        }
        this.catToString = catToString;
        this.m = new Map();
        this.key_string_to_key_object = new Map();
    }

    set(k: K, v: V) {
        let key_string = this.catToString(k);
        if (key_string === undefined) {
            throw new Error('Key encoding undefined.');
        }
        this.key_string_to_key_object.set(key_string, k);
        let s = this.m.set(key_string, v);
        return s;
    }

    get(k: K) {
        let key_string = this.catToString(k);
        if (key_string === undefined) {
            return undefined;
        }
        return this.m.get(this.catToString(k));
    }

    delete(k: K) {
        this.key_string_to_key_object.delete(this.catToString(k));
        return this.m.delete(this.catToString(k));
    }

    has(k: K) {
        if (k === undefined) {
            return false;
        }
        return this.m.has(this.catToString(k));
    }

    getOrElse(key: K, value: V) {
        return this.has(key) ? this.get(key) : value;
    }

    [Symbol.iterator]() {
        return function* (this: StringifyingMap<K, V>) {
            for (let k of this.m) {
                yield [this.key_string_to_key_object.get(k[0]), k[1]];
            }
        }.bind(this)();
    }

    keys() {
        return this.key_string_to_key_object.values();
    }

    toJSON() {
        return [...this];
    }

    // Object.defineProperty(StringifyingMap.prototype, "size", {
    //     get: function () {
    //         return this.m.size;
    //     },
    //     enumerable: true,
    //     configurable: true
    // });
}
