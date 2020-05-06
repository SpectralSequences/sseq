import * as webclient from "./lib.js";

window.infinity = webclient.infinity;
window.mod = webclient.mod;

window.C2S = webclient.C2S;
window.EventEmitter = webclient.EventEmitter;

window.Interface = webclient.Interface;
window.IO = webclient.Interface.IO;
window.BasicDisplay = webclient.Interface.BasicDisplay;

window.spectralsequences = webclient.spectralsequences;
window.SpectralSequenceChart = webclient.spectralsequences.SpectralSequenceChart;
window.ChartShape = webclient.spectralsequences.ChartShape;
window.ChartNode = webclient.spectralsequences.ChartNode;
window.ChartClass = webclient.spectralsequences.ChartClass;
window.ChartEdge = webclient.spectralsequences.ChartEdge;
window.ChartStructline = webclient.spectralsequences.ChartStructline;
window.ChartDifferential = webclient.spectralsequences.ChartDifferential;
window.ChartExtension = webclient.spectralsequences.ChartExtension;

window.SpectralSequenceSocketListener = webclient.SpectralSequenceSocketListener;

import * as d3 from "d3";
window.d3 = d3; 
window.Mousetrap = webclient.Mousetrap;
