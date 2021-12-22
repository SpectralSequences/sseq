import {
    generalPanel,
    classPanel,
    createButton,
    createButtonRow,
} from './panels.js';
import { MIN_PAGE } from './sseq.js';
import { Sidebar } from './chart.js';

export const STATE_ADD_DIFFERENTIAL = 1;
export const STATE_QUERY_TABLE = 2;
export const STATE_QUERY_COCYCLE_STRING = 3;

export class MainDisplay {
    constructor(container, sseq, isUnit) {
        container = document.getElementById(container);

        container.style.display = 'flex';
        container.style.displayDirection = 'row';

        sseq.display = this;
        this.sseq = sseq;

        sseq.chart.style.height = '100%';
        sseq.chart.style.minHeight = '100%';
        sseq.chart.style.overflow = 'hidden';
        sseq.chart.style.flexGrow = '1';
        sseq.chart.style.flexBasis = '0';

        container.appendChild(sseq.chart);

        this.sidebar = new Sidebar();
        this.sidebar.style.width = '250px';
        container.appendChild(this.sidebar);
        this.sidebar.addEventListener('resize', () =>
            this.sseq.chart.onResize(),
        );
        this.sseq.chart.onResize();

        this.panels = [];
        this.currentPanel = null;

        this.isUnit = isUnit;

        window.addEventListener('keydown', this._onKeyDown.bind(this));

        sseq.chart.addEventListener('newpage', this.refreshPanel.bind(this));
        sseq.refreshPanel = this.refreshPanel.bind(this);
        sseq.onClick = this._onClick.bind(this);

        // Populate sidebar

        const inner = document.createElement('div');
        inner.classList.add('inner');
        this.sidebar.appendChild(inner);

        this.generalPanel = generalPanel(sseq);
        inner.appendChild(this.generalPanel);
        this.currentPanel = this.generalPanel;

        this.classPanel = classPanel(sseq);
        inner.appendChild(this.classPanel);

        this.classPanel.hide();

        this.footer = document.createElement('div');
        this.footer.classList.add('footer');

        this.footer.style.textAlign = 'center';
        this.runningSign = document.createElement('p');
        this.runningSign.innerHTML = 'Running...';
        this.footer.appendChild(this.runningSign);

        this.footer.appendChild(
            createButtonRow([
                ['Undo', () => this.sseq.undo()],
                ['Redo', () => this.sseq.redo()],
            ]),
        );

        this.footer.appendChild(createButton('Save', () => window.save()));
        inner.appendChild(this.footer);
    }

    _onKeyDown(e) {
        switch (e.key) {
            case 'J':
                this.currentPanel.prevTab();
                break;
            case 'K':
                this.currentPanel.nextTab();
                break;
            case 'd':
                this.state = STATE_ADD_DIFFERENTIAL;
                break;
            case 'p':
                if (this.sseq.selected)
                    this.sseq.addPermanentClassInteractive(
                        ...this.sseq.selected,
                    );
                break;
            case 'y':
                this.state = STATE_QUERY_TABLE;
                break;
            case 'x':
                this.state = STATE_QUERY_COCYCLE_STRING;
                break;
            case 'n':
                if (!this.sseq.selected) break;
                {
                    // Keep the variable declarations within the block
                    const [x, y] = this.sseq.selected;
                    const num = this.sseq.getClasses(x, y, MIN_PAGE).length;

                    let idx = 0;
                    if (num != 1) {
                        while (true) {
                            idx = prompt('Class index');
                            if (idx === null) return;

                            idx = parseInt(idx);
                            if (Number.isNaN(idx) || idx >= num || idx < 0) {
                                alert(
                                    `Invalid index. Enter integer between 0 and ${num} (inclusive)`,
                                );
                            } else {
                                break;
                            }
                        }
                    }

                    const name = prompt('New class name');
                    if (name !== null) {
                        this.sseq.setClassName(x, y, idx, name);
                    }
                }
                break;
            case 'm':
                if (this.isUnit && this.sseq.selected) {
                    const [x, y] = this.sseq.selected;
                    const num = this.sseq.getClasses(x, y, MIN_PAGE).length;
                    this.sseq.addProductInteractive(x, y, num);
                }
                break;
        }
    }

