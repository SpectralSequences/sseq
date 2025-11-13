import { dialog } from './utils.js';
import { svgNS } from './chart.js';
import './components.js';

export const MIN_PAGE = 2;
const CHART_STYLE = `
.class-error {
    fill: #a6001a;
}
.class-done {
    fill: gray;
}
.class-group:hover > circle {
    fill: red;
}
.highlight > circle, .selected > circle {
    fill: red;
}
.differential {
    stroke-width: 0.02;
    stroke: orange;
}
.structline {
    stroke-width: 0.02;
    stroke: black;
}
.d2 {
    stroke: cyan;
}
.d3 {
    stroke: red;
}
.d4 {
    stroke: green;
}
.d5 {
    stroke: blue;
}
`;

const KEEP_LOG = new Set([
    'AddDifferential',
    'AddProductType',
    'AddProductDifferential',
    'AddPermanentClass',
    'SetClassName',
]);

function bidegreeToCoordinates({ n, s }) {
    return [n, s];
}

function coordinatesToBidegree(x, y) {
    return { n: x, s: y };
}

export class BiVec {
    constructor(minDegree, data) {
        this.data = data ? data : [];
        this.minDegree = minDegree;
    }
    set(x, y, data) {
        while (this.data.length <= x - this.minDegree) {
            this.data.push([]);
        }
        this.data[x - this.minDegree][y] = data;
    }
    get(x, y) {
        return this.data?.[x - this.minDegree]?.[y];
    }
}

export class ExtSseq {
    constructor(name, minDegree) {
        this.minDegree = minDegree;
        this.maxDegree = minDegree;

        this.history = [];
        this.redoStack = [];
        this.name = name;

        this.vanishingSlope = '1/2';
        this.vanishingIntercept = 1;
        this.visibleStructlines = new Set(['h_0', 'a_0', 'h_1', 'h_2']);

        this.classes = new BiVec(minDegree);
        this.classState = new BiVec(minDegree);
        this.permanentClasses = new BiVec(minDegree);
        this.classNames = new BiVec(minDegree);
        this.decompositions = new BiVec(minDegree);
        this.products = new Map();
        this.trueDifferentials = new BiVec(minDegree);

        this.chart = document.createElement('paged-chart');
        this.chart.addStyle(CHART_STYLE);
        this.chart.setAttribute('minx', minDegree);
        this.chart.addEventListener('click', () => this.select(null));

        this.selected = null;
        this.refreshPanel = undefined;
        this.onClick = undefined;

        this.chart.newPage();

        this._onClassClick = this.__onClassClick.bind(this);
    }

    get page() {
        return this.chart.page + MIN_PAGE;
    }

    newPage() {
        const prevPage = this.chart.pages[this.chart.pages.length - 1];
        const page = prevPage.cloneNode(true);
        for (const node of page.getElementsByClassName('class-group')) {
            node.childNodes.forEach(x => (x.onclick = this._onClassClick));
        }
        this.chart.appendPage(page);
    }

    send(data, log = true) {
        if (KEEP_LOG.has(Object.keys(data.action)[0])) {
            if (log) {
                this.history.push(data);
            }
        }

        data.sseq = this.name;
        window.send(data);
    }

    removeHistoryItem(msg) {
        const actionKey = Object.keys(msg['action'])[0];
        dialog(
            'Undo action',
            `<section>Undoing action:
            <pre>${actionKey}: ${JSON.stringify(
                msg['action'][actionKey],
                null,
                4,
            )}</pre>
            </section>`,
            () => {
                msg = JSON.stringify(msg);
                this.history = this.history.filter(
                    m => JSON.stringify(m) != msg,
                );

                this.block();
                this.send({
                    recipients: ['Sseq'],
                    action: { Clear: {} },
                });

                for (const msg of this.history) {
                    this.send(msg, false);
                }
                this.refreshPanel?.();
                this.block(false);
            },
            'Undo',
        );
    }

    block(block = true) {
        this.send({
            recipients: ['Sseq'],
            action: { BlockRefresh: { block: block } },
        });
    }

