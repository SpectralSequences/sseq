import StringifyingMap from "../StringifyingMap.js";
import { ChartClass } from "./ChartClass";
import { ChartDifferential, ChartStructline, ChartExtension } from "./ChartEdge";

import { renderLatex } from "../interface/Latex.js";

import * as EventEmitter from "events";



function check_argument_is_integer(name, value){
    if(!Number.isInteger(value)) {
        throw TypeError(`Argument "${name}" is ${x} which is not an integer. "${name}" is expected to be an integer.`);
    }
}

/**
 * Adds an entry to a map keys ==> lists.
 * If the current key isn't present in the map, add an empty list first.
 * @param dictionary The dictionary of lists to add the entry to
 * @param key
 * @param value
 */
function add_to_dictionary_of_lists(dictionary, key,value){
    if(!dictionary.has(key)){
        dictionary.set(key, []);
    }
    dictionary.get(key).push(value);
}

function filter_dictionary_of_lists(dictionary, key,callback){
    if(!dictionary.has(key)){
        dictionary.set(key, []);
    }
    dictionary.set(dictionary.get(key).filter(callback));
}

export class SpectralSequenceChart extends EventEmitter {
    constructor() {
        super();
        this.offset_size = 8;
        this.min_class_size = 1;
        this.max_class_size = 3;
        this.class_scale = 10;
        this.highlightScale = 2;
        this.defaultClassStrokeColor = "black";
        this.defaultClassFillColor = "black";
        this.highlightColor = "orange";
        this.bidegreeDistanceScale = 1;
        this.mouseoverScale = 2; // How much bigger should the mouseover region be than the clas itself?
        this.classes_by_degree = new StringifyingMap();
        this.classes = [];
        this.edges = [];

        this.page_list = [2];
        this.min_page_idx = 0;

        this.x_range = [0, 10];
        this.y_range = [0, 10];
        this.initial_x_range = [0, 10];
        this.initial_y_range = [0, 10];
        this.next_uuid = 0;
    }

    static from_JSON(json) {
        if(!json){
            throw ReferenceError("json is undefined");
        }
        let chart = new SpectralSequenceChart();

        if(json.classes === undefined) {
            throw ReferenceError("json.classes is undefined.");
        }

        if(json.edges === undefined) {
            throw ReferenceError("json.edges is undefined.");
        }
        
        // Make sure to assign fields to chart first in case they are used in process of add_class, add_edge.
        Object.assign(chart, json);

        let json_classes = chart.classes;
        let json_edges = chart.edges;
        chart.classes = {};
        chart.edges = {};
        

        for(let [id, c] of Object.entries(json_classes)){ // in iterates over object keys.
            chart.classes[id] = chart.add_class(c);
        }
        for(let [id, e] of Object.entries(json_edges)){
            chart.edges[id] = chart.add_edge(e)
        }
        
        return chart;
    }

    classes_in_bidegree(x, y) {
        if(x.constructor === Array) {
            if(y !== undefined) {
                throw Error("If first argument is an array, second argument should be undefined.");
            }
            if(x.length != 2){
                throw Error("If first argument is an array, it should have length 2.");
            }
            [x, y] = x;
        }
        check_argument_is_integer("x", x);
        check_argument_is_integer("y", y);
        if(!this.classes_by_degree.has([x,y])){
            return [];
        }
        return this.classes_by_degree.get([x, y]);
    }

    class_by_index(x, y, idx){
        check_argument_is_integer("idx", idx);
        let classes = this.classes_in_bidegree(x, y);
        if(idx >= classes.length) {
            throw Error(`Fewer than ${idx} classes exist in bidegree (${x}, ${y}).`);
        }
        return classes[idx];
    }

    add_class(kwargs) {
        let c = new ChartClass(this, kwargs);
        if("uuid" in kwargs){
            c.uuid = kwargs["uuid"]
        } else {
            c.uuid = this.next_uuid;
            this.next_uuid++;
        }
        let degree = [c.x, c.y];
        this.classes[c.uuid] = c;
        filter_dictionary_of_lists(this.classes_by_degree, degree, c => c._valid);
        if(c.idx === undefined){
            c.idx = this.classes_by_degree.get(degree).length;
        }
        add_to_dictionary_of_lists(this.classes_by_degree, degree, c);
        this.emit("class-added", c);
        this.emit("update");
        return c;
    }

    delete_class(c){
        delete this.classes[c.uuid];
        throw Error("Not implemented");
    }

