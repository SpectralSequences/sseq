import { STATE_ADD_DIFFERENTIAL } from './display.js';
import {
    rowToKaTeX,
    rowToLaTeX,
    matrixToKaTeX,
    vecToName,
    KATEX_ARGS,
    html,
} from './utils.js';
import { MIN_PAGE } from './sseq.js';

class InputRow extends HTMLElement {
    static attributeMap = {
        label: ['label', 'innerHTML'],
        type: ['input', 'type'],
        value: ['input', 'value'],
        title: ['input', 'title'],
    };

    attributeChangedCallback(name, _oldValue, newValue) {
        const attr = InputRow.attributeMap[name];
        this[attr[0]][attr[1]] = newValue;
    }

    static get observedAttributes() {
        return ['label', 'type', 'value', 'title'];
    }

    constructor() {
        super();

        this.label = document.createElement('label');
        this.input = document.createElement('input');
        this.input.addEventListener('change', e =>
            this.dispatchEvent(new Event('change', e)),
        );
    }

    connectedCallback() {
        if (!this.label.isConnected) {
            this.appendChild(this.label);
            this.appendChild(this.input);
        }
    }
}
customElements.define('input-row', InputRow);

export const ACTION_TO_DISPLAY = {
    AddDifferential: (details, sseq) => {
        const x = details.x;
        const y = details.y;
        const r = details.r;
        const sourceNames = sseq.classNames.get(x, y);
        const targetNames = sseq.classNames.get(x - 1, y + r);
        if (!sourceNames || !targetNames) {
            return [
                '',
                [
                    [details.x, details.y],
                    [details.x - 1, details.y + details.r],
                ],
            ];
        }

        return [
            katex.renderToString(
                `d_{${r}}(${vecToName(
                    details.source,
                    sourceNames,
                )}) = ${vecToName(details.target, targetNames)}`,
                KATEX_ARGS,
            ),
            [
                [details.x, details.y],
                [details.x - 1, details.y + details.r],
            ],
        ];
    },

    AddProductDifferential: (details, sseq) => {
        const content = `
<ul class="text-left" style="padding-left: 20px; list-style-type: none">
  <li>
    <details>
      <summary>source: ${katex.renderToString(
          details.source.name,
          KATEX_ARGS,
      )}</summary>
      (${details.source.x}, ${details.source.y}): ${katex.renderToString(
            rowToLaTeX(details.source.class),
            KATEX_ARGS,
        )}
    </details>
  </li>
  <li>
    <details>
      <summary>target: ${katex.renderToString(
          details.target.name,
          KATEX_ARGS,
      )}</summary>
      (${details.target.x}, ${details.target.y}): ${katex.renderToString(
            rowToLaTeX(details.target.class),
            KATEX_ARGS,
        )}
    </details>
  </li>
</ul>`;
        const diffString = `d_{${details.target.y - details.source.y}}(${
            details.source.name
        }) = ${details.target.name}`;
        return [
            `Propagate ${katex.renderToString(diffString, KATEX_ARGS)}`,
            sseq.isUnit
                ? [
                      [details.source.x, details.source.y],
                      [details.target.x, details.target.y],
                  ]
                : [],
            sseq.isUnit ? undefined : content,
        ];
    },

    AddProductType: (details, sseq) => {
        return [
            `<span>${
                details.permanent ? 'Permanent p' : 'P'
            }roduct ${katex.renderToString(details.name, KATEX_ARGS)}</span>`,
            sseq.isUnit ? [[details.x, details.y]] : [],
            sseq.isUnit
                ? undefined
                : `(${details.x}, ${details.y}): ${katex.renderToString(
                      rowToLaTeX(details.class),
                      KATEX_ARGS,
                  )}`,
        ];
    },

    AddPermanentClass: (details, sseq) => {
        return [
            `Permanent class ${katex.renderToString(
                vecToName(
                    details.class,
                    sseq.classNames.get(details.x, details.y),
                ),
                KATEX_ARGS,
            )}`,
            [[details.x, details.y]],
        ];
    },

    SetClassName: (details, sseq) => {
        const x = details.x;
        const y = details.y;
        const originalName =
            sseq.getClasses(x, y, MIN_PAGE) == 1
                ? `x_{${x},${y}}`
                : `x_{${x},${y}}^{(${details.idx})}`;

        return [
            'Rename ' +
                katex.renderToString(
                    `${originalName} \\rightsquigarrow ${details.name}`,
                    KATEX_ARGS,
                ),
            [[details.x, details.y]],
        ];
    },
};

