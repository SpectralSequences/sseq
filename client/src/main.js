let webclient = require("./lib.js")

window.infinity = webclient.infinity;
window.mod = function(n,d){
    return (n % d + d)%d;
};

// window.Util = require("./sseq/Util.js");

window.C2S = require("canvas2svg");
window.EventEmitter = require("events");


window.Interface = webclient.interface;
window.IO = webclient.interface.IO;
window.BasicDisplay = webclient.interface.BasicDisplay;

window.spectralsequences = webclient.spectralsequences;
window.SpectralSequenceChart = webclient.spectralsequences.SpectralSequenceChart;
window.ChartShape = webclient.spectralsequences.ChartShape;
window.ChartNode = webclient.spectralsequences.ChartNode;
window.ChartClass = webclient.spectralsequences.ChartClass;
window.ChartEdge = webclient.spectralsequences.ChartEdge;
window.ChartStructline = webclient.spectralsequences.ChartStructline;
window.ChartDifferential = webclient.spectralsequences.ChartDifferential;
window.ChartExtension = webclient.spectralsequences.ChartExtension;


window.d3 = webclient.d3;
window.Mousetrap = webclient.Mousetrap;
