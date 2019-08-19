const OFFSET_SIZE = 0.3;

const NODE_COLOR = {
    "InProgress": "black",
    "Error": "#a6001a",
    "Done": "gray"
};

// Prompts for an array of length `length`
function promptClass(text, error, length) {
    while (true) {
        let response = prompt(text);
        if (!response) {
            return null;
        }
        try {
            let vec = JSON.parse(response.trim());
            if (Array.isArray(vec) &&
                vec.length == length &&
                vec.reduce((b, x) => b && Number.isInteger(x), true)) {
                return vec;
            }
        } catch(e) { // If we can't parse, try again
        }
        alert(error);
    }
}

function promptInteger(text, error) {
    while (true) {
        let response = prompt(text);
        if (!response) {
            return null;
        }
        let c = parseInt(response.trim());
        if (!isNaN(c)) {
            return c;
            break;
        }
        alert(error);
    }
}

export class ExtSseq extends EventEmitter {
    constructor(name, webSocket) {
        super();

        this.name = name;
        this.webSocket = webSocket;

        this.maxDegree = 0;
        this.initial_page_idx = 0;
        this.min_page_idx = 0;

        this.classes = new StringifyingMap();
        this.structlines = new StringifyingMap();
        this.products = new StringifyingMap();
        this.structlineTypes = new Set();
        this.permanentClasses = new StringifyingMap();
        this.differentials = new StringifyingMap();
        this.trueDifferentials = new StringifyingMap();

        this.differentialColors = [undefined, undefined, "cyan", "red", "green"];
        this.page_list = [2];

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

    send(data) {
        data.sseq = this.name;
        this.webSocket.send(JSON.stringify(data));
    }

    undo() {
        this.send({
            recipients: ["Sseq"],
            action : { Undo : {} }
        });
    }

    redo() {
        this.send({
            recipients: ["Sseq"],
            action : { Redo : {} }
        });
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
        let len = this.classes.get([x, y])[2].length;
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

    addPermanentClassInteractive(node) {
        let classes = this.classes.get([node.x, node.y]);

        let last = classes[classes.length - 1];
        let target;
        if (last.length == 0) {
            alert("There are no surviving classes. Action ignored");
        } else if (last.length == 1) {
            this.addPermanentClass(node.x, node.y, last[0].data);
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
        });
        window.setUnitRange();
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
        });
    }

    processResolving(data) {
        this.p = data.p;
        this.minDegree = data.min_degree;
        this.maxDegree = data.max_degree;
        this.xRange = [this.minDegree, this.maxDegree];
        this.yRange = [0, Math.ceil((this.maxDegree - this.minDegree)/2) + 1];
        this.initialxRange = [this.minDegree, this.maxDegree];
        this.initialyRange = [0, Math.ceil((this.maxDegree - this.minDegree)/2) + 1];
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
        classes.splice(0, 0, undefined, undefined);
        this.classes.set([x, y], classes);
        this.permanentClasses.set([x, y], data.permanents);

        this.emit("update", x, y);
    }

    processSetDifferential(data) {
        let x = data.x;
        let y = data.y;

        let differentials = [];
        for (let [page, matrix] of data.differentials.entries()) {
            page = page + 2;
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
        data.true_differentials.splice(0, 0, undefined, undefined);
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
                this.emit("new-structline");
            }

            for (let [page, matrix] of mult["matrices"].entries()) {
                page = page + 2;
                let name = mult["name"];
                let multX = mult["mult_x"];
                let multY = mult["mult_y"];

                for (let i = 0; i < matrix.length; i++) {
                    for (let j = 0; j < matrix[i].length; j++) {
                        if (matrix[i][j] != 0) {
                            let line = new Structline(this, [x, y, i], [x + multX, y + multY, j]);
                            line.setProduct(name);
                            if (!structlines[page])
                                structlines[page] = [];
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
        if (result.length == 2) return undefined;

        if (page >= result.length) page = result.length - 1;
        return result[page];
    }

    getStructlines(x, y, page) {
        let result = this.structlines.get([x, y]);
        if (!result) return undefined;
        if (result.length == 2) return undefined;

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