function msgToDisplay(msg, sseq) {
    if (!msg) {
        return ['', []];
    }
    const action = msg.action;
    const actionName = Object.keys(action)[0];
    const actionInfo = action[actionName];

    return ACTION_TO_DISPLAY[actionName](actionInfo, sseq);
}

function createPanelLine(sseq, html, callback, highlights) {
    const node = document.createElement('div');
    node.className = 'panel-line';
    node.innerHTML = html;
    if (callback) {
        node.addEventListener('click', callback);
    }
    if (highlights) {
        node.addEventListener('mouseover', () => {
            node.style.color = 'blue';
            for (const highlight of highlights) {
                sseq.highlightClass(highlight[0], highlight[1]);
            }
        });
        node.addEventListener('mouseout', () => {
            node.style.removeProperty('color');
            sseq.clearHighlight();
        });
    }
    return node;
}

/**
 * Create a button.
 *
 * @param {string} text - Text to appear on the button.
 * @param {function} callback - Function to call when button is clicked.
 */
export function createButton(text, callback) {
    const o = document.createElement('button');
    o.innerHTML = text;
    o.classList.add('button');
    o.setAttribute('type', 'button');
    o.addEventListener('click', callback);
    o.addEventListener('mouseup', () => o.blur());
    o.style.width = '100%';

    return o;
}

/**
 * This adds several buttons placed side-by-side on a row.
 *
 * @param {Array[]} buttons - An array of arguments specifying the buttons
 * to be added. Each entry in the array should itself be an array, which
 * consists of the arguments to createButton for the corresponding
 * button.
 */
export function createButtonRow(buttons) {
    const div = document.createElement('div');
    div.className = 'button-row';

    for (const button of buttons) {
        const p = document.createElement('button');
        p.innerHTML = button[0];
        p.classList.add('button');
        p.setAttribute('type', 'button');
        p.addEventListener('click', button[1]);
        p.addEventListener('mouseup', () => p.blur());

        div.appendChild(p);
    }
    return div;
}

/**
 * This is a panel consisting of multiple tabs. A tab is a generator function
 * that takes in the sseq in question and yields the elements to be displayed.
 */
export class TabbedPanel extends HTMLElement {
    constructor() {
        super();

        this.head = document.createElement('div');
        this.head.className = 'tab-header';

        this.inner = document.createElement('div');
        this.inner.className = 'tab-main';

        this.tabs = [];
        this.currentIndex = 0;
        this.sseq = undefined;
    }

    connectedCallback() {
        if (!this.head.isConnected) {
            this.appendChild(this.head);
            this.appendChild(this.inner);
        }
    }

    /**
     * This adds a tab to TabbedPanel.
     *
     * @param {string} name - The name of the tab, to be displayed in the
     * header. Avoid making this too long.
     * @param tab - The tab to be added.
     */
    addTab(name, tab) {
        const a = document.createElement('a');
        a.className = 'tab-header-item';
        a.href = '#';
        a.innerHTML = name;
        this.head.appendChild(a);

        const idx = this.tabs.length;
        a.addEventListener('click', () => {
            this.currentIndex = idx;
            this.update();
        });
        this.tabs[idx] = [tab, a];
    }

    hide() {
        this.style.display = 'none';
    }

    show() {
        this.style.removeProperty('display');
        this.update();
    }

    nextTab() {
        this.currentIndex += 1;
        this.currentIndex %= this.tabs.length;

        this.update();
    }

    prevTab() {
        this.currentIndex += this.tabs.length - 1;
        this.currentIndex %= this.tabs.length;

        this.update();
    }

