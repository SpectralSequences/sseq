/**
 * Forked from https://github.com/usablica/intro.js
 */
const getBoundingClientRect = (element) => { const {top, right, bottom, left, width, height, x, y} = element.getBoundingClientRect(); return {top, right, bottom, left, width, height, x, y} }

/**
 * To set the show element
 * This function set a relative (in most cases) position and changes the z-index
 *
 * @api private
 * @method _setShowElement
 * @param {Object} targetElement
 */
function _setShowElement(targetElement) {
    var parentElm;
    // we need to add this show element class to the parent of SVG elements
    // because the SVG elements can't have independent z-index
    if (targetElement.element instanceof SVGElement) {
        parentElm = targetElement.element.parentNode;

        while (targetElement.element.parentNode !== null) {
            if (!parentElm.tagName || parentElm.tagName.toLowerCase() === 'body') break;

            if (parentElm.tagName.toLowerCase() === 'svg') {
                parentElm.classList.add('introjs-showElement');
                parentElm.classList.add('introjs-relativePosition');
            }

            parentElm = parentElm.parentNode;
        }
    }

    targetElement.element.classList.add('introjs-showElement');

    var currentElementPosition = _getPropValue(targetElement.element, 'position');
    if (currentElementPosition !== 'absolute' &&
        currentElementPosition !== 'relative' &&
        currentElementPosition !== 'fixed') {
        //change to new intro item
        targetElement.element.classList.add('introjs-relativePosition');
    }

    parentElm = targetElement.element.parentNode;
    while (parentElm !== null) {
        if (!parentElm.tagName || parentElm.tagName.toLowerCase() === 'body') break;

        //fix The Stacking Context problem.
        //More detail: https://developer.mozilla.org/en-US/docs/Web/Guide/CSS/Understanding_z_index/The_stacking_context
        var zIndex = _getPropValue(parentElm, 'z-index');
        var opacity = parseFloat(_getPropValue(parentElm, 'opacity'));
        var transform = _getPropValue(parentElm, 'transform') || _getPropValue(parentElm, '-webkit-transform') || _getPropValue(parentElm, '-moz-transform') || _getPropValue(parentElm, '-ms-transform') || _getPropValue(parentElm, '-o-transform');
        if (/[0-9]+/.test(zIndex) || opacity < 1 || (transform !== 'none' && transform !== undefined)) {
            parentElm.classList.add('introjs-fixParent');
        }

        parentElm = parentElm.parentNode;
    }
}

/**
 * Get an element position on the page
 * Thanks to `meouw`: http://stackoverflow.com/a/442474/375966
 *
 * @api private
 * @method _getOffset
 * @param {Object} element
 * @returns Element's position info
 */
function _getOffset(element) {
    var body = document.body;
    var docEl = document.documentElement;
    var scrollTop = window.pageYOffset || docEl.scrollTop || body.scrollTop;
    var scrollLeft = window.pageXOffset || docEl.scrollLeft || body.scrollLeft;
    var x = element.getBoundingClientRect();
    return {
        top: x.top + scrollTop,
        width: x.width,
        height: x.height,
        left: x.left + scrollLeft
    };
}

/**
 * Iterates arrays
 *
 * @param {Array} arr
 * @param {Function} forEachFnc
 * @param {Function} completeFnc
 * @return {Null}
 */
function _forEach(arr, forEachFnc, completeFnc) {
    // in case arr is an empty query selector node list
    if (arr) {
        for (var i = 0, len = arr.length; i < len; i++) {
            forEachFnc(arr[i], i);
        }
    }

    if (typeof(completeFnc) === 'function') {
        completeFnc();
    }
}

/**
 * Setting anchors to behave like buttons
 *
 * @api private
 * @method _setAnchorAsButton
 */
function _setAnchorAsButton(anchor){
    anchor.setAttribute('role', 'button');
    anchor.tabIndex = 0;
}

/**
 * Remove an entry from an array if it's there, does nothing if it isn't there.
 */
function removeEntry(arr, item) {
    if (arr.indexOf(item) > -1) {
        arr.splice(arr.indexOf(item), 1);
    }
}

/**
 * Get an element CSS property on the page
 * Thanks to JavaScript Kit: http://www.javascriptkit.com/dhtmltutors/dhtmlcascade4.shtml
 *
 * @api private
 * @method _getPropValue
 * @param {Object} element
 * @param {String} propName
 * @returns Element's property value
 */
function _getPropValue (element, propName) {
    var propValue = '';
    if (element.currentStyle) { //IE
        propValue = element.currentStyle[propName];
    } else if (document.defaultView && document.defaultView.getComputedStyle) { //Others
        propValue = document.defaultView.getComputedStyle(element, null).getPropertyValue(propName);
    }

    //Prevent exception in IE
    if (propValue && propValue.toLowerCase) {
        return propValue.toLowerCase();
    } else {
        return propValue;
    }
}

/**
 * Checks to see if target element (or parents) position is fixed or not
 *
 * @api private
 * @method _isFixed
 * @param {Object} element
 * @returns Boolean
 */
function _isFixed (element) {
    var p = element.parentNode;

    if (!p || p.nodeName === 'HTML') {
        return false;
    }

    if (_getPropValue(element, 'position') === 'fixed') {
        return true;
    }

    return _isFixed(p);
}