    add_edge(kwargs) {
        let edge_type = kwargs["type"];
        kwargs.source = this.classes[kwargs.source];
        kwargs.target = this.classes[kwargs.target];
        let e;
        switch(edge_type) {
            case ChartDifferential.name:
                e = this.add_differential(kwargs);
                break;
            case ChartStructline.name:
                e = this.add_structline(kwargs);
                break;
            case ChartExtension.name:
                e = this.add_extension(kwargs);
                break;
            default:
                throw TypeError(`Argument "type" expected to contain one of "${ChartDifferential.name}" \
                                 "${ChartStructline.name}", or "${ChartExtension.name}", not "${edge_type}".`);
        }
        if(kwargs.color){
            Object.assign(e, kwargs);
        }
        return e;
    }

    delete_edge(e){
        if(!(e.uuid in this.edges)){
            console.error("Failed to delete edge", e);
            throw Error(`Failed to delete edge!`);
        }
        delete this.edges[e.uuid];
    }

    add_differential(kwargs) {
        let e = new ChartDifferential(kwargs);
        if("uuid" in kwargs){
            e.uuid = kwargs["uuid"];
        } else {
            e.uuid = this.next_uuid;
            this.next_uuid++;
        }
        this.edges[e["uuid"]] = e;
        this.emit("differential-added", e);
        this.emit("edge-added", e);
        this.emit("update");
        return e;
    }

    add_structline(kwargs) {
        let e = new ChartStructline(kwargs);
        if("uuid" in kwargs){
            e.uuid = kwargs["uuid"];
        } else {
            e.uuid = this.next_uuid;
            this.next_uuid++;
        }
        this.edges[e.uuid] = e;
        this.emit("structline-added", e);
        this.emit("edge-added", e);
        this.emit("update");
        return e;
    }

    add_extension(kwargs) {
        let e = new ChartExtension(kwargs);
        if("uuid" in kwargs){
            e.uuid = kwargs["uuid"];
        } else {
            e.uuid = this.next_uuid;
            this.next_uuid++;
        }
        this.edges[e.uuid] = e;
        this.emit("extension-added", e);
        this.emit("edge-added", e);
        this.emit("update");
        return e;    
    }

    /**
     * Gets the tooltip for the current class on the given page (currently ignores the page).
     * @param c
     * @param page
     * @returns {string}
     */
    getClassTooltip(c, page) {
        let tooltip = c.getNameCoord(page);
        // let extra_info = Tooltip.toTooltipString(c.extra_info, page);

        if(extra_info) {
            tooltip += extra_info;
        }

        return tooltip;
    }

    getClassTooltipHTML(c, page) {
        return renderLatex(this.getClassTooltip(c,page));
    }

    getElementsToDraw(page, xmin, xmax, ymin, ymax) {
        // Util.checkArgumentsDefined(SpectralSequenceChart.prototype.getDrawnElements, arguments);
        let pageRange;
        // TODO: clean up pageRange. Probably we should always pass pages as pairs?
        if(Array.isArray(page)) {
            pageRange = page;
            page = page[0];
        } else {
            pageRange = [page, page];
        }
        let display_classes = Object.values(this.classes).filter(c => {
            c._displayed = c && !c.invalid && c._inRangeQ(xmin, xmax, ymin, ymax) && c._drawOnPageQ(page);
            return c._displayed;
        });

        // Display edges such that
        // 1) e is a valid edge
        // 2) e is supposed to be drawn on the current pageRange.
        // 3) e.source and e.target are supposed to be drawn on the current pageRange
        // 4) At least one of the source or target is in bounds.
        let display_edges = Object.values(this.edges).filter(e =>
            e && !e.invalid && 
            e._drawOnPageQ(pageRange)
            && e._source._drawOnPageQ(page) 
            && e._target._drawOnPageQ(page)
            && (e._source._inRangeQ(xmin, xmax, ymin, ymax) || e._target._inRangeQ(xmin, xmax, ymin, ymax))
        );

        // We need to go back and make sure that for every edge we are planning to  draw, we draw both its source and
        // target even if one of them is out of bounds. Check for out of bounds sources / targets and add them to the
        // list of edges to draw.
        for (let e of display_edges) {
            if (!e._source._displayed) {
                display_classes.push(e._source);
                e._source._displayed = true;
            }
            if (!e._target._displayed) {
                e._target._displayed = true;
                display_classes.push(e._target);
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
}