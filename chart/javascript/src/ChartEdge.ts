import { ChartClass } from "./ChartClass";
import { 
    PageProperty, PagePropertyOrValue, 
    initialPagePropertyValue, initialOptionalPagePropertyValue,
    pagePropertyOrValueToPageProperty,
    PageProperties, PagePropertyOrValues,
} from "./PageProperty";
import { INFINITY } from "./infinity";
import { v4 as uuidv4 } from 'uuid';
import { Color, DashPattern } from "./Color";
import { SseqChart, MessageUpdate } from "./SseqChart";
import { Walker } from "./json_utils"

const DefaultLineWidth : number = 3;

export interface ArrowTip {
    arrowName : string;

}

export interface EdgeStyle {
    start_tip : ArrowTip | undefined;
    end_tip : ArrowTip | undefined;
    bend : number;
    color : Color;
    dash_pattern : DashPattern
    line_width : number;
    visible : boolean;
}

export interface ChartEdgeConstructorArgs {
    source_uuid? : string;
    target_uuid? : string;
    type ? : string;
    uuid? : string;
    user_data? : Map<string, any>;
}


export interface ChartStructlineConstructorArgs extends ChartEdgeConstructorArgs, PagePropertyOrValues<EdgeStyle> {
    type? : "ChartStructline";
}


export interface ChartDifferentialConstructorArgs extends ChartEdgeConstructorArgs, EdgeStyle {
    page : number;
    type? : "ChartDifferential";
}

export interface ChartExtensionConstructorArgs extends ChartEdgeConstructorArgs, EdgeStyle {
    type? : "ChartExtension";    
}


export abstract class ChartEdge {
    _sseq? : SseqChart;
    uuid : string;
    source_uuid : string;
    target_uuid : string;
    source? : ChartClass;
    target? : ChartClass;
    user_data : Map<string, any>;

    constructor(kwargs : ChartEdgeConstructorArgs) {
        if(!kwargs.source_uuid){
            throw TypeError(`Missing mandatory argument "source_uuid"`);
        }
        if(!kwargs.target_uuid){
            throw TypeError(`Missing mandatory argument "target_uuid"`);
        }        
        let errorContext = "";
        this.uuid = kwargs.uuid || uuidv4();

        this.source_uuid = kwargs.source_uuid;
        this.target_uuid = kwargs.target_uuid;
        this.user_data = kwargs.user_data || new Map();
    }

    abstract update(kwargs : MessageUpdate<ChartEdge>) : void;

    protected _baseUpdate(msg : MessageUpdate<ChartEdge>){
        let kwargs = msg.update_fields;
        // if(kwargs.source_uuid){
        //     if(this._source_uuid !== kwargs.source_uuid){
        //         throw TypeError(`Inconsistent values for "source_uuid".`);
        //     }
        // }
        // if(kwargs.target_uuid){
        //     if(this._target_uuid !== kwargs.target_uuid){
        //         throw TypeError(`Inconsistent values for "target_uuid".`);
        //     }
        // }
        if(kwargs.uuid){
            if(this.uuid !== kwargs.uuid){
                throw TypeError(`Inconsistent values for "uuid".`);
            }
        }
        // if(kwargs.type){
        //     if(kwargs.type !== this.constructor.name){
        //         throw TypeError(`Invalid value for "type"`)
        //     }
        // }
        this.user_data = this.user_data || new Map();
    }

    abstract getEdgeStyle(page : number) : EdgeStyle;

    delete(){
        this.source?.edges.delete(this);
        this.target?.edges.delete(this);
        this._sseq!.edges.delete(this.uuid);
    }

    abstract drawOnPageQ(pageRange : [number, number]) : boolean;

    toJSON() : any {
        return {
            type : this.constructor.name,
            uuid : this.uuid,
            source_uuid : this.source_uuid,
            target_uuid : this.target_uuid,
        };
    }
}


export class ChartStructline extends ChartEdge {
    start_tip : PageProperty<ArrowTip | undefined>;
    end_tip : PageProperty<ArrowTip | undefined>;
    bend : PageProperty<number>;
    color : PageProperty<Color>;
    dash_pattern : PageProperty<DashPattern>;
    line_width : PageProperty<number>;
    visible : PageProperty<boolean>;
    constructor(kwargs : ChartStructlineConstructorArgs){
        super(kwargs);
        let errorContext = "";
        this.visible = initialPagePropertyValue(kwargs.visible, true, "visible", errorContext);
        this.start_tip = initialOptionalPagePropertyValue(kwargs.start_tip, "start_tip", errorContext);
        this.end_tip = initialOptionalPagePropertyValue(kwargs.end_tip, "end_tip", errorContext);
        this.bend = initialPagePropertyValue(kwargs.bend, 0, "bend", errorContext);
        this.color = initialPagePropertyValue(kwargs.color, [0, 0, 0, 1], "color", errorContext);
        this.dash_pattern = initialPagePropertyValue(kwargs.dash_pattern, [], "dash_pattern", errorContext);
        this.line_width = initialPagePropertyValue(kwargs.line_width, 3, "line_width", errorContext);
    }

