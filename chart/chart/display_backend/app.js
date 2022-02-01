function getTouchesInfo(touchEvent) {
    let { touches } = touchEvent;
    let touchCount = touches.length;
    let centerX = 0;
    let centerY = 0;
    let averageDistance = 0;
    for (let touch of touches) {
        let { screenX, screenY } = touch;
        centerX += screenX;
        centerY += screenY;
    }
    centerX /= touchCount;
    centerY /= touchCount;
    for (let touch of touches) {
        let { screenX, screenY } = touch;
        let dx = screenX - centerX;
        let dy = screenY - centerY;
        averageDistance += Math.sqrt(dx * dx + dy * dy);
    }
    averageDistance /= touchCount;
    return { centerX, centerY, averageDistance, touchCount };
}

function getTime() {
    return new Date().getTime();
}

const WEBGL_OPTIONS = {
    stencil: true,
    alpha: true,
    preserveDrawingBuffer: true,
    antialias: true,
};

export class App {
    constructor(pkg, wrapperSelector, font) {
        this._oldTouches = [];
        this._previousMouseX = 0;
        this._previousMouseY = 0;
        this.wrapperElement = document.querySelector(wrapperSelector);
        let canvasElement = this.wrapperElement.querySelector('canvas');
        let canvasContext = canvasElement.getContext('webgl2', WEBGL_OPTIONS);
        this.pkg = pkg;
        this._canvas = new pkg.Canvas(canvasContext);
        canvasElement.addEventListener(
            'mousedown',
            this.handleMouseDown.bind(this),
        );
        canvasElement.addEventListener(
            'mouseup',
            this.handleMouseUp.bind(this),
        );
        canvasElement.addEventListener(
            'mousemove',
            this.handleMouseMove.bind(this),
        );
        canvasElement.addEventListener(
            'touchstart',
            this.handleTouchStart.bind(this),
        );
        canvasElement.addEventListener(
            'touchmove',
            this.handleTouchMove.bind(this),
        );
        canvasElement.addEventListener(
            'touchend',
            this.handleTouchEnd.bind(this),
        );

        canvasElement.addEventListener('webglcontextlost', event => {
            event.preventDefault();
            console.log('context lost!');
            cancelAnimationFrame(this.requestAnimationFrameId);
        });
        canvasElement.addEventListener('webglcontextrestored', event => {
            console.log('context retored!');
            let canvasContext = canvasElement.getContext(
                'webgl2',
                WEBGL_OPTIONS,
            );
            this._canvas.restore_context(canvasContext);
            this._requestRedraw();
            this._requestFrame();
        });

        canvasElement.addEventListener('wheel', this.handleScroll.bind(this));
        this._needsRedraw = true;
        this._idleFrames = 0;
        this.font = font;
        this._resizeObserver = new ResizeObserver(_entries => {
            this._resize();
        });
        this._resizeObserver.observe(this.wrapperElement);

        this._canvas.resize(
            this.wrapperElement.offsetWidth,
            this.wrapperElement.offsetHeight,
            window.devicePixelRatio,
        );
        this._canvas.set_current_xrange(-10, 10);
        this._canvas.set_current_yrange(-10, 10);
        this._requestRedraw();
        this._requestFrame();
    }

    _resize() {
        this._canvas.resize(
            this.wrapperElement.offsetWidth,
            this.wrapperElement.offsetHeight,
            window.devicePixelRatio,
        );
        this._requestRedraw();
        this._requestFrame();
    }

    _requestFrame() {
        this.requestAnimationFrameId = requestAnimationFrame(() =>
            this.handleFrame(),
        );
    }

    _requestRedraw() {
        this._needsRedraw = true;
    }

    _stopAnimation() {}

