import { Display } from "chart/interface/Display.js";
import { SpectralSequenceChart } from "chart/sseq/SpectralSequenceChart.js";
window.SpectralSequenceChart = SpectralSequenceChart;

import { SseqPageIndicator } from "chart/interface/SseqPageIndicator.js";
import { Panel } from "chart/interface/Panel.js";

import { SseqSocketListener } from "chart/SseqSocketListener.js";
window.SseqSocketListener = SseqSocketListener;
import Mousetrap from "mousetrap";

class Mode {
    static set(mode) {
        console.log("mode.set:", mode);
        if(Mode.currentMode){
            Mode.currentMode.end();
        }
        if(mode.constructor === String) {
            mode = Mode.dict[mode];
        }
        Mode.currentMode = mode;
        Mode.currentMode.start();
        socket_listener.send("interact.mode.set", {"mode" : mode.constructor.name});        
        display.mode_elt.innerText = mode.constructor.name;
    }
    
    static cancel(){
        if(!Mode.currentMode){
            return;
            throw ReferenceError("No current mode.");
        }
        Mode.currentMode.cancel();
    }

    static click(cls, x, y){
        if(!Mode.currentMode){
            return;
            throw ReferenceError("No current mode.");
        }        
        Mode.currentMode.click(cls, x, y);
    }
    
    constructor(display) {
        this.display = display;
    }

    start() {
        
    }

    end() {
        
    }

    click(cls, x, y){

    }

    cancel(){
        
    }

    /* Handlers */
}
Mode.currentMode = undefined;


class AddClassMode extends Mode {}
class AddEdgeMode extends Mode {}
class ColorMode extends Mode {}
class AddDifferentialMode extends Mode {}
class AddExtensionMode extends Mode {
    handle__extension__adjust_bend() {
        reset_key_bindings();
        add_key_bindings(AddExtensionMode.key_bindings);
        set_mode_info("Adjust edge bend")
    }

    cancel(){
        this.current_class = undefined;
        set_normal_key_bindings();
    }

    static change_edge_bend(delta){
        console.log("CEB: delta=", delta);
        socket_listener.send("interact.mode.extension.adjust_bend", {"delta" : delta })
    }
}

AddExtensionMode.key_bindings = {
    "q" : () => AddExtensionMode.change_edge_bend(3),
    "w" : () => AddExtensionMode.change_edge_bend(-3)
}



class NudgeClassMode extends Mode {
    click(cls, x, y) {
        if(!cls){
            return;
        }
        this.current_class = cls;
        reset_key_bindings();
        add_key_bindings(NudgeClassMode.key_bindings);
        console.log("nudge bindings??")
    }

    cancel(){
        this.current_class = undefined;
        set_normal_key_bindings();
    }

    static nudge_class(x, y){
        console.log(`Nudge ${x}, ${y}`);
        display.send("interact.mode.nudge_class", {"x" : x, "y" : y});
    }
}
NudgeClassMode.key_bindings = {
    'w' : () => NudgeClassMode.nudge_class(0, -1),
    'a' : () => NudgeClassMode.nudge_class(-1, 0),
    's' : () => NudgeClassMode.nudge_class(0, 1),
    'd' : () => NudgeClassMode.nudge_class(1, 0),
}


window.main = main;

function main(display, socket_address){
    let ws = new WebSocket(socket_address);
    window.socket_listener = new SseqSocketListener(ws);
    socket_listener.attachDisplay(display);
    display.mode_elt = document.querySelector("#mode");
    display.mode_info_elt = document.querySelector("#mode_info");
    
    function set_mode_info(text){
        display.mode_info_elt.innerText = text;
    }
    
    let always_bindings = {
        "t" : () => socket_listener.send("console.take", {}),
        "s s s" : (e) => {
            // e.preventDefault();
            socket_listener.send("io.save", {});
        },
        "escape" : () => {
            socket_listener.send("interact.mode.cancel", {});
            Mode.cancel();
        },
        'left' : () => display.previousPage(),
        'right' : () => display.nextPage(),
        "f" : () => {
            socket_listener.send("io.process_screenshot", {})
        }
    }
    
    function add_key_bindings(bindings){
        for(let [k, v] of Object.entries(bindings)){
            Mousetrap.bind(k, v);
        }
    }
    
    function reset_key_bindings(bindings){
        Mousetrap.reset();
        add_key_bindings(always_bindings);
    }
    
    function set_normal_key_bindings(bindings) {
        reset_key_bindings();
        add_mode_key_bindings();
    }
    
    display.addEventListener("click", function(o){
        console.log("mode click", o);
        Mode.click(o.mouseover_class, o.real_x, o.real_y);
    });
    
    socket_listener.add_message_handlers_from_object({
        "interact.mode.set_info" : function(cmd, args, kwargs) {
            set_mode_info(kwargs["info"])
        },
    
        "interact.mode" : function(cmd, args, kwargs) {
            let handler_name = "handle__" + cmd[0].split(".").slice(2).join("__");
            if(!Mode.currentMode[handler_name]){
                throw Error(`Unknown mode command ${handler_name}`);
            }
            Mode.currentMode[handler_name](args, kwargs)
        },
    });


    let add_class_mode = new AddClassMode(display);
    let add_edge_mode = new AddEdgeMode(display);
    let color_mode = new ColorMode(display);
    let add_differential_mode = new AddDifferentialMode(display);
    let add_extension_mode = new AddExtensionMode(display);
    let nudge_mode = new NudgeClassMode(display);
    
    Mode.dict = {
        "AddClassMode" : add_class_mode,
        "AddEdgeMode" : add_edge_mode,
        "ColorMode" : color_mode,
        "AddDifferentialMode" : add_differential_mode,
        "NudgeClassMode" : nudge_mode,
        "AddExtensionMode" : add_extension_mode
    }    
    

    let mode_change_bindings = {
        "c" : () => Mode.set(add_class_mode),
        "e" : () => Mode.set(add_extension_mode),
        "w" : () => Mode.set(add_edge_mode),
        "r" : () => Mode.set(color_mode),
        "d" : () => Mode.set(add_differential_mode),
        "n" : () => Mode.set(nudge_mode),
    }
    function add_mode_key_bindings() {
        add_key_bindings(mode_change_bindings);
    }
    
    
    function start(display){
        set_normal_key_bindings();
        socket_listener.start();
    }
    
    start(display);
}
