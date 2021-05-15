'use strict';

import { download, promptClass, promptInteger, vecToName } from "./utils.js";

export const MIN_PAGE = 2;

const KEEP_LOG = new Set(["AddDifferential", "AddProductType", "AddProductDifferential", "AddPermanentClass", "SetClassName"]);

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
        if (x < this.minDegree || y < 0 || this.data.length <= x - this.minDegree) {
            return undefined;
        } else {
            return this.data[x - this.minDegree][y];
        }
    }
}

export class ExtSseq extends EventEmitter {
    constructor(name, minDegree) {
        super();

        this.minDegree = minDegree;
        this.maxDegree = minDegree;

        this.history = [];
        this.redoStack = [];
        this.name = name;

        this.vanishingSlope = "1/2";
        this.vanishingIntercept = 1;

        this.classes = new BiVec(minDegree);
        this.classState = new BiVec(minDegree);
        this.permanentClasses = new BiVec(minDegree);
        this.classNames = new BiVec(minDegree);
        this.decompositions = new BiVec(minDegree);
        this.products = new Map();
        this.structlineTypes = new Set();
        this.differentials = new BiVec(minDegree);
        this.trueDifferentials = new BiVec(minDegree);

        this.pageList = [MIN_PAGE];

        // The largest x/y of the products we have. This is useful for figuring which structlines to draw.
        this.maxMultX = 0;
        this.maxMultY = 0;
        this.maxDiffPage = 0;
    }

    get maxX() {
        return this.maxDegree;
    }

    send(data, log=true) {
        if (KEEP_LOG.has(Object.keys(data.action)[0])) {
            this.emit("new-history", data);
            if (log) {
                this.history.push(data);
            }
        }

        data.sseq = this.name;
        window.send(data);
    }

    removeHistoryItem(msg) {
        msg = JSON.stringify(msg);
        if (confirm(`Are you sure you want to remove ${msg}?`)) {
            this.history = this.history.filter(m => JSON.stringify(m) != msg);

            this.block();
            this.send({
                recipients: ["Sseq"],
                action : { Clear : {} }
            });
            this.emit("clear-history");

            for (let msg of this.history) {
                this.send(msg, false);
            }

            this.block(false);
        }
    }

    block(block = true) {
        this.send({
            recipients: ["Sseq"],
            action : { BlockRefresh : { block : block } }
        });
    }

    undo() {
        this.redoStack.push(this.history.pop());

        this.block();
        this.send({
            recipients: ["Sseq"],
            action : { Clear : {} }
        });
        this.emit("clear-history");

        for (let msg of this.history) {
            this.send(msg, false);
        }
        this.block(false);
    }

    redo() {
        this.send(this.redoStack.pop());
    }

    addPermanentClass(x, y, target) {
        this.send({
            recipients: ["Sseq"],
            action: {
                "AddPermanentClass" : {
                    x: x,
                    y: y,
                    class: target,
                }
            }
        });
    }

    pageBasisToE2Basis(r, x, y, c) {
        let len = this.classes.get(x, y)[0].length;
        let pageBasis = this.getClasses(x, y, r);

        let result = [];
        for (let i = 0; i < len; i ++) {
            result.push(0);
        }
        for (let i = 0; i < pageBasis.length; i ++) {
            let coef = c[i];
            for (let j = 0; j < len; j++) {
                result[j] += coef * pageBasis[i][j];
            }
        }
        for (let i = 0; i < len; i ++) {
            result[i] = result[i] % this.p;
        }
        return result;
    }