/**
 * Provides a cross-browser way to get the screen dimensions
 * via: http://stackoverflow.com/questions/5864467/internet-explorer-innerheight
 *
 * @api private
 * @method _getWinSize
 * @returns {Object} width and height attributes
 */
function _getWinSize() {
    if (window.innerWidth !== undefined) {
        return { width: window.innerWidth, height: window.innerHeight };
    } else {
        var D = document.documentElement;
        return { width: D.clientWidth, height: D.clientHeight };
    }
}

export class IntroJS {
    /**
     * IntroJs main class
     *
     * @class IntroJs
     */
    constructor() {
        this._introItems = [];

        this._onResize = this._onResize.bind(this);
        this._onKeyDown = this._onKeyDown.bind(this);

        this._dummyTargetElement = null;

        this._options = {
            /* Next button label in tooltip box */
            nextLabel: 'Next &rarr;',
            /* Previous button label in tooltip box */
            prevLabel: '&larr; Back',
            /* Skip button label in tooltip box */
            skipLabel: 'Skip',
            /* Done button label in tooltip box */
            doneLabel: 'Done',
            /* Hide previous button in the first step? Otherwise, it will be disabled button. */
            hidePrev: false,
            /* Hide next button in the last step? Otherwise, it will be disabled button. */
            hideNext: false,
            /* Default tooltip box position */
            tooltipPosition: 'bottom',
            /* Next CSS class for tooltip boxes */
            tooltipClass: '',
            /* CSS class that is added to the helperLayer */
            highlightClass: '',
            /* Close introduction when pressing Escape button? */
            exitOnEsc: true,
            /* Close introduction when clicking on overlay layer? */
            exitOnOverlayClick: true,
            /* Let user use keyboard to navigate the tour? */
            keyboardNavigation: true,
            /* Show tour control buttons? */
            showButtons: true,
            /* Show tour bullets? */
            showBullets: true,
            /* Set the overlay opacity */
            overlayOpacity: 0.8,
            /* Precedence of positions, when auto is enabled */
            positionPrecedence: ["bottom", "top", "right", "left"],
            /* Disable an interaction with element? */
            disableInteraction: false,
            /* Set how much padding to be used around helper element */
            helperElementPadding: 10,
            /* additional classes to put on the buttons */
            buttonClass: "introjs-button"
        };
    }

    getDummyTargetElement() {
        if (!this._dummyTargetElement) {
            this._dummyTargetElement = document.createElement('div');
            this._dummyTargetElement.className = 'introjsFloatingElement';
            document.body.appendChild(this._dummyTargetElement);
        }
        return this._dummyTargetElement;
    }

    /**
     * Initiate a new introduction/guide
     *
     * @method start
     */
    start() {
        let introItems = [];

        for (let step of this._options.steps) {
            let currentItem = IntroJS._cloneObject(step);

            //set the step
            currentItem.step = introItems.length + 1;

            if (typeof (currentItem.disableInteraction) === 'undefined') {
                currentItem.disableInteraction = this._options.disableInteraction;
            }

            introItems.push(currentItem);
        }

        //set it to the introJs object
        this._introItems = introItems;

        this._addOverlayLayer()

        this.nextStep();

        window.addEventListener('keydown', this._onKeyDown, true);
        window.addEventListener('resize', this._onResize, true);
        return false;
    }

    _onResize () {
        this.refresh();
    }

    /**
     * on keyCode:
     * https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode
     * This feature has been removed from the Web standards.
     * Though some browsers may still support it, it is in
     * the process of being dropped.
     * Instead, you should use KeyboardEvent.code,
     * if it's implemented.
     *
     * jQuery's approach is to test for
     *   (1) e.which, then
     *   (2) e.charCode, then
     *   (3) e.keyCode
     * https://github.com/jquery/jquery/blob/a6b0705294d336ae2f63f7276de0da1195495363/src/event.js#L638
     *
     * @param type var
     * @return type
     */
    _onKeyDown (e) {
        var code = (e.code === null) ? e.which : e.code;

        // if code/e.which is null
        if (code === null) {
            code = (e.charCode === null) ? e.keyCode : e.charCode;
        }

        if ((code === 'Escape' || code === 27) && this._options.exitOnEsc === true) {
            //escape key pressed, exit the intro
            //check if exit callback is defined
            this.exit();
            e.stopPropagation();
        } else if (code === 'ArrowLeft' || code === 37) {
            //left arrow
            this._previousStep();
            e.stopPropagation();
        } else if (code === 'ArrowRight' || code === 39) {
            //right arrow
            this.nextStep();
            e.stopPropagation();
        } else if (code === 'Enter' || code === 13) {
            //srcElement === ie
            var target = e.target || e.srcElement;
            if (target && target.className.match('introjs-prevbutton')) {
                //user hit enter while focusing on previous button
                this._previousStep();
            } else if (target && target.className.match('introjs-skipbutton')) {
                //user hit enter while focusing on skip button
                if (this._introItems.length - 1 === this._currentStep && typeof (this._introCompleteCallback) === 'function') {
                    this._introCompleteCallback();
                }

                this.exit();
            } else if (target && target.getAttribute('data-stepnumber')) {
                // user hit enter while focusing on step bullet
                target.click();
            } else {
                //default behavior for responding to enter
                this.nextStep();
            }

            e.preventDefault();
            e.stopPropagation();
        }
    }

