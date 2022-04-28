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

export let dialogOpen = 0;

class MyDialog extends HTMLDialogElement {
    attributeChangedCallback(name, _oldValue, newValue) {
        if (name === 'title') {
            this.header.innerHTML = newValue;
        }
    }

    static get observedAttributes() {
        return ['title'];
    }

    constructor() {
        super();
        this.form = document.createElement('form');
        this.form.setAttribute('method', 'dialog');
        this.header = document.createElement('h5');
        this.form.appendChild(this.header);
    }

    connectedCallback() {
        if (!this.form.isConnected) {
            while (this.hasChildNodes()) {
                this.form.appendChild(this.firstChild);
            }
            this.appendChild(this.form);
            if (this.getAttribute('permanent') === null) {
                this.addEventListener('close', () => {
                    this.parentNode.removeChild(this);
                });
            }
            this.addEventListener('close', () => (dialogOpen -= 1));
        }
    }

    showModal() {
        dialogOpen += 1;
        super.showModal();
    }
}
customElements.define('my-dialog', MyDialog, { extends: 'dialog' });

/**
 * A slider checkbox
 */
class CheckboxSwitch extends HTMLElement {
    static get observedAttributes() {
        return ['checked'];
    }

    attributeChangedCallback(name, _oldValue, newValue) {
        this.checkbox.setAttribute(name, newValue);
    }

    static STYLE = `
/**
 * Adapted from https://www.w3schools.com/howto/howto_css_switch.asp
 * There are three parameters one can control - the width (w), the height (h)
 * and the margin (m).
 * */
label {
    position: relative;
    display: inline-block;
    width: 32px; /* w */
    height: 20px; /* h */
}

input {
    opacity: 0;
    width: 0;
    height: 0;
}

span {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    border-radius: 20px; /* h */
    background-color: #ccc;
    transition: 0.4s;
}

span:before {
    position: absolute;
    content: '';
    height: 14px; /* h - 2m */
    width: 14px; /* h - 2m */
    left: 3px; /* m */
    bottom: 3px; /* m */
    background-color: white;
    border-radius: 50%;
    transition: 0.4s;
}

input:checked + span {
    background-color: #67a1f8;
}

input:focus + span {
    box-shadow: 0 0 3px #67a1f8;
}

input:checked + span:before {
    transform: translateX(12px); /* w - h */
}
`;

    constructor() {
        super();

        const shadow = this.attachShadow({ mode: 'open' });

        const style = document.createElement('style');
        style.innerHTML = CheckboxSwitch.STYLE;

        const label = document.createElement('label');

        this.checkbox = document.createElement('input');
        this.checkbox.setAttribute('type', 'checkbox');

        const slider = document.createElement('span');

        shadow.appendChild(style);

        label.appendChild(this.checkbox);
        label.appendChild(slider);
        shadow.appendChild(label);
    }

    get checked() {
        return this.checkbox.checked;
    }

    set checked(checked) {
        this.checkbox.checked = checked;
    }
}
customElements.define('checkbox-switch', CheckboxSwitch);