    update() {
        const tab = this.tabs[this.currentIndex];

        this.head
            .querySelectorAll('.active')
            .forEach(x => x.classList.remove('active'));
        tab[1].classList.add('active');

        while (this.inner.firstChild)
            this.inner.removeChild(this.inner.firstChild);

        for (let group of tab[0](this.sseq)) {
            if (typeof group === 'string') {
                group = html(group);
            }
            this.inner.appendChild(group);
        }
    }
}
customElements.define('tabbed-panel', TabbedPanel);

export function generalPanel(sseq) {
    const panel = document.createElement('tabbed-panel');
    panel.sseq = sseq;
    panel.addTab('Main', overviewPanel);
    panel.addTab('Prod', structlinePanel);
    panel.addTab('Hist', historyPanel);
    panel.update();
    return panel;
}

function* historyPanel(sseq) {
    for (const data of sseq.history) {
        const [titleText, highlightClasses, content] = msgToDisplay(data, sseq);

        let div;
        if (content !== undefined) {
            div = html(`
            <details>
                <summary>
                    <span>${titleText}</span>
                    <a style="float: right; color: #dc3545" href="#">&times;</a>
                </summary>
                <div style="text-align: center">${content}</div>
            </details>`);
        } else {
            div = html(`
                <div>
                    <span>${titleText}</span>
                    <a style="float: right; color: #dc3545" href="#">&times;</a>
                </div>`);
        }
        div.querySelector('a').addEventListener('click', () => {
            sseq.clearHighlight();
            sseq.removeHistoryItem(data);
        });

        div.addEventListener('mouseover', () => {
            div.style.color = 'blue';
            for (const pair of highlightClasses) {
                sseq.highlightClass(pair[0], pair[1]);
            }
        });
        div.addEventListener('mouseout', () => {
            div.style.removeProperty('color');
            sseq.clearHighlight();
        });
        yield div;
    }
}

function* overviewPanel(sseq) {
    yield '<h2>Vanishing line</h2>';

    const slope = html(
        `<input-row label="Slope" value="${sseq.vanishingSlope}"></input-row>"`,
    );

    slope.addEventListener('change', e => {
        sseq.vanishingSlope = e.target.value;
        sseq.updateDegrees();
    });
    yield slope;

    const intercept = html(
        `<input-row label="Intercept" value="${sseq.vanishingIntercept}"></input-row>`,
    );

    intercept.addEventListener('change', e => {
        sseq.vanishingIntercept = e.target.value;
        sseq.updateDegrees();
    });
    yield intercept;

    yield createButton('Resolve further', () => sseq.resolveFurther());
}

function* structlinePanel(sseq) {
    const prod = Array.from(sseq.products.entries()).sort();
    for (const [name, mult] of prod) {
        const div = html(`
<div style="position: relative">
    <details class="product-item">
        <summary class="product-summary" style="width: 100%" onmouseup="event.target.blur()">
            <label>${katex.renderToString(name, KATEX_ARGS)}</label>
            <span style="flex-grow: 1"></span>
        </summary>
        <div class="structline-style">
            <input-row label="Color" value="${mult.style.color}"></input-row>
            <input-row label="Bend" value="${mult.style.bend}"></input-row>
            <input-row label="Dash" value="${mult.style.dash}"
             title="A dash pattern, in the format of SVG's stroke-dasharray. Note that each grid square has side length 1."></input-row>
        </div>
    </details>
    <label class="switch" style="position: absolute; right: 0px">
        <input type="checkbox"${
            sseq.visibleStructlines.has(name) ? ' checked' : ''
        }></input>
        <span class="slider"></span>
    </label>
</div>`);

        const i = div.querySelector('label.switch');
        i.style.top =
            div.querySelector('summary').clientHeight - i.clientHeight + 'px';

        div.querySelector('label.switch > input').addEventListener(
            'change',
            e => {
                if (e.target.checked) {
                    sseq.showStructlines(name);
                } else {
                    sseq.hideStructlines(name);
                }
            },
        );

        const updateStyleObject = () => {
            if (mult.style.styleObject === null) {
                mult.style.styleObject = sseq.chart.addStyle();
            }

            let styleText = `.structline-${CSS.escape(name)} {`;
            if (mult.style.color !== 'black') {
                styleText += `stroke: ${mult.style.color};`;
            }
            if (mult.style.dash !== '') {
                styleText += `stroke-dasharray: ${mult.style.dash};`;
            }
            styleText += '}';
            mult.style.styleObject.textContent = styleText;
        };

        div.querySelector('input-row[label="Color"]').addEventListener(
            'change',
            e => {
                mult.style.color = e.target.value;
                updateStyleObject();
            },
        );

        div.querySelector('input-row[label="Bend"]').addEventListener(
            'change',
            e => {
                mult.style.bend = parseInt(e.target.value);
                sseq.hideStructlines(name);
                sseq.showStructlines(name);
            },
        );

        div.querySelector('input-row[label="Dash"]').addEventListener(
            'change',
            e => {
                mult.style.dash = e.target.value;
                updateStyleObject();
            },
        );

        yield div;
    }

    if (!sseq.isUnit) {
        yield createButton('Add', () => window.unitDisplay.openModal());
    }
}

