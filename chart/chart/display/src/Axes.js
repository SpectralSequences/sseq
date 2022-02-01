import { LitElement, html, css } from 'lit-element';

import { sleep, promiseFromDomEvent } from './utils.js';

export class AxesElement extends LitElement {
    static get styles() {
        return css`
            * {
                z-index: 1;
            }

            :host {
                --axes-thickness: 1px;
                user-select: none;
            }

            .axis {
                position: absolute;
                background: black;
            }

            #x-axis {
                height: var(--axes-thickness);
            }

            #y-axis {
                width: var(--axes-thickness);
            }

            .tick {
                position: absolute;
                line-height: 0pt;
            }

            .tick[type='x'] {
                transform: translateY(5pt);
            }

            .tick[type='y'] {
                transform: translateX(-5pt);
            }

            .tick[transition] {
                transition: opacity ease-out 0.3s;
            }
        `;
    }

    constructor() {
        super();
        this.handleScaleUpdate = this.handleScaleUpdate.bind(this);
        this.numTickElements = 0;
        this.tickMap = { x: {}, y: {} };
    }

    firstUpdated() {
        this.parentElement.addEventListener(
            'canvas-resize',
            this.resize.bind(this),
        );
        this.parentElement.addEventListener(
            'scale-update',
            this.handleScaleUpdate,
        );
    }

    resize() {
        let left = this.parentElement.leftMargin;
        {
            let bottom = this.parentElement.bottomMargin - 1;
            let width =
                this.parentElement.offsetWidth -
                this.parentElement.rightMargin -
                this.parentElement.leftMargin;
            let x_axis = this.shadowRoot.querySelector('#x-axis');
            x_axis.style.left = `${left}px`;
            x_axis.style.bottom = `${bottom}px`;
            x_axis.style.width = `${width}px`;
        }

        {
            let bottom = this.parentElement.bottomMargin - 1;
            let height =
                this.parentElement.offsetHeight -
                this.parentElement.topMargin -
                bottom;
            let y_axis = this.shadowRoot.querySelector('#y-axis');
            y_axis.style.left = `${left}px`;
            y_axis.style.bottom = `${bottom}px`;
            y_axis.style.height = `${height}px`;
        }
        this.updateTicks('pan');
    }

    async handleScaleUpdate(event) {
        await this.updateTicks(event.detail.type);
    }

