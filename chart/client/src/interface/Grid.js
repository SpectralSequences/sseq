const GridEnum = Object.freeze({ go : 1, chess : 2 });

export class GridElement extends HTMLElement {
    get clipped(){
        return true;
    }

    constructor(){
        super();
        this.gridStyle = GridEnum.go;
        this.gridColor = "#c6c6c6";
        this.gridStrokeWidth = 0.3;
    }

    paint(disp, context){
        context.strokeStyle = this.gridColor;
        context.lineWidth = this.gridStrokeWidth;
        this._updateGridStep(disp);
        switch(this.gridStyle){
            case GridEnum.go:
                this._drawGoGrid(disp, context);
                break;
            case GridEnum.chess:
                this._drawChessGrid(disp, context);
                break;
            default:
                throw Error("Undefined grid type.");
                break;
        }
    }


    _updateGridStep(disp){
        // TODO: This 70 is a magic number. Maybe I should give it a name?
        let xTicks = disp.xScale.ticks(disp._canvasWidth / 70);
        let yTicks = disp.yScale.ticks(disp._canvasHeight / 70);

        let xTickStep = Math.ceil(xTicks[1] - xTicks[0]);
        let yTickStep = Math.ceil(yTicks[1] - yTicks[0]);
        

        if(this.manualxGridStep){
            this.xGridStep = this.manualxGridStep;
        } else {
            this.xGridStep = (Math.floor(xTickStep / 5) === 0) ? 2 : Math.floor(xTickStep / 5);
        }
        if(this.manualyGridStep){
            this.yGridStep = this.manualxGridStep;
        } else {
            this.yGridStep = (Math.floor(yTickStep / 5) === 0) ? 2 : Math.floor(yTickStep / 5);
        }
        // // TODO: This is an ad-hoc modification requested by Danny to ensure that the grid boxes are square.
        // // Probably it's a useful thing to be able to have square grid boxes, how do we want to deal with this?
        // if(this.sseq.squareAspectRatio){
        //     this.xGridStep = 1;
        //     this.yGridStep = this.xGridStep;
        // }
    }


    _drawGoGrid(disp, context) {
        this._drawGridWithOffset(disp, context, 0, 0);
    }

    _drawChessGrid(disp, context) {
        this._drawGridWithOffset(disp, context, 0.5, 0.5);
    }

    _drawGridWithOffset(disp, context, xoffset, yoffset){
        context.beginPath();
        for (let col = Math.floor(disp.xmin / this.xGridStep) * this.xGridStep - xoffset; col <= disp.xmax; col += this.xGridStep) {
            context.moveTo(disp.xScale(col), 0);
            context.lineTo(disp.xScale(col), disp._clipHeight);
        }
        context.stroke();

        context.beginPath();
        for (let row = Math.floor(disp.ymin / this.yGridStep) * this.yGridStep - yoffset; row <= disp.ymax; row += this.yGridStep) {
            context.moveTo(disp._leftMargin, disp.yScale(row));
            context.lineTo(disp._canvasWidth - disp._rightMargin, disp.yScale(row));
        }
        context.stroke();
    }
}
customElements.define('sseq-grid', GridElement);