export function classPanel(sseq) {
    const panel = document.createElement('tabbed-panel');
    panel.sseq = sseq;
    panel.addTab('Main', mainPanel);
    panel.addTab('Diff', differentialPanel);
    panel.addTab('Prod', productsPanel);
    panel.update();
    return panel;
}

function* mainPanel(sseq) {
    if (!sseq.selected) return;

    yield '<h2>Classes</h2>';

    const [x, y] = sseq.selected;

    const classes = sseq.getClasses(x, y, sseq.page);
    const names = sseq.classNames.get(x, y);

    const div = document.createElement('div');
    div.style.textAlign = 'center';
    for (const c of classes) {
        const n = html(`<span style="padding: 0 0.6em">
            ${katex.renderToString(vecToName(c, names), KATEX_ARGS)}
        </span>`);

        if (classes.length == sseq.classes.get(x, y)[0].length) {
            n.addEventListener('click', () => {
                const name = prompt('New class name');
                if (name !== null) {
                    sseq.setClassName(x, y, c.indexOf(1), name);
                }
            });
        }
        div.appendChild(n);
    }
    yield div;

    const decompositions = sseq.decompositions.get(x, y);
    if (decompositions && decompositions.length > 0) {
        yield '<h2>Decompositions</h2>';
        for (const d of decompositions) {
            const single = d[0].reduce((a, b) => a + b, 0) == 1;

            const highlights = [[x - d[2], y - d[3]]];
            if (sseq.isUnit) {
                highlights[1] = [d[2], d[3]];
            }
            if (single) {
                const idx = d[0].indexOf(1);
                // If we named the element after the decomposition, there is no point in displaying it...
                if (
                    katex.renderToString(names[idx], KATEX_ARGS) !=
                    katex.renderToString(d[1], KATEX_ARGS)
                ) {
                    yield createPanelLine(
                        sseq,
                        katex.renderToString(
                            names[idx] + ' = ' + d[1],
                            KATEX_ARGS,
                        ),
                        () => {
                            if (confirm(`Rename ${names[idx]} as ${d[1]}?`)) {
                                sseq.setClassName(x, y, idx, d[1]);
                                this.display.clearHighlight();
                            }
                        },
                        highlights,
                    );
                }
            } else {
                yield createPanelLine(
                    sseq,
                    katex.renderToString(
                        vecToName(d[0], names) + ' = ' + d[1],
                        KATEX_ARGS,
                    ),
                    undefined,
                    highlights,
                );
            }
        }
    }

    if (sseq.isUnit) {
        yield '<div class="panel-spacer"></div>';
        yield createButton('Add Product', () => {
            const [x, y] = sseq.selected;
            const num = sseq.getClasses(x, y, MIN_PAGE).length;
            sseq.addProductInteractive(x, y, num);
        });
    }
}

