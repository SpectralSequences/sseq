import { Color } from './Color';
import { Shape } from './ChartShape';
import {
    PageProperty,
    PagePropertyOrValue,
    pagePropertyOrValueToPageProperty,
    initialPagePropertyValue,
} from './PageProperty';
import { v4 as uuidv4 } from 'uuid';
import { INFINITY } from './infinity';
import { SseqChart, MessageUpdate } from './SseqChart';
import { ChartEdge } from './ChartEdge';
import { Walker } from './json_utils';

function arrayEqual<T>(array1: T[], array2: T[]): boolean {
    return (
        array1.length === array2.length &&
        array1.every((array1_i, i) => array1_i === array2[i])
    );
}

export interface ChartClassConstructorArgs {
    degree?: Array<number>;
    type?: 'ChartClass';
    idx?: number;
    uuid?: string; // = "";
    name?: PagePropertyOrValue<string>; // = "";
    max_page?: number;
    shape?: PagePropertyOrValue<Shape>; // = "default";
    background_color?: PagePropertyOrValue<Color>; // = "default";
    border_color?: PagePropertyOrValue<Color>; // = "default";
    border_width: PagePropertyOrValue<number>;
    foreground_color?: PagePropertyOrValue<Color>; // = "default";
    scale?: PagePropertyOrValue<number>; // = 1;
    visible?: PagePropertyOrValue<boolean>; // = true;
    x_nudge?: PagePropertyOrValue<number>; // = 0,
    y_nudge?: PagePropertyOrValue<number>; // = 0,
    user_data?: Map<string, any>;
}

export class ChartClass {
    _sseq?: SseqChart;
    _valid: boolean = true;
    degree: number[];
    x?: number;
    y?: number;
    idx?: number;
    uuid: string;
    name: PageProperty<string>;
    max_page: number;
    visible: PageProperty<boolean>;
    x_nudge: PageProperty<number>;
    y_nudge: PageProperty<number>;
    scale: PageProperty<number>;
    shape: PageProperty<Shape>;
    background_color: PageProperty<Color>; // = "default";
    border_color: PageProperty<Color>; // = "default";
    border_width: PageProperty<number>;
    foreground_color: PageProperty<Color>; // = "default";
    _canvas_x?: number;
    _canvas_y?: number;
    user_data: Map<string, any>;
    edges: Set<ChartEdge> = new Set();
    extra_tooltip?: string;
    constructor(kwargs: ChartClassConstructorArgs) {
        if (!kwargs.degree) {
            throw new TypeError(
                `Mandatory constructor argument "degree" is missing.`,
            );
        }
        this.degree = kwargs.degree;
        if (kwargs.type && kwargs.type !== this.constructor.name) {
            throw Error(`Internal error: bad value for parameter "type"`);
        }
        this.idx = kwargs.idx;
        this.uuid = kwargs.uuid || uuidv4();
        this.max_page = kwargs.max_page || INFINITY;

        let errorContext = ' in constructor for ChartClass.';
        this.name = initialPagePropertyValue(
            kwargs.name,
            '',
            'name',
            errorContext,
        );
        this.shape = initialPagePropertyValue(
            kwargs.shape,
            { ty: 'empty' },
            'shape',
            errorContext,
        );
        this.background_color = initialPagePropertyValue(
            kwargs.background_color,
            [0, 0, 0, 1],
            'shape',
            errorContext,
        );
        this.border_color = initialPagePropertyValue(
            kwargs.border_color,
            [0, 0, 0, 1],
            'shape',
            errorContext,
        );
        this.border_width = initialPagePropertyValue(
            kwargs.border_width,
            3,
            'shape',
            errorContext,
        );
        this.foreground_color = initialPagePropertyValue(
            kwargs.foreground_color,
            [0, 0, 0, 1],
            'shape',
            errorContext,
        );
        this.scale = initialPagePropertyValue(
            kwargs.scale,
            1,
            'scale',
            errorContext,
        );
        this.visible = initialPagePropertyValue(
            kwargs.visible,
            true,
            'visible',
            errorContext,
        );
        this.x_nudge = initialPagePropertyValue(
            kwargs.x_nudge,
            0,
            'x_nudge',
            errorContext,
        );
        this.y_nudge = initialPagePropertyValue(
            kwargs.y_nudge,
            0,
            'y_nudge',
            errorContext,
        );
        this.user_data = kwargs.user_data || new Map();
    }

    _updateProjection() {
        if (!this._sseq) {
            throw Error('Undefined _sseq.');
        }
        let x = 0;
        let y = 0;
        for (let i = 0; i < this._sseq.num_gradings; i++) {
            x += this._sseq.x_projection[i] * this.degree[i];
            y += this._sseq.y_projection[i] * this.degree[i];
        }
        this.x = x;
        this.y = y;
    }

