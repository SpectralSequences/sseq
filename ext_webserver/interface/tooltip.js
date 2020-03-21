'use strict';

const MARGIN = 10;

export class Tooltip {
    constructor(display) {
        this.display = display;

        this.div = document.createElement("div");
        this.div.style.opacity = 0;
        this.div.style.position = "absolute";
        this.div.style["z-index"] = 999999;
        this.div.className = "tooltip";

        document.body.appendChild(this.div);
    }

    setHTML(html) {
        this.div.innerHTML = html;
    }

    show(x, y) {
        /**
         * Reset the tooltip position. This prevents a bug that occurs when the
         * previously displayed tooltip is positioned near the edge (but still
         * positioned to the right of the node), and the new tooltip text is
         * longer than the previous tooltip text. This may cause the new
         * (undisplayed) tooltip text to wrap, which gives an incorrect value
         * of rect.width and rect.height. The bug also occurs after resizing,
         * where the location of the previous tooltip is now outside of the
         * window.
         */
        this.div.style.left = "0px";
        this.div.style.top = "0px";

        let rect = this.div.getBoundingClientRect();
        let canvasRect = this.display.canvas.getBoundingClientRect();

        x = x + canvasRect.x;
        y = y + canvasRect.y;

        /**
         * By default, show the tooltip to the top and right of (x, y), offset
         * by MARGIN. If this cuases the tooltip to leave the window, position
         * it to the bottom/left accordingly.
         */
        if (x + MARGIN + rect.width < window.innerWidth)
            x = x + MARGIN;
        else
            x = x - rect.width - MARGIN;

        if (y - rect.height - MARGIN > 0)
            y = y - rect.height - MARGIN;
        else
            y = y + MARGIN;

        this.div.style.left = `${x}px`;
        this.div.style.top = `${y}px`;

        this.div.style.transition = "opacity 200ms";
        this.div.style.opacity = 0.9;
    }

    hide () {
        this.div.style.transition = "opacity 500ms";
        this.div.style.opacity = 0;
    }
}


