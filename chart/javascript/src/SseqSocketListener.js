import { SocketListener } from "./SocketListener.js";

export class SseqSocketListener extends SocketListener {
    constructor(socket) {
        super(socket);
        this._onclick = this._onclick.bind(this);
        this.add_message_handlers_from_object(default_message_handlers);
    }

    attachDisplay(disp){
        this.detachDisplay();
        this.display = display;
        display.addEventListener("click", this._onclick);
    }

    detachDisplay(){
        if(!this.display){
            return;
        }
        this.display.removeListener("click", this._onclick);
    }


    console_log_if_debug(msg) {
        if(this.debug_mode) {
            console.log(msg);
        }
    }

    _onclick(e){
        // console.log("sseqSocketListener onclick:", e);
        let o = e.detail[0];
        this.send("click", { "chart_class" : o.mouseover_class, "x" : o.real_x, "y" : o.real_y });
    }
}

let default_message_handlers = {
    "start" : function(){
        this.send("new_user", {});
    },

    "initialize.chart.state" : function(cmd, args, kwargs) {
        this.console_log_if_debug("accepted user:", kwargs.state);
        this.sseq = SpectralSequenceChart.from_JSON(kwargs.state);
        this.display.y_clip_offset = this.sseq.y_clip_offset;
        let chart = document.querySelector("sseq-chart");
        chart.setSseq(this.sseq);
        document.querySelector("sseq-ui").start();
        this.send("initialize.complete", {});
    },

    "chart.batched" : function(cmd, args, kwargs) {
        for(let msg of kwargs.messages) {
            try {
                this.handle_message(msg);
            } catch(err) {
                console.error(err);
            }
        }
        document.querySelector("sseq-chart").update();
    },
    
    "chart.state.reset" : function(cmd, args, kwargs) {
        this.console_log_if_debug("accepted user:", kwargs.state);
        this.sseq = SpectralSequenceChart.from_JSON(kwargs.state);
        let chart = document.querySelector("sseq-chart");
        chart.setSseq(this.sseq);
        if(kwargs.display_state !== undefined){
            this.set_display_state(kwargs.display_state);
        }
        this.display.y_clip_offset = this.sseq.y_clip_offset;
    },

    "chart.set_x_range" : function(cmd, args, kwargs){
        this.sseq.x_range = [kwargs.x_min, kwargs.x_max];
    },
    "chart.set_y_range" : function(cmd, args, kwargs){
        this.sseq.y_range = [kwargs.y_min, kwargs.y_max];
    },
    "chart.set_initial_x_range" : function(cmd, args, kwargs){
        this.sseq.initial_x_range = [kwargs.x_min, kwargs.x_max];
    },
    "chart.set_initial_y_range" : function(cmd, args, kwargs){
        this.sseq.initial_y_range = [kwargs.y_min, kwargs.y_max];
    },    
    "chart.insert_page_range" : function(cmd, args, kwargs) {
        this.sseq.page_list.splice(kwargs.idx, 0, kwargs.page_range);
    },

    "chart.class.add" : function(cmd, args, kwargs) {
        let c = this.sseq.add_class(kwargs.new_class);
        this.display.update();
    },

    "chart.class.update" : function(cmd, args, kwargs) {
        let c = kwargs.class_to_update;
        // console.log("class.update", kwargs, "c", c, "this.sseq.classes[c.uuid]", this.sseq.classes[c.uuid]);
        this.sseq.classes[c.uuid].update(c);
    },

    "chart.class.delete" : function(cmd, args, kwargs) {
        let c = kwargs.class_to_delete;
        this.sseq.delete_class(c);
    },

    "chart.class.set_name" : function(cmd, args, kwargs) {
        let [x,y,idx] = load_args({
            "x" : Number.isInteger, 
            "y" : Number.isInteger, 
            "idx" : Number.isInteger
        });
        this.sseq.classes_by_degree.get([kwargs.x, msg.arguments.y])[msg.arguments.idx].name = msg.arguments.name;
    },

    "chart.edge.add" : function(cmd, args, kwargs) {
        this.console_log_if_debug(kwargs);
        this.sseq.add_edge(kwargs);
    },

    "chart.edge.update" : function(cmd, args, kwargs) {
        this.console_log_if_debug(kwargs);
        let e = kwargs.edge_to_update;
        this.sseq.edges[e.uuid].update(e);
    },

    "chart.edge.delete" : function(cmd, args, kwargs) {
        let e = kwargs.edge_to_delete;
        this.sseq.delete_edge(e);
    },      

    "display.set_background_color" : function(cmd, args, kwargs) {
        this.display.setBackgroundColor(kwargs.color);
    },

    "interact.alert" : function(cmd, args, kwargs) {
        alert(kwargs.msg);
    },
    "interact.prompt" : function(cmd, args, kwargs) {
        let result = prompt(kwargs.msg, kwargs.default);
        this.send("interact.result", {"result" : result});
    }
};