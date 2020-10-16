"use strict";
import { Color } from "./Color";
export type Shape = any;

export interface ChartNode {
    shape : Shape,
    scale : number,
    color : Color,
    stroke : Color,
    fill : Color,
    opacity : number
};

export interface DrawParams {
    shape : Shape,
    size : number,
    x : number,
    y : number,
    fillQ : boolean,
    strokeQ : boolean,
    node : ChartNode
};


export class ChartShape {
    static draw(context : CanvasRenderingContext2D, params : DrawParams) {
        let shape;
        if(params.shape === "default"){
            shape = DEFAULT_SHAPE;
        } else {
            shape = Shapes[params.shape.type];
        }
        return shape.draw.bind(shape)(context, params);
    }
    
    static outline(context : CanvasRenderingContext2D | Path2D, params : DrawParams) {
        let shape;
        if(params.shape === "default"){
            shape = DEFAULT_SHAPE;
        } else {
            shape = Shapes[params.shape.type];
        }
        return shape.outline.bind(shape)(context, params);
    }

    static fillStrokeContext(context : CanvasRenderingContext2D, params : DrawParams) {
        if(params.strokeQ){
            context.stroke();
        }
        if(params.fillQ){
            context.fill();
        }
    }
}


let Shapes = {};

Shapes["text"] = {
    outline : function(context : CanvasRenderingContext2D | Path2D, params : DrawParams) {
        context.moveTo(params.x, params.y);
        context.arc(params.x, params.y, params.size * 0.1, 0, 2*Math.PI);
    },
    draw : function(context : CanvasRenderingContext2D, params : DrawParams) {
        let text = params.shape.text;
        let fontFace = params.shape.font;
        context.save();
        context.font = `${params.size * 0.5}px ${fontFace}`;
        console.log("font:", context.font);
        context.textAlign = "center";
        context.textBaseline = "middle";
        context.fillText(text, params.x, params.y);
        context.restore();
    }
}


Shapes["circle"] = {
    outline : function(context : CanvasRenderingContext2D | Path2D, params : DrawParams) {
        context.moveTo(params.x, params.y);
        context.arc(params.x, params.y, params.size * 0.1, 0, 2*Math.PI);
    },
    draw: function(context : CanvasRenderingContext2D, params : DrawParams) {
        // console.log("shape_draw");
        this.outline(context, params);
        ChartShape.fillStrokeContext(context, params);
    }
}


Shapes["circlen"] = {
    outline : function(context : CanvasRenderingContext2D | Path2D, params : DrawParams) {
        context.arc(params.x, params.y, params.size * 0.1, 0, 2*Math.PI);
    },
    draw: function(context : CanvasRenderingContext2D, params : DrawParams) {
        this.outline(context, params);
        ChartShape.fillStrokeContext(context, params);
        
        context.textAlign = "center";
        context.fillStyle = "black";
        let fontsize = 0.15*params.size | 0;
        context.font = `${fontsize}px Arial`;
        context.fillText(params.shape.order, params.x, params.y + params.size*0.06);
    }
};

Shapes["square"] = {
    outline: function(context : CanvasRenderingContext2D | Path2D, params : DrawParams) {
        let x = params.x;
        let y = params.y;
        let size = params.size;
        let hwidth = 0.1 * size;
        context.rect(x - hwidth, y - hwidth, 2*hwidth, 2*hwidth);
    },
    
    draw : function(context : CanvasRenderingContext2D, params : DrawParams) {
        this.outline(context, params);
        ChartShape.fillStrokeContext(context, params);
    }
};

let DEFAULT_SHAPE = Shapes["circle"];


for(let k of Object.getOwnPropertyNames(Shapes)){
    Shapes[k].name = k;
}
