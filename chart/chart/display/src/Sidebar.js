import { LitElement, html, css } from 'lit-element';
import { styleMap } from 'lit-html/directives/style-map';

import { sleep, promiseFromDomEvent } from './utils.js';
const RESIZER_WIDTH = 8;

export class SidebarElement extends LitElement {
    static get properties() {
        return {
            width: { type: Number },
            closed: { type: Boolean },
        };
    }

    get closed() {
        return this.hasAttribute('closed');
    }

    set closed(v) {
        let oldValue = this.closed;
        if (v) {
            this.setAttribute('closed', '');
        } else {
            this.removeAttribute('closed');
        }
        this.requestUpdate('closed', oldValue);
    }

    constructor() {
        super();
        this.hide = this.hide.bind(this);
        this.show = this.show.bind(this);
        this.toggle = this.toggle.bind(this);
        this._startResize = this._startResize.bind(this);
        this._resize = this._resize.bind(this);
        this._endResize = this._endResize.bind(this);
        this._selectTab = this._selectTab.bind(this);
    }

    static get styles() {
        return css`
            [hidden] {
                display: none !important;
            }

            #sidebar {
                --header-width: 28.7px;
                --content-width: calc(
                    var(--sidebar-width) - var(--header-width)
                );
                height: 100%;
                width: var(--sidebar-width);
                margin-left: ${RESIZER_WIDTH / 2}px;
                border-left: 1px solid #ddd;
                float: left;
                display: inline;
            }

            #sidebar[transition='open'] {
                transition: 0.5s ease-out;
            }

            #sidebar[transition='close'] {
                transition: 0.5s ease-in;
            }

            :host([closed]) #sidebar {
                transform: translateX(var(--content-width));
                margin-left: calc(-1 * var(--content-width));
            }

            #divider {
                height: 100%;
                cursor: ew-resize;
                width: ${RESIZER_WIDTH}px;
                position: absolute;
                display: inline;
                z-index: 10000;
            }

            #header {
                background: rgba(var(--header-background-color), 1);
                color: rgba(var(--header-text-color), 1);
                font-size: 20pt;
                display: flex;
                flex-direction: column;
            }

            #btn-collapse {
                width: var(--header-width);
                height: 26.9px;
            }

            #btn-collapse[open] {
                font-size: var(--close-icon-font-size);
            }

            .tab-btn {
                writing-mode: tb-rl;
                transform: rotate(-180deg) translateX(-0.2px);
                padding: 0.4rem 0px;
                width: var(--header-width);
            }

            .tab-btn[active] {
                background-color: rgba(var(--body-background-color), 1);
            }

            #content-and-footer {
                background: rgba(
                    var(--body-background-color),
                    1
                ); /*var(--body-background-opacity)*/
                color: rgba(
                    --body-text-color,
                    1
                ); /* Is 1 correct for opacity here? */
                width: var(--content-width);
                display: flex;
                flex-direction: column;
            }

            #content {
                overflow-x: none;
                overflow-y: overlay;
                width: 100%;
                display: flex;
                flex-direction: column;
            }

            ::slotted(*) {
                --text-color: var(--body-text-color);
                color: rgba(var(--body-text-color), var(--body-text-opacity));
            }

            ::slotted(div) {
                overflow-x: hidden;
            }
        `;
    }

    get minWidth() {
        return parseFloat(this.getAttribute('min-width')) || 200; // px
    }

    get maxWidth() {
        return parseFloat(this.getAttribute('max-width')) || 100000; // px
    }

    async firstUpdated() {
        this.width = parseFloat(this.getAttribute('initial-width')) || 240; // px
        await sleep(100);
        let tabs = this.querySelectorAll('[tab]');
        for (let e of this.querySelectorAll('[tab]')) {
            e.setAttribute('slot', e.getAttribute('tab'));
        }
        this.requestUpdate();
        await sleep(100);
        this.selectTab(tabs[0]);
    }

    render() {
        let tabs = Array.from(this.querySelectorAll('[tab]'));
        return html`
            <div
                id="divider"
                @pointerdown=${this._startResize}
                @pointerup=${this._endResize}
                ?hidden="${this.closed}"
            ></div>
            <div id="sidebar" style="--sidebar-width : ${this.width}px">
                <div style="display:flex; height:100%;">
                    <div id="header">
                        <sseq-button
                            @click=${this.toggle}
                            id="btn-collapse"
                            ?open="${!this.closed}"
                        >
                            ${this.closed ? html`&#9776;` : html`&times;`}
                        </sseq-button>
                        ${tabs.length > 1
                            ? tabs.map(
                                  e => html`
                                      <sseq-button
                                          class="tab-btn"
                                          @click=${this._selectTab}
                                          tab=${e.getAttribute('tab')}
                                      >
                                          ${e.getAttribute('tab')}
                                      </sseq-button>
                                  `,
                              )
                            : ''}
                    </div>
                    <div id="content-and-footer">
                        <div id="content">
                            <slot></slot>
                        </div>
                        <span style="flex-grow : 1; height : 2rem;"></span>
                        <slot name="footer"></slot>
                    </div>
                </div>
            </div>
        `;
    }

    _selectTab() {
        this.selectTab(this.shadowRoot.activeElement);
    }

    selectTab(tabName) {
        if (tabName instanceof Element) {
            tabName = tabName.getAttribute('tab');
        }
        this.shadowRoot.querySelector('slot').name = tabName;
        for (let e of this.shadowRoot.querySelectorAll('[tab]')) {
            e.toggleAttribute('active', e.getAttribute('tab') === tabName);
        }
    }

    _startResize(e) {
        e.preventDefault();
        window.addEventListener('pointermove', this._resize);
        this.shadowRoot
            .querySelector('#divider')
            .setPointerCapture(e.pointerId);
    }

    _resize(e) {
        // e.preventDefault();
        this.width = Math.min(
            Math.max(
                this.getBoundingClientRect().right - e.pageX,
                this.minWidth,
            ),
            this.maxWidth,
        );
    }

    _endResize(e) {
        window.removeEventListener('pointermove', this._resize);
        // This next line doesn't really seem to do anything. I don't understand setPointerCapture very well...
        this.shadowRoot
            .querySelector('#divider')
            .releasePointerCapture(e.pointerId);
    }

    async toggle() {
        let transition_direction = this.closed ? 'open' : 'close';
        let sidebar = this.shadowRoot.querySelector('#sidebar');
        sidebar.setAttribute('transition', transition_direction);
        this.closed = !this.closed;
        let chart = this.parentElement.startReflow();
        await promiseFromDomEvent(sidebar, 'transitionend');
        this.parentElement.endReflow();
        sidebar.removeAttribute('transition');
    }

    hide() {
        this.closed = true;
    }

    show() {
        this.closed = false;
    }

    focus() {
        let focusElt = this.querySelector("[focus][tabindex='0']");
        if (focusElt) {
            focusElt.focus();
        } else {
            this.closest('sseq-ui').focus();
        }
        return this;
    }
}
customElements.define('sseq-sidebar', SidebarElement);
