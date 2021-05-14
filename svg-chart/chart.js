export const svgNS = 'http://www.w3.org/2000/svg';

/**
 * A Web Component for a chart.
 *
 * @property {SVGGElement} contents The group containing the actual chart, as
 * opposed to e.g. the axes. Users should add their chart into this group.
 */
export class Chart extends HTMLElement {
    /**
     * The amount of space reserved for the axes and axes labels
     */
    static MARGIN = 30;
    /**
     * The amount of space between the axes and the axes labels
     */
    static LABEL_MARGIN = 5;
    /**
     * The amount of extra space from the edge of the chart. For example, if
     * minX = 0, then we allow users to pan to up to x = -GRID_MARGIN. This
     * allows us to fully display the class, instead of cutting it in half
     * along the grid lines.
     */
    static GRID_MARGIN = 0.5;

    static get observedAttributes() {
        return ['minx', 'miny', 'maxx', 'maxy'];
    }

    attributeChangedCallback(name, _oldValue, newValue) {
        if (name == 'minx') {
            this.minX = parseInt(newValue) - Chart.GRID_MARGIN;
        } else if (name == 'miny') {
            this.minY = parseInt(newValue) - Chart.GRID_MARGIN;
        } else if (name == 'maxx') {
            this.maxX = parseInt(newValue) + Chart.GRID_MARGIN;
        } else if (name == 'maxy') {
            this.maxY = parseInt(newValue) + Chart.GRID_MARGIN;
        }
        this.onResize();
    }

    connectedCallback() {
        this.onResize();
    }

    constructor() {
        super();

        this.attachShadow({ mode: 'open' });

        this.animationId = null;

        this.minX = -Chart.GRID_MARGIN;
        this.minY = -Chart.GRID_MARGIN;
        this.maxX = Chart.GRID_MARGIN;
        this.maxY = Chart.GRID_MARGIN;

        this.svg = document.createElementNS(svgNS, 'svg');

        const node = document.createElement('style');
        node.textContent = ':host { display: block; }';
        this.shadowRoot.appendChild(node);

        this.shadowRoot.appendChild(this.svg);

        this.svg.innerHTML = `
<defs>
  <pattern id="smallGrid" width="1" height="1" patternUnits="userSpaceOnUse">
    <path d="M 1 1 L 0 1 0 0" fill="none" stroke="black" stroke-width="0.01" />
  </pattern>
  <pattern id="bigGrid" width="4" height="4" patternUnits="userSpaceOnUse">
    <rect width="4" height="4" fill="url(#smallGrid)" />
    <path d="M 4 4 L 0 4 0 0" fill="none" stroke="black" stroke-width="0.03" />
  </pattern>
</defs>
<g id="inner">
  <rect id="grid" fill="url(#bigGrid)" />
  <g id="contents"></g>
</g>
<rect id="xBlock" x="${-Chart.MARGIN}" height="${
            Chart.MARGIN
        }" y="0" fill="white"/>
<rect id="yBlock" x="${-Chart.MARGIN}" width="${Chart.MARGIN}" fill="white"/>
<path id="axis" stroke="black" stroke-width="2" fill="none" />
<g id="axisLabels"></g>
`;

        for (const item of [
            'inner',
            'axis',
            'axisLabels',
            'grid',
            'contents',
            'xBlock',
            'yBlock',
        ]) {
            this[item] = this.shadowRoot.getElementById(`${item}`);
        }

        this.select = d3.select(this.svg);
        this.zoom = d3.zoom().on('zoom', this._zoomFunc.bind(this));

        if (navigator.userAgent.includes('Firefox')) {
            this.zoom.on('zoom', e => {
                this._zoomFunc(e);
                clearTimeout(this.zoomTimeout);
                this.zoomTimeout = setTimeout(() => this._zoomFunc(e), 500);
            });
        }
        window.addEventListener('resize', this.onResize.bind(this));

        this.onResize();
        this.select.call(this.zoom).on('dblclick.zoom', null);

        this.shadowRoot.addEventListener('click', this._onClick.bind(this));
    }

