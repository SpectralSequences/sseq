"use strict";
import { Color } from "./Color";

interface CharacterShape {
    ty: "character";
    font : string;
    char : string;
}

interface BoundedShape {
    ty : "composed";
    operation : "circled" | "boxed";
    padding : number;
    innerShape? : Shape;
}

interface DiacriticShape {
    ty : "diacritic";
    diacritic : string;
    innerShape : Shape;
}

export type Shape = CharacterShape | BoundedShape | DiacriticShape | { ty : "empty" };

interface NormalNode {
    shape : Shape;
    foreground : Color;
    stroke : Color;
    fill : Color;
    strokeThickness? : number;
};
export const DefaultNode = "DefaultNode";

export type Node = NormalNode | (typeof DefaultNode);