import { Walker } from './json_utils';
type ColorVec = [number, number, number, number];
export class Color {
    color: string;
    color_vec: ColorVec;
    name: string;
    type: string = "Color";
    static black : Color;
    constructor({color , name, type} : {color : string, name: string, type: "Color"}){
        this.color = color;
        this.name = name;
        this.color_vec = hexStringToColor(color);
    }
    toJSON(): any {
        let result : any = Object.assign({}, this);
        delete result["color_vec"];
        return result;
    }
    static fromJSON(walker: Walker, o : any){
        console.log(o);
        return new Color(o);
    }
};

export const Black : Color = new Color({color: "0x000000", name : "black", type: "Color"});

export type DashPattern = number[];

export function hexStringToColor(s: string): ColorVec {
    let hexs = [s.slice(1, 3), s.slice(3, 5), s.slice(5, 7), '0xff'];
    if (s.length === 9) {
        hexs[3] = s.slice(7, 9);
    }
    return <ColorVec>hexs.map(s => Number.parseInt(s, 16) / 255);
}
