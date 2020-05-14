"use strict"

import { Tooltip } from "chart/interface/Tooltip.js";
import Mousetrap from "mousetrap";

import { Display } from "chart/interface/display/Display.js";
import { SpectralSequenceChart } from "chart/sseq/SpectralSequenceChart.js";
window.SpectralSequenceChart = SpectralSequenceChart;
import { SseqPageIndicator } from "chart/interface/display/SseqPageIndicator.js";
import { Panel } from "chart/interface/panel/Panel.js";
import { SseqSocketListener } from "chart/SseqSocketListener.js";
window.SseqSocketListener = SseqSocketListener;