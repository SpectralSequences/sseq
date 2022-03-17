import StringifyingMap from './StringifyingMap';
import { ChartClass, ChartClassConstructorArgs } from './ChartClass';
import {
    ChartDifferential,
    ChartStructline,
    ChartExtension,
    ChartEdge,
    ChartDifferentialConstructorArgs,
    ChartStructlineConstructorArgs,
    ChartExtensionConstructorArgs,
} from './ChartEdge';
import { Walker } from './json_utils';

import { EventEmitter } from 'ee-ts';
import { INFINITY } from './infinity';
import { v4 as uuidv4 } from 'uuid';

function check_argument_is_integer(name: string, value: any) {
    if (!Number.isInteger(value)) {
        throw TypeError(
            `Argument "${name}" is ${value} which is not an integer. "${name}" is expected to be an integer.`,
        );
    }
}

interface MessageUpdateGlobal {
    chart_id: string;
    target_type: 'SseqChart';
    command: 'update';
    target_fields: Partial<SseqChart>;
}

interface MessageCreate<T> {
    chart_id: string;
    command: 'create';
    target: T;
}

export interface MessageUpdate<T> {
    chart_id: string;
    command: 'update';
    update_fields: Partial<T> & { uuid: string };
}

interface MessageDelete {
    chart_id: string;
    command: 'delete';
    target_uuid: string;
}

interface MessageCreateClass extends MessageCreate<ChartClass> {
    target_type: 'ChartClass';
}
interface MessageCreateStructline extends MessageCreate<ChartStructline> {
    target_type: 'ChartStructline';
}
interface MessageCreateDifferential extends MessageCreate<ChartDifferential> {
    target_type: 'ChartDifferential';
}
interface MessageCreateExtension extends MessageCreate<ChartExtension> {
    target_type: 'ChartExtension';
}

export interface MessageUpdateClass extends MessageUpdate<ChartClass> {
    target_type: 'ChartClass';
}
export interface MessageUpdateStructline
    extends MessageUpdate<ChartStructline> {
    target_type: 'ChartStructline';
}
export interface MessageUpdateDifferential
    extends MessageUpdate<ChartDifferential> {
    target_type: 'ChartDifferential';
}
export interface MessageUpdateExtension extends MessageUpdate<ChartExtension> {
    target_type: 'ChartExtension';
}

interface MessageDeleteClass extends MessageDelete {
    target_type: 'ChartClass';
}
interface MessageDeleteStructline extends MessageDelete {
    target_type: 'ChartStructline';
}
interface MessageDeleteDifferential extends MessageDelete {
    target_type: 'ChartDifferential';
}
interface MessageDeleteExtension extends MessageDelete {
    target_type: 'ChartExtension';
}

type Message =
    | MessageUpdateGlobal
    | MessageCreateClass
    | MessageCreateStructline
    | MessageCreateDifferential
    | MessageCreateExtension
    | MessageUpdateClass
    | MessageUpdateStructline
    | MessageUpdateDifferential
    | MessageUpdateExtension
    | MessageDeleteClass
    | MessageDeleteStructline
    | MessageDeleteDifferential
    | MessageDeleteExtension;

interface Events {
    class_added(c: ChartClass): void;
    differential_added(e: ChartDifferential): void;
    structline_added(e: ChartStructline): void;
    extension_added(e: ChartExtension): void;
    edge_added(e: ChartEdge): void;
    update(): void;
}

export class SseqChart extends EventEmitter<Events> {
    name: string = '';
    readonly uuid: string;
    page_list: [number, number][] = [
        [2, 2],
        [INFINITY, INFINITY],
    ];
    initial_x_range: [number, number] = [0, 10];
    initial_y_range: [number, number] = [0, 10];
    x_range: [number, number] = [0, 10];
    y_range: [number, number] = [0, 10];
    readonly num_gradings = 2;
    x_projection = [1, 0];
    y_projection = [0, 1];

    // No version of these in python version yet...
    offset_size: number = 45;
    // min_class_size : number = 1;
    // max_class_size : number = 3;
    // class_scale : number = 10;
    // highlightScale : number = 2;
    // highlightColor = "orange";
    // mouseoverScale = 2; // How much bigger should the mouseover region be than the class itself?

    _classes_by_degree: StringifyingMap<number[], ChartClass[]> =
        new StringifyingMap();
    classes: Map<string, ChartClass> = new Map();
    edges: Map<string, ChartEdge> = new Map();
    objects: Map<string, ChartClass | ChartEdge> = new Map();

    static charts: Map<string, SseqChart> = new Map();