    handleScroll(event) {
        event.preventDefault();
        this._stopAnimation();
        let mousePoint = new Vec2(event.offsetX, event.offsetY);
        // If we are close to a grid point (currently within 10px) lock on to it.
        let [nearestX, nearestY, distance] =
            this._canvas.nearest_gridpoint(mousePoint);
        if (distance < 10) {
            this._canvas.translate(
                new Vec2(-nearestX + mousePoint.x, -nearestY + mousePoint.y),
            );
        }
        let direction = Math.sign(event.deltaY);
        this._canvas.scale_around(Math.pow(0.6, direction), mousePoint);
        this._requestRedraw();
    }

    handlePinch(x, y, delta) {
        this._stopAnimation();
        this._canvas.scale_around(Math.pow(0.98, delta), new Vec2(x, y));
        this._requestRedraw();
    }

    handleResize() {
        // _canvas.translateOrigin((_platform.width - _oldPlatformWidth) / 2, (_platform.height - _oldPlatformHeight) / 2)
        // _oldPlatformWidth = _platform.width
        // _oldPlatformHeight = _platform.height
        // this._stopAnimation();
        // this._draw();
    }

    handleTouchStart(event) {
        event.preventDefault();
        let { centerX, centerY, averageDistance, touchCount } =
            getTouchesInfo(event);
        let time = getTime();
        this._stopAnimation();
        this._oldTouches.push({
            centerX,
            centerY,
            averageDistance,
            touchCount,
            time,
        });
    }

    handleTouchMove(event) {
        event.preventDefault();
        let { centerX, centerY, averageDistance, touchCount } =
            getTouchesInfo(event);
        let previous = this._oldTouches[this._oldTouches.length - 1];
        if (previous.touchCount === touchCount) {
            if (averageDistance !== 0 && previous.averageDistance !== 0) {
                this._canvas.scale_around(
                    averageDistance / previous.averageDistance,
                    new Vec2(previous.centerX, previous.centerY),
                );
            }
            this._canvas.translate(
                new Vec2(
                    centerX - previous.centerX,
                    centerY - previous.centerY,
                ),
            );
            this._requestRedraw();
        }
        let time = getTime();
        this._oldTouches.push({
            centerX,
            centerY,
            averageDistance,
            touchCount,
            time,
        });
    }

    handleTouchEnd(event) {
        event.preventDefault();
        let { centerX, centerY, averageDistance, touchCount } =
            getTouchesInfo(event);
        let time = getTime();
        if (touchCount !== 0) {
            this._oldTouches.push({
                centerX,
                centerY,
                averageDistance,
                touchCount,
                time,
            });
            return;
        }

        let oldTouches = this._oldTouches;
        this._oldTouches = [];

        // Search for an old touch that was long enough ago that the velocity should be stable
        for (let i = oldTouches.length - 2; i >= 0; i--) {
            // Ignore touches due to a pinch gesture
            if (oldTouches[i].touchCount > 1) {
                return;
            }

            // If we find an old enough touch, maybe do a fling
            if (time - oldTouches[i].time > 0.1 * 1000) {
                this._maybeFling(oldTouches[i], oldTouches[i + 1]);
                return;
            }
        }
    }

    _maybeFling(beforeTouch, afterTouch) {
        // let scale = 1 / (afterTouch.time - beforeTouch.time);
        // let vx = (afterTouch.centerX - beforeTouch.centerX) * scale;
        // let vy = (afterTouch.centerY - beforeTouch.centerY) * scale;
        // let speed = Math.sqrt(vx * vx + vy * vy);
        // let duration = Math.log(1 + speed) / 5;
        // let flingDistance = speed * duration / 5; // Divide by 5 since a quintic decay function has an initial slope of 5
        // // Only fling if the speed is fast enough
        // if(speed > 50) {
        //     _startAnimation(.DECAY, duration);
        //     _endOrigin += velocity * (flingDistance / speed);
        // }
    }

