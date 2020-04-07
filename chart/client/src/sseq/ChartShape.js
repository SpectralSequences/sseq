"use strict";

class ChartShape {
    static draw(shape, ...rest) {
        return Shapes[shape].draw(...rest);
    }
}

exports.ChartShape = ChartShape;

let Shapes = {};

Shapes.circle = {
    draw: function(context, x, y, size, path2d=true) {
        context.beginPath();
        context.arc(x, y, size * 0.1, 0, 2*Math.PI);
        context.fill();
        context.stroke();

        let path = new Path2D();
        path.arc(x, y, size * 0.2, 0, 2 * Math.PI);
        return path;
    }
}


Shapes.circlen = {
    draw: function(context, x, y, size, node) {
        context.beginPath();
        context.arc(x, y, size * 0.1, 0, 2*Math.PI);
        context.fill();
        context.stroke();

        context.textAlign = "center";
        context.fillStyle = "black";
        let fontsize = 0.15*size | 0;
        context.font = `${fontsize}px Arial`;
        context.fillText(node.order, x, y + size*0.06);

        let path = new Path2D();
        path.arc(x, y, size * 0.2, 0, 2 * Math.PI);

        return path;
    }
};

Shapes.square = {
    draw: function(context, x, y, size) {
        let hwidth = 0.1 * size;

        context.beginPath();
        context.rect(x - hwidth, y - hwidth, 2*hwidth, 2*hwidth);
        context.fill();
        context.stroke();

        let path = new Path2D();
        path.rect(x - 2*hwidth, y - 2*hwidth, 4*hwidth, 4*hwidth);

        return path;
    }
}

for(let k of Object.getOwnPropertyNames(Shapes)){
    Shapes[k].name = k;
    exports[k] = Shapes[k];
}
