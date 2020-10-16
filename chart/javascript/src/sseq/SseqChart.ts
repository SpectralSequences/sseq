import StringifyingMap from "../StringifyingMap";
import { ChartClass, ChartClassConstructorArgs} from "./ChartClass";
import { 
    ChartDifferential, ChartStructline, ChartExtension, ChartEdge,
    ChartDifferentialConstructorArgs, ChartStructlineConstructorArgs, ChartExtensionConstructorArgs
} from "./ChartEdge";

import { renderLatex } from "../interface/Latex";

import { EventEmitter } from 'ee-ts';
import { INFINITY } from "../infinity";
import { uuidv4 } from "../interface/utils";
// import { assert } from "console";


function check_argument_is_integer(name : string, value : any) {
    if(!Number.isInteger(value)) {
        throw TypeError(`Argument "${name}" is ${value} which is not an integer. "${name}" is expected to be an integer.`);
    }
}

/**
 * Adds an entry to a map keys ==> lists.
 * If the current key isn't present in the map, add an empty list first.
 * @param dictionary The dictionary of lists to add the entry to
 * @param key
 * @param value
 */
function add_to_dictionary_of_lists<K, V>(dictionary : StringifyingMap<K, V[]>, key : K, value : V){
    if(!dictionary.has(key)){
        dictionary.set(key, []);
    }
    dictionary.get(key)!.push(value);
}

function filter_dictionary_of_lists<K, V>(dictionary : StringifyingMap<K, V[]>, key : K, callback : (e : V) => boolean){
    if(!dictionary.has(key)){
        dictionary.set(key, []);
    }
    dictionary.set(key, dictionary.get(key)!.filter(callback));
}

interface MessageUpdateGlobal {
    chart_id : string;
    type : "SseqChart";
    command : "update";
    target_fields : Partial<SseqChart>;
}

interface MessageCreate<T> {
    chart_id : string;
    command : "create";
    target : T;
}

interface MessageUpdate<T> {
    chart_id : string;
    command : "update";
    target_uuid : string;
    update_fields : Partial<T>;
}

interface MessageDelete {
    chart_id : string;
    command : "delete";
    target_uuid : string;
}

interface MessageCreateClass extends MessageCreate<ChartClass> { type : "SseqClass"; }
interface MessageCreateStructline extends MessageCreate<ChartStructline> { type : "ChartStructline"; }
interface MessageCreateDifferential extends MessageCreate<ChartDifferential> { type : "ChartDifferential"; }
interface MessageCreateExtension extends MessageCreate<ChartExtension> { type : "ChartExtension"; }

interface MessageUpdateClass extends MessageUpdate<ChartClass> { type : "SseqClass"; }
interface MessageUpdateStructline extends MessageUpdate<ChartStructline> { type : "ChartStructline"; }
interface MessageUpdateDifferential extends MessageUpdate<ChartDifferential> { type : "ChartDifferential"; }
interface MessageUpdateExtension extends MessageUpdate<ChartExtension> { type : "ChartExtension"; }

interface MessageDeleteClass extends MessageDelete { type : "SseqClass"; }
interface MessageDeleteStructline extends MessageDelete { type : "ChartStructline"; }
interface MessageDeleteDifferential extends MessageDelete { type : "ChartDifferential"; }
interface MessageDeleteExtension extends MessageDelete { type : "ChartExtension"; }

type Message = MessageUpdateGlobal |
    MessageCreateClass | MessageCreateStructline | MessageCreateDifferential | MessageCreateExtension
    | MessageUpdateClass | MessageUpdateStructline | MessageUpdateDifferential | MessageUpdateExtension
    | MessageDeleteClass | MessageDeleteStructline | MessageDeleteDifferential | MessageDeleteExtension

interface Events {
    class_added( c : ChartClass) : void
    differential_added( e : ChartDifferential) : void
    structline_added( e : ChartStructline) : void
    extension_added( e : ChartExtension ) : void
    edge_added( e : ChartEdge) : void
    update() : void
}


export class SseqChart extends EventEmitter<Events> {
    readonly name : string = "";
    readonly uuid : string;
    offset_size : number = 8;
    min_class_size : number = 1;
    max_class_size : number = 3;
    class_scale : number = 10;
    highlightScale : number = 2;
    highlightColor = "orange";
    bidegreeDistanceScale = 1;
    mouseoverScale = 2; // How much bigger should the mouseover region be than the clas itself?
    defaultClassShape = {"type" : "circle"};
    defaultClassScale = 1;
    defaultClassStrokeColor = true;
    defaultClassFillColor = true;
    defaultClassColor = "black";
    initial_x_range : [number, number] = [0, 10];
    initial_y_range : [number, number] = [0, 10];
    x_range : [number, number] = [0, 10];
    y_range : [number, number] = [0, 10];
    page_list = [[2, 2], [INFINITY, INFINITY]];
    num_gradings = 2;
    x_projection = [1, 0];
    y_projection = [0, 1];
    _classes_by_degree : StringifyingMap<number[], ChartClass[]> = new StringifyingMap();
    classes : Map<string, ChartClass> = new Map();
    edges : Map<string, ChartEdge> = new Map();
    objects : Map<string, ChartClass | ChartEdge> = new Map();