    update(msg : MessageUpdate<ChartStructline>){
        super._baseUpdate(msg);
        let kwargs = msg.update_fields;
        if(kwargs.visible){
            this.visible = pagePropertyOrValueToPageProperty(kwargs.visible);
        }

        if(kwargs.color){
            this.color = pagePropertyOrValueToPageProperty(kwargs.color);
        }
        if(kwargs.dash_pattern){
            this.dash_pattern = pagePropertyOrValueToPageProperty(kwargs.dash_pattern);
        }
        if(kwargs.line_width){
            this.line_width = pagePropertyOrValueToPageProperty(kwargs.line_width);
        }
        if(kwargs.bend){
            this.bend = pagePropertyOrValueToPageProperty(kwargs.bend);
        }        
    }

    drawOnPageQ(pageRange : [number, number]) : boolean {
        return this.visible[pageRange[0]];
    }

    getEdgeStyle(page : number) : EdgeStyle {
        return {
            start_tip : this.start_tip[page],
            end_tip : this.end_tip[page],
            bend : this.bend[page],
            color : this.color[page],
            dash_pattern : this.dash_pattern[page],
            line_width : this.line_width[page],
            visible : this.visible[page]
        };
    }

    toJSON() : any {
        let result = super.toJSON();
        let edge_style : PageProperties<EdgeStyle> & {user_data : any} = {
            visible : this.visible,
            color : this.color,
            dash_pattern : this.dash_pattern,
            line_width : this.line_width,
            bend : this.bend,
            start_tip : this.start_tip,
            end_tip : this.end_tip,
            user_data : this.user_data,
        };
        Object.assign(result, edge_style);
        return result;
    }

    static fromJSON(walker : Walker, obj : any) : ChartStructline {
        return new ChartStructline(obj);
    }    
}

abstract class SinglePageChartEdge extends ChartEdge {
    start_tip? : ArrowTip;
    end_tip? : ArrowTip;
    bend : number;
    color : Color;
    dash_pattern : DashPattern;
    line_width : number;
    visible : boolean;    
    constructor(kwargs : ChartEdgeConstructorArgs & Partial<EdgeStyle>){
        super(kwargs);
        this.start_tip = kwargs.start_tip;
        this.end_tip = kwargs.end_tip;
        this.bend = kwargs.bend || 0;
        this.color = kwargs.color || [0, 0, 0, 1];
        this.dash_pattern = kwargs.dash_pattern || [];
        this.line_width = kwargs.line_width || DefaultLineWidth;
        this.visible = kwargs.visible || true;
    }

    _baseUpdate(msg : MessageUpdate<SinglePageChartEdge>){
        let kwargs = msg.update_fields;
        if(kwargs.visible){
            this.visible = kwargs.visible;
        }
        if(kwargs.color){
            this.color = kwargs.color;
        }
        if(kwargs.dash_pattern){
            this.dash_pattern = kwargs.dash_pattern;
        }
        if(kwargs.line_width){
            this.line_width = kwargs.line_width;
        }
        if(kwargs.bend){
            this.bend = kwargs.bend;
        }      
    }


    getEdgeStyle(_page : number) : EdgeStyle {
        return {
            start_tip : this.start_tip,
            end_tip : this.end_tip,
            bend : this.bend,
            color : this.color,
            dash_pattern : this.dash_pattern,
            line_width : this.line_width,
            visible : this.visible
        };
    }    

    toJSON() : any {
        let result = super.toJSON();
        Object.assign(result, this.getEdgeStyle(0));
        return result;
    }
}


export class ChartDifferential extends SinglePageChartEdge {
    page : number;
    constructor(kwargs : ChartDifferentialConstructorArgs){
        super(kwargs);
        this.page = kwargs.page;
    }

    update(kwargs : MessageUpdate<ChartDifferential>){
        super._baseUpdate(kwargs);
    }

    drawOnPageQ(pageRange : [number, number]){
        return pageRange[0] === 0 || (pageRange[0] <= this.page && this.page <= pageRange[1]);
    }  

    toJSON() : any {
        let result = super.toJSON();
        result.page = this.page;
        return result;
    }

    static fromJSON(walker : Walker, obj : any) : ChartDifferential {
        return new ChartDifferential(obj);
    }
}

export class ChartExtension extends SinglePageChartEdge {
    constructor(kwargs : ChartExtensionConstructorArgs){
        super(kwargs);
    }

    update(kwargs : MessageUpdate<ChartExtension>){
        super._baseUpdate(kwargs);
    }

    drawOnPageQ(pageRange : [number, number]) : boolean {
        return pageRange[0] === INFINITY;
    }

    static fromJSON(walker : Walker, obj : any) : ChartExtension {
        return new ChartExtension(obj);
    }    
}