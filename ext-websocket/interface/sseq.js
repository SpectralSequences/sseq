import { promptClass, promptInteger, vecToName } from "./utils.js";

export const MIN_PAGE = 2;
const OFFSET_SIZE = 0.3;

const NODE_COLOR = {
    "InProgress": "black",
    "Error": "#a6001a",
    "Done": "gray"
};

const KEEP_LOG = new Set(["AddDifferential", "AddProductType", "AddProductDifferential", "AddPermanentClass", "SetClassName"]);

export class ExtSseq extends EventEmitter {
    constructor(name, webSocket) {
        super();

        this.history = [];
        this.redoStack = [];
        this.name = name;
        this.webSocket = webSocket;

        this.minDegree = 0;
        this.maxDegree = 0;
        this.initial_page_idx = 0;
        this.min_page_idx = 0;

        this._vanishingSlope = "1/2";
        this._vanishingIntercept = 1;

        this.classes = new StringifyingMap();
        this.structlines = new StringifyingMap();
        this.products = new StringifyingMap();
        this.structlineTypes = new Set();
        this.permanentClasses = new StringifyingMap();
        this.classNames = new StringifyingMap();
        this.decompositions = new StringifyingMap();
        this.differentials = new StringifyingMap();
        this.trueDifferentials = new StringifyingMap();

        this.differentialColors = [undefined, undefined, "cyan", "red", "green"];
        this.page_list = [MIN_PAGE];

        this.class_scale = 1;
        this.min_class_size = 20;
        this.max_class_size = 60;

        // The largest x/y of the products we have. This is useful for figuring which structlines to draw.
        this.maxMultX = 0;
        this.maxMultY = 0;
        this.maxDiffPage = 0;

        this.defaultNode = new Node();
        this.defaultNode.hcolor = "red";
        this.defaultNode.fill = true;
        this.defaultNode.stroke = true;
        this.defaultNode.shape = Shapes.circle;
    }

    get vanishingSlope() {
        return this._vanishingSlope;
    }

    get vanishingIntercept() {
        return this._vanishingIntercept;
    }

    set vanishingSlope(m) {
        this._vanishingSlope = m;
        this.emit("update");
    }

    set vanishingIntercept(c) {
        this._vanishingIntercept = c;
        this.emit("update");
    }

    send(data, log=true) {
        if (KEEP_LOG.has(Object.keys(data.action)[0])) {
            this.emit("new-history", data);
            if (log) {
                this.history.push(data);
            }
        }

        data.sseq = this.name;
        this.webSocket.send(JSON.stringify(data));
    }

    removeHistoryItem(msg) {
        msg = JSON.stringify(msg);
        if (confirm(`Are you sure you want to remove ${msg}?`)) {
            this.history = this.history.filter(m => JSON.stringify(m) != msg);

            this.send({
                recipients: ["Sseq"],
                action : { Clear : {} }
            });
            this.emit("clear-history");

            for (let msg of this.history) {
                this.send(msg, false);
            }
        }
    }
    undo() {
        this.redoStack.push(this.history.pop());

        this.send({
            recipients: ["Sseq"],
            refresh : false,
            action : { Clear : {} }
        });
        this.emit("clear-history");

        for (let msg of this.history) {
            msg.refresh = false;
            this.send(msg, false);
            delete msg.refresh;
        }
        this.send({
            recipients: ["Sseq"],
            refresh : true,
            action : { RefreshAll : {} }
        });
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
        let len = this.classes.get([x, y])[MIN_PAGE].length;
        let pageBasis = this.getClasses(x, y, r);

        let result = [];
        for (let i = 0; i < len; i ++) {
            result.push(0);
        }
        for (let i = 0; i < pageBasis.length; i ++) {
            let coef = c[i];
            for (let j = 0; j < len; j++) {
                result[j] += coef * pageBasis[i].data[j];
            }
        }
        for (let i = 0; i < len; i ++) {
            result[i] = result[i] % this.p;
        }
        return result;
    }

    addDifferentialInteractive(source, target) {
        let page = target.y - source.y;
        let source_dim = this.getClasses(source.x, source.y, page).length;
        let target_dim = this.getClasses(target.x, target.y, page).length;

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

        source_vec = this.pageBasisToE2Basis(page, source.x, source.y, source_vec);
        target_vec = this.pageBasisToE2Basis(page, source.x - 1, source.y + page, target_vec);

        this.addDifferential(page, source.x, source.y, source_vec, target_vec);
    }