    refreshPanel() {
        if (this.sseq.selected) {
            this.generalPanel.hide();
            this.classPanel.show();
            this.currentPanel = this.classPanel;
        } else {
            this.classPanel.hide();
            this.generalPanel.show();
            this.currentPanel = this.generalPanel;
        }
    }

    _onClick(oldSelected) {
        if (!this.sseq.selected) {
            this.state = null;
            return;
        }

        switch (this.state) {
            case STATE_QUERY_TABLE:
                this.sseq.queryTable(...this.sseq.selected);
                this.state = null;
                break;
            case STATE_QUERY_COCYCLE_STRING:
                this.sseq.queryCocycleString(...this.sseq.selected);
                this.state = null;
                break;
            case STATE_ADD_DIFFERENTIAL:
                if (
                    oldSelected &&
                    oldSelected[0] == this.sseq.selected[0] + 1 &&
                    this.sseq.selected[1] - oldSelected[1] >= MIN_PAGE
                ) {
                    this.sseq.addDifferentialInteractive(
                        oldSelected,
                        this.sseq.selected,
                    );
                    this.state = null;
                    this.sseq.select(oldSelected);
                }
                break;
        }
    }
}

export class UnitDisplay {
    constructor(container, sseq) {
        this.sseq = sseq;
        document.getElementById(container).appendChild(sseq.chart);

        document.querySelectorAll('.close-modal').forEach(c => {
            c.addEventListener('click', this.closeModal.bind(this));
        });

        document.querySelector('#modal-diff').addEventListener('click', () => {
            document.querySelector('#modal-title').innerHTML =
                'Select target element';
            this.state = STATE_ADD_DIFFERENTIAL;
        });

        document.querySelector('#modal-ok').addEventListener('click', () => {
            const [x, y] = this.sseq.selected;
            const num = this.sseq.getClasses(x, y, MIN_PAGE).length;
            window.mainSseq.addProductInteractive(x, y, num);
            this.closeModal();
        });

        document
            .querySelector('#modal-more')
            .addEventListener('click', () => this.sseq.resolveFurther());
        document
            .querySelector('#modal-more')
            .addEventListener('mouseup', () =>
                document.querySelector('#modal-more').blur(),
            );

        sseq.onClick = this._onClick.bind(this);
    }

    openModal() {
        this.sseq.resolveFurther(10);
        this.sseq.chart.onResize();
        document.querySelector('#overlay').style.removeProperty('display');
        document.querySelector('#modal-ok').disabled = true;
        document.querySelector('#modal-diff').disabled = true;
        const dialog = document.querySelector('#unitsseq-dialog');
        dialog.classList.add('modal-shown');
    }

    closeModal() {
        document.querySelector('#overlay').style.display = 'none';
        const dialog = document.querySelector('#unitsseq-dialog');
        dialog.classList.remove('modal-shown');
    }

    _onClick(oldSelected) {
        if (!this.sseq.selected) {
            this.state = null;

            document.querySelector('#modal-title').innerHTML =
                'Select element to multiply with';
            document.querySelector('#modal-ok').disabled = true;
            document.querySelector('#modal-diff').disabled = true;
            return;
        }

        if (this.state == STATE_ADD_DIFFERENTIAL) {
            if (
                this.sseq.selected[0] == oldSelected[0] - 1 &&
                this.sseq.selected[1] - oldSelected[1] >= MIN_PAGE
            ) {
                const check = confirm(
                    `Add differential from (${oldSelected[0]}, ${oldSelected[1]}) to (${this.sseq.selected[0]}, ${this.sseq.selected[1]})?`,
                );
                if (check) {
                    this.sseq.addProductDifferentialInteractive(
                        oldSelected[0],
                        oldSelected[1],
                        this.sseq.selected[1] - oldSelected[1],
                    );
                    this.state = null;
                    this.closeModal();
                }
            } else {
                alert('Invalid target for differential');
            }
        } else {
            this.state = null;
        }
        document.querySelector('#modal-ok').disabled = false;
        document.querySelector('#modal-diff').disabled = false;
    }
}