    undo() {
        this.redoStack.push(this.history.pop());

        this.block();
        this.send({
            recipients: ['Sseq'],
            action: { Clear: {} },
        });

        for (const msg of this.history) {
            this.send(msg, false);
        }
        this.refreshPanel?.();
        this.block(false);
    }

    redo() {
        this.send(this.redoStack.pop());
    }

    addPermanentClass(x, y, target) {
        const b = coordinatesToBidegree(x, y);
        this.send({
            recipients: ['Sseq'],
            action: {
                AddPermanentClass: {
                    b: b,
                    class: target,
                },
            },
        });
    }

    pageBasisToE2Basis(r, x, y, c) {
        const len = this.classes.get(x, y)[0].length;
        const pageBasis = this.getClasses(x, y, r);

        const result = [];
        for (let i = 0; i < len; i++) {
            result.push(0);
        }
        for (let i = 0; i < pageBasis.length; i++) {
            const coef = c[i];
            for (let j = 0; j < len; j++) {
                result[j] += coef * pageBasis[i][j];
            }
        }
        for (let i = 0; i < len; i++) {
            result[i] = result[i] % this.p;
        }
        return result;
    }

    addDifferentialInteractive(source, target) {
        const page = target[1] - source[1];
        const sourceDim = this.getClasses(source[0], source[1], page).length;
        const targetDim = this.getClasses(target[0], target[1], page).length;

        dialog(
            `Input differential at (${source[0]}, ${source[1]})`,
            `<section style="text-align: center">
                ${katex.renderToString(`d_{${page}}`)}
                <input name="source" is="class-input"
                    title="Express source in E${page} page basis"
                    length="${sourceDim}" p=${this.p}></input>
                =
                <input name="target" is="class-input"
                    title="Express target in E${page} page basis"
                    length="${targetDim}" p=${this.p}></input>
            </section>`,
            dialog => {
                const sourceVec = this.pageBasisToE2Basis(
                    page,
                    source[0],
                    source[1],
                    eval(dialog.querySelector("input[name='source']").value),
                );
                const targetVec = this.pageBasisToE2Basis(
                    page,
                    source[0] - 1,
                    source[1] + page,
                    eval(dialog.querySelector("input[name='target']").value),
                );

                this.addDifferential(
                    page,
                    source[0],
                    source[1],
                    sourceVec,
                    targetVec,
                );
            },
        );
    }

    setClassName(x, y, idx, name) {
        this.send({
            recipients: ['Sseq'],
            action: { SetClassName: { x: x, y: y, idx: idx, name: name } },
        });
    }

    // addProductInteractive takes in the number of classes in bidegree (x, y), because this should be the number of classes in the *unit* spectral sequence, not the main spectral sequence
    addProductInteractive(x, y, num) {
        const b = coordinatesToBidegree(x, y);
        dialog(
            `Add product at (${x}, ${y})`,
            `<section style="display: flex; justify-content: center; align-items: center; gap: 1em">
                <katex-input style='text-align: right' width='5em' input placeholder='name' title='Name of product'></katex-input>
                =
                <input name='class' is='class-input' title='Class in E2 page basis' length='${num}' p=${this.p}></input>
            </section>
            <section style="display: flex; justify-content: center; align-items: center; gap: 1em">
                Permanent <checkbox-switch checked></checkbox-switch>
            </section>`,
            dialog => {
                this.send({
                    recipients: ['Sseq', 'Resolver'],
                    action: {
                        AddProductType: {
                            permanent:
                                dialog.querySelector('checkbox-switch')
                                    .checked === true,
                            b: b,
                            class: eval(
                                dialog.querySelector("input[name='class']")
                                    .value,
                            ),
                            name: dialog.querySelector('katex-input').value,
                        },
                    },
                });
            },
        );
    }