    setClassName(x, y, idx, name) {
        this.send({
            "recipients": ["Sseq"],
            action: { "SetClassName": { x : x, y : y, idx : idx, name : name } }
        });
    }

    addProductInteractive(x, y, num) {
        let c;
        if (num == 1 && this.p == 2)
            c = [1];
        else
            c = promptClass("Input class",`Invalid class. Express in terms of basis on E_2 page`, num);

        let name = prompt("Name for product", this.isUnit ? vecToName(c, this.classNames.get([x, y])) : undefined);
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
                        name: prompt("Name of source", this.isUnit ? vecToName(sourceClass, this.classNames.get([sourceX, sourceY])) : undefined).trim()
                    },
                    target : {
                        permanent : false,
                        x: sourceX - 1,
                        y: sourceY + page,
                        "class": targetClass,
                        name: prompt("Name of target", this.isUnit ? vecToName(targetClass, this.classNames.get([sourceX - 1, sourceY + page])) : undefined).trim()
                    }
                }
            }
        });
    }

    addPermanentClassInteractive(node) {
        let classes = this.classes.get([node.x, node.y]);

        let last = classes[classes.length - 1];
        let target;
        if (last.length == 0) {
            alert("There are no surviving classes. Action ignored");
        } else if (classes[MIN_PAGE].length == 1) {
            this.addPermanentClass(node.x, node.y, classes[MIN_PAGE][0].data);
        } else {
            target = promptClass("Input new permanent class", "Invalid class. Express in terms of basis on last page", last.length);
        }
        if (target) {
            this.addPermanentClass(node.x, node.y, target);
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
            newmax = parseInt(prompt("New maximum degree", this.maxDegree + 10).trim());
        }
        if (newmax <= this.maxDegree) {
            return;
        }
        this.maxDegree = newmax;
        this.send({
            recipients: ["Resolver"],
            action: {
                "Resolve": {
                    max_degree: newmax
                }
            }
        }, false);
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

    get xRange() {
        return [this.minDegree, this.maxDegree];
    }

    get yRange() {
        // Because of the slope -1 ridge at the end of, the y-to-x ratio is smaller.
        let realSlope = 1/(1/eval(this._vanishingSlope) + 1);

        let maxY = Math.ceil((this.maxDegree - this.minDegree) * realSlope + 1 + eval(this._vanishingIntercept)); // We trust our inputs *so* much.
        return [0, maxY];
    }

    get initialxRange() { return this.xRange; }
    get initialyRange() { return this.yRange; }

    processResolving(data) {
        this.p = data.p;
        this.minDegree = data.min_degree;
        this.maxDegree = data.max_degree;
    }

    processSetPageList(data) {
        this.page_list = data.page_list;
    }

    processSetClass(data) {
        let x = data.x;
        let y = data.y;
        let classes = data.classes;

        // classes is a list, and each member of the list corresponds to a
        // page. Each page itself is a list of classes. We turn the raw class
        // data into nodes.

        classes.forEach(l => {
            for (let i of l.keys()) {
                let node = new Node(this.defaultNode);
                node.x = data.x;
                node.y = data.y;
                node.idx = i;
                node.total_classes = l.length;
                node.data = l[i];
                node.state = data.state;
                node.color = NODE_COLOR[node.state];
                l[i] = node;
            }
        });
        // Insert empty space at r = 0, 1
        for (let i = 0; i < MIN_PAGE; i++) {
            classes.splice(0, 0, undefined);
        }
        this.classes.set([x, y], classes);
        this.permanentClasses.set([x, y], data.permanents);
        this.classNames.set([x, y], data.class_names);
        this.decompositions.set([x, y], data.decompositions);

        this.emit("update", x, y);
    }

    processSetDifferential(data) {
        let x = data.x;
        let y = data.y;

        let differentials = [];
        for (let [page, matrix] of data.differentials.entries()) {
            page = page + MIN_PAGE;
            this.maxDiffPage = Math.max(this.maxDiffPage, page);

            for (let i = 0; i < matrix.length; i++) {
                for (let j = 0; j < matrix[i].length; j++) {
                    if (matrix[i][j] != 0) {
                        let line = new Differential(this, [x, y, i], [x - 1, y + page, j], page);
                        if (this.differentialColors[page]) {
                            line.color = this.differentialColors[page];
                        }

                        if (!differentials[page])
                            differentials[page] = [];
                        differentials[page].push(line);
                    }
                }
            }
        }

        this.differentials.set([x, y], differentials);
        for (let i = 0; i < MIN_PAGE; i++) {
            data.true_differentials.splice(0, 0, undefined);
        }
        this.trueDifferentials.set([x, y], data.true_differentials);
        this.emit("update", x, y);
    }

    processSetStructline(data) {
        let x = data.x;
        let y = data.y;

        let structlines = [];
        let products = [];
        for (let mult of data.structlines) {
            if (!this.structlineTypes.has(mult["name"])) {
                this.structlineTypes.add(mult["name"]);
                this.emit("new-structline", mult["name"]);
            }

            for (let [page, matrix] of mult["matrices"].entries()) {
                page = page + MIN_PAGE;
                if (!structlines[page])
                    structlines[page] = [];
                let name = mult["name"];
                let multX = mult["mult_x"];
                let multY = mult["mult_y"];

                for (let i = 0; i < matrix.length; i++) {
                    for (let j = 0; j < matrix[i].length; j++) {
                        if (matrix[i][j] != 0) {
                            let line = new Structline(this, [x, y, i], [x + multX, y + multY, j]);
                            line.setProduct(name);
                            structlines[page].push(line);
                        }
                    }
                }
                if (!products[page])
                    products[page] = [];
                products[page].push({
                    name : name,
                    x : multX,
                    y : multY,
                    matrix : matrix
                });
                this.maxMultX = Math.max(this.maxMultX, multX);
                this.maxMultY = Math.max(this.maxMultY, multY);
            }
        }

        this.structlines.set([x, y], structlines);
        this.products.set([x, y], products);
        this.emit("update", x, y);
    }

    getDrawnElements(page, xmin, xmax, ymin, ymax) {
        // We are bad and can't handle page ranges.
        if (Array.isArray(page)) {
            page = page[0];
        }

        let displayClasses = [];
        for (let x = xmin; x <= xmax; x++) {
            for (let y = ymin; y <= ymax; y++) {
                let result = this.classes.get([x, y]);
                if (!result) continue;

                if (page >= result.length)
                    result = result[result.length - 1];
                else
                    result = result[page];

                for (let node of result) {
                    displayClasses.push(node);
                }
            }
        }

        let displayEdges = [];

        let xbuffer = Math.max(this.maxMultX, 1);
        let ybuffer = Math.max(this.maxMultY, this.maxDiffPage);
        for (let x = xmin - xbuffer; x <= xmax + xbuffer; x++) {
            for (let y = ymin - ybuffer; y <= ymax + ybuffer; y++) {
                let edges = this.getEdges(x, y, page);
                for (let edge of edges) {
                    edge.source_node = this.getClasses(x, y, page)[edge.source[2]];
                    edge.target_node = this.getClasses(edge.target[0], edge.target[1], page)[edge.target[2]];

                    if (edge.source_node && !displayClasses.includes(edge.source_node)) {
                        displayClasses.push(edge.source_node);
                    }
                    if (edge.target_node && !displayClasses.includes(edge.target_node)) {
                        displayClasses.push(edge.target_node);
                    }
                    displayEdges.push(edge);
                }
            }
        }

        return [displayClasses, displayEdges];
    }

    getEdges(x, y, page) {
        let differentials = this.getDifferentials(x, y, page);
        let structlines = this.getStructlines(x, y, page);

        if (!differentials) {
            differentials = [];
        }
        if (!structlines) {
            structlines = [];
        }
        return differentials.concat(structlines);
    }

    getDifferentials(x, y, page) {
        let result = this.differentials.get([x, y]);
        if (!result) return undefined;
        return result[page];
    }

    getProducts(x, y, page) {
        let result = this.products.get([x, y]);
        if (!result) return undefined;
        if (result.length == MIN_PAGE) return undefined;

        if (page >= result.length) page = result.length - 1;
        return result[page];
    }

    getStructlines(x, y, page) {
        let result = this.structlines.get([x, y]);
        if (!result) return undefined;
        if (result.length == MIN_PAGE) return undefined;

        if (page >= result.length) page = result.length - 1;
        return result[page];
    }

    getClasses(x, y, page) {
        let result = this.classes.get([x, y]);
        if (!result) return undefined;

        if (page >= result.length) page = result.length - 1;

        return result[page];
    }

    _getXOffset(node, page) {
        return (node.idx - (node.total_classes - 1)/2) * OFFSET_SIZE;
    }

    _getYOffset(node, page) {
        return 0;
    }
}