    static charts : Map<string, SseqChart> = new Map();

    constructor(kwargs? : any) {
        super();
        this.uuid = uuidv4();
        SseqChart.charts.set(this.uuid, this);
        if(kwargs.name){
            this.name = kwargs.name;
        }
        if(kwargs.offset_size){
            this.offset_size = kwargs.offset_size;
        }
        if(kwargs.min_class_size){
            this.min_class_size = kwargs.min_class_size;
        }
        if(kwargs.max_class_size){
            this.max_class_size = kwargs.max_class_size;
        }
        if(kwargs.highlightScale){
            this.highlightScale = kwargs.highlightScale;
        }
        // bidegreeDistanceScale = 1;
        // mouseoverScale = 2; // How much bigger should the mouseover region be than the clas itself?
        // defaultClassShape = {"type" : "circle"};
        // defaultClassScale = 1;
        // defaultClassStrokeColor = true;
        // defaultClassFillColor = true;
        // defaultClassColor = "black";

        if(kwargs.initial_x_range){
            this.initial_x_range = kwargs.initial_x_range;
        }
        if(kwargs.initial_y_range){
            this.initial_y_range = kwargs.initial_y_range;
        }
        if(kwargs.x_range){
            this.x_range = kwargs.x_range;
        }
        if(kwargs.y_range){
            this.y_range = kwargs.y_range;
        }
        if(kwargs.page_list){
            this.page_list = kwargs.page_list;
        }
        if(kwargs.num_gradings){
            this.num_gradings = kwargs.num_gradings;
        }
        if(kwargs.classes){
            for(let c of kwargs.classes){
                this._commit_class(c);
            }
        }
        if(kwargs.edges){
            for(let c of kwargs.edges){
                this._commit_edge(c);
            }
        }        
    }

    static fromJSON(json : any) : any {   
        console.log(json);   
        return new SseqChart(json);
    }

    classes_in_degree(...args : number[]) : ChartClass[] {
        if(args.length !== this.num_gradings){
            throw TypeError(`Expected this.num_gradings = ${this.num_gradings} arguments to classes_in_degree.`);
        }
        args.forEach((v, idx) => check_argument_is_integer(`${idx}`, v));
        if(!this._classes_by_degree.has(args)){
            return [];
        }
        return this._classes_by_degree.get(args)!;
    }

    class_by_index(...x : number[]){
        if(x.length !== this.num_gradings + 1){
            throw TypeError(`Expected this.num_gradings + 1 = ${this.num_gradings + 1} arguments to classes_in_degree.`);
        }
        let idx = x.pop()!;
        check_argument_is_integer("idx", idx);
        let classes = this.classes_in_degree(...x);
        if(idx >= classes.length) {
            throw Error(`Fewer than ${idx} classes exist in degree (${x.join(", ")}).`);
        }
        return classes[idx];
    }

    add_class(kwargs : ChartClassConstructorArgs) {
        let c = new ChartClass(kwargs);
        this._commit_class(c);
        this.emit("class_added", c);
        this.emit("update");
        return c;
    }

    /** Common logic between add_class and deserialization of classes. **/
    _commit_class(c : ChartClass){
        if(c.degree.length !== this.num_gradings){
            throw TypeError(`Wrong number of gradings: degree {c.degree} has length {len(c.degree)} but num_gradings is {self.num_gradings}`)
        }
        c._sseq = this;
        let degree = c.degree!;
        this.classes.set(c.uuid, c);
        this.objects.set(c.uuid, c);
        if(!this._classes_by_degree.has(degree)){
            this._classes_by_degree.set(degree, []);
        }        
        // filter_dictionary_of_lists<number[], ChartClass>(this._classes_by_degree, degree, (c => c._valid) as (arg : ChartClass) => boolean);
        if(c.idx === undefined){
            c.idx = this._classes_by_degree.get(degree)!.length;
        }
        this._classes_by_degree.get(c.degree)!.push(c);
        c._updateProjection();
    }

    delete_class(c : ChartClass){
        throw Error("Not implemented"); // ??
        this.classes.delete(c.uuid);
    }

    delete_edge(e : ChartEdge){
        if(!this.edges.has(e.uuid)){
            console.error("Failed to delete edge", e);
            throw Error(`Failed to delete edge!`);
        }
        this.edges.delete(e.uuid);
    }

    /** Common logic between add_structline, add_differential, add_extension, and deserialization. */
    _commit_edge(e : ChartEdge){
        e._sseq = this;
        this.edges.set(e.uuid, e);
        this.objects.set(e.uuid, e);
        e.source = this.classes.get(e._source_uuid);
        e.target = this.classes.get(e._target_uuid);
        if(!e.source){
            throw Error(`No class with uuid ${e._source_uuid}`);
        }
        if(!e.target){
            throw Error(`No class with uuid ${e._target_uuid}`);
        }        
        e.source.edges.add(e);
        e.target.edges.add(e);
    }

