"use strict";

function stdCatToString(x){
    if(x === undefined){
        return undefined;
    }
    if(x.getStringifyingMapKey !== undefined){
        return x.getStringifyingMapKey();
    } else {
        return x.toString();
    }
}

var StringifyingMap = (function () {
    function StringifyingMap(catToString) {
        if(catToString === undefined){
            catToString = stdCatToString
        }
        this.catToString = catToString;
        this.m = new Map();
        this.key_string_to_key_object = new Map();
    }
    StringifyingMap.prototype.set = function (k, v) {
        let key_string = this.catToString(k);
        if(key_string === undefined){
            throw new Error("Key encoding undefined.");
        }
        this.key_string_to_key_object.set(key_string, k);
        let s = this.m.set(key_string, v);
        return s;
    };
    StringifyingMap.prototype.get = function (k) {
        let key_string = this.catToString(k);
        if(key_string === undefined){
            return undefined;
        }
        return this.m.get(this.catToString(k));
    };
    StringifyingMap.prototype.delete = function (k) {
        this.key_string_to_key_object.delete(this.catToString(k));
        return this.m.delete(this.catToString(k));
    };
    StringifyingMap.prototype.has = function (k) {
        if(k === undefined){
            return false;
        }
        return this.m.has(this.catToString(k));
    };    

    StringifyingMap.prototype.getOrElse = function(key, value) {
      return this.has(key) ? this.get(key) : value;
    };
    
    StringifyingMap.prototype[Symbol.iterator] = function*(){
        for(let k of this.m){
            yield [this.key_string_to_key_object.get(k[0]),k[1]];
        }
    };

    StringifyingMap.prototype.keys = function(){
        return this.key_string_to_key_object.values();
    };

    StringifyingMap.prototype.toJSON = function(){
        return [...this];
    }
    
    Object.defineProperty(StringifyingMap.prototype, "size", {
        get: function () {
            return this.m.size;
        },
        enumerable: true,
        configurable: true
    });
    return StringifyingMap;
}());


module.exports = StringifyingMap;
