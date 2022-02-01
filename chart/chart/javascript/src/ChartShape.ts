'use strict';
import { Color } from './Color';

interface CharacterShape {
    ty: 'character';
    font: string;
    char: string;
    scale?: number;
    whole_shape: boolean;
}

interface CircledShape {
    ty: 'composed';
    operation: 'circled';
    padding: number;
    num_circles: number;
    circle_gap?: number;
    include_background: boolean;
    innerShape?: Shape;
}

interface BoxedShape {
    ty: 'composed';
    operation: 'boxed';
    padding: number;
    include_background: boolean;
    innerShape?: Shape;
}

interface DiacriticShape {
    ty: 'diacritic';
    diacritic: string;
    innerShape: Shape;
}

export type Shape =
    | CharacterShape
    | CircledShape
    | BoxedShape
    | DiacriticShape
    | { ty: 'empty' };
