import { SseqChart } from './chart/SseqChart';

import { AxesElement } from './Axes';
import { BidegreeHighlighterElement } from './BidegreeHighlighter';
import { ButtonElement } from './Button';
import { ClassHighlighterElement } from './ClassHighlighter';
import { MatrixElement } from './Matrix';
import { PageIndicatorElement } from './PageIndicator';
import { PopupElement } from './Popup';
import { SidebarElement } from './Sidebar';
import { TooltipElement } from './Tooltip';
import { UIElement } from './UI';

import './theme.css';

window.SseqChart = SseqChart;

// wasm module has to be loaded asynchronously.
async function main() {
    await import('./Chart.ts');
}
main().catch(console.error);
