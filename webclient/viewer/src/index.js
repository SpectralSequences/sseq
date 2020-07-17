import { AxesElement } from "chart/interface/Axes.js";
import { BidegreeHighlighterElement } from "chart/interface/BidegreeHighlighter";
import { ChartElement } from "chart/interface/Chart.js";
import { ClassHighlighter } from "chart/interface/ClassHighlighter";
import { DisplayElement } from "chart/interface/Display.js";
import { GridElement } from "chart/interface/Grid.js";
import { KatexExprElement } from "chart/interface/KatexExpr.js";
import { MatrixElement } from "chart/interface/Matrix.js";
import { PageIndicatorElement } from "chart/interface/PageIndicator.js";
import { PopupElement } from "chart/interface/Popup.js";
import { SidebarElement } from "chart/interface/Sidebar.js";
import { TooltipElement } from "chart/interface/Tooltip.js";
import { UIElement } from "chart/interface/UI.js";
import StixMath from "chart_fonts/STIX2Math.woff2";
// import "chart/chart.css";

import { SpectralSequenceChart } from "chart/sseq/SpectralSequenceChart.js";

window.SpectralSequenceChart = SpectralSequenceChart;
const font_sheet = document.createElement('style');
font_sheet.innerText = `
@font-face {
    font-family: 'Stix Math';
    font-style: normal;
    font-weight: normal;
    src:
        url('${StixMath}') format('woff2'),
}`
document.head.appendChild(font_sheet);