function* differentialPanel(sseq) {
    if (!sseq.selected) return;

    const [x, y] = sseq.selected;
    const page = sseq.page;

    // We don't use display.selected because this would refer to the wrong object after we add a differential.
    if (sseq.classState.get(x, y) == 'InProgress') {
        yield '<h2>Possible Differentials</h2>';

        const div = html('<div style="text-align: center"></div>');
        const maxR =
            Math.ceil(
                eval(sseq.vanishingSlope) * x + eval(sseq.vanishingIntercept),
            ) - y;

        for (let r = MIN_PAGE; r <= maxR; r++) {
            const classes = sseq.getClasses(x - 1, y + r, r);
            if (
                classes &&
                classes.length > 0 &&
                (!sseq.trueDifferentials.get(x, y) ||
                    !sseq.trueDifferentials.get(x, y)[r - MIN_PAGE] ||
                    sseq.getClasses(x, y, r).length !=
                        sseq.trueDifferentials.get(x, y)[r - MIN_PAGE].length)
            ) {
                const spn = createPanelLine(sseq, r, null, [[x - 1, y + r]]);
                spn.style.padding = '0.4rem 0.75rem';
                spn.style.margin = '0';
                spn.style.display = 'inline-block';
                div.appendChild(spn);
            }
        }

        if (div.childElementCount === 0) {
            div.appendChild(
                document.createTextNode('No possible differentials!'),
            );
        }
        yield div;
    }

    yield '<h2>Differentials</h2>';
    let hasDifferential = false;
    const trueDifferentials = sseq.trueDifferentials.get(x, y);
    if (trueDifferentials && trueDifferentials.length > page - MIN_PAGE) {
        for (const [source, target] of trueDifferentials[page - MIN_PAGE]) {
            hasDifferential = true;
            let callback;
            if (sseq.isUnit) {
                callback = () => {
                    const source_ = sseq.pageBasisToE2Basis(page, x, y, source);
                    const target_ = sseq.pageBasisToE2Basis(
                        page,
                        x - 1,
                        y + page,
                        target,
                    );
                    sseq.addProductDifferentialInteractive(
                        x,
                        y,
                        page,
                        source_,
                        target_,
                    );
                };
            }
            yield createPanelLine(
                sseq,
                katex.renderToString(
                    `d_${page}(${rowToLaTeX(source)}) = ${rowToLaTeX(target)}`,
                    KATEX_ARGS,
                ),
                callback,
            );
        }
    }
    if (sseq.isUnit && hasDifferential) {
        yield createPanelLine(
            sseq,
            "<span style='font-size: 80%'>Click differential to propagate</span>",
        );
    }
    if (sseq.classState.get(x, y) === 'InProgress') {
        yield '<div class="panel-spacer"></div>';
        yield createButton(
            'Add Differential',
            () => (sseq.display.state = STATE_ADD_DIFFERENTIAL),
        );
    }

    yield '<h2>Permanent Classes</h2>';
    const permanentClasses = sseq.permanentClasses.get(x, y);
    if (permanentClasses.length > 0) {
        yield createPanelLine(
            sseq,
            permanentClasses.map(rowToKaTeX).join('<br />'),
        );
    }
    if (sseq.classState.get(x, y) === 'InProgress') {
        yield '<div class="panel-spacer"></div>';
        yield createButton('Add Permanent Class', () => {
            sseq.addPermanentClassInteractive(x, y);
        });
    }
}

function* productsPanel(sseq) {
    if (!sseq.selected) return;

    const [x, y] = sseq.selected;
    const page = sseq.page;

    for (const [name, mult] of sseq.products) {
        const matrices = mult.matrices.get(x, y);
        if (matrices === undefined || matrices === null) continue;

        const page_idx = Math.min(matrices.length - 1, page - MIN_PAGE);
        const matrix = matrices[page_idx];

        if (matrix.length === 0 || matrix[0].length == 0) {
            continue;
        }

        yield createPanelLine(
            sseq,
            `${katex.renderToString(name, KATEX_ARGS)}: ${matrixToKaTeX(
                matrix,
            )}`,
            null,
            [
                [x + mult.x, y + mult.y],
                [x - mult.x, y - mult.y],
            ],
        );
    }
}
