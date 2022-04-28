/**
 * A Web Component for an editable LaTeX field
 *
 * This renders and display `this.value` using KaTeX. When the element is
 * clicked, the display is replaced by an input box that allows the user to
 * edit the contents.
 *
 * If the content is modified, then a `change` event will be emitted (by the
 * embedded `<input>`, since `change` events bubble).
 *
 * This element contains exactly one `<input>` element (`this.input`)and
 * exactly one `<span>` element (`this.display`), of which exactly one will be
 * visible at any point in time. These components can be styled individually as
 * desired.
 *
 * @property {HTMLInputElement} input The underlying input element.
 * @property {HTMLSpanElement} display The underlying display element.
 * @property {string} value The value of the field as LaTeX code.
 */
class KaTeXInput extends HTMLElement {
    attributeChangedCallback(name, _oldValue, newValue) {
        this.input.setAttribute(name, newValue);
        if (name == 'value') {
            this.update();
        }
    }

    static get observedAttributes() {
        return ['value', 'readonly', 'required', 'placeholder'];
    }

    get value() {
        return this.input.value;
    }

    set value(value) {
        this.input.value = value;
    }

    update() {
        katex.render(this.value, this.display, {
            throwOnError: false,
        });
        if (this.value !== '') {
            this.display.style.removeProperty('display');
            this.input.style.display = 'none';
        }
    }

    constructor() {
        super();

        this.input = document.createElement('input');
        this.display = document.createElement('span');
        this.display.style.display = 'none';

        this.display.addEventListener('click', () => {
            this.display.style.display = 'none';
            this.input.style.removeProperty('display');
            this.input.focus();
        });
        this.input.addEventListener('keyup', e => {
            if (e.key === 'Enter') {
                this.input.blur();
            }
        });
        this.input.addEventListener('focusout', () => {
            this.update();
        });
    }

    connectedCallback() {
        this.update();
        if (!this.input.isConnected) {
            this.appendChild(this.input);
            this.appendChild(this.display);
        }
    }
}
customElements.define('katex-input', KaTeXInput);
