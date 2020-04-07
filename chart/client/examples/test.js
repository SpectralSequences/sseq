let sseq = new SpectralSequenceChart();
sseq.page_list = [2, 3];
sseq.initialxRange = [0, 10];
sseq.initialyRange = [0, 10];
sseq.classes = [];
sseq.edges = [];
sseq.min_page_idx = 0;
sseq.node_list = [new ChartNode({"shape" : "square"})];
sseq.add_class({
    "x" : 0,  "y" : 0, 
    "transition_pages" : [2], 
    "node_list" : [0, null]
});
sseq.add_class({
    "x" : 1,  "y" : 1, 
    "transition_pages" : [], 
    "node_list" : [0]
});
sseq.add_structline({
    "source" : 0,
    "target" : 1
})
let display = new BasicDisplay("#main", sseq);