    addProductDifferentialInteractive(
        sourceX,
        sourceY,
        page,
        sourceClass,
        targetClass,
    ) {
        const sourceDim = this.getClasses(sourceX, sourceY, MIN_PAGE).length;
        const targetDim = this.getClasses(
            sourceX - 1,
            sourceY + page,
            MIN_PAGE,
        ).length;
        const sourceB = coordinatesToBidegree(sourceX, sourceY);
        const targetB = coordinatesToBidegree(sourceX - 1, sourceY + page);
        dialog(
            `Add product differential at (${sourceX}, ${sourceY})`,
            `<section style="text-align: center">
                ${katex.renderToString(`d_{${page}}`)}
                <input name="source" is="class-input"
                    title="Express source in E${page} page basis"
                    length="${sourceDim}" p=${this.p}
                    value="${sourceClass ? '[' + sourceClass.join(', ') + ']' : ''
            }"
                ></input>
                =
                <input name="target" is="class-input"
                    title="Express target in E${page} page basis"
                    length="${targetDim}" p=${this.p}
                    value="${targetClass ? '[' + targetClass.join(', ') + ']' : ''
            }"
                ></input>
            </section>
            <section>
                <div class="input-row">
                    <label style="width: 6em">Source name</label>
                    <katex-input width="10em" input title='Name of source' placeholder='source name' name='source-name'></katex-input>
                </div>
                <div class="input-row">
                    <label style="width: 6em">Target name</label>
                    <katex-input width="10em" input title='Name of target' placeholder='target name' name='target-name'></katex-input>
                </div>
            </section>`,
            dialog => {
                window.mainSseq.send({
                    recipients: ['Sseq', 'Resolver'],
                    action: {
                        AddProductDifferential: {
                            source: {
                                permanent: false,
                                b: sourceB,
                                class: eval(
                                    dialog.querySelector("input[name='source']")
                                        .value,
                                ),
                                name: dialog
                                    .querySelector(
                                        "katex-input[name='source-name']",
                                    )
                                    .value.trim(),
                            },
                            target: {
                                permanent: false,
                                b: targetB,
                                class: eval(
                                    dialog.querySelector("input[name='target']")
                                        .value,
                                ),
                                name: dialog
                                    .querySelector(
                                        "katex-input[name='target-name']",
                                    )
                                    .value.trim(),
                            },
                        },
                    },
                });
            },
        );
    }

    addPermanentClassInteractive(x, y) {
        const classes = this.classes.get(x, y);

        const last = classes[classes.length - 1];
        if (last.length == 0) {
            dialog(
                `Add permanent class at (${x}, ${y})`,
                '<section>There are no surviving classes</section>',
                () => { },
                'OK',
            );
        } else if (classes[0].length == 1) {
            this.addPermanentClass(x, y, classes[0][0]);
        } else {
            dialog(
                `Add permanent class at (${x}, ${y})`,
                `<section class="input-row">
                    <label>Class</label><input is="class-input" p="${this.p}" length="${classes[0].length}"></input>
                </section>`,
                dialog =>
                    this.addPermanentClass(
                        x,
                        y,
                        eval(dialog.querySelector('input').value),
                    ),
            );
        }
    }

    addDifferential(r, source_x, source_y, source, target) {
        const sourceB = coordinatesToBidegree(source_x, source_y);
        this.send({
            recipients: ['Sseq'],
            action: {
                AddDifferential: {
                    r: r,
                    b: sourceB,
                    source: source,
                    target: target,
                },
            },
        });
    }

    resolveFurther(newmax) {
        if (Number.isInteger(newmax)) {
            this.maxDegree = newmax;
            this.send({
                recipients: ['Resolver'],
                action: {
                    Resolve: {
                        max_degree: newmax,
                    },
                },
            });
            return;
        }

        dialog(
            'Resolve further',
            `<section style="input-row">
                <label>New maximum degree</label>
                <input style="width: 5em" type="number" value="${this.maxDegree + 10
            }">
            </section>`,
            dialog => {
                newmax = parseInt(dialog.querySelector('input').value);
                if (newmax <= this.maxDegree) {
                    return;
                }
                this.maxDegree = newmax;

                this.send({
                    recipients: ['Resolver'],
                    action: {
                        Resolve: {
                            max_degree: newmax,
                        },
                    },
                });
            },
            'Resolve',
        );
    }