    add_differential(kwargs : ChartDifferentialConstructorArgs) {
        let e = new ChartDifferential(kwargs);
        this._commit_edge(e);
        this.emit("differential_added", e);
        this.emit("edge_added", e);
        this.emit("update");
        return e;
    }

    add_structline(kwargs : ChartStructlineConstructorArgs) {
        let e = new ChartStructline(kwargs);
        this._commit_edge(e);
        this.emit("structline_added", e);
        this.emit("edge_added", e);
        this.emit("update");
        return e;
    }

    add_extension(kwargs : ChartExtensionConstructorArgs) {
        let e = new ChartExtension(kwargs);
        this._commit_edge(e);
        this.emit("extension_added", e);
        this.emit("edge_added", e);
        this.emit("update");
        return e;    
    }

    /**
     * Gets the tooltip for the current class on the given page (currently ignores the page).
     * @param c
     * @param page
     * @returns {string}
     */
    getClassTooltip(c : ChartClass, page : number) : string {
        let tooltip = c.getNameCoord(page);
        // let extra_info = Tooltip.toTooltipString(c.extra_info, page);
        let extra_info = "";
        if(extra_info) {
            tooltip += extra_info;
        }

        return tooltip;
    }

    getClassTooltipHTML(c : ChartClass, page : number) : string {
        return renderLatex(this.getClassTooltip(c,page));
    }

    getElementsToDraw(pageRange : [number, number], xmin : number, xmax : number, ymin : number, ymax : number) : [ChartClass[], ChartEdge[]] {
        // Util.checkArgumentsDefined(SpectralSequenceChart.prototype.getDrawnElements, arguments);
        // TODO: clean up pageRange. Probably we should always pass pages as pairs?
        let page = pageRange[0];
        let display_classes = Array.from(this.classes.values()).filter(c => {
            c._displayed = c 
                // && !c.invalid
                && c._inRangeQ(xmin, xmax, ymin, ymax) 
                && c._drawOnPageQ(page);
            return c._displayed;
        });

        // Display edges such that
        // 1) e is a valid edge
        // 2) e is supposed to be drawn on the current pageRange.
        // 3) e.source and e.target are supposed to be drawn on the current pageRange
        // 4) At least one of the source or target is in bounds.
        let display_edges = Array.from(this.edges.values()).filter(e =>
            e 
            // && !e.invalid
            && e._drawOnPageQ(pageRange)
            && e.source!._drawOnPageQ(page) 
            && e.target!._drawOnPageQ(page)
            && (e.source!._inRangeQ(xmin, xmax, ymin, ymax) || e.target!._inRangeQ(xmin, xmax, ymin, ymax))
        );

        // We need to go back and make sure that for every edge we are planning to  draw, we draw both its source and
        // target even if one of them is out of bounds. Check for out of bounds sources / targets and add them to the
        // list of edges to draw.
        for (let e of display_edges) {
            if (!e.source!._displayed) {
                display_classes.push(e.source!);
                e.source!._displayed = true;
            }
            if (!e.target!._displayed) {
                e.target!._displayed = true;
                display_classes.push(e.target!);
            }
        }

        for(let c of display_classes) {
            let node = c.getNode(page);
            if(node === undefined) {
                console.error("Undefined node for:", c);
                throw ReferenceError(`Undefined node on page ${page} for class: ${c}`);
            }
            c._node = node;
        }
        
        // for (let e of display_edges) {
            //     e.source_node = e.source.node;
        //     e.target_node = e.target.node;
        // }
        return [display_classes, display_edges];
    }

    toJSON() {
        return {
            type : this.constructor.name,
            name : this.name,
            initial_x_range : this.initial_x_range,
            initial_y_range : this.initial_y_range,
            x_range : this.x_range,
            y_range : this.y_range,
            page_list : this.page_list,
            num_gradings : this.num_gradings,
            x_projection : this.x_projection,
            y_projection : this.y_projection,
            classes : Array.from(this.classes.values()),
            edges : Array.from(this.edges.values())
            // offset_size : number = 8;
            // min_class_size : number = 1;
            // max_class_size : number = 3;
            // class_scale : number = 10;
            // highlightScale : number = 2;
            // highlightColor = "orange";
            // bidegreeDistanceScale = 1;
            // mouseoverScale = 2; // How much bigger should the mouseover region be than the clas itself?
            // defaultClassShape = {"type" : "circle"};
            // defaultClassScale = 1;
            // defaultClassStrokeColor = true;
            // defaultClassFillColor = true;
            // defaultClassColor = "black";
        }
    }

    handleMessage(msg : Message){
        switch(msg.command){
            case "create":
                if(msg.type === "SseqClass"){
                    this._commit_class(msg.target);
                } else {
                    this._commit_edge(msg.target);
                }
                return;
            case "update": {
                if(msg.type === "SseqChart"){

                    return;
                }
                const target = this.objects.get(msg.target_uuid);
                // assert(target && target.constructor.name === msg.type);
                throw Error("Not implemented");
                // target.update(msg);
                return;
            }
            case "delete": {
                let target = this.objects.get(msg.target_uuid);
                // assert(target && target.constructor.name === msg.type); // TODO: get an assert function
                target!.delete();
            }

        }
    }
}
