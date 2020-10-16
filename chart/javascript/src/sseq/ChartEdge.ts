import { ChartClass } from "./ChartClass";
import { 
    PageProperty, PagePropertyOrValue, 
    initialPagePropertyValue, PagePropertyOrValueToPageProperty 
} from "./PageProperty";
import { INFINITY } from "../infinity";
import { v4 as uuidv4 } from 'uuid';
import { BendSpecifier, Color, DashPattern, LineWidth } from "./Color";
import { SseqChart } from "./SseqChart";


export interface ChartEdgeConstructorArgs {
    source_uuid? : string;
    target_uuid? : string;
    type ? : string;
    uuid? : string;
    color? : PagePropertyOrValue<Color>;// = "default",
    dash_pattern? : PagePropertyOrValue<DashPattern>;// = "default",
    line_width? : PagePropertyOrValue<LineWidth>; //= "default",
    bend? : PagePropertyOrValue<BendSpecifier>; // = 0,
    user_data? : Map<string, any>;
}

export interface ChartDifferentialConstructorArgs extends ChartEdgeConstructorArgs {
    type? : "ChartDifferential";
    page : number;
}

export interface ChartStructlineConstructorArgs extends ChartEdgeConstructorArgs {
    type? : "ChartStructline";
    visible? : PagePropertyOrValue<boolean>;
}


export interface ChartExtensionConstructorArgs extends ChartEdgeConstructorArgs {
    type? : "ChartExtension";
}


export class ChartEdge {
    _sseq? : SseqChart;
    uuid : string;
    _source_uuid : string;
    _target_uuid : string;
    source? : ChartClass;
    target? : ChartClass;
    bend : PageProperty<BendSpecifier>;
    color : PageProperty<Color>;
    dash_pattern : PageProperty<DashPattern>;
    line_width : PageProperty<LineWidth>;
    user_data : Map<string, any>;
    // arrow_type : ArrowSpecifier; // TODO??

    constructor(kwargs : ChartEdgeConstructorArgs) {
        if(!kwargs.source_uuid){
            throw TypeError(`Missing mandatory argument "source_uuid"`);
        }
        if(!kwargs.target_uuid){
            throw TypeError(`Missing mandatory argument "target_uuid"`);
        }        
        let errorContext = "";
        this.uuid = kwargs.uuid || uuidv4();

        this._source_uuid = kwargs.source_uuid;
        this._target_uuid = kwargs.target_uuid;
        this.bend = initialPagePropertyValue(kwargs.bend, 0, "bend", errorContext);
        this.color = initialPagePropertyValue(kwargs.color, "black", "color", errorContext);
        this.dash_pattern = initialPagePropertyValue(kwargs.dash_pattern, 0, "dash_pattern", errorContext);
        this.line_width = initialPagePropertyValue(kwargs.line_width, 3, "line_width", errorContext);
        this.user_data = kwargs.user_data || new Map();
    }

    update(kwargs : ChartEdgeConstructorArgs){
        if(kwargs.source_uuid){
            if(this._source_uuid !== kwargs.source_uuid){
                throw TypeError(`Inconsistent values for "source_uuid".`);
            }
        }
        if(kwargs.target_uuid){
            if(this._target_uuid !== kwargs.target_uuid){
                throw TypeError(`Inconsistent values for "target_uuid".`);
            }
        }
        if(kwargs.uuid){
            if(this.uuid !== kwargs.uuid){
                throw TypeError(`Inconsistent values for "uuid".`);
            }
        }
        if(kwargs.type){
            if(kwargs.type !== this.constructor.name){
                throw TypeError(`Invalid value for "type"`)
            }
        }
        if(kwargs.color){
            this.color = PagePropertyOrValueToPageProperty(kwargs.color);
        }
        if(kwargs.dash_pattern){
            this.dash_pattern = PagePropertyOrValueToPageProperty(kwargs.dash_pattern);
        }
        if(kwargs.line_width){
            this.line_width = PagePropertyOrValueToPageProperty(kwargs.line_width);
        }
        if(kwargs.bend){
            this.bend = PagePropertyOrValueToPageProperty(kwargs.bend);
        }
        this.user_data = this.user_data || new Map();
    }

    delete(){
        this.source?.edges.delete(this);
        this.target?.edges.delete(this);
        this._sseq!.edges.delete(this.uuid);
    }

    _drawOnPageQ(pageRange : [number, number]) : boolean {
        throw new Error("This should be overridden...");
    }

    toJSON() : any {
        return {
            type : this.constructor.name,
            uuid : this.uuid,
            source_uuid : this._source_uuid,
            target_uuid : this._target_uuid,
            color : this.color,
            dash_pattern : this.dash_pattern,
            line_width : this.line_width,
            bend : this.bend,
            user_data : this.user_data
        };
    }
}

export class ChartDifferential extends ChartEdge {
    page : number;
    constructor(kwargs : ChartDifferentialConstructorArgs){
        super(kwargs);
        this.page = kwargs.page;
    }

    _drawOnPageQ(pageRange : [number, number]){
        return pageRange[0] === 0 || (pageRange[0] <= this.page && this.page <= pageRange[1]);
    }

    toJSON() : any {
        let result = super.toJSON();
        result.page = this.page;
        return result;
    }

    static fromJSON(obj : any) : ChartDifferential {
        return new ChartDifferential(obj);
    }
}

export class ChartStructline extends ChartEdge {
    visible : PageProperty<boolean>;
    constructor(kwargs : ChartStructlineConstructorArgs){
        super(kwargs);
        let errorContext = "";
        this.visible = initialPagePropertyValue(kwargs.visible, true, "visible", errorContext);
    }

    update(kwargs : ChartStructlineConstructorArgs){
        super.update(kwargs);
        if(kwargs.visible){
            this.visible = PagePropertyOrValueToPageProperty(kwargs.visible);
        }
    }

    _drawOnPageQ(pageRange : [number, number]) : boolean {
        return this.visible[pageRange[0]];
    }

    toJSON() : any {
        let result = super.toJSON();
        result.visible = this.visible;
        return result;
    }

    static fromJSON(obj : any) : ChartStructline {
        return new ChartStructline(obj);
    }    
}

export class ChartExtension extends ChartEdge {
    constructor(kwargs : ChartEdgeConstructorArgs){
        super(kwargs);
    }

    _drawOnPageQ(pageRange : [number, number]) : boolean {
        return pageRange[0] === INFINITY;
    }

    static fromJSON(obj : any) : ChartExtension {
        return new ChartExtension(obj);
    }    
}