    queryBoundaryString(x, y) {
        const classes = this.classes.get(x, y);
        if (!classes) return;

        const len = classes[0].length;

        for (let i = 0; i < len; i++) {
            this.send(
                {
                    recipients: ['Resolver'],
                    action: {
                        QueryBoundaryString: {
                            s: y,
                            t: x + y,
                            idx: i,
                        },
                    },
                },
                false,
            );
        }
    }

    queryTable(x, y) {
        if (y < 0) {
            return;
        }

        this.send(
            {
                recipients: ['Resolver'],
                action: {
                    QueryTable: {
                        s: y,
                        t: x + y,
                    },
                },
            },
            false,
        );
    }

    processResolving(data) {
        this.p = data.p;
        this.maxDegree = data.max_degree;
        this.updateDegrees();
    }

    updateDegrees() {
        this.chart.setAttribute('minx', this.minDegree);
        this.chart.setAttribute('maxx', this.maxDegree);

        this.chart.setAttribute(
            'maxy',
            Math.ceil(
                (this.maxDegree - this.minDegree) * eval(this.vanishingSlope) +
                1 +
                eval(this.vanishingIntercept),
            ),
        ); // We trust our inputs *so* much.
    }

    clearOld(type, x, y, p) {
        if (p === undefined) p = this.chart.contents;

        const classes = Array.from(
            p.getElementsByClassName(`${type}-${x}-${y}`),
        );
        for (const c of classes) {
            c.remove();
        }
    }

    static getPosition(x, dim, i) {
        const offset = i - (dim - 1) / 2;
        return x + offset * 0.3;
    }

    __onClassClick(e) {
        e.stopPropagation();
        const x = parseInt(e.target.parentNode.getAttribute('data-x'));
        const y = parseInt(e.target.parentNode.getAttribute('data-y'));
        this.select([x, y]);
    }

    processSetClass(data) {
        const [x, y] = bidegreeToCoordinates(data.b);

        const oldClasses = this.classes.get(x, y);
        // classes is a list, and each member of the list corresponds to a
        // page. Each page itself is a list of classes.
        this.classes.set(x, y, data.classes);
        this.classState.set(x, y, data.state);
        this.permanentClasses.set(x, y, data.permanents);
        this.classNames.set(x, y, data.class_names);
        this.decompositions.set(x, y, data.decompositions);

        for (const [r, page] of this.chart.pages.entries()) {
            const num = this.getClasses(x, y, r + MIN_PAGE).length;
            const oldNum =
                ExtSseq.getPage(oldClasses, r + MIN_PAGE)?.length || 0;

            let classname = 'class';
            if (data.state === 'Done') {
                classname = 'class-done';
            } else if (data.state === 'Error') {
                classname = 'class-error';
            }

            if (oldNum === num) {
                if (num > 0) {
                    const grp = page.getElementsByClassName(
                        `class-group-${x}-${y}`,
                    )[0];

                    for (const child of grp.children) {
                        child.setAttribute('class', classname);
                    }
                }
                continue;
            }

            this.clearOld('class-group', x, y, page);
            if (num == 0) {
                continue;
            }
            const grp = document.createElementNS(svgNS, 'g');
            grp.classList.add(`class-group`);
            grp.classList.add(`class-group-${x}-${y}`);
            grp.setAttribute('data-x', x);
            grp.setAttribute('data-y', y);
            for (let i = 0; i < num; i++) {
                const node = document.createElementNS(svgNS, 'circle');
                node.setAttribute('cx', ExtSseq.getPosition(x, num, i));
                node.setAttribute('cy', -y);
                node.setAttribute('r', 0.1);
                node.setAttribute('class', classname);

                const title = document.createElementNS(svgNS, 'title');
                title.textContent = `(${x}, ${y})`;
                node.appendChild(title);

                node.onclick = this._onClassClick;
                grp.appendChild(node);
            }
            page.appendChild(grp);
        }
        if (this.hasSelected(x, y)) {
            this.select([x, y]);
        }
    }