    handleMouseDown(event) {
        let { offsetX: x, offsetY: y } = event;
        // this.setCursor(.MOVE);
        this._mouseDown = true;
        this._previousMouseX = x;
        this._previousMouseY = y;
        // console.log(this._canvas.object_underneath_pixel(new Vec2(x, y)));
    }

    handleMouseMove(event) {
        let { offsetX: x, offsetY: y, buttons } = event;
        if (buttons > 0) {
            this._canvas.translate(
                new Vec2(x - this._previousMouseX, y - this._previousMouseY),
            );
            this._requestRedraw();
            // this.setCursor(.MOVE);
        }

        this._previousMouseX = x;
        this._previousMouseY = y;
    }

    handleMouseUp(event) {
        let { offsetX: x, offsetY: y, buttons } = event;
        if (buttons === 0) {
            this._mouseDown = false;
            // this._mouseAction = .NONE
            // this.setCursor(.DEFAULT);
        }

        this._previousMouseX = x;
        this._previousMouseY = y;
    }

    handleFrame() {
        this._requestFrame();

        // let time = getTime();

        // if _animation != .NONE {
        // 	var t = (time - _startTime) / (_endTime - _startTime)

        // 	# Stop the animation once it's done
        // 	if t > 1 {
        // 		_canvas.setOriginAndScale(_endOrigin.x, _endOrigin.y, _endScale)
        // 		_animation = .NONE
        // 	}

        // 	else {
        // 		# Bend the animation curve for a more pleasant animation
        // 		if _animation == .EASE_IN_OUT {
        // 			t *= t * t * (t * (t * 6 - 15) + 10)
        // 		} else {
        // 			assert(_animation == .DECAY)
        // 			t = 1 - t
        // 			t = 1 - t * t * t * t * t
        // 		}

        // 		# Animate both origin and scale
        // 		_canvas.setOriginAndScale(
        // 			_startOrigin.x + (_endOrigin.x - _startOrigin.x) * t,
        // 			_startOrigin.y + (_endOrigin.y - _startOrigin.y) * t,
        // 			1 / (1 / _startScale + (1 / _endScale - 1 / _startScale) * t))
        // 	}

        // 	_requestRedraw
        // }

        if (this._needsRedraw) {
            this._idleFrames = 0;
            this._needsRedraw = false;
            this._draw();
            return;
        }
        // Render occasionally even when idle. Chrome must render at least 10fps to
        // avoid stutter when starting to render at 60fps again.
        // this._idleFrames ++;
        // if(this._idleFrames % 6 == 0 && this._idleFrames < 60 * 2) {
        // 	this._draw();
        // }
    }

    _draw() {
        this._canvas.render();
        // this._canvas.test_speed("\u220e", 0.0);
        // this._canvas.draw_box(
        //     this._canvas.transform_x(1), this._canvas.transform_y(1),
        //     10, 10
        // );
        // this._canvas.test_stix_math();

        // this._canvas.draw_box(
        //     app._canvas.transform_x(0), app._canvas.transform_y(0),
        //     10, 10
        // );
        // this._canvas.draw_letter(this.font, "+".codePointAt(0),
        //     new Vec2(app._canvas.transform_x(0), app._canvas.transform_y(0)),
        //     50,
        //     this.pkg.HorizontalAlignment.Center,
        //     this.pkg.VerticalAlignment.Center,
        //     new Vec4(0, 0, 0, 0)
        // );

        //     this._canvas.draw_letter(this.font, "e".codePointAt(0),
        //     new Vec2(app._canvas.transform_x(-1), app._canvas.transform_y(1)),
        //     50,
        //     this.pkg.HorizontalAlignment.Center,
        //     this.pkg.VerticalAlignment.Center,
        //     new Vec4(0, 0, 0, 0)
        // );

        // this._canvas.draw_letter_convex_hull(this.font, "g".codePointAt(0), new Vec2(0, 0), 100, true)
        // this._canvas.draw_arrow(3, true);
        // _canvas.endFrame
    }
}