    /*
     * makes a copy of the object
     * @api private
     * @method _cloneObject
     */
    static _cloneObject(object) {
        if (object === null || typeof (object) !== 'object' || typeof (object.nodeType) !== 'undefined') {
            return object;
        }
        var temp = {};
        for (var key in object) {
            if (typeof(window.jQuery) !== 'undefined' && object[key] instanceof window.jQuery) {
                temp[key] = object[key];
            } else {
                temp[key] = IntroJS._cloneObject(object[key]);
            }
        }
        return temp;
    }
    /**
     * Go to specific step of introduction
     *
     * @method goToStep
     */
    goToStep(step) {
        //because steps starts with zero
        this._currentStep = step - 1;
        this._showElement();
        return this;
    }

    /**
     * Go to next step on intro
     *
     * @method nextStep
     */
    nextStep() {
        if (typeof (this._currentStep) === 'undefined') {
            this._currentStep = 0;
        } else {
            ++this._currentStep;
        }

        if ((this._introItems.length) <= this._currentStep) {
            //end of the intro
            //check if any callback is defined
            this.exit();
            return;
        }

        this._showElement();
        return this;
    }

    /**
     * Go to previous step on intro
     *
     * @api private
     * @method _previousStep
     */
    _previousStep() {
        if (this._currentStep === 0) {
            return false;
        }

        --this._currentStep;

        this._showElement();
    }

    /**
     * Update placement of the intro objects on the screen
     * @api private
     */
    refresh() {
        // re-align intros
        this._setHelperLayerPosition(document.querySelector('.introjs-helperLayer'));
        this._setHelperLayerPosition(document.querySelector('.introjs-tooltipReferenceLayer'));
        this._setHelperLayerPosition(document.querySelector('.introjs-disableInteraction'));

        // re-align tooltip
        if(this._currentStep !== undefined && this._currentStep !== null) {
            let oldArrowLayer        = document.querySelector('.introjs-arrow'),
                oldtooltipContainer  = document.querySelector('.introjs-tooltip');
            this._placeTooltip(this._introItems[this._currentStep].element, oldtooltipContainer, oldArrowLayer);
        }

        return this;
    }

    /**
     * Exit from intro
     *
     * @method exit
     */
    exit() {
        //remove overlay layers from the page
        if (this._overlayLayer) {
            this._overlayLayer.parentNode.removeChild(this._overlayLayer);
        }

        //remove all helper layers
        var helperLayer = document.querySelector('.introjs-helperLayer');
        if (helperLayer) {
            helperLayer.parentNode.removeChild(helperLayer);
        }

        var referenceLayer = document.querySelector('.introjs-tooltipReferenceLayer');
        if (referenceLayer) {
            referenceLayer.parentNode.removeChild(referenceLayer);
        }

        //remove disableInteractionLayer
        var disableInteractionLayer = document.querySelector('.introjs-disableInteraction');
        if (disableInteractionLayer) {
            disableInteractionLayer.parentNode.removeChild(disableInteractionLayer);
        }

        //remove intro floating element
        var floatingElement = document.querySelector('.introjsFloatingElement');
        if (floatingElement) {
            floatingElement.parentNode.removeChild(floatingElement);
        }

        this._removeShowElement();

        //remove `introjs-fixParent` class from the elements
        var fixParents = document.querySelectorAll('.introjs-fixParent');
        _forEach(fixParents, (parent) => {
            parent.classList.remove('introjs-fixParent');
        });

        //clean listeners
        window.removeEventListener('keydown', this._onKeyDown, true);
        window.removeEventListener('resize', this._onResize, true);

        //set the step to zero
        this._currentStep = undefined;

        if (this.onEnd) {
            this.onEnd();
        }
    }

