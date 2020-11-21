import { ChartClass } from "./ChartClass";
import { ChartStructline, ChartDifferential, ChartExtension } from "./ChartEdge";
import { PageProperty } from "./PageProperty";
import { SseqChart } from "./SseqChart";

export function parse(json : string) : any {
    let result = JSON.parse(json);
    return new Walker().walk({"" : result}, "");
}

interface Deserializable {
    visit?(walker : Walker, obj : object) : void;
    fromJSON(walker : Walker, obj : object) : any;
}

let jsonTypes : Map<string, Deserializable>;

export class Walker {
    static getType(value : any) : Deserializable | undefined {
        if(typeof(value) !== "object" || value === null || !("type" in value)){
            return undefined;
        }
        let ty = getJsonTypes().get(value.type);
        if(!ty){
            throw TypeError(`Unknown type ${value.type}`);
        }
        return ty;
    }

    reviver(holder : object, key : string, value : any) : any {
        let ty = Walker.getType(value);
        if(!ty){
            return value;
        }
        let result = ty.fromJSON(this, value);
        return result;
    }

    visit(holder : object, key : string, value : any) : boolean {
        let ty = Walker.getType(value);
        if(!ty || !ty.visit){
            return true;
        }
        ty.visit(this, value);
        return false;
    }

    visitChildren(value : any, ignoreFields : string[] = []){
        if (value && typeof value === "object") {
            for (let key in value) {
                if (Object.prototype.hasOwnProperty.call(value, key) && !ignoreFields.includes(key)) {
                    let v = this.walk(value, key);
                    if (v !== undefined) {
                        value[key] = v;
                    } else {
                        delete value[key];
                    }
                }
            }
        }
    }

    walk(holder : any, key : any) {
        let value = holder[key];
        if(this.visit(holder, key, value)){
            this.visitChildren(value);
        }
        return this.reviver(holder, key, value);
    }
}



function getJsonTypes(){
    if(jsonTypes){
        return jsonTypes;
    }
    jsonTypes = new Map();
    for(let t of [            
        SseqChart,
        ChartClass, ChartStructline, ChartDifferential, ChartExtension,
        PageProperty
    ]){
        jsonTypes.set(t.name, t);
    }
    let trivialDeserializer = { fromJSON : (walker : Walker, obj : object) => { delete obj["type"]; return obj }};
    jsonTypes.set("ArrowTip", trivialDeserializer);
    jsonTypes.set("Shape", trivialDeserializer);
    jsonTypes.set("SignalDict", trivialDeserializer);
    jsonTypes.set("SignalList", { fromJSON : (walker : Walker, obj : object) => obj["list"] });
    jsonTypes.set("Color", { fromJSON : (walker : Walker, obj : object) => obj["color"] });
    return jsonTypes;
}

