exports.INFINITY = require("./infinity.js").INFINITY;

exports.mod = function(n,d){
    return (n % d + d)%d;
};

// window.Util = require("./sseq/Util.js");

exports.C2S = require("canvas2svg");
exports.EventEmitter = require("events");


exports.interface = require("./interface/mod.js");
exports.spectralsequences = require("./sseq/mod.js");
exports.SpectralSequenceSocketListener = require("./spectralsequence_socket_listener.js").SpectralSequenceSocketListener;


exports.d3 = require("d3-selection");
exports.Mousetrap = require("mousetrap");