    /**
     * Render tooltip box in the page
     *
     * @api private
     * @method _placeTooltip
     * @param {HTMLElement} targetElement
     * @param {HTMLElement} tooltipLayer
     * @param {HTMLElement} arrowLayer
     */
    _placeTooltip(targetElement, tooltipLayer, arrowLayer) {
        var tooltipCssClass = '',
            currentStepObj,
            tooltipOffset,
            targetOffset,
            windowSize,
            currentTooltipPosition;

        //reset the old style
        tooltipLayer.style.top        = null;
        tooltipLayer.style.right      = null;
        tooltipLayer.style.bottom     = null;
        tooltipLayer.style.left       = null;
        tooltipLayer.style.marginLeft = null;
        tooltipLayer.style.marginTop  = null;

        arrowLayer.style.display = 'inherit';

        //prevent error when `this._currentStep` is undefined
        if (!this._introItems[this._currentStep]) return;

        //if we have a custom css class for each step
        currentStepObj = this._introItems[this._currentStep];
        if (typeof (currentStepObj.tooltipClass) === 'string') {
            tooltipCssClass = currentStepObj.tooltipClass;
        } else {
            tooltipCssClass = this._options.tooltipClass;
        }
        if (currentStepObj.width) {
            tooltipLayer.style.maxWidth = currentStepObj.width + "px";
            tooltipLayer.style.width = currentStepObj.width + "px";
        } else {
            tooltipLayer.style.removeProperty("max-width");
            tooltipLayer.style.removeProperty("width");
        }

        tooltipLayer.className = ('introjs-tooltip ' + tooltipCssClass).replace(/^\s+|\s+$/g, '');
        tooltipLayer.setAttribute('role', 'dialog');

        currentTooltipPosition = this._introItems[this._currentStep].position;

        if (this._introItems[this._currentStep].element.className.includes('introjsFloatingElement'))
            currentTooltipPosition = 'floating';

        // Floating is always valid, no point in calculating
        if (currentTooltipPosition !== "floating") { 
            currentTooltipPosition = this._determineAutoPosition(targetElement, tooltipLayer, currentTooltipPosition);
        }

        var tooltipLayerStyleLeft;
        targetOffset  = _getOffset(targetElement);
        tooltipOffset = _getOffset(tooltipLayer);
        windowSize    = _getWinSize();

        tooltipLayer.classList.add('introjs-' + currentTooltipPosition);

        switch (currentTooltipPosition) {
            case 'top-right-aligned':
                arrowLayer.className      = 'introjs-arrow bottom-right';

                var tooltipLayerStyleRight = 0;
                this._checkLeft(targetOffset, tooltipLayerStyleRight, tooltipOffset, tooltipLayer);
                tooltipLayer.style.bottom    = (targetOffset.height +  20) + 'px';
                break;

            case 'top-middle-aligned':
                arrowLayer.className      = 'introjs-arrow bottom-middle';

                var tooltipLayerStyleLeftRight = targetOffset.width / 2 - tooltipOffset.width / 2;

                if (this._checkLeft(targetOffset, tooltipLayerStyleLeftRight, tooltipOffset, tooltipLayer)) {
                    tooltipLayer.style.right = null;
                    this._checkRight(targetOffset, tooltipLayerStyleLeftRight, tooltipOffset, windowSize, tooltipLayer);
                }
                tooltipLayer.style.bottom = (targetOffset.height + 20) + 'px';
                break;

            case 'top-left-aligned':
                // top-left-aligned is the same as the default top
            case 'top':
                arrowLayer.className = 'introjs-arrow bottom';

                tooltipLayerStyleLeft = 15;

                this._checkRight(targetOffset, tooltipLayerStyleLeft, tooltipOffset, windowSize, tooltipLayer);
                tooltipLayer.style.bottom = (targetOffset.height +  20) + 'px';
                break;
            case 'right':
                tooltipLayer.style.left = (targetOffset.width + 20) + 'px';
                if (targetOffset.top + tooltipOffset.height > windowSize.height) {
                    // In this case, right would have fallen below the bottom of the screen.
                    // Modify so that the bottom of the tooltip connects with the target
                    arrowLayer.className = "introjs-arrow left-bottom";
                    tooltipLayer.style.top = "-" + (tooltipOffset.height - targetOffset.height - 20) + "px";
                } else {
                    arrowLayer.className = 'introjs-arrow left';
                }
                break;
            case 'left':
                if (targetOffset.top + tooltipOffset.height > windowSize.height) {
                    // In this case, left would have fallen below the bottom of the screen.
                    // Modify so that the bottom of the tooltip connects with the target
                    tooltipLayer.style.top = "-" + (tooltipOffset.height - targetOffset.height - 20) + "px";
                    arrowLayer.className = 'introjs-arrow right-bottom';
                } else {
                    arrowLayer.className = 'introjs-arrow right';
                }
                tooltipLayer.style.right = (targetOffset.width + 20) + 'px';

                break;
            case 'floating':
                arrowLayer.style.display = 'none';

                //we have to adjust the top and left of layer manually for intro items without element
                tooltipLayer.style.left   = '50%';
                tooltipLayer.style.top    = '50%';
                tooltipLayer.style.marginLeft = '-' + (tooltipOffset.width / 2)  + 'px';
                tooltipLayer.style.marginTop  = '-' + (tooltipOffset.height / 2) + 'px';

                break;
            case 'bottom-right-aligned':
                arrowLayer.className      = 'introjs-arrow top-right';

                tooltipLayerStyleRight = 0;
                this._checkLeft(targetOffset, tooltipLayerStyleRight, tooltipOffset, tooltipLayer);
                tooltipLayer.style.top    = (targetOffset.height +  20) + 'px';
                break;

            case 'bottom-middle-aligned':
                arrowLayer.className      = 'introjs-arrow top-middle';

                tooltipLayerStyleLeftRight = targetOffset.width / 2 - tooltipOffset.width / 2;

                if (this._checkLeft(targetOffset, tooltipLayerStyleLeftRight, tooltipOffset, tooltipLayer)) {
                    tooltipLayer.style.right = null;
                    this._checkRight(targetOffset, tooltipLayerStyleLeftRight, tooltipOffset, windowSize, tooltipLayer);
                }
                tooltipLayer.style.top = (targetOffset.height + 20) + 'px';
                break;

                // case 'bottom-left-aligned':
                // Bottom-left-aligned is the same as the default bottom
                // case 'bottom':
                // Bottom going to follow the default behavior
            default:
                arrowLayer.className = 'introjs-arrow top';

                tooltipLayerStyleLeft = 0;
                this._checkRight(targetOffset, tooltipLayerStyleLeft, tooltipOffset, windowSize, tooltipLayer);
                tooltipLayer.style.top    = (targetOffset.height +  20) + 'px';
        }
    }

