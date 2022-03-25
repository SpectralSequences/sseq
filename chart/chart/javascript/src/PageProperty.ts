import { INFINITY } from './infinity';
import { Walker } from './json_utils';

export class PageProperty<V> {
    values: [number, V][];

    static fromValue<T>(value: T): PageProperty<T> {
        return new PageProperty([[-INFINITY, value]]);
    }

    constructor(values: [number, V][]) {
        this.values = values;
        return new Proxy(this, {
            get: (obj, key) => {
                // key will either be a string or a symbol.
                // Number(key) works fine if it's a string, but if it's a symbol it throws a type error.
                // So first use key.toString.
                if (Number.isInteger(Number(key.toString()))) {
                    return obj.valueOnPage(key);
                } else {
                    //@ts-ignore
                    return obj[key];
                }
            },
            set: (obj, key, value) => {
                const newKey = (key || '')
                    .toString()
                    .replace(/\s/g, '') // Remove all whitespace.
                    .replace(/,/g, ':'); // Replace commas with colons.
                if (/^(-?\d+)$/.test(newKey)) {
                    this.setItemSingle(Number(key), value);
                    this.mergeRedundant();
                    return true;
                }

                // Handle slices.
                if (!/^(-?\d+)?(:(-?\d+)?)?$/.test(newKey)) {
                    return Reflect['set'](obj, key, value);
                }
                let [start, stop] = newKey
                    .split(':')
                    .map(part =>
                        part.length ? Number.parseInt(part) : undefined,
                    );
                start = start || -INFINITY;
                stop = stop || INFINITY;
                let orig_value = this.valueOnPage(stop);
                let [start_idx, hit_start] = this.setItemSingle(start, value);
                let [end_idx, hit_end] = this.findIndex(stop);
                if (!hit_end && stop < INFINITY) {
                    [end_idx] = this.setItemSingle(stop, orig_value);
                }
                if (stop == INFINITY) {
                    end_idx++;
                }
                this.values.splice(start_idx + 1, end_idx - start_idx - 1);
                this.mergeRedundant();
                return true;
            },
        });
    }

    findIndex(target_page: number): [number, boolean] {
        let result_idx: number;
        for (let idx = 0; idx < this.values.length; idx++) {
            let [page, value] = this.values[idx];
            if (page > target_page) {
                break;
            }
            result_idx = idx;
        }
        return [result_idx!, this.values[result_idx!][0] === target_page];
    }

    setItemSingle(page: number, value: V): [number, boolean] {
        let [idx, hit] = this.findIndex(page);
        if (hit) {
            this.values[idx][1] = value;
        } else {
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
        if(this.values.length === 1){
            let res = this.values[0][1];
            if(res && (res as any).toJSON){
                return (res as any).toJSON();
            }
            return this.values[0][1];
        }
        return { type: 'PageProperty', values: this.values };
    }

    static fromJSON(walker: Walker, obj: any): PageProperty<any> {
        return new PageProperty(obj.values);
    }

    toString() {
        return `PageProperty(${JSON.stringify(this.values)})`;
    }

    valueOnPage(target_page: any): V {
        let result: V;
        for (let [page, v] of this.values) {
            if (page > target_page) {
                break;
            }
            result = v;
        }
        return result!;
    }
}

export type PagePropertyOrValue<V> = V | PageProperty<V>;

export function pagePropertyOrValueToPageProperty<V>(
    propertyValue: PageProperty<V> | V,
): PageProperty<V> {
    if (propertyValue instanceof PageProperty) {
        return propertyValue;
    } else {
        return PageProperty.fromValue(propertyValue);
    }
}

export function initialPagePropertyValue<V>(
    propertyValue: PageProperty<V> | V | undefined | null,
    defaultValue: V,
    propertyName: string,
    context: string,
): PageProperty<V> {
    if (propertyValue !== undefined && propertyValue !== null) {
        return pagePropertyOrValueToPageProperty(propertyValue);
    } else if (defaultValue !== undefined) {
        return PageProperty.fromValue(defaultValue);
    } else {
        throw TypeError(`Missing property ${propertyName}${context}`);
    }
}

export function initialOptionalPagePropertyValue<V>(
    propertyValue: PageProperty<V | undefined> | V | undefined | null,
    propertyName: string,
    context: string,
): PageProperty<V | undefined> {
    if (propertyValue) {
        return pagePropertyOrValueToPageProperty(propertyValue);
    } else {
        // @ ts-expect-error
        return PageProperty.fromValue(undefined);
    }
}

export type PageProperties<T> = {
    [P in keyof T]: PageProperty<T[P]>;
};

export type PagePropertyOrValues<T> = {
    [P in keyof T]: PagePropertyOrValue<T[P]>;
};