    update(msg: MessageUpdate<ChartClass>) {
        let kwargs = msg.update_fields;
        // TODO: new utils function that ensures no "_" fields present, raises error "bad serialized class".
        if (kwargs.degree) {
            if (!arrayEqual(this.degree, kwargs.degree)) {
                throw TypeError(`Inconsistent values for "degree".`);
            }
        }
        // if(kwargs.type){
        //     if(kwargs.type !== this.constructor.name){
        //         throw TypeError(`Invalid value for "type"`)
        //     }
        // }
        if (kwargs.uuid) {
            if (this.uuid !== kwargs.uuid) {
                throw TypeError(`Inconsistent values for "uuid".`);
            }
        }
        if (kwargs.idx) {
            this.idx = kwargs.idx;
        }
        if (kwargs.name) {
            this.name = pagePropertyOrValueToPageProperty(kwargs.name);
        }
        if (kwargs.max_page) {
            this.max_page = kwargs.max_page;
        }
        if (kwargs.shape) {
            this.shape = pagePropertyOrValueToPageProperty(kwargs.shape);
        }
        if (kwargs.background_color) {
            this.background_color = pagePropertyOrValueToPageProperty(
                kwargs.background_color,
            );
        }
        if (kwargs.border_color) {
            this.border_color = pagePropertyOrValueToPageProperty(
                kwargs.border_color,
            );
        }
        if (kwargs.border_width) {
            this.border_width = pagePropertyOrValueToPageProperty(
                kwargs.border_width,
            );
        }
        if (kwargs.foreground_color) {
            this.foreground_color = pagePropertyOrValueToPageProperty(
                kwargs.foreground_color,
            );
        }

        if (kwargs.scale) {
            this.scale = pagePropertyOrValueToPageProperty(kwargs.scale);
        }
        if (kwargs.visible) {
            this.visible = pagePropertyOrValueToPageProperty(kwargs.visible);
        }
        if (kwargs.x_nudge) {
            this.x_nudge = pagePropertyOrValueToPageProperty(kwargs.x_nudge);
        }
        if (kwargs.y_nudge) {
            this.y_nudge = pagePropertyOrValueToPageProperty(kwargs.y_nudge);
        }
        if (kwargs.user_data) {
            this.user_data = kwargs.user_data;
        }
    }

    delete() {
        for (let e of this.edges) {
            this._sseq!.edges.delete(e.uuid);
        }
        this._sseq!.classes.delete(this.uuid);
    }

    toJSON(): any {
        return {
            type: this.constructor.name,
            degree: this.degree,
            idx: this.idx,
            uuid: this.uuid,
            name: this.name,
            max_page: this.max_page,
            shape: this.shape,
            background_color: this.background_color,
            border_color: this.border_color,
            border_width: this.border_width,
            foreground_color: this.foreground_color,
            scale: this.scale,
            visible: this.visible,
            x_nudge: this.x_nudge,
            y_nudge: this.y_nudge,
            user_data: this.user_data,
        };
    }

    static fromJSON(walker: Walker, obj: object): ChartClass {
        //@ts-ignore
        return new ChartClass(obj);
    }

    drawOnPageQ(page: number): boolean {
        return page <= this.max_page && this.visible[page];
    }

    inRangeQ(xmin: number, xmax: number, ymin: number, ymax: number): boolean {
        // TODO: maybe remove the need for this guard?
        if (this.x === undefined || this.y === undefined) {
            throw TypeError('Undefined field x or y');
        }
        return (
            xmin <= this.x && this.x <= xmax && ymin <= this.y && this.y <= ymax
        );
    }

    getNameCoord(page: number): string {
        let tooltip = '';
        let name =
            this.name.constructor === PageProperty
                ? this.name[page]
                : this.name;
        if (name) {
            tooltip = `\\(\\large ${name}\\)&nbsp;&mdash;&nbsp;`;
        }
        tooltip += `(${this.x}, ${this.y})`;
        return tooltip;
    }

    getTooltip(page: number): string {
        let tooltip = this.getNameCoord(page);
        if (this.extra_tooltip) {
            tooltip += this.extra_tooltip;
        }
        return tooltip;
    }

    getXOffset(page: number): number {
        let x_offset;
        let classes = this._sseq!.classes_in_degree(...this.degree);
        let num_classes = classes.length;
        if (this.idx === undefined) {
            throw TypeError('Class has undefined index.');
        }
        let idx = this.idx;
        let out = (idx - (num_classes - 1) / 2) * this._sseq!.offset_size;
        if (isNaN(out)) {
            console.error('Invalid offset for class:', this);
            x_offset = 0;
        } else {
            x_offset = out;
        }

        let x_nudge = this.x_nudge[page] ? this.x_nudge[page] : 0;
        return x_offset + x_nudge;
    }

    getYOffset(page: number): number {
        let y_offset = 0;
        let y_nudge = this.y_nudge[page] ? this.y_nudge[page] : 0;
        return y_offset + y_nudge;
    }
}