    /**
     * Set tooltip left so it doesn't go off the right side of the window
     *
     * @return boolean true, if tooltipLayerStyleLeft is ok.  false, otherwise.
     */
    _checkRight(targetOffset, tooltipLayerStyleLeft, tooltipOffset, windowSize, tooltipLayer) {
        if (targetOffset.left + tooltipLayerStyleLeft + tooltipOffset.width > windowSize.width) {
            // off the right side of the window
            tooltipLayer.style.left = (windowSize.width - tooltipOffset.width - targetOffset.left) + 'px';
            return false;
        }
        tooltipLayer.style.left = tooltipLayerStyleLeft + 'px';
        return true;
    }

    /**
     * Set tooltip right so it doesn't go off the left side of the window
     *
     * @return boolean true, if tooltipLayerStyleRight is ok.  false, otherwise.
     */
    _checkLeft(targetOffset, tooltipLayerStyleRight, tooltipOffset, tooltipLayer) {
        if (targetOffset.left + targetOffset.width - tooltipLayerStyleRight - tooltipOffset.width < 0) {
            // off the left side of the window
            tooltipLayer.style.left = (-targetOffset.left) + 'px';
            return false;
        }
        tooltipLayer.style.right = tooltipLayerStyleRight + 'px';
        return true;
    }

    static getRecursiveBoundingBox(element, offset=false) {
        var rect = getBoundingClientRect(element); // Makes it mutable
        let parentElement = element;
        while (parentElement = parentElement.parentElement) {
            let parentBox = parentElement.getBoundingClientRect();
            rect.left = Math.max(rect.left, parentBox.left);
            rect.right = Math.min(rect.right, parentBox.right);
            rect.top = Math.max(rect.top, parentBox.top);
            rect.bottom = Math.min(rect.bottom, parentBox.bottom);
        }
        rect.height = rect.bottom - rect.top;
        rect.width = rect.right - rect.left;
        if (offset) {
            let body = document.body;
            let docEl = document.documentElement;
            let scrollTop = window.pageYOffset || docEl.scrollTop || body.scrollTop;
            let scrollLeft = window.pageXOffset || docEl.scrollLeft || body.scrollLeft;
            rect.left += scrollLeft;
            rect.top += scrollLeft;
        }

        return rect;
    }

    /**
     * Determines the position of the tooltip based on the position precedence and availability
     * of screen space.
     *
     * @param {Object}    targetElement
     * @param {Object}    tooltipLayer
     * @param {String}    desiredTooltipPosition
     * @return {String}   calculatedPosition
     */
    _determineAutoPosition(targetElement, tooltipLayer, desiredTooltipPosition) {

        // Take a clone of position precedence. These will be the available
        var possiblePositions = this._options.positionPrecedence.slice();

        var windowSize = _getWinSize();
        var tooltipElementRect = tooltipLayer.getBoundingClientRect();
        var tooltipHeight = tooltipElementRect.height + 10;
        var tooltipWidth = tooltipElementRect.width + 20;
        var targetElementRect = IntroJS.getRecursiveBoundingBox(targetElement); // Makes it mutable

        // If we check all the possible areas, and there are no valid places for the tooltip, the element
        // must take up most of the screen real estate. Show the tooltip floating in the middle of the screen.
        var calculatedPosition = "floating";

        /*
         * auto determine position 
         */

        // Check for space below
        if (targetElementRect.bottom + tooltipHeight > windowSize.height) {
            removeEntry(possiblePositions, "bottom");
        }

        // Check for space above
        if (targetElementRect.top - tooltipHeight < 0) {
            removeEntry(possiblePositions, "top");
        }

        // Check for space to the right
        if (targetElementRect.right + tooltipWidth > windowSize.width) {
            removeEntry(possiblePositions, "right");
        }

        // Check for space to the left
        if (targetElementRect.left - tooltipWidth < 0) {
            removeEntry(possiblePositions, "left");
        }

        // @var {String}  ex: 'right-aligned'
        var desiredAlignment = ((pos) => {
            var hyphenIndex = pos.indexOf('-');
            if (hyphenIndex !== -1) {
                // has alignment
                return pos.substr(hyphenIndex);
            }
            return '';
        })(desiredTooltipPosition || '');

        // strip alignment from position
        if (desiredTooltipPosition) {
            // ex: "bottom-right-aligned"
            // should return 'bottom'
            desiredTooltipPosition = desiredTooltipPosition.split('-')[0];
        }

        if (possiblePositions.length) {
            if (desiredTooltipPosition !== "auto" &&
                possiblePositions.indexOf(desiredTooltipPosition) > -1) {
                // If the requested position is in the list, choose that
                calculatedPosition = desiredTooltipPosition;
            } else {
                // Pick the first valid position, in order
                calculatedPosition = possiblePositions[0];
            }
        }

        // only top and bottom positions have optional alignments
        if (['top', 'bottom'].indexOf(calculatedPosition) !== -1) {
            calculatedPosition += this._determineAutoAlignment(targetElementRect.left, tooltipWidth, windowSize, desiredAlignment);
        }

        return calculatedPosition;
    }