    addDifferentialInteractive(source, target) {
        let page = target[1] - source[1];
        let source_dim = this.getClasses(source[0], source[1], page).length;
        let target_dim = this.getClasses(target[0], target[1], page).length;

        let source_vec = [];
        let target_vec = [];
        if (source_dim == 1) {
            source_vec = [1];
        } else {
            source_vec = promptClass("Input source", `Invalid source. Express in terms of basis on page ${page}`, source_dim);
            if (source_vec === null) {
                return;
            }
        }
        if (source_dim == 1 && target_dim == 1) {
            if (this.p == 2) {
                target_vec = [1];
            } else {
                let c = promptInteger("Coefficient of differential", `Invalid coefficient. Write down a single number.`);
                if (c === null) {
                    return;
                }
                target_vec = [c];
            }
        } else {
            target_vec = promptClass("Input target",`Invalid target. Express in terms of basis on page ${page}`, target_dim);
            if (target_vec === null) {
                return;
            }
        }

        source_vec = this.pageBasisToE2Basis(page, source[0], source[1], source_vec);
        target_vec = this.pageBasisToE2Basis(page, source[0] - 1, source[1] + page, target_vec);

        this.addDifferential(page, source[0], source[1], source_vec, target_vec);
    }

    setClassName(x, y, idx, name) {
        this.send({
            "recipients": ["Sseq"],
            action: { "SetClassName": { x : x, y : y, idx : idx, name : name } }
        });
    }

    // addProductInteractive takes in the number of classes in bidegree (x, y), because this should be the number of classes in the *unit* spectral sequence, not the main spectral sequence
    addProductInteractive(x, y, num) {
        let c;
        if (num == 1 && this.p == 2)
            c = [1];
        else
            c = promptClass("Input class",`Invalid class. Express in terms of basis on E_2 page`, num);

        let name = prompt("Name for product", this.isUnit ? vecToName(c, this.classNames.get(x, y)) : undefined);
        if (name === null) {
            return;
        }

        let permanent = confirm("Permanent class?");
        this.send({
            recipients : ["Sseq", "Resolver"],
            action : {
                "AddProductType": {
                    permanent : permanent,
                    x: x,
                    y: y,
                    "class": c,
                    name: name
                }
            }
        });
    }

    addProductDifferentialInteractive(sourceX, sourceY, page, sourceClass, targetClass) {
        if (!sourceClass) {
            let num = this.getClasses(sourceX, sourceY, MIN_PAGE).length;
            if (num == 1 && this.p == 2) {
                sourceClass = [1];
            } else {
                sourceClass = promptClass("Enter source class", "Invalid class. Express in terms of basis on E2", num);
            }
        }
        if (!targetClass) {
            let num = this.getClasses(sourceX - 1, sourceY + page, MIN_PAGE).length;
            if (num == 1 && this.p == 2) {
                targetClass = [1];
            } else {
                targetClass = promptClass("Enter target class", "Invalid class. Express in terms of basis on E2", num);
            }
        }

        if (!(sourceClass && targetClass)) {
            return;
        }
        window.mainSseq.send({
            recipients : ["Sseq", "Resolver"],
            action : {
                "AddProductDifferential": {
                    source : {
                        permanent : false,
                        x: sourceX,
                        y: sourceY,
                        "class": sourceClass,
                        name: prompt("Name of source", this.isUnit ? vecToName(sourceClass, this.classNames.get(sourceX, sourceY)) : undefined).trim()
                    },
                    target : {
                        permanent : false,
                        x: sourceX - 1,
                        y: sourceY + page,
                        "class": targetClass,
                        name: prompt("Name of target", this.isUnit ? vecToName(targetClass, this.classNames.get(sourceX - 1, sourceY + page)) : undefined).trim()
                    }
                }
            }
        });
    }

    addPermanentClassInteractive(x, y) {
        let classes = this.classes.get(x, y);

        let last = classes[classes.length - 1];
        let target;
        if (last.length == 0) {
            alert("There are no surviving classes. Action ignored");
        } else if (classes[0].length == 1) {
            this.addPermanentClass(x, y, classes[0][0]);
        } else {
            target = promptClass("Input new permanent class", "Invalid class. Express in terms of basis on E_2 page", classes[0].length);
        }
        if (target) {
            this.addPermanentClass(x, y, target);
        }
    }