    hasSelected(x, y) {
        return (
            this.selected !== null &&
            this.selected[0] == x &&
            this.selected[1] == y
        );
    }

    select(select) {
        this.chart.shadowRoot
            .querySelectorAll(`.selected`)
            .forEach(x => x.classList.remove('selected'));
        const oldSelect = this.selected;
        this.selected = select;
        if (select !== null) {
            this.chart.shadowRoot
                .querySelectorAll(`.class-group-${select[0]}-${select[1]}`)
                .forEach(x => x.classList.add('selected'));
        }
        this.onClick?.(oldSelect);
        this.refreshPanel?.();
    }

    static *drawMatrix(matrix, sourceX, targetX, sourceY, targetY, bend = 0) {
        for (const [sourceIdx, row] of matrix.entries()) {
            for (const [targetIdx, val] of row.entries()) {
                if (val === 0) {
                    continue;
                }
                const x1 = ExtSseq.getPosition(
                    sourceX,
                    matrix.length,
                    sourceIdx,
                );
                const x2 = ExtSseq.getPosition(targetX, row.length, targetIdx);
                if (bend === 0) {
                    const line = document.createElementNS(svgNS, 'line');
                    line.setAttribute('x1', x1);
                    line.setAttribute('x2', x2);
                    line.setAttribute('y1', -sourceY);
                    line.setAttribute('y2', -targetY);
                    yield line;
                } else {
                    const midX = (x1 + x2) / 2;
                    const midY = (sourceY + targetY) / 2;
                    const controlX = midX - ((targetY - sourceY) * bend) / 100;
                    const controlY = midY + ((x2 - x1) * bend) / 100;
                    const path = document.createElementNS(svgNS, 'path');

                    path.style.fill = 'none';
                    path.setAttribute(
                        'd',
                        `M ${x1} ${-sourceY} Q ${controlX} ${-controlY}, ${x2} ${-targetY}`,
                    );
                    yield path;
                }
            }
        }
    }

    processSetDifferential(data) {
        const [x, y] = bidegreeToCoordinates(data.b);

        while (this.chart.pages.length <= data.differentials.length) {
            this.newPage();
        }
        this.trueDifferentials.set(x, y, data.true_differentials);

        this.clearOld('differential', x, y);

        for (const [r, diffs] of data.differentials.entries()) {
            const page = this.chart.pages[r];
            for (const diff of ExtSseq.drawMatrix(
                diffs,
                x,
                x - 1,
                y,
                y + r + MIN_PAGE,
            )) {
                diff.classList.add(`differential`);
                diff.classList.add(`differential-${x}-${y}`);
                diff.classList.add(`d${r + MIN_PAGE}`);
                // Go under classes
                page.insertBefore(diff, page.firstChild);
            }
        }
        if (this.hasSelected(x, y)) {
            this.refreshPanel?.();
        }
    }

    processSetStructline(data) {
        const [x, y] = bidegreeToCoordinates(data.b);

        for (const mult of data.structlines) {
            const [mult_x, mult_y] = bidegreeToCoordinates(mult.mult_b);
            if (!this.products.has(mult.name)) {
                this.products.set(mult.name, {
                    x: mult_x,
                    y: mult_y,
                    matrices: new BiVec(this.minDegree),
                    style: {
                        bend: 0,
                        dash: '',
                        color: 'black',
                        styleObject: null,
                    },
                });
            }
            const product = this.products.get(mult.name);
            const oldMatrices = product.matrices.get(x, y);
            if (JSON.stringify(oldMatrices) === JSON.stringify(mult.matrices)) {
                continue;
            }
            product.matrices.set(x, y, mult.matrices);

            if (this.visibleStructlines.has(mult.name)) {
                for (const [r, page] of this.chart.pages.entries()) {
                    const matrix = ExtSseq.getPage(mult.matrices, r + MIN_PAGE);
                    const oldMatrix = ExtSseq.getPage(
                        oldMatrices,
                        r + MIN_PAGE,
                    );

                    if (JSON.stringify(matrix) === JSON.stringify(oldMatrix)) {
                        continue;
                    }

                    if (oldMatrices !== undefined) {
                        this.clearOld(`structline-${mult.name}`, x, y, page);
                    }

                    for (const line of ExtSseq.drawMatrix(
                        matrix,
                        x,
                        x + mult_x,
                        y,
                        y + mult_y,
                        product.style.bend,
                    )) {
                        line.classList.add(`structline`);
                        line.classList.add(`structline-${mult.name}`);
                        line.classList.add(`structline-${mult.name}-${x}-${y}`);
                        // Go under classes
                        page.insertBefore(line, page.firstChild);
                    }
                }
            }
        }
        if (this.hasSelected(x, y)) {
            this.refreshPanel?.();
        }
    }

