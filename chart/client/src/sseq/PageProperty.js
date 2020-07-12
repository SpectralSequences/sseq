import { INFINITY } from "../infinity.js";
export class PageProperty {
    constructor(value){
        if(value.constructor === Array){
            this.values = value;
        } else {
            this.values = [[-INFINITY, value]];
        }
        return new Proxy(this, {
            get : (obj, key) => {
                if (typeof(key) === 'string' && (Number.isInteger(Number(key)))){
                    return obj.valueOnPage(key);
                } else {
                    return obj[key];
                }
            },
            set : (obj, key, value) => {
                const newKey = (key || '').toString()
                    .replace(/\s/g, '')  // Remove all whitespace.
                    .replace(/,/g, ':')  // Replace commas with colons.
                if(/^(-?\d+)$/.test(newKey)) {
                    this.setItemSingle(Number.parseInt(key), value);
                    this.mergeRedundant();
                    return value;
                }
                
                // Handle slices.
                if(!/^(-?\d+)?(:(-?\d+)?)?$/.test(newKey)) {
                    return Reflect["set"](obj, key, value);
                }
                let [start, stop] = newKey.split(':').map(part => part.length ? Number.parseInt(part) : undefined);
                start = start || -INFINITY;
                stop = stop || INFINITY;
                let orig_value = this.valueOnPage(stop);
                let [start_idx, hit_start] = this.setItemSingle(start, value);
                let [end_idx, hit_end] = this.findIndex(stop);
                if(!hit_end && stop < INFINITY){
                    [end_idx, ] = this.setItemSingle(stop, orig_value)
                }
                if(stop == INFINITY){
                    end_idx ++;
                }
                this.values.splice(start_idx + 1, end_idx - start_idx - 1);
                this.mergeRedundant();
            }
        })
    }

    findIndex(target_page){
        let result_idx;
        for(let idx = 0; idx < this.values.length; idx ++){
            let [page, value] = this.values[idx]
            if(page > target_page){
                break
            }
            result_idx = idx 
        }
        return [result_idx, this.values[result_idx][0] === target_page]
    }

    setItemSingle(page, value) {
        let [idx, hit] = this.findIndex(page);
        if(hit){
            this.values[idx][1] = value;
        } else {
            idx ++;
            this.values.splice(idx, 0, [page, value]);
        }
        return [idx, hit];
    }

    mergeRedundant(){
        for(let i = this.values.length - 1; i >= 1; i--){
            if(this.values[i][1] === this.values[i-1][1]){
                this.values.splice(i, 1);
            }
        }
    }

    toJSON(){
        return {"type" : "PageProperty", "data" : this.values };
    }

    toString(){
        return `PageProperty(${JSON.stringify(this.values)})`;
    }

    valueOnPage(target_page){
        let result;
        for(let [page, v] of this.values){
            if(page > target_page){
                break
            }
            result = v;
        }
        return result;
    }

}