    async updateTicks(reason) {
        if (!this.parentElement) {
            return;
        }
        let disp = this.parentElement;
        let [xminFloat, xmaxFloat] = disp.current_xrange();
        let [yminFloat, ymaxFloat] = disp.current_yrange();

        let width = disp.clientWidth - disp.leftMargin - disp.rightMargin;
        let height = disp.clientHeight - disp.topMargin - disp.bottomMargin;
        let targetNumXTicks = width / 50;
        let targetNumYTicks = height / 50;

        let approximateXTickStep =
            (xmaxFloat - xminFloat) / (targetNumXTicks - 1);
        let approximateYTickStep =
            (ymaxFloat - yminFloat) / (targetNumYTicks - 1);

        let [xts1, xts2, yts1, yts2] = [
            approximateXTickStep,
            approximateXTickStep / 5,
            approximateYTickStep,
            approximateYTickStep / 5,
        ].map(x => Math.ceil(Math.pow(10, Math.ceil(Math.log10(x)))));
        xts2 *= 5;
        yts2 *= 5;
        let xTickStep = Math.min(xts1, xts2);
        let yTickStep = Math.min(yts1, yts2);

        let minXTick = Math.ceil(xminFloat / xTickStep) * xTickStep;
        let maxXTick = Math.floor(xmaxFloat);

        let minYTick = Math.ceil(yminFloat / yTickStep) * yTickStep;
        let maxYTick = Math.floor(ymaxFloat);

        let xTicks = [];
        for (let i = minXTick; i <= maxXTick; i += xTickStep) {
            xTicks.push(i);
        }

        let yTicks = [];
        for (let i = minYTick; i <= maxYTick; i += yTickStep) {
            yTicks.push(i);
        }

        let numElementsNeeded =
            xTicks.filter(i => this.tickMap['x'][i] === undefined).length +
            yTicks.filter(i => this.tickMap['y'][i] === undefined).length;

        let allElements = Array.from(this.shadowRoot.querySelectorAll('.tick'));
        let availableElements = allElements.filter(
            e => e.updateId === undefined,
        );
        if (numElementsNeeded > availableElements.length) {
            this.numTickElements +=
                numElementsNeeded - availableElements.length;
            this.requestUpdate();
            await sleep(10);
            allElements = Array.from(this.shadowRoot.querySelectorAll('.tick'));
            availableElements = allElements.filter(
                e => e.updateId === undefined,
            );
        }

        let xTickBottom = disp.bottomMargin;
        let curUpdateId = Math.random();
        for (let i of xTicks) {
            let elt = this.tickMap['x'][i];
            if (elt === undefined) {
                elt = availableElements.pop();
                elt.setAttribute('type', 'x');
                elt.tickType = 'x';
                elt.tickValue = i;
                elt.innerText = i;
                this.tickMap['x'][i] = elt;
            }
            elt.updateId = curUpdateId;
            elt.style.opacity = 1;
            let fontSize = parseInt(window.getComputedStyle(elt).fontSize);
            elt.style.bottom = `${xTickBottom - fontSize / 2}px`;
            elt.style.top = '';
            elt.style.left = `${disp.xScale(i) - elt.clientWidth / 2}px`;
        }

        let yTickRight = disp.leftMargin;
        for (let i of yTicks) {
            let elt = this.tickMap['y'][i];
            if (elt === undefined) {
                elt = availableElements.pop();
                elt.setAttribute('type', 'y');
                elt.tickType = 'y';
                elt.innerText = i;
                elt.tickValue = i;
                this.tickMap['y'][i] = elt;
                // If the following goes outside the conditional, single digit labels jitter.
                // For some reason we don't see a similar problem on x axis.
                elt.style.left = `${yTickRight - elt.offsetWidth}px`;
            }
            elt.updateId = curUpdateId;
            elt.style.opacity = 1;
            elt.style.top = `${disp.yScale(i) - elt.offsetHeight / 2}px`;
            elt.style.bottom = '';
        }
        for (let elt of this.shadowRoot.querySelectorAll('.tick')) {
            if (elt.updateId === undefined || elt.updateId === curUpdateId) {
                continue;
            }
            if (elt.tickValue === undefined) {
                console.error(elt);
            }
            elt.style.opacity = 0;
            if (elt.tickType === 'x') {
                elt.style.left = `${
                    disp.xScale(elt.tickValue) - elt.offsetWidth / 2
                }px`;
            } else {
                elt.style.top = `${
                    disp.yScale(elt.tickValue) - elt.offsetHeight / 2
                }px`;
            }
            let cleanUpElement = () => {
                if (!elt.updateId || elt.style.opacity === '1') {
                    return;
                }
                elt.removeAttribute('transition');
                delete this.tickMap[elt.tickType][elt.tickValue];
                delete elt.updateId;
                delete elt.tickType;
                delete elt.tickValue;
                elt.style.opacity = 0;
            };
            switch (reason) {
                case 'zoom':
                    cleanUpElement();
                    break;

                case 'pan':
                    elt.setAttribute('transition', '');
                    promiseFromDomEvent(elt, 'transitionend').then(
                        cleanUpElement,
                    );
                    break;

                default:
                    throw Error('Unknown scale change event type');
            }
            elt.style.opacity = 0;
        }
    }

    render() {
        return html`
            <div id="x-axis" class="axis"></div>
            <div id="y-axis" class="axis"></div>
            <div>
                ${Array(this.numTickElements)
                    .fill()
                    .map(() => html`<span class="tick"></span>`)}
            </div>
        `;
    }
}
customElements.define('sseq-axes', AxesElement);