    /**
     * auto-determine alignment
     * @param {Integer}  offsetLeft
     * @param {Integer}  tooltipWidth
     * @param {Object}   windowSize
     * @param {String}   desiredAlignment
     * @return {String}  calculatedAlignment
     */
    _determineAutoAlignment (offsetLeft, tooltipWidth, windowSize, desiredAlignment) {
        var halfTooltipWidth = tooltipWidth / 2,
            winWidth = Math.min(windowSize.width, window.screen.width),
            possibleAlignments = ['-left-aligned', '-middle-aligned', '-right-aligned'],
            calculatedAlignment = '';

        // valid left must be at least a tooltipWidth
        // away from right side
        if (winWidth - offsetLeft < tooltipWidth) {
            removeEntry(possibleAlignments, '-left-aligned');
        }

        // valid middle must be at least half 
        // width away from both sides
        if (offsetLeft < halfTooltipWidth || 
            winWidth - offsetLeft < halfTooltipWidth) {
            removeEntry(possibleAlignments, '-middle-aligned');
        }

        // valid right must be at least a tooltipWidth
        // width away from left side
        if (offsetLeft < tooltipWidth) {
            removeEntry(possibleAlignments, '-right-aligned');
        }

        if (possibleAlignments.length) {
            if (possibleAlignments.indexOf(desiredAlignment) !== -1) {
                // the desired alignment is valid
                calculatedAlignment = desiredAlignment;
            } else {
                // pick the first valid position, in order
                calculatedAlignment = possibleAlignments[0];
            }
        } else {
            // if screen width is too small 
            // for ANY alignment, middle is 
            // probably the best for visibility
            calculatedAlignment = '-middle-aligned';
        }

        return calculatedAlignment;
    }

    /**
     * Update the position of the helper layer on the screen
     *
     * @api private
     * @method _setHelperLayerPosition
     * @param {Object} helperLayer
     */
    _setHelperLayerPosition(helperLayer) {
        if (helperLayer) {
            //prevent error when `this._currentStep` in undefined
            if (!this._introItems[this._currentStep]) return;

            let currentElement  = this._introItems[this._currentStep];
            let elementPosition = IntroJS.getRecursiveBoundingBox(currentElement.element, true);
            let widthHeightPadding = this._options.helperElementPadding;

            // If the target element is fixed, the tooltip should be fixed as well.
            // Otherwise, remove a fixed class that may be left over from the previous
            // step.
            if (_isFixed(currentElement.element)) {
                helperLayer.classList.add('introjs-fixedTooltip');
            } else {
                helperLayer.classList.remove('introjs-fixedTooltip');
            }

            if (currentElement.position === 'floating') {
                widthHeightPadding = 0;
            }

            //set new position to helper layer
            helperLayer.style.cssText = 'width: ' + (elementPosition.width  + widthHeightPadding)  + 'px; ' +
                'height:' + (elementPosition.height + widthHeightPadding)  + 'px; ' +
                'top:'    + (elementPosition.top    - widthHeightPadding / 2)   + 'px;' +
                'left: '  + (elementPosition.left   - widthHeightPadding / 2)   + 'px;';

        }
    }

    /**
     * Add disableinteraction layer and adjust the size and position of the layer
     *
     * @api private
     * @method _disableInteraction
     */
    _disableInteraction() {
        var disableInteractionLayer = document.querySelector('.introjs-disableInteraction');

        if (disableInteractionLayer === null) {
            disableInteractionLayer = document.createElement('div');
            disableInteractionLayer.className = 'introjs-disableInteraction';
            document.body.appendChild(disableInteractionLayer);
        }

        this._setHelperLayerPosition(disableInteractionLayer);
    }