    /**
     * Add a stylesheet to the SVG.
     *
     * @return {HTMLStyleElement} The node containing the stylesheet
     */
    addStyle(style) {
        const node = document.createElementNS(svgNS, 'style');
        node.textContent = style;
        this.contents.appendChild(node);
        return node;
    }

    _onClick(e) {
        const box = this.getBoundingClientRect();
        const [innerX, innerY] = [e.clientX - box.left, e.clientY - box.top];
        if (innerX < Chart.MARGIN || innerY > this.height) {
            return;
        }
        const [chartX, chartY] = d3
            .zoomTransform(this.inner)
            .invert([innerX - Chart.MARGIN, innerY - this.height]);

        e = new MouseEvent('clickinner', e);
        e.chartX = Math.round(chartX);
        e.chartY = Math.round(-chartY);

        /**
         * ClickInner event. This event is fired if the interior of the chart
         * is clicked. The event is identical to the original click event
         * except the chart coordinates of the events are also included
         *
         * @event Chart#clickinner
         * @type {object}
         * @augments MouseEvent
         * @property {number} chartX - the X coordinate in chart coordinates, rounded to nearest integer
         * @property {number} chartY - the Y coordinate in chart coordinates, rounded to nearest integer
         */
        this.dispatchEvent(e);
    }

    /**
     * Pan the chart so that the given coordinates (x, y) are at the center of the chart.
     * @param {number} x
     * @param {number} y
     */
    goto(x, y) {
        this.zoom.translateTo(this.select, x, -y);
    }

    _zoomFunc(e) {
        window.cancelAnimationFrame(this.animationId);
        this.animationId = requestAnimationFrame(() => this._zoomFuncInner(e));
    }

    _zoomFuncInner({ transform }) {
        this.inner.setAttribute('transform', transform);
        while (this.axisLabels.firstChild) {
            this.axisLabels.removeChild(this.axisLabels.firstChild);
        }
        let sep = 4;
        while (transform.k * sep < 80) {
            sep *= 2;
        }

        const minX = Math.ceil(transform.invertX(0) / sep) * sep;
        const maxX = Math.floor(transform.invertX(this.width) / sep) * sep;

        for (let x = minX; x <= maxX; x += sep) {
            const textNode = document.createElementNS(svgNS, 'text');
            textNode.textContent = x;
            textNode.setAttribute('x', transform.applyX(x));
            textNode.setAttribute('y', Chart.LABEL_MARGIN);
            textNode.setAttribute('text-anchor', 'middle');
            textNode.setAttribute('dominant-baseline', 'text-before-edge');
            this.axisLabels.appendChild(textNode);
        }

        const minY = Math.ceil(-transform.invertY(0) / sep) * sep;
        const maxY = Math.floor(-transform.invertY(-this.height) / sep) * sep;

        for (let y = minY; y <= maxY; y += sep) {
            const textNode = document.createElementNS(svgNS, 'text');
            textNode.textContent = y;
            textNode.setAttribute('y', transform.applyY(-y));
            textNode.setAttribute('x', -Chart.LABEL_MARGIN);
            textNode.setAttribute('text-anchor', 'end');
            textNode.setAttribute('dominant-baseline', 'middle');
            this.axisLabels.appendChild(textNode);
        }
    }