    constructor(name: string, num_gradings: number, uuid?: string) {
        super();
        // assert num_gradings >= 2;
        this.name = name;
        this.uuid = uuid ? uuid : uuidv4();
        SseqChart.charts.set(this.uuid, this);
        this.page_list = [
            [2, INFINITY],
            [INFINITY, INFINITY],
        ];
        this.initial_x_range = [0, 10];
        this.initial_y_range = [0, 10];
        this.x_range = [0, 10];
        this.y_range = [0, 10];
        this.classes = new Map();
        this.edges = new Map();
        this.x_projection = [1, 0].concat(Array(num_gradings - 2).fill(0));
        this.y_projection = [0, 1].concat(Array(num_gradings - 2).fill(0));
    }

    static visit(walker: Walker, obj: any) {
        walker.visitChildren(obj);
    }

    static fromJSON(walker: Walker, json: any): any {
        let chart = new SseqChart(json.name, json.num_gradings, json.uuid);
        chart._fromJSONHelper(json);
        return chart;
    }

    _fromJSONHelper(kwargs: any) {
        this.page_list = kwargs.page_list;
        this.name = kwargs.name;
        this.initial_x_range = kwargs.initial_x_range;
        this.initial_y_range = kwargs.initial_y_range;
        this.x_range = kwargs.x_range;
        this.y_range = kwargs.y_range;
        this.x_projection = kwargs.x_projection;
        this.y_projection = kwargs.y_projection;

        // this.offset_size = kwargs.offset_size;
        // if(kwargs.min_class_size){
        //     this.min_class_size = kwargs.min_class_size;
        // }
        // if(kwargs.max_class_size){
        //     this.max_class_size = kwargs.max_class_size;
        // }
        // if(kwargs.highlightScale){
        //     this.highlightScale = kwargs.highlightScale;
        // }

        for (let c of kwargs.classes) {
            this._commit_class(c);
        }
        for (let c of kwargs.edges) {
            this._commit_edge(c);
        }
    }

    classes_in_degree(...args: number[]): ChartClass[] {
        if (args.length !== this.num_gradings) {
            throw TypeError(
                `Expected this.num_gradings = ${this.num_gradings} arguments to classes_in_degree.`,
            );
        }
        args.forEach((v, idx) => check_argument_is_integer(`${idx}`, v));
        if (!this._classes_by_degree.has(args)) {
            return [];
        }
        return this._classes_by_degree.get(args)!;
    }

    class_by_index(...x: number[]) {
        if (x.length !== this.num_gradings + 1) {
            throw TypeError(
                `Expected this.num_gradings + 1 = ${
                    this.num_gradings + 1
                } arguments to classes_in_degree.`,
            );
        }
        let idx = x.pop()!;
        check_argument_is_integer('idx', idx);
        let classes = this.classes_in_degree(...x);
        if (idx >= classes.length) {
            throw Error(
                `Fewer than ${idx} classes exist in degree (${x.join(', ')}).`,
            );
        }
        return classes[idx];
    }

    add_class(kwargs: ChartClassConstructorArgs) {
        let c = new ChartClass(kwargs);
        this._commit_class(c);
        this.emit('class_added', c);
        this.emit('update');
        return c;
    }

    /** Common logic between add_class and deserialization of classes. **/
    _commit_class(c: ChartClass) {
        if (c.degree.length !== this.num_gradings) {
            throw TypeError(
                `Wrong number of gradings: degree {c.degree} has length {len(c.degree)} but num_gradings is {self.num_gradings}`,
            );
        }
        c._sseq = this;
        let degree = c.degree!;
        this.classes.set(c.uuid, c);
        this.objects.set(c.uuid, c);
        if (!this._classes_by_degree.has(degree)) {
            this._classes_by_degree.set(degree, []);
        }
        // filter_dictionary_of_lists<number[], ChartClass>(this._classes_by_degree, degree, (c => c._valid) as (arg : ChartClass) => boolean);
        if (c.idx === undefined) {
            c.idx = this._classes_by_degree.get(degree)!.length;
        }
        this._classes_by_degree.get(c.degree)!.push(c);
        c._updateProjection();
    }

    delete_class(c: ChartClass) {
        throw Error('Not implemented'); // ??
        this.classes.delete(c.uuid);
    }

    delete_edge(e: ChartEdge) {
        if (!this.edges.has(e.uuid)) {
            console.error('Failed to delete edge', e);
            throw Error(`Failed to delete edge!`);
        }
        this.edges.delete(e.uuid);
    }

