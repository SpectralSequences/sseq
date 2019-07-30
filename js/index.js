import "spectral-sequences";

window.sseq = new Sseq();
window.display = new BasicDisplay("#main");
display.setSseq(sseq);

const worker = new Worker("./worker.js");

worker.addEventListener("message", ev => {
    let m = ev.data;
    switch (m.cmd) {
        case "addClass":
            sseq.addClass(m.x, m.y)
            break;
        default:
            break;
    }
});