    /**
     * This function should be called whenever the component's size changes.
     * This is automatically triggered when window#resize is fired, but
     * otherwise the user should call this function when the dimensions change.
     */
    onResize() {
        if (!this.isConnected) {
            return;
        }

        const size = this.getBoundingClientRect();

        this.height = size.height - Chart.MARGIN;
        this.width = size.width - Chart.MARGIN;

        const min_k = Math.min(
            this.width / (this.maxX - this.minX),
            this.height / (this.maxY - this.minY),
        );

        this.svg.setAttribute(
            'viewBox',
            `${-Chart.MARGIN} ${-this.height} ${size.width} ${size.height}`,
        );

        this.zoom.constrain(transform => {
            let x = transform.x;
            let y = transform.y;
            let k = transform.k;

            k = Math.max(k, min_k);

            x = Math.max(x, -this.maxX * k + this.width);
            x = Math.min(x, -this.minX * k);

            y = Math.min(y, this.maxY * k - this.height);
            y = Math.max(y, this.minY * k);

            return d3.zoomIdentity.translate(x, y).scale(k);
        });

        this.axis.setAttribute(
            'd',
            `M ${this.width} 0 L 0 0 0 ${-this.height}`,
        );

        this.xBlock.setAttribute('width', size.width);
        this.yBlock.setAttribute('y', -this.height);
        this.yBlock.setAttribute('height', size.height);

        const grid_min = Math.floor(this.minX / 4) * 4;
        this.grid.setAttribute('x', grid_min);
        this.grid.setAttribute(
            'width',
            Math.ceil(this.width / min_k) + (this.minX - grid_min),
        );

        const gridHeight = Math.ceil((this.minY + this.height / min_k) / 4) * 4;
        this.grid.setAttribute('y', -gridHeight);
        this.grid.setAttribute('height', gridHeight + 4);

        this.zoom.scaleBy(this.select, 1);
    }
}
customElements.define('svg-chart', Chart);

export class PagedChart extends Chart {
    constructor() {
        super();
        this.page = 0;
        this.pages = [];

        document.addEventListener('keydown', e => {
            let newpage = this.page;
            if (e.key == 'ArrowRight') {
                newpage = Math.min(this.page + 1, this.pages.length - 1);
            } else if (e.key == 'ArrowLeft') {
                newpage = Math.max(this.page - 1, 0);
            }
            if (newpage != this.page) {
                this.pages[this.page].style.display = 'none';
                this.page = newpage;
                this.pages[this.page].style.removeProperty('display');
                this.dispatchEvent(new CustomEvent('newpage'));
            }
        });
    }

    newPage() {
        const page = document.createElementNS(svgNS, 'g');
        this.appendPage(page);
        return page;
    }

    appendPage(page) {
        this.contents.appendChild(page);
        if (this.pages.length > 0) {
            page.style.display = 'none';
        }
        this.pages.push(page);
    }
}
customElements.define('paged-chart', PagedChart);

/**
 * A resizable side bar. This has a vertical border on the left that can be dragged to resize.
 */
export class Sidebar extends HTMLElement {
    constructor() {
        super();

        this.adjuster = document.createElement('div');
        this.adjuster.style.height = '100%';
        this.adjuster.style.cursor = 'ew-resize';
        this.adjuster.style.width = '2px';
        this.adjuster.style.left = '0';
        this.adjuster.style.position = 'absolute';
        this.adjuster.class = 'sidebar-adjuster';

        this._resize = this._resize.bind(this);
        this._stopResize = this._stopResize.bind(this);

        this.animationId = null;

        this.adjuster.addEventListener('mousedown', e => {
            e.preventDefault();
            window.addEventListener('mousemove', this._resize);
            window.addEventListener('mouseup', this._stopResize);
        });
    }

    connectedCallback() {
        if (!this.adjuster.isConnected) {
            this.style.position = 'relative';
            this.appendChild(this.adjuster);
            this.dispatchEvent(new CustomEvent('resize'));
        }
    }

    _resize(e) {
        window.cancelAnimationFrame(this.animationId);
        this.animationId = requestAnimationFrame(() => {
            const width = this.getBoundingClientRect().right - e.pageX;
            this.style.width = `${width}px`;
            this.dispatchEvent(new CustomEvent('resize'));
        });
    }

    _stopResize() {
        window.removeEventListener('mousemove', this._resize);
        window.removeEventListener('mouseup', this._stopResize);
    }
}
customElements.define('side-bar', Sidebar);
