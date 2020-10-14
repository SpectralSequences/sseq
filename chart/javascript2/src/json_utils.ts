import { ChartClass } from "./ChartClass";
import { ChartStructline, ChartDifferential, ChartExtension } from "./ChartEdge";
import { PageProperty } from "./PageProperty";
import { SseqChart } from "./SseqChart";

export function parse(json : string){
    return JSON.parse(json, parseReviver);
}

interface Deserializable {
    fromJSON(obj : any) : any;
}

let jsonTypes : Map<string, Deserializable>;

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
    return jsonTypes;
}

function parseReviver(key : string, value : any) : any {
    if(typeof(value) !== "object" || value === null || !("type" in value)){
        return value;
    }
    let ty = getJsonTypes().get(value.type);
    if(!ty){
        throw TypeError(`Unknown type ${value.type}`);
    }
    return ty.fromJSON(value);
}