    addDifferential(r, source_x, source_y, source, target) {
        this.send({
            recipients: ["Sseq"],
            action: {
                "AddDifferential" : {
                    r: r,
                    x: source_x,
                    y: source_y,
                    source: source,
                    target: target
                }
            }
        });
    }

    resolveFurther(newmax) {
        // This is usually an event callback and the argument could be any random thing.
        if (!Number.isInteger(newmax)) {
            newmax = prompt("New maximum degree", this.maxDegree + 10);
            if (newmax === null) return;
            newmax = parseInt(newmax.trim());
        }

        if (newmax <= this.maxDegree) {
            return;
        }
        this.maxDegree = newmax;

        this.block();
        this.send({
            recipients: ["Resolver"],
            action: {
                "Resolve": {
                    max_degree: newmax
                }
            }
        });
        this.block(false)
    }

    queryCocycleString(x, y) {
        let classes = this.classes.get(x, y);
        if (!classes) return;

        let len = classes[0].length;

        for (let i = 0; i < len; i++) {
            this.send({
                recipients: ["Resolver"],
                action: {
                    "QueryCocycleString" : {
                        s: y,
                        t: x + y,
                        idx: i
                    }
                }
            }, false);
        }
    }

    queryTable(x, y) {
        if (y < 0) { return; }

        this.send({
            recipients: ["Resolver"],
            action: {
                "QueryTable" : {
                    s: y,
                    t: x + y
                }
            }
        }, false);
    }

    get maxY() {
        // Because of the slope -1 ridge at the end of, the y-to-x ratio is smaller.
        let realSlope = 1/(1/eval(this.vanishingSlope) + 1);

        return Math.ceil((this.maxDegree - this.minDegree) * realSlope + 1 + eval(this.vanishingIntercept)); // We trust our inputs *so* much.
    }

    processResolving(data) {
        this.p = data.p;
        this.maxDegree = data.max_degree;
    }

    processSetClass(data) {
        let x = data.x;
        let y = data.y;
        let classes = data.classes;

        // classes is a list, and each member of the list corresponds to a
        // page. Each page itself is a list of classes.
        this.classes.set(x, y, classes);
        this.classState.set(x, y, data.state);
        this.permanentClasses.set(x, y, data.permanents);
        this.classNames.set(x, y, data.class_names);
        this.decompositions.set(x, y, data.decompositions);

        this.emit("update", x, y);
    }

    processSetDifferential(data) {
        let x = data.x;
        let y = data.y;

        while (this.pageList.length <= data.differentials.length) {
            this.pageList.push(this.pageList.length + 2);
        }
        this.differentials.set(x, y, data.differentials);
        this.trueDifferentials.set(x, y, data.true_differentials);
        this.emit("update", x, y);
    }

    processSetStructline(data) {
        let x = data.x;
        let y = data.y;

        for (let mult of data.structlines) {
            if (!this.products.has(mult.name)) {
                this.products.set(mult.name, {
                    "x": mult.mult_x,
                    "y": mult.mult_y,
                    matrices : new BiVec(this.minDegree)
                });
                this.emit("new-structline", mult.name);
            }
            let matrices = this.products.get(mult.name).matrices;
            matrices.set(x, y, mult.matrices);
        }
        this.emit("update", x, y);
    }

    getDifferentials(x, y, page) {
        let result = this.differentials.get(x, y);
        if (!result) return undefined;
        return result[page - MIN_PAGE];
    }

    hasClasses(x, y, page) {
        let classes = this.getClasses(x, y, page);
        return classes !== undefined && classes.length > 0;
    }

    getClasses(x, y, page) {
        page -= MIN_PAGE;
        let result = this.classes.get(x, y);
        if (!result) return undefined;

        if (page >= result.length) page = result.length - 1;

        return result[page];
    }
}
