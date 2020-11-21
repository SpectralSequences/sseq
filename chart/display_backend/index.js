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
    app._canvas.set_current_xrange(-1, 4);
    app._canvas.set_current_yrange(-1, 6);
    app._canvas.set_max_xrange(-10, 10);
    app._canvas.set_max_yrange(-10, 10);

    let shape1 = pkg.GlyphBuilder.from_stix("\u{2612}", 50, true).build(0.1, 1);
    let shape2;
    {
        let builder = pkg.GlyphBuilder.from_stix("\u{2644}", 30, false);
        builder.boxed(3, true);
        shape2 = builder.build(0.1, 2);
    }

    let shape3;
    {
        let builder = pkg.GlyphBuilder.from_stix("\u{E0F2}", 30, false);
        builder.circled(3, 2, 3, true);
        shape3 = builder.build(0.1, 1);
    }

    let shape4 = pkg.GlyphBuilder.from_stix("\u{29E8}", 30, true).build(0.1, 1);
    let shape5 = pkg.GlyphBuilder.from_stix("\u{2B51}", 60, true).build(0.1, 1);
    
    let normal_arrow = pkg.Arrow.normal_arrow(2,true, true, false, false);
    let hook_arrow = pkg.Arrow.hook_arrow(2, 180, true, true, true, false);

    let options = pkg.EdgeOptions.new();
    options.set_thickness(2);

    let gi1 = app._canvas.add_glyph(new Vec2(0,0), new Vec2(0,0), shape1, 1, new Vec4(0,0,0,1), new Vec4(0.5,0.2, 0, 1), new Vec4(0,0,0,0));
    let gi2 = app._canvas.add_glyph(new Vec2(1,1), new Vec2(0,0), shape2, 1, new Vec4(0,0,0.4,1), new Vec4(0, 0.5, 0.5, 1), new Vec4(1,1,1,1));
    let gi3 = app._canvas.add_glyph(new Vec2(2,3), new Vec2(0,0), shape4, 1, new Vec4(0,0,0,1), new Vec4(0.3,0.7, 0.1, 1), new Vec4(0,0,0,0));
    let gi4 = app._canvas.add_glyph(new Vec2(2,0), new Vec2(0,0), shape3, 1, new Vec4(0,0, 1,1), new Vec4(0, 0.8, 0.3, 1), new Vec4(1.0, 0.8, 1.0,1));
    let gi5 = app._canvas.add_glyph(new Vec2(2,5), new Vec2(0,0), shape5, 1, new Vec4(0,0, 0,1), new Vec4(0, 0, 0, 0), new Vec4(0, 0, 0,0));
    
    // options.set_bend_degrees(bend);
    // options.set_dash_pattern(new Uint8Array(dash_pattern));
    // options.set_color(new Vec4(...color));
    options.set_end_tip(normal_arrow);
    options.set_start_tip(hook_arrow);
    app._canvas.add_edge(gi1, gi2, options);
    options.no_tips();
    app._canvas.add_edge(gi2, gi3, options);
    
    options.set_bend_degrees(20);
    options.set_dash_pattern([5]);
    app._canvas.add_edge(gi4, gi5, options);

}

main().catch(console.error);