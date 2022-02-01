import { LitElement, html, css } from 'lit-element';
import { sleep, promiseFromDomEvent } from './utils.js';

export class ButtonElement extends LitElement {
    static get styles() {
        return css`
            :host {
                font-variant: all-small-caps;
                font: 400 14pt Arial;
                text-align: center;
                box-sizing: border-box;
                padding: 2px 6px;
                outline: none;
                color: rgba(
                    var(--button-text-color),
                    var(--button-text-opacity)
                );
                background: rgba(
                    var(--button-background-color),
                    var(--button-background-opacity)
                );
                user-select: none;
                display: flex;
                justify-content: center;
                align-items: center;
                font-variant: all-small-caps;
            }

            :host(:not([disabled])) {
                cursor: pointer;
            }

            :host([disabled]) {
                opacity: 0.5;
                cursor: default;
                pointer-events: none;
                user-select: none;
            }

            :host(:focus) {
                box-shadow: inset 0px 0px 5px #ccc;
                outline: rgba(var(--complement-2), 1) solid 2px;
            }

            :host(:not([disabled]):hover) {
                box-shadow: inset 0px 0px 5px #ccc;
            }

            :host(:not([disabled]):active),
            :host(.active) {
                box-shadow: 0px 0px 8px #ccc;
                background-color: rgb(224, 224, 224, 0.5);
            }
        `;
    }

    constructor() {
        super();
        this.addEventListener('click', e => this.focus());
        this.addEventListener('interact-toggle', e => {
            e.stopPropagation();
            this.submit(e);
        });
        this.addEventListener('interact-submit', e => {
            e.stopPropagation();
            this.submit(e);
        });
    }

    async submit(e) {
        this.classList.add('active');
        if (e.constructor === CustomEvent) {
            e = e.detail.originalEvent;
        }
        if (e.constructor === KeyboardEvent) {
            this.classList.add('active');
            await promiseFromDomEvent(
                window,
                'keyup',
                keyupEvent => keyupEvent.key === e.key,
            );
            this.classList.remove('active');
        }
        this.click();
    }

    get enabled() {
        return !this.hasAttribute('disabled');
    }

    set enabled(v) {
        if (v) {
            this.removeAttribute('disabled');
        } else {
            this.setAttribute('disabled', '');
        }
    }

    firstUpdated() {
        if (!this.hasAttribute('tabindex')) {
            let disabled = this.hasAttribute('disabled');
            this.setAttribute('tabindex', disabled ? -1 : 0);
            this.mutationObserver = new MutationObserver(mutations => {
                mutations.forEach(mutation => {
                    if (
                        mutation.type == 'attributes' &&
                        mutation.attributeName === 'disabled'
                    ) {
                        let disabled = this.hasAttribute('disabled');
                        this.setAttribute('tabindex', disabled ? -1 : 0);
                    }
                });
            });
            this.mutationObserver.observe(this, { attributes: true });
        }
    }

    render() {
        return html`
            <!-- <button class="btn"> -->
            <slot></slot>
            <!-- </button> -->
        `;
    }
}

customElements.define('sseq-button', ButtonElement);