    hideStructlines(name) {
        if (!this.visibleStructlines.has(name)) {
            return;
        }
        this.visibleStructlines.delete(name);
        this.chart.shadowRoot
            .querySelectorAll(`.structline-${CSS.escape(name)}`)
            .forEach(x => x.remove());
    }

    showStructlines(name) {
        if (this.visibleStructlines.has(name)) {
            return;
        }
        this.visibleStructlines.add(name);
        const mult = this.products.get(name);
        const matrices = mult.matrices;
        for (const [x_, row] of matrices.data.entries()) {
            const x = x_ + this.minDegree;
            for (const [y, pageMatrices] of row.entries()) {
                if (pageMatrices === undefined) {
                    continue;
                }
                for (const [r, page] of this.chart.pages.entries()) {
                    const pageIdx = Math.min(pageMatrices.length - 1, r);
                    const matrix = pageMatrices[pageIdx];

                    for (const line of ExtSseq.drawMatrix(
                        matrix,
                        x,
                        x + mult.x,
                        y,
                        y + mult.y,
                        mult.style.bend,
                    )) {
                        line.classList.add(`structline`);
                        line.classList.add(`structline-${name}`);
                        line.classList.add(`structline-${name}-${x}-${y}`);
                        // Go under classes
                        page.insertBefore(line, page.firstChild);
                    }
                }
            }
        }
    }

    /**
     * Sort the nodes in the svg in some consistent way. This is useful for
     * integration testing.
     */
    sort() {
        const compare = (a, b) => {
            if (a == b) {
                return 0;
            } else if (a > b) {
                return 1;
            } else {
                return -1;
            }
        };

        for (const page of this.chart.pages) {
            [...page.querySelectorAll('.structline')]
                .sort((a, b) =>
                    compare(a.className.baseVal, b.className.baseVal),
                )
                .forEach(n => page.appendChild(n));

            [...page.querySelectorAll('.differential')]
                .sort((a, b) =>
                    compare(a.className.baseVal, b.className.baseVal),
                )
                .forEach(n => page.appendChild(n));

            [...page.querySelectorAll('.class-group')]
                .sort((a, b) =>
                    compare(a.className.baseVal, b.className.baseVal),
                )
                .forEach(n => page.appendChild(n));
        }
    }

    getDifferentials(x, y, page) {
        return this.differentials.get(x, y)?.[page - MIN_PAGE];
    }

    hasClasses(x, y, page) {
        const classes = this.getClasses(x, y, page);
        return classes !== undefined && classes.length > 0;
    }

    static getPage(v, r) {
        if (v === undefined) {
            return undefined;
        }
        r -= MIN_PAGE;
        if (r >= v.length) r = v.length - 1;
        return v[r];
    }

    /**
     * Get the list of classes on a given page.
     */
    getClasses(x, y, page) {
        return ExtSseq.getPage(this.classes.get(x, y), page);
    }

    highlightClass(x, y) {
        this.chart.shadowRoot
            .querySelectorAll(`.class-group-${x}-${y}`)
            .forEach(x => x.classList.add('highlight'));
    }

    clearHighlight() {
        this.chart.shadowRoot
            .querySelectorAll(`.highlight`)
            .forEach(x => x.classList.remove('highlight'));
    }
}
