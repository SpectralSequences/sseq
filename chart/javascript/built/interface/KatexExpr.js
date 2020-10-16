// Stolen from:
//      https://github.com/justinfagnani/katex-elements/blob/master/src/katex-expr-element.ts
import katex from 'katex/dist/katex.mjs';
import styles from 'katex/dist/katex.min.css';
let styleSheet = undefined;
function getStyleSheet() {
    if (styleSheet === undefined) {
        try {
            styleSheet = new CSSStyleSheet();
            styleSheet.replace(styles);
        }
        catch (e) {
            if (e instanceof TypeError) {
            }
            else {
                throw e;
            }
        }
    }
    return styleSheet;
}
const template = document.createElement('template');
template.innerHTML = `
    <style>
    :host {
        display: inline-block;
    }
    :host([display-mode]) {
        display: block;
    }
    </style>
    <div id="container"></div>
    <div hidden><slot></slot></div>
`;
export class KatexExprElement extends HTMLElement {
    static get observedAttributes() {
        return ['display-mode', 'leqno', 'fleqn', 'macros'];
    }
    get macros() {
        return this._macros;
    }
    set macros(v) {
        if (v == null) {
            this.removeAttribute('macros');
        }
        else {
            try {
                const json = JSON.stringify(v);
                this._macros = v;
                this.setAttribute('macros', json);
            }
            catch (e) {
                this._macros = undefined;
                this.removeAttribute('macros');
                throw e;
            }
        }
    }
    /**
     * The Katex displayMode:
     *
     * If true, math will be rendered in display mode (math in display style and
     * center math on page)
     *
     * If false, math will be rendered in inline mode
     */
    get displayMode() {
        return this.hasAttribute('display-mode');
    }
    set displayMode(v) {
        if (v) {
            this.setAttribute('display-mode', '');
        }
        else {
            this.removeAttribute('display-mode');
        }
    }
    get leqno() {
        return this.hasAttribute('leqno');
    }
    set leqno(v) {
        if (v) {
            this.setAttribute('leqno', '');
        }
        else {
            this.removeAttribute('leqno');
        }
    }
    get fleqn() {
        return this.hasAttribute('fleqn');
    }
    set fleqn(v) {
        if (v) {
            this.setAttribute('fleqn', '');
        }
        else {
            this.removeAttribute('fleqn');
        }
    }
    /**
     * Specifies a minimum thickness, in ems, for fraction lines, \sqrt top lines,
     * {array} vertical lines, \hline, \hdashline, \underline, \overline, and the
     * borders of \fbox, \boxed, and \fcolorbox. The usual value for these items
     * is 0.04, so for minRuleThickness to be effective it should probably take a
     * value slightly above 0.04, say 0.05 or 0.06. Negative values will be
     * ignored.
     */
    get minRuleThickness() {
        const attrValue = this.getAttribute('min-rule-thickness');
        if (attrValue == null) {
            return undefined;
        }
        return parseFloat(attrValue);
    }
    set minRuleThickness(v) {
        if (v == null) {
            this.removeAttribute('min-rule-thickness');
        }
        else {
            this.setAttribute('min-rule-thickness', String(v));
        }
    }
    constructor() {
        super();
        const shadowRoot = this.attachShadow({ mode: 'open' });
        let styleSheet = getStyleSheet();
        if (styleSheet) {
            shadowRoot.adoptedStyleSheets = [styleSheet];
        }
        else {
            styleSheet = document.createElement("style");
            styleSheet.innerText = styles;
            shadowRoot.appendChild(styleSheet);
        }
        shadowRoot.appendChild(document.importNode(template.content, true));
        this._container = shadowRoot.querySelector('#container');
        this._slot = shadowRoot.querySelector('slot');
        this._slot.addEventListener('slotchange', () => this._render());
        this._styleTag = shadowRoot.querySelector('style');
    }
    attributeChangedCallback(_name) {
        this._render();
    }
    _render() {
        const inputText = this._slot.assignedNodes().map((n) => n.textContent).join('');
        katex.render(inputText, this._container, { displayMode: this.displayMode });
    }
}
customElements.define('katex-expr', KatexExprElement);
