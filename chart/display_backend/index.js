import { App } from "./app.js";

function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}


async function main(){
    window.pkg = await import("./pkg");
    window.Vec2 = pkg.JsPoint;
    window.Vec4 = pkg.Vec4;

    window.App = App;
    window.app = new App(pkg, "div");
}

main().catch(console.error);