    /**
     * Show an element on the page
     *
     * @api private
     * @method _showElement
     */
    _showElement() {
        let targetElement = this._introItems[this._currentStep];

        if (targetElement.before) {
            targetElement.before();
        }

        if (targetElement.elementFunction) {
            targetElement.element = targetElement.elementFunction();
        } else if (!targetElement.element) {
            targetElement.element = this.getDummyTargetElement();
        }

        let oldHelperLayer = document.querySelector('.introjs-helperLayer');
        let oldReferenceLayer = document.querySelector('.introjs-tooltipReferenceLayer');
        let highlightClass = 'introjs-helperLayer';
        let nextTooltipButton, prevTooltipButton, skipTooltipButton;

        //check for a current step highlight class
        if (typeof (targetElement.highlightClass) === 'string') {
            highlightClass += (' ' + targetElement.highlightClass);
        }
        //check for options highlight class
        if (typeof (this._options.highlightClass) === 'string') {
            highlightClass += (' ' + this._options.highlightClass);
        }

        if (oldHelperLayer !== null) {
            let oldtooltipLayer      = oldReferenceLayer.querySelector('.introjs-tooltiptext'),
                oldArrowLayer        = oldReferenceLayer.querySelector('.introjs-arrow'),
                oldtooltipContainer  = oldReferenceLayer.querySelector('.introjs-tooltip');

            skipTooltipButton    = oldReferenceLayer.querySelector('.introjs-skipbutton');
            prevTooltipButton    = oldReferenceLayer.querySelector('.introjs-prevbutton');
            nextTooltipButton    = oldReferenceLayer.querySelector('.introjs-nextbutton');

            //update or reset the helper highlight class
            oldHelperLayer.className = highlightClass;
            //hide the tooltip
            oldtooltipContainer.style.opacity = 0;
            oldtooltipContainer.style.display = "none";


            // set new position to helper layer
            this._setHelperLayerPosition(oldHelperLayer);
            this._setHelperLayerPosition(oldReferenceLayer);

            //remove `introjs-fixParent` class from the elements
            var fixParents = document.querySelectorAll('.introjs-fixParent');
            _forEach(fixParents, (parent) => {
                parent.classList.remove('introjs-fixParent');
            });

            //remove old classes if the element still exist
            this._removeShowElement();

            //we should wait until the CSS3 transition is competed (it's 0.3 sec) to prevent incorrect `height` and `width` calculation
            if (this._lastShowElementTimer) {
                window.clearTimeout(this._lastShowElementTimer);
            }

            this._lastShowElementTimer = window.setTimeout(() => {
                //set current tooltip text
                oldtooltipLayer.innerHTML = targetElement.intro;
                //set the tooltip position
                oldtooltipContainer.style.display = "block";
                this._placeTooltip(targetElement.element, oldtooltipContainer, oldArrowLayer);

                //change active bullet
                if (this._options.showBullets) {
                    oldReferenceLayer.querySelector('.introjs-bullets li > a.active').className = '';
                    oldReferenceLayer.querySelector('.introjs-bullets li > a[data-stepnumber="' + targetElement.step + '"]').className = 'active';
                }

                //show the tooltip
                oldtooltipContainer.style.opacity = 1;

                //reset button focus
                if (typeof skipTooltipButton !== "undefined" && skipTooltipButton !== null && /introjs-donebutton/gi.test(skipTooltipButton.className)) {
                    // skip button is now "done" button
                    skipTooltipButton.focus();
                } else if (typeof nextTooltipButton !== "undefined" && nextTooltipButton !== null) {
                    //still in the tour, focus on next
                    nextTooltipButton.focus();
                }
            }, 350);

            // end of old element if-else condition
        } else {
            var helperLayer       = document.createElement('div'),
                referenceLayer    = document.createElement('div'),
                arrowLayer        = document.createElement('div'),
                tooltipLayer      = document.createElement('div'),
                tooltipTextLayer  = document.createElement('div'),
                bulletsLayer      = document.createElement('div'),
                buttonsLayer      = document.createElement('div');

            helperLayer.className = highlightClass;
            referenceLayer.className = 'introjs-tooltipReferenceLayer';

            //set new position to helper layer
            this._setHelperLayerPosition(helperLayer);
            this._setHelperLayerPosition(referenceLayer);

            //add helper layer to target element
            document.body.appendChild(helperLayer);
            document.body.appendChild(referenceLayer);

            arrowLayer.className = 'introjs-arrow';

            tooltipTextLayer.className = 'introjs-tooltiptext';
            tooltipTextLayer.innerHTML = targetElement.intro;

            bulletsLayer.className = 'introjs-bullets';

            if (this._options.showBullets === false) {
                bulletsLayer.style.display = 'none';
            }

            var ulContainer = document.createElement('ul');
            ulContainer.setAttribute('role', 'tablist');

            var anchorClick = () => this.goToStep(this.getAttribute('data-stepnumber'));

            _forEach(this._introItems, (item, i) => {
                var innerLi    = document.createElement('li');
                var anchorLink = document.createElement('a');

                innerLi.setAttribute('role', 'presentation');
                anchorLink.setAttribute('role', 'tab');

                anchorLink.addEventListener('click', () => {
                    this.goToStep(i + 1);
                });

                if (i === (targetElement.step-1)) {
                    anchorLink.className = 'active';
                } 

                _setAnchorAsButton(anchorLink);
                anchorLink.innerHTML = "&nbsp;";
                anchorLink.setAttribute('data-stepnumber', item.step);

                innerLi.appendChild(anchorLink);
                ulContainer.appendChild(innerLi);
            });

            bulletsLayer.appendChild(ulContainer);

            buttonsLayer.className = 'introjs-tooltipbuttons';
            if (this._options.showButtons === false) {
                buttonsLayer.style.display = 'none';
            }

            tooltipLayer.className = 'introjs-tooltip';
            tooltipLayer.appendChild(tooltipTextLayer);
            tooltipLayer.appendChild(bulletsLayer);

            tooltipLayer.appendChild(arrowLayer);
            referenceLayer.appendChild(tooltipLayer);

            //next button
            nextTooltipButton = document.createElement('a');

            nextTooltipButton.onclick = () => {
                if (this._introItems.length - 1 !== this._currentStep) {
                    this.nextStep();
                }
            };

            _setAnchorAsButton(nextTooltipButton);
            nextTooltipButton.innerHTML = this._options.nextLabel;

            //previous button
            prevTooltipButton = document.createElement('a');

            prevTooltipButton.onclick = () => {
                if (this._currentStep !== 0) {
                    this._previousStep();
                }
            };

            _setAnchorAsButton(prevTooltipButton);
            prevTooltipButton.innerHTML = this._options.prevLabel;

            //skip button
            skipTooltipButton = document.createElement('a');
            skipTooltipButton.className = this._options.buttonClass + ' introjs-skipbutton ';
            _setAnchorAsButton(skipTooltipButton);
            skipTooltipButton.innerHTML = this._options.skipLabel;

            skipTooltipButton.onclick = () => this.exit();

            buttonsLayer.appendChild(skipTooltipButton);

            //in order to prevent displaying next/previous button always
            if (this._introItems.length > 1) {
                buttonsLayer.appendChild(prevTooltipButton);
                buttonsLayer.appendChild(nextTooltipButton);
            }

            tooltipLayer.appendChild(buttonsLayer);

            //set proper position
            this._placeTooltip(targetElement.element, tooltipLayer, arrowLayer);

            //end of new element if-else condition
        }

        // removing previous disable interaction layer
        var disableInteractionLayer = document.body.querySelector('.introjs-disableInteraction');
        if (disableInteractionLayer) {
            disableInteractionLayer.parentNode.removeChild(disableInteractionLayer);
        }

        //disable interaction
        if (targetElement.disableInteraction) {
            this._disableInteraction();
        }

        // when it's the first step of tour
        if (this._currentStep === 0 && this._introItems.length > 1) {
            if (typeof skipTooltipButton !== "undefined" && skipTooltipButton !== null) {
                skipTooltipButton.className = this._options.buttonClass + ' introjs-skipbutton';
            }
            if (typeof nextTooltipButton !== "undefined" && nextTooltipButton !== null) {
                nextTooltipButton.className = this._options.buttonClass + ' introjs-nextbutton';
            }

            if (this._options.hidePrev === true) {
                if (typeof prevTooltipButton !== "undefined" && prevTooltipButton !== null) {
                    prevTooltipButton.className = this._options.buttonClass + ' introjs-prevbutton introjs-hidden';
                }
                if (typeof nextTooltipButton !== "undefined" && nextTooltipButton !== null) {
                    nextTooltipButton.classList.add('introjs-fullbutton');
                }
            } else {
                if (typeof prevTooltipButton !== "undefined" && prevTooltipButton !== null) {
                    prevTooltipButton.className = this._options.buttonClass + ' introjs-prevbutton introjs-disabled';
                }
            }

            if (typeof skipTooltipButton !== "undefined" && skipTooltipButton !== null) {
                skipTooltipButton.innerHTML = this._options.skipLabel;
            }
        } else if (this._introItems.length - 1 === this._currentStep || this._introItems.length === 1) {
            // last step of tour
            if (typeof skipTooltipButton !== "undefined" && skipTooltipButton !== null) {
                skipTooltipButton.innerHTML = this._options.doneLabel;
                // adding donebutton class in addition to skipbutton
                skipTooltipButton.classList.add('introjs-donebutton');
            }
            if (typeof prevTooltipButton !== "undefined" && prevTooltipButton !== null) {
                prevTooltipButton.className = this._options.buttonClass + ' introjs-prevbutton';
            }

            if (this._options.hideNext === true) {
                if (typeof nextTooltipButton !== "undefined" && nextTooltipButton !== null) {
                    nextTooltipButton.className = this._options.buttonClass + ' introjs-nextbutton introjs-hidden';
                }
                if (typeof prevTooltipButton !== "undefined" && prevTooltipButton !== null) {
                    prevTooltipButton.classList.add('introjs-fullbutton');
                }
            } else {
                if (typeof nextTooltipButton !== "undefined" && nextTooltipButton !== null) {
                    nextTooltipButton.className = this._options.buttonClass + ' introjs-nextbutton introjs-disabled';
                }
            }
        } else {
            // steps between start and end
            if (typeof skipTooltipButton !== "undefined" && skipTooltipButton !== null) {
                skipTooltipButton.className = this._options.buttonClass + ' introjs-skipbutton';
            }
            if (typeof prevTooltipButton !== "undefined" && prevTooltipButton !== null) {
                prevTooltipButton.className = this._options.buttonClass + ' introjs-prevbutton';
            }
            if (typeof nextTooltipButton !== "undefined" && nextTooltipButton !== null) {
                nextTooltipButton.className = this._options.buttonClass + ' introjs-nextbutton';
            }
            if (typeof skipTooltipButton !== "undefined" && skipTooltipButton !== null) {
                skipTooltipButton.innerHTML = this._options.skipLabel;
            }
        }

        prevTooltipButton.setAttribute('role', 'button');
        nextTooltipButton.setAttribute('role', 'button');
        skipTooltipButton.setAttribute('role', 'button');

        //Set focus on "next" button, so that hitting Enter always moves you onto the next step
        if (typeof nextTooltipButton !== "undefined" && nextTooltipButton !== null) {
            nextTooltipButton.focus();
        }

        _setShowElement(targetElement);
    }

