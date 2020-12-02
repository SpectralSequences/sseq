export type Color = [number, number, number, number];
export type DashPattern = number[];

export function hexStringToColor(s : string) : Color {
    let hexs = [s.slice(1,3), s.slice(3,5), s.slice(5,7)];
    return <Color>hexs.map(s => Number.parseInt(s, 16)/255);
}