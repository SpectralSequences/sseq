import { SidebarDisplay } from "./interface/mod.js";
import { SocketListener } from "./SocketListener.js";

export class SocketDisplay extends SidebarDisplay {
    constructor(container, socket) {
        super(container);
        this.socket = new SocketListener(socket);
        this._onclick = this._onclick.bind(this);
        this.on("click", this._onclick);
        this.add_message_handlers_from_object(default_message_handlers);
    }

    add_message_handlers_from_object(handlers) {
        for(let [cmd_filter, handler] of Object.entries(handlers)) {
            this.socket.add_message_handler(cmd_filter, handler.bind(this));
        }
    }

    send(...args){
        this.socket.send(...args);
    }

    console_log_if_debug(msg) {
        if(this.debug_mode) {
            console.log(msg);
        }
    }

    _onclick(o){        
        this.send("click", { "chart_class" : o.mouseover_class, "x" : o.real_x, "y" : o.real_y });
    }
}

let default_message_handlers = {
    "start" : function(){
        this.send("new_user", {});
    },

    "initialize.chart.state" : function(cmd, args, kwargs) {
        this.console_log_if_debug("accepted user:", kwargs.state);
        this.setSseq(SpectralSequenceChart.from_JSON(kwargs.state));
        this.y_clip_offset = this.sseq.y_clip_offset;
        this.send("initialize.complete", {});
    },

    "chart.batched" : function(cmd, args, kwargs) {
        console.log("chart.batched", kwargs.messages);
        for(let msg of kwargs.messages) {
            try {
                this.socket.handle_message(msg);
            } catch(err) {
                console.error(err);
            }
        }
        this.update()
    },
    
    "chart.state.reset" : function(cmd, args, kwargs) {
        this.console_log_if_debug("accepted user:", kwargs.state);
        this.setSseq(SpectralSequenceChart.from_JSON(kwargs.state));
        if(kwargs.display_state !== undefined){
            this.set_display_state(kwargs.display_state);
        }
        this.y_clip_offset = this.sseq.y_clip_offset;
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

    "chart.node.add" : function(cmd, args, kwargs) {
        this.console_log_if_debug("add node", cmd, kwargs)
        // this.info(msg);
    },

    "chart.class.add" : function(cmd, args, kwargs) {
        let c = this.sseq.add_class(kwargs.new_class);
        this.update();
    },

    "chart.class.update" : function(cmd, args, kwargs) {
        let c = kwargs.class_to_update;
        Object.assign(this.sseq.classes[c.uuid], c);
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
        console.log("chart.edge.add");
        this.console_log_if_debug(kwargs);
        this.sseq.add_edge(kwargs);
    },

    "chart.edge.update" : function(cmd, args, kwargs) {
        this.console_log_if_debug(kwargs);
        let e = kwargs.edge_to_update;
        Object.assign(this.sseq.edges[e.uuid], e);
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