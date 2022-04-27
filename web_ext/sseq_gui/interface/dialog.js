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
        }
    }
}
customElements.define('my-dialog', MyDialog, { extends: 'dialog' });