    /**
     * To remove all show element(s)
     *
     * @api private
     * @method _removeShowElement
     */
    _removeShowElement() {
        var elms = document.querySelectorAll('.introjs-showElement');

        for (let elm of elms) {
            elm.className = elm.className.replace(/introjs-[a-zA-Z]+/g, '').replace(/^\s+|\s+$/g, '');
        }
    }
    /**
     * Add overlay layer to the page
     *
     * @api private
     * @method _addOverlayLayer
     * @param {Object} targetElm
     */
    _addOverlayLayer() {
        this._overlayLayer = document.createElement('div');
        let styleText = 'top: 0;bottom: 0; left: 0;right: 0;position: fixed;';

        //set css class name
        this._overlayLayer.className = 'introjs-overlay';
        this._overlayLayer.style.cssText = styleText;

        document.body.appendChild(this._overlayLayer);

        this._overlayLayer.onclick = () => {
            if (this._options.exitOnOverlayClick === true) {
                this.exit();
            }
        };

        window.setTimeout(() => {
            styleText += 'opacity: ' + this._options.overlayOpacity.toString() + ';';
            this._overlayLayer.style.cssText = styleText;
        }, 10);
    }

    setOptions(options) {
        for (let [k, v] of Object.entries(options)) {
            this._options[k] = v;
        }
        return this;
    }
}
