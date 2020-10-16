import { INFINITY } from "../infinity";
export class PageProperty {
    constructor(value) {
        if (value instanceof Array) {
            this.values = value;
        }
        else {
            this.values = [[-INFINITY, value]];
        }
        return new Proxy(this, {
            get: (obj, key) => {
                // key will either be a string or a symbol.
                // Number(key) works fine if it's a string, but if it's a symbol it throws a type error.
                // So first use key.toString.
                if (Number.isInteger(Number(key.toString()))) {
                    return obj.valueOnPage(key);
                }
                else {
                    //@ts-ignore
                    return obj[key];
                }
            },
            set: (obj, key, value) => {
                const newKey = (key || '').toString()
                    .replace(/\s/g, '') // Remove all whitespace.
                    .replace(/,/g, ':'); // Replace commas with colons.
                if (/^(-?\d+)$/.test(newKey)) {
                    this.setItemSingle(Number(key), value);
                    this.mergeRedundant();
                    return true;
                }
                // Handle slices.
                if (!/^(-?\d+)?(:(-?\d+)?)?$/.test(newKey)) {
                    return Reflect["set"](obj, key, value);
                }
                let [start, stop] = newKey.split(':').map(part => part.length ? Number.parseInt(part) : undefined);
                start = start || -INFINITY;
                stop = stop || INFINITY;
                let orig_value = this.valueOnPage(stop);
                let [start_idx, hit_start] = this.setItemSingle(start, value);
                let [end_idx, hit_end] = this.findIndex(stop);
                if (!hit_end && stop < INFINITY) {
                    [end_idx,] = this.setItemSingle(stop, orig_value);
                }
                if (stop == INFINITY) {
                    end_idx++;
                }
                this.values.splice(start_idx + 1, end_idx - start_idx - 1);
                this.mergeRedundant();
                return true;
            }
        });
    }
    findIndex(target_page) {
        let result_idx;
        for (let idx = 0; idx < this.values.length; idx++) {
            let [page, value] = this.values[idx];
            if (page > target_page) {
                break;
            }
            result_idx = idx;
        }
        return [result_idx, this.values[result_idx][0] === target_page];
    }
    setItemSingle(page, value) {
        let [idx, hit] = this.findIndex(page);
        if (hit) {
            this.values[idx][1] = value;
        }
        else {
            idx++;
            this.values.splice(idx, 0, [page, value]);
        }
        return [idx, hit];
    }
    mergeRedundant() {
        for (let i = this.values.length - 1; i >= 1; i--) {
            if (this.values[i][1] === this.values[i - 1][1]) {
                this.values.splice(i, 1);
            }
        }
    }
    toJSON() {
        return { "type": "PageProperty", "values": this.values };
    }
    static fromJSON(obj) {
        return new PageProperty(obj.values);
    }
    toString() {
        return `PageProperty(${JSON.stringify(this.values)})`;
    }
    valueOnPage(target_page) {
        let result;
        for (let [page, v] of this.values) {
            if (page > target_page) {
                break;
            }
            result = v;
        }
        return result;
    }
}
export function PagePropertyOrValueToPageProperty(propertyValue) {
    if (propertyValue instanceof PageProperty) {
        return propertyValue;
    }
    else {
        return new PageProperty(propertyValue);
    }
}
export function initialPagePropertyValue(propertyValue, defaultValue, propertyName, context) {
    if (propertyValue) {
        return PagePropertyOrValueToPageProperty(propertyValue);
    }
    else if (defaultValue !== undefined) {
        return new PageProperty(defaultValue);
    }
    else {
        throw TypeError(`Missing property ${propertyName}${context}`);
    }
}
