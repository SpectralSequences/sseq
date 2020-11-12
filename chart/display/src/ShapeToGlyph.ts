import { Glyph, GlyphBuilder, Vec4 as RustColor } from "./display_backend/pkg/sseq_display_backend";
import { Shape } from "./chart/lib";

export function shapeToGlyph(shape : Shape, tolerance : number, line_width : number) : Glyph {
    return shapeToGlyphBuilder(shape).build(tolerance, line_width);
}

// Recursive helper function
function shapeToGlyphBuilder(shape : Shape) : GlyphBuilder {
    switch (shape.ty) {
        case "character":
            if(shape.font === "stix"){
                return GlyphBuilder.from_stix(shape.char, shape.whole_shape);
            } else {
                throw Error("Not implemented");
            }
        case "empty":
            return GlyphBuilder.empty();
        case "composed":
            let builder = shapeToGlyphBuilder(shape.innerShape || { ty : "empty" });
            switch(shape.operation){
                case "circled":
                    builder.circled(shape.padding, shape.num_circles, shape.circle_gap || 0, shape.include_background);
                    break;
                case "boxed":
                    builder.boxed(shape.padding, shape.include_background);
                    break;
                default:
                    throw Error("Unknown composition operation.");
            }
            return builder;
        case "diacritic":
            throw Error("Not implemented.")
    }
}