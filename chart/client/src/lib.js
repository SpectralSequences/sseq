export { INFINITY } from "./infinity.js";

export function mod(n,d){
    return (n % d + d)%d;
};

import * as C2S from "canvas2svg";
import * as EventEmitter from "events";
export {C2S, EventEmitter};


import * as Interface from "./interface/mod.js";
import * as spectralsequences from "./sseq/mod.js";
export { Interface, spectralsequences };
export { SocketListener } from "./SocketListener.js";


import * as Mousetrap from "mousetrap";
export { Mousetrap };