    /** Common logic between add_structline, add_differential, add_extension, and deserialization. */
    _commit_edge(e: ChartEdge) {
        e._sseq = this;
        this.edges.set(e.uuid, e);
        this.objects.set(e.uuid, e);
        e.source = this.classes.get(e.source_uuid);
        e.target = this.classes.get(e.target_uuid);
        if (!e.source) {
            throw Error(`No class with uuid ${e.source_uuid}`);
        }
        if (!e.target) {
            throw Error(`No class with uuid ${e.target_uuid}`);
        }
        e.source.edges.add(e);
        e.target.edges.add(e);
    }

    add_differential(kwargs: ChartDifferentialConstructorArgs) {
        let e = new ChartDifferential(kwargs);
        this._commit_edge(e);
        this.emit('differential_added', e);
        this.emit('edge_added', e);
        this.emit('update');
        return e;
    }

    add_structline(kwargs: ChartStructlineConstructorArgs) {
        let e = new ChartStructline(kwargs);
        this._commit_edge(<ChartEdge>e);
        this.emit('structline_added', e);
        this.emit('edge_added', <ChartEdge>e);
        this.emit('update');
        return e;
    }

    add_extension(kwargs: ChartExtensionConstructorArgs) {
        let e = new ChartExtension(kwargs);
        this._commit_edge(e);
        this.emit('extension_added', e);
        this.emit('edge_added', e);
        this.emit('update');
        return e;
    }

    /**
     * Gets the tooltip for the current class on the given page (currently ignores the page).
     * @param c
     * @param page
     * @returns {string}
     */
    getClassTooltip(c: ChartClass, page: number): string {
        let tooltip = c.getNameCoord(page);
        // let extra_info = Tooltip.toTooltipString(c.extra_info, page);
        let extra_info = '';
        if (extra_info) {
            tooltip += extra_info;
        }
        return tooltip;
    }

    toJSON() {
        let type = this.constructor.name;
        if(type.startsWith("_")){
            type = type.slice(1);
        }
        return {
            type,
            name: this.name,
            initial_x_range: this.initial_x_range,
            initial_y_range: this.initial_y_range,
            x_range: this.x_range,
            y_range: this.y_range,
            page_list: this.page_list,
            num_gradings: this.num_gradings,
            x_projection: this.x_projection,
            y_projection: this.y_projection,
            classes: Array.from(this.classes.values()),
            edges: Array.from(this.edges.values()),
            uuid: this.uuid,
            // version: this.version,
            // offset_size : number = 8;
            // min_class_size : number = 1;
            // max_class_size : number = 3;
            // class_scale : number = 10;
            // highlightScale : number = 2;
            // highlightColor = "orange";
            // bidegreeDistanceScale = 1;
            // mouseoverScale = 2; // How much bigger should the mouseover region be than the clas itself?
            // defaultClassShape = {"type" : "circle"};
            // defaultClassScale = 1;
            // defaultClassStrokeColor = true;
            // defaultClassFillColor = true;
            // defaultClassColor = "black";
        };
    }

    update(msg: MessageUpdateGlobal) {
        Object.assign(this, msg.target_fields);
    }

    handleMessage(msg: Message) {
        switch (msg.command) {
            case 'create':
                switch (msg.target_type) {
                    case 'ChartClass':
                        this._commit_class(msg.target);
                        return;
                    case 'ChartStructline':
                    case 'ChartDifferential':
                    case 'ChartExtension':
                        this._commit_edge(<ChartEdge>msg.target);
                        return;
                    default:
                        throw Error(
                            // @ts-expect-error // Typescript thinks this case is impossible.
                            `Cannot create object of unknown type: ${msg.target_type}`,
                        );
                }
            case 'update': {
                if (msg.target_type === 'SseqChart') {
                    this.update(msg);
                    return;
                }
                const target = this.objects.get(msg.update_fields.uuid);
                if (!target) {
                    throw new Error(
                        `Asked to update unknown object: msg "${JSON.stringify(
                            msg,
                        )}"`,
                    );
                }
                if (target.constructor.name !== msg.target_type) {
                    throw new Error(
                        `Target of update has type "${target.constructor.name}" but message claims it has type "${msg.target_type}"`,
                    );
                }
                // @ts-expect-error
                target.update(msg);
                return;
            }
            case 'delete': {
                let target = this.objects.get(msg.target_uuid);
                if (!target) {
                    throw new Error(
                        `Asked to delete unknown object: msg "${JSON.stringify(
                            msg,
                        )}"`,
                    );
                }
                if (target.constructor.name !== msg.target_type) {
                    throw new Error(
                        `Target of delete has type "${target.constructor.name}" but message claims it has type "${msg.target_type}"`,
                    );
                }
                target.delete();
            }
        }
    }
}
