export class PageProperty {
    constructor(values){
        this.values = values;
        return new Proxy(this, {
            get : (obj, key) => {
                if (typeof(key) === 'string' && (Number.isInteger(Number(key)))){
                    return obj.valueOnPage(key);
                } else {
                    return obj[key];
                }
            }
        })
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