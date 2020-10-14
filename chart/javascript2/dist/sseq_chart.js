/******/ (function(modules) { // webpackBootstrap
/******/ 	// The module cache
/******/ 	var installedModules = {};
/******/
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/
/******/ 		// Check if module is in cache
/******/ 		if(installedModules[moduleId]) {
/******/ 			return installedModules[moduleId].exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		var module = installedModules[moduleId] = {
/******/ 			i: moduleId,
/******/ 			l: false,
/******/ 			exports: {}
/******/ 		};
/******/
/******/ 		// Execute the module function
/******/ 		var threw = true;
/******/ 		try {
/******/ 			modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
/******/ 			threw = false;
/******/ 		} finally {
/******/ 			if(threw) delete installedModules[moduleId];
/******/ 		}
/******/
/******/ 		// Flag the module as loaded
/******/ 		module.l = true;
/******/
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/
/******/
/******/ 	// expose the modules object (__webpack_modules__)
/******/ 	__webpack_require__.m = modules;
/******/
/******/ 	// expose the module cache
/******/ 	__webpack_require__.c = installedModules;
/******/
/******/ 	// define getter function for harmony exports
/******/ 	__webpack_require__.d = function(exports, name, getter) {
/******/ 		if(!__webpack_require__.o(exports, name)) {
/******/ 			Object.defineProperty(exports, name, { enumerable: true, get: getter });
/******/ 		}
/******/ 	};
/******/
/******/ 	// define __esModule on exports
/******/ 	__webpack_require__.r = function(exports) {
/******/ 		if(typeof Symbol !== 'undefined' && Symbol.toStringTag) {
/******/ 			Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });
/******/ 		}
/******/ 		Object.defineProperty(exports, '__esModule', { value: true });
/******/ 	};
/******/
/******/ 	// create a fake namespace object
/******/ 	// mode & 1: value is a module id, require it
/******/ 	// mode & 2: merge all properties of value into the ns
/******/ 	// mode & 4: return value when already ns object
/******/ 	// mode & 8|1: behave like require
/******/ 	__webpack_require__.t = function(value, mode) {
/******/ 		if(mode & 1) value = __webpack_require__(value);
/******/ 		if(mode & 8) return value;
/******/ 		if((mode & 4) && typeof value === 'object' && value && value.__esModule) return value;
/******/ 		var ns = Object.create(null);
/******/ 		__webpack_require__.r(ns);
/******/ 		Object.defineProperty(ns, 'default', { enumerable: true, value: value });
/******/ 		if(mode & 2 && typeof value != 'string') for(var key in value) __webpack_require__.d(ns, key, function(key) { return value[key]; }.bind(null, key));
/******/ 		return ns;
/******/ 	};
/******/
/******/ 	// getDefaultExport function for compatibility with non-harmony modules
/******/ 	__webpack_require__.n = function(module) {
/******/ 		var getter = module && module.__esModule ?
/******/ 			function getDefault() { return module['default']; } :
/******/ 			function getModuleExports() { return module; };
/******/ 		__webpack_require__.d(getter, 'a', getter);
/******/ 		return getter;
/******/ 	};
/******/
/******/ 	// Object.prototype.hasOwnProperty.call
/******/ 	__webpack_require__.o = function(object, property) { return Object.prototype.hasOwnProperty.call(object, property); };
/******/
/******/ 	// __webpack_public_path__
/******/ 	__webpack_require__.p = "";
/******/
/******/
/******/ 	// Load entry module and return exports
/******/ 	return __webpack_require__(__webpack_require__.s = "./src/lib.ts");
/******/ })
/************************************************************************/
/******/ ({

/***/ "./node_modules/ee-ts/lib/ee.js":
/*!**************************************!*\
  !*** ./node_modules/ee-ts/lib/ee.js ***!
  \**************************************/
/*! no static exports found */
/***/ (function(module, exports, __webpack_require__) {

"use strict";
eval("\nvar __generator = (this && this.__generator) || function (thisArg, body) {\n    var _ = { label: 0, sent: function() { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, f, y, t, g;\n    return g = { next: verb(0), \"throw\": verb(1), \"return\": verb(2) }, typeof Symbol === \"function\" && (g[Symbol.iterator] = function() { return this; }), g;\n    function verb(n) { return function (v) { return step([n, v]); }; }\n    function step(op) {\n        if (f) throw new TypeError(\"Generator is already executing.\");\n        while (_) try {\n            if (f = 1, y && (t = op[0] & 2 ? y[\"return\"] : op[0] ? y[\"throw\"] || ((t = y[\"return\"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;\n            if (y = 0, t) op = [op[0] & 2, t.value];\n            switch (op[0]) {\n                case 0: case 1: t = op; break;\n                case 4: _.label++; return { value: op[1], done: false };\n                case 5: _.label++; y = op[1]; op = [0]; continue;\n                case 7: op = _.ops.pop(); _.trys.pop(); continue;\n                default:\n                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) { _ = 0; continue; }\n                    if (op[0] === 3 && (!t || (op[1] > t[0] && op[1] < t[3]))) { _.label = op[1]; break; }\n                    if (op[0] === 6 && _.label < t[1]) { _.label = t[1]; t = op; break; }\n                    if (t && _.label < t[2]) { _.label = t[2]; _.ops.push(op); break; }\n                    if (t[2]) _.ops.pop();\n                    _.trys.pop(); continue;\n            }\n            op = body.call(thisArg, _);\n        } catch (e) { op = [6, e]; y = 0; } finally { f = t = 0; }\n        if (op[0] & 5) throw op[1]; return { value: op[0] ? op[1] : void 0, done: true };\n    }\n};\nObject.defineProperty(exports, \"__esModule\", { value: true });\nexports.$listeners = Symbol('EventEmitter.listeners');\nexports.$addListener = Symbol('EventEmitter.addListener');\n/** Statically typed event emitter */\nvar EventEmitter = /** @class */ (function () {\n    function EventEmitter() {\n        this[exports.$listeners] = {};\n    }\n    /** Count the number of listeners for an event */\n    EventEmitter.count = function (ee, key) {\n        var count = 0;\n        var list = ee[exports.$listeners][key];\n        if (list) {\n            var cb = list.first;\n            while (++count) {\n                if (cb.next) {\n                    cb = cb.next;\n                }\n                else\n                    break;\n            }\n        }\n        return count;\n    };\n    /** Check if an event has listeners */\n    EventEmitter.has = function (ee, key) {\n        if (key == '*') {\n            for (key in ee[exports.$listeners])\n                return true;\n            return false;\n        }\n        return ee[exports.$listeners][key] !== undefined;\n    };\n    /** Get an array of event keys that have listeners */\n    EventEmitter.keys = function (ee) {\n        return Object.keys(ee[exports.$listeners]);\n    };\n    /** Call the given listener when no other listeners exist */\n    EventEmitter.unhandle = function (ee, key, impl, disposables) {\n        var listener = function () {\n            var args = [];\n            for (var _i = 0; _i < arguments.length; _i++) {\n                args[_i] = arguments[_i];\n            }\n            if (!ee[exports.$listeners][key].first.next)\n                return impl.apply(void 0, args);\n        };\n        return ee.on(key, listener, disposables);\n    };\n    /** Implementation */\n    EventEmitter.prototype.on = function (arg, fn, disposables) {\n        if (typeof fn == 'function') {\n            return this[exports.$addListener](arg, fn, disposables);\n        }\n        return this[exports.$addListener](arg, undefined, fn);\n    };\n    /** Implementation */\n    EventEmitter.prototype.one = function (arg, fn, disposables) {\n        if (typeof fn == 'function') {\n            return this[exports.$addListener](arg, fn, disposables, true);\n        }\n        return this[exports.$addListener](arg, undefined, fn, true);\n    };\n    /** Implementation */\n    EventEmitter.prototype.off = function (arg, fn) {\n        if (arg == '*') {\n            var cache = this[exports.$listeners];\n            this[exports.$listeners] = {};\n            if (this._onEventUnhandled) {\n                for (var key in cache) {\n                    this._onEventUnhandled(key);\n                }\n            }\n            return this;\n        }\n        if (typeof fn == 'function') {\n            var list = this[exports.$listeners][arg];\n            if (!list || unlink(list, function (l) { return l.fn == fn; })) {\n                return this;\n            }\n        }\n        delete this[exports.$listeners][arg];\n        if (this._onEventUnhandled) {\n            this._onEventUnhandled(arg);\n        }\n        return this;\n    };\n    /** Implementation */\n    EventEmitter.prototype.emit = function (key) {\n        var args = [];\n        for (var _i = 1; _i < arguments.length; _i++) {\n            args[_i - 1] = arguments[_i];\n        }\n        var result;\n        var gen = this.listeners(key);\n        while (true) {\n            var _a = gen.next(), listener = _a.value, done = _a.done;\n            if (done) {\n                return result;\n            }\n            else {\n                var generated = listener.apply(void 0, args);\n                if (generated !== undefined) {\n                    result = generated;\n                }\n            }\n        }\n    };\n    /** Iterate over the listeners of an event */\n    EventEmitter.prototype.listeners = function (key) {\n        var list, prev, curr;\n        return __generator(this, function (_a) {\n            switch (_a.label) {\n                case 0:\n                    list = this[exports.$listeners][key];\n                    if (!list)\n                        return [2 /*return*/];\n                    prev = null;\n                    curr = list.first;\n                    _a.label = 1;\n                case 1:\n                    if (false) {}\n                    return [4 /*yield*/, curr.fn\n                        // One-time listener\n                    ];\n                case 2:\n                    _a.sent();\n                    // One-time listener\n                    if (curr.once) {\n                        // Splice it.\n                        if (prev) {\n                            prev.next = curr.next;\n                        }\n                        // Shift it.\n                        else if (curr.next) {\n                            list.first = curr = curr.next;\n                            return [3 /*break*/, 1];\n                        }\n                        // Delete it.\n                        else {\n                            delete this[exports.$listeners][key];\n                            if (this._onEventUnhandled) {\n                                this._onEventUnhandled(key);\n                            }\n                            return [2 /*return*/];\n                        }\n                    }\n                    // Recurring listener\n                    else {\n                        prev = curr;\n                    }\n                    // Continue to the next listener.\n                    if (curr.next) {\n                        curr = curr.next;\n                        return [3 /*break*/, 1];\n                    }\n                    // Update the last listener.\n                    list.last = curr;\n                    // All done.\n                    return [2 /*return*/];\n                case 3: return [2 /*return*/];\n            }\n        });\n    };\n    /** Implementation of the `on` and `one` methods */\n    EventEmitter.prototype[exports.$addListener] = function (arg, fn, disposables, once) {\n        var _this = this;\n        if (once === void 0) { once = false; }\n        if (typeof arg == 'object') {\n            var key_1;\n            var _loop_1 = function () {\n                if (typeof arg[key_1] == 'function') {\n                    var fn_1 = arg[key_1];\n                    var list = addListener(this_1[exports.$listeners], key_1, {\n                        fn: fn_1,\n                        once: once,\n                        next: null,\n                    });\n                    if (disposables) {\n                        disposables.push({\n                            dispose: function () { return _this.off(key_1, fn_1); },\n                        });\n                    }\n                    if (fn_1 == list.first.fn && this_1._onEventHandled) {\n                        this_1._onEventHandled(key_1);\n                    }\n                }\n            };\n            var this_1 = this;\n            for (key_1 in arg) {\n                _loop_1();\n            }\n            return this;\n        }\n        if (typeof fn == 'function') {\n            var key_2 = arg;\n            var list = addListener(this[exports.$listeners], key_2, {\n                fn: fn,\n                once: once,\n                next: null,\n            });\n            if (disposables) {\n                disposables.push({\n                    dispose: function () { return _this.off(key_2, fn); },\n                });\n            }\n            if (fn == list.first.fn && this._onEventHandled) {\n                this._onEventHandled(arg);\n            }\n        }\n        return fn;\n    };\n    /** Unique symbol for accessing the internal listener cache */\n    EventEmitter.ev = exports.$listeners;\n    return EventEmitter;\n}());\nexports.EventEmitter = EventEmitter;\nfunction addListener(cache, key, cb) {\n    var list = cache[key];\n    if (list) {\n        list.last.next = cb;\n        list.last = cb;\n    }\n    else {\n        cache[key] = list = { first: cb, last: cb };\n    }\n    return list;\n}\n/** Remove listeners that match the filter function */\nfunction unlink(list, filter) {\n    var prev = null;\n    var curr = list.first;\n    while (true) {\n        // Return true to unlink the listener.\n        if (filter(curr)) {\n            // Splice it.\n            if (prev) {\n                prev.next = curr.next;\n                if (curr.next) {\n                    curr = curr.next;\n                }\n                else\n                    break;\n            }\n            // Shift it.\n            else if (curr.next) {\n                list.first = curr = curr.next;\n            }\n            // No listeners remain.\n            else {\n                return null;\n            }\n        }\n        // Keep this listener.\n        else {\n            prev = curr;\n            if (curr.next) {\n                curr = curr.next;\n            }\n            else\n                break;\n        }\n    }\n    // At least one listener remains.\n    list.last = prev;\n    return list;\n}\n\n\n//# sourceURL=webpack:///./node_modules/ee-ts/lib/ee.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/index.js":
/*!**************************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/index.js ***!
  \**************************************************/
/*! exports provided: v1, v3, v4, v5, NIL, version, validate, stringify, parse */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _v1_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./v1.js */ \"./node_modules/uuid/dist/esm-node/v1.js\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"v1\", function() { return _v1_js__WEBPACK_IMPORTED_MODULE_0__[\"default\"]; });\n\n/* harmony import */ var _v3_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./v3.js */ \"./node_modules/uuid/dist/esm-node/v3.js\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"v3\", function() { return _v3_js__WEBPACK_IMPORTED_MODULE_1__[\"default\"]; });\n\n/* harmony import */ var _v4_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./v4.js */ \"./node_modules/uuid/dist/esm-node/v4.js\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"v4\", function() { return _v4_js__WEBPACK_IMPORTED_MODULE_2__[\"default\"]; });\n\n/* harmony import */ var _v5_js__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./v5.js */ \"./node_modules/uuid/dist/esm-node/v5.js\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"v5\", function() { return _v5_js__WEBPACK_IMPORTED_MODULE_3__[\"default\"]; });\n\n/* harmony import */ var _nil_js__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(/*! ./nil.js */ \"./node_modules/uuid/dist/esm-node/nil.js\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"NIL\", function() { return _nil_js__WEBPACK_IMPORTED_MODULE_4__[\"default\"]; });\n\n/* harmony import */ var _version_js__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(/*! ./version.js */ \"./node_modules/uuid/dist/esm-node/version.js\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"version\", function() { return _version_js__WEBPACK_IMPORTED_MODULE_5__[\"default\"]; });\n\n/* harmony import */ var _validate_js__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(/*! ./validate.js */ \"./node_modules/uuid/dist/esm-node/validate.js\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"validate\", function() { return _validate_js__WEBPACK_IMPORTED_MODULE_6__[\"default\"]; });\n\n/* harmony import */ var _stringify_js__WEBPACK_IMPORTED_MODULE_7__ = __webpack_require__(/*! ./stringify.js */ \"./node_modules/uuid/dist/esm-node/stringify.js\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"stringify\", function() { return _stringify_js__WEBPACK_IMPORTED_MODULE_7__[\"default\"]; });\n\n/* harmony import */ var _parse_js__WEBPACK_IMPORTED_MODULE_8__ = __webpack_require__(/*! ./parse.js */ \"./node_modules/uuid/dist/esm-node/parse.js\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"parse\", function() { return _parse_js__WEBPACK_IMPORTED_MODULE_8__[\"default\"]; });\n\n\n\n\n\n\n\n\n\n\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/index.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/md5.js":
/*!************************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/md5.js ***!
  \************************************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var crypto__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! crypto */ \"crypto\");\n/* harmony import */ var crypto__WEBPACK_IMPORTED_MODULE_0___default = /*#__PURE__*/__webpack_require__.n(crypto__WEBPACK_IMPORTED_MODULE_0__);\n\n\nfunction md5(bytes) {\n  if (Array.isArray(bytes)) {\n    bytes = Buffer.from(bytes);\n  } else if (typeof bytes === 'string') {\n    bytes = Buffer.from(bytes, 'utf8');\n  }\n\n  return crypto__WEBPACK_IMPORTED_MODULE_0___default.a.createHash('md5').update(bytes).digest();\n}\n\n/* harmony default export */ __webpack_exports__[\"default\"] = (md5);\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/md5.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/nil.js":
/*!************************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/nil.js ***!
  \************************************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony default export */ __webpack_exports__[\"default\"] = ('00000000-0000-0000-0000-000000000000');\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/nil.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/parse.js":
/*!**************************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/parse.js ***!
  \**************************************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _validate_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./validate.js */ \"./node_modules/uuid/dist/esm-node/validate.js\");\n\n\nfunction parse(uuid) {\n  if (!Object(_validate_js__WEBPACK_IMPORTED_MODULE_0__[\"default\"])(uuid)) {\n    throw TypeError('Invalid UUID');\n  }\n\n  let v;\n  const arr = new Uint8Array(16); // Parse ########-....-....-....-............\n\n  arr[0] = (v = parseInt(uuid.slice(0, 8), 16)) >>> 24;\n  arr[1] = v >>> 16 & 0xff;\n  arr[2] = v >>> 8 & 0xff;\n  arr[3] = v & 0xff; // Parse ........-####-....-....-............\n\n  arr[4] = (v = parseInt(uuid.slice(9, 13), 16)) >>> 8;\n  arr[5] = v & 0xff; // Parse ........-....-####-....-............\n\n  arr[6] = (v = parseInt(uuid.slice(14, 18), 16)) >>> 8;\n  arr[7] = v & 0xff; // Parse ........-....-....-####-............\n\n  arr[8] = (v = parseInt(uuid.slice(19, 23), 16)) >>> 8;\n  arr[9] = v & 0xff; // Parse ........-....-....-....-############\n  // (Use \"/\" to avoid 32-bit truncation when bit-shifting high-order bytes)\n\n  arr[10] = (v = parseInt(uuid.slice(24, 36), 16)) / 0x10000000000 & 0xff;\n  arr[11] = v / 0x100000000 & 0xff;\n  arr[12] = v >>> 24 & 0xff;\n  arr[13] = v >>> 16 & 0xff;\n  arr[14] = v >>> 8 & 0xff;\n  arr[15] = v & 0xff;\n  return arr;\n}\n\n/* harmony default export */ __webpack_exports__[\"default\"] = (parse);\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/parse.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/regex.js":
/*!**************************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/regex.js ***!
  \**************************************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony default export */ __webpack_exports__[\"default\"] = (/^(?:[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}|00000000-0000-0000-0000-000000000000)$/i);\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/regex.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/rng.js":
/*!************************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/rng.js ***!
  \************************************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"default\", function() { return rng; });\n/* harmony import */ var crypto__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! crypto */ \"crypto\");\n/* harmony import */ var crypto__WEBPACK_IMPORTED_MODULE_0___default = /*#__PURE__*/__webpack_require__.n(crypto__WEBPACK_IMPORTED_MODULE_0__);\n\nconst rnds8Pool = new Uint8Array(256); // # of random values to pre-allocate\n\nlet poolPtr = rnds8Pool.length;\nfunction rng() {\n  if (poolPtr > rnds8Pool.length - 16) {\n    crypto__WEBPACK_IMPORTED_MODULE_0___default.a.randomFillSync(rnds8Pool);\n    poolPtr = 0;\n  }\n\n  return rnds8Pool.slice(poolPtr, poolPtr += 16);\n}\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/rng.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/sha1.js":
/*!*************************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/sha1.js ***!
  \*************************************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var crypto__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! crypto */ \"crypto\");\n/* harmony import */ var crypto__WEBPACK_IMPORTED_MODULE_0___default = /*#__PURE__*/__webpack_require__.n(crypto__WEBPACK_IMPORTED_MODULE_0__);\n\n\nfunction sha1(bytes) {\n  if (Array.isArray(bytes)) {\n    bytes = Buffer.from(bytes);\n  } else if (typeof bytes === 'string') {\n    bytes = Buffer.from(bytes, 'utf8');\n  }\n\n  return crypto__WEBPACK_IMPORTED_MODULE_0___default.a.createHash('sha1').update(bytes).digest();\n}\n\n/* harmony default export */ __webpack_exports__[\"default\"] = (sha1);\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/sha1.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/stringify.js":
/*!******************************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/stringify.js ***!
  \******************************************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _validate_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./validate.js */ \"./node_modules/uuid/dist/esm-node/validate.js\");\n\n/**\n * Convert array of 16 byte values to UUID string format of the form:\n * XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX\n */\n\nconst byteToHex = [];\n\nfor (let i = 0; i < 256; ++i) {\n  byteToHex.push((i + 0x100).toString(16).substr(1));\n}\n\nfunction stringify(arr, offset = 0) {\n  // Note: Be careful editing this code!  It's been tuned for performance\n  // and works in ways you may not expect. See https://github.com/uuidjs/uuid/pull/434\n  const uuid = (byteToHex[arr[offset + 0]] + byteToHex[arr[offset + 1]] + byteToHex[arr[offset + 2]] + byteToHex[arr[offset + 3]] + '-' + byteToHex[arr[offset + 4]] + byteToHex[arr[offset + 5]] + '-' + byteToHex[arr[offset + 6]] + byteToHex[arr[offset + 7]] + '-' + byteToHex[arr[offset + 8]] + byteToHex[arr[offset + 9]] + '-' + byteToHex[arr[offset + 10]] + byteToHex[arr[offset + 11]] + byteToHex[arr[offset + 12]] + byteToHex[arr[offset + 13]] + byteToHex[arr[offset + 14]] + byteToHex[arr[offset + 15]]).toLowerCase(); // Consistency check for valid UUID.  If this throws, it's likely due to one\n  // of the following:\n  // - One or more input array values don't map to a hex octet (leading to\n  // \"undefined\" in the uuid)\n  // - Invalid input values for the RFC `version` or `variant` fields\n\n  if (!Object(_validate_js__WEBPACK_IMPORTED_MODULE_0__[\"default\"])(uuid)) {\n    throw TypeError('Stringified UUID is invalid');\n  }\n\n  return uuid;\n}\n\n/* harmony default export */ __webpack_exports__[\"default\"] = (stringify);\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/stringify.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/v1.js":
/*!***********************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/v1.js ***!
  \***********************************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _rng_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./rng.js */ \"./node_modules/uuid/dist/esm-node/rng.js\");\n/* harmony import */ var _stringify_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./stringify.js */ \"./node_modules/uuid/dist/esm-node/stringify.js\");\n\n // **`v1()` - Generate time-based UUID**\n//\n// Inspired by https://github.com/LiosK/UUID.js\n// and http://docs.python.org/library/uuid.html\n\nlet _nodeId;\n\nlet _clockseq; // Previous uuid creation time\n\n\nlet _lastMSecs = 0;\nlet _lastNSecs = 0; // See https://github.com/uuidjs/uuid for API details\n\nfunction v1(options, buf, offset) {\n  let i = buf && offset || 0;\n  const b = buf || new Array(16);\n  options = options || {};\n  let node = options.node || _nodeId;\n  let clockseq = options.clockseq !== undefined ? options.clockseq : _clockseq; // node and clockseq need to be initialized to random values if they're not\n  // specified.  We do this lazily to minimize issues related to insufficient\n  // system entropy.  See #189\n\n  if (node == null || clockseq == null) {\n    const seedBytes = options.random || (options.rng || _rng_js__WEBPACK_IMPORTED_MODULE_0__[\"default\"])();\n\n    if (node == null) {\n      // Per 4.5, create and 48-bit node id, (47 random bits + multicast bit = 1)\n      node = _nodeId = [seedBytes[0] | 0x01, seedBytes[1], seedBytes[2], seedBytes[3], seedBytes[4], seedBytes[5]];\n    }\n\n    if (clockseq == null) {\n      // Per 4.2.2, randomize (14 bit) clockseq\n      clockseq = _clockseq = (seedBytes[6] << 8 | seedBytes[7]) & 0x3fff;\n    }\n  } // UUID timestamps are 100 nano-second units since the Gregorian epoch,\n  // (1582-10-15 00:00).  JSNumbers aren't precise enough for this, so\n  // time is handled internally as 'msecs' (integer milliseconds) and 'nsecs'\n  // (100-nanoseconds offset from msecs) since unix epoch, 1970-01-01 00:00.\n\n\n  let msecs = options.msecs !== undefined ? options.msecs : Date.now(); // Per 4.2.1.2, use count of uuid's generated during the current clock\n  // cycle to simulate higher resolution clock\n\n  let nsecs = options.nsecs !== undefined ? options.nsecs : _lastNSecs + 1; // Time since last uuid creation (in msecs)\n\n  const dt = msecs - _lastMSecs + (nsecs - _lastNSecs) / 10000; // Per 4.2.1.2, Bump clockseq on clock regression\n\n  if (dt < 0 && options.clockseq === undefined) {\n    clockseq = clockseq + 1 & 0x3fff;\n  } // Reset nsecs if clock regresses (new clockseq) or we've moved onto a new\n  // time interval\n\n\n  if ((dt < 0 || msecs > _lastMSecs) && options.nsecs === undefined) {\n    nsecs = 0;\n  } // Per 4.2.1.2 Throw error if too many uuids are requested\n\n\n  if (nsecs >= 10000) {\n    throw new Error(\"uuid.v1(): Can't create more than 10M uuids/sec\");\n  }\n\n  _lastMSecs = msecs;\n  _lastNSecs = nsecs;\n  _clockseq = clockseq; // Per 4.1.4 - Convert from unix epoch to Gregorian epoch\n\n  msecs += 12219292800000; // `time_low`\n\n  const tl = ((msecs & 0xfffffff) * 10000 + nsecs) % 0x100000000;\n  b[i++] = tl >>> 24 & 0xff;\n  b[i++] = tl >>> 16 & 0xff;\n  b[i++] = tl >>> 8 & 0xff;\n  b[i++] = tl & 0xff; // `time_mid`\n\n  const tmh = msecs / 0x100000000 * 10000 & 0xfffffff;\n  b[i++] = tmh >>> 8 & 0xff;\n  b[i++] = tmh & 0xff; // `time_high_and_version`\n\n  b[i++] = tmh >>> 24 & 0xf | 0x10; // include version\n\n  b[i++] = tmh >>> 16 & 0xff; // `clock_seq_hi_and_reserved` (Per 4.2.2 - include variant)\n\n  b[i++] = clockseq >>> 8 | 0x80; // `clock_seq_low`\n\n  b[i++] = clockseq & 0xff; // `node`\n\n  for (let n = 0; n < 6; ++n) {\n    b[i + n] = node[n];\n  }\n\n  return buf || Object(_stringify_js__WEBPACK_IMPORTED_MODULE_1__[\"default\"])(b);\n}\n\n/* harmony default export */ __webpack_exports__[\"default\"] = (v1);\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/v1.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/v3.js":
/*!***********************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/v3.js ***!
  \***********************************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _v35_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./v35.js */ \"./node_modules/uuid/dist/esm-node/v35.js\");\n/* harmony import */ var _md5_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./md5.js */ \"./node_modules/uuid/dist/esm-node/md5.js\");\n\n\nconst v3 = Object(_v35_js__WEBPACK_IMPORTED_MODULE_0__[\"default\"])('v3', 0x30, _md5_js__WEBPACK_IMPORTED_MODULE_1__[\"default\"]);\n/* harmony default export */ __webpack_exports__[\"default\"] = (v3);\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/v3.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/v35.js":
/*!************************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/v35.js ***!
  \************************************************/
/*! exports provided: DNS, URL, default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"DNS\", function() { return DNS; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"URL\", function() { return URL; });\n/* harmony import */ var _stringify_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./stringify.js */ \"./node_modules/uuid/dist/esm-node/stringify.js\");\n/* harmony import */ var _parse_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./parse.js */ \"./node_modules/uuid/dist/esm-node/parse.js\");\n\n\n\nfunction stringToBytes(str) {\n  str = unescape(encodeURIComponent(str)); // UTF8 escape\n\n  const bytes = [];\n\n  for (let i = 0; i < str.length; ++i) {\n    bytes.push(str.charCodeAt(i));\n  }\n\n  return bytes;\n}\n\nconst DNS = '6ba7b810-9dad-11d1-80b4-00c04fd430c8';\nconst URL = '6ba7b811-9dad-11d1-80b4-00c04fd430c8';\n/* harmony default export */ __webpack_exports__[\"default\"] = (function (name, version, hashfunc) {\n  function generateUUID(value, namespace, buf, offset) {\n    if (typeof value === 'string') {\n      value = stringToBytes(value);\n    }\n\n    if (typeof namespace === 'string') {\n      namespace = Object(_parse_js__WEBPACK_IMPORTED_MODULE_1__[\"default\"])(namespace);\n    }\n\n    if (namespace.length !== 16) {\n      throw TypeError('Namespace must be array-like (16 iterable integer values, 0-255)');\n    } // Compute hash of namespace and value, Per 4.3\n    // Future: Use spread syntax when supported on all platforms, e.g. `bytes =\n    // hashfunc([...namespace, ... value])`\n\n\n    let bytes = new Uint8Array(16 + value.length);\n    bytes.set(namespace);\n    bytes.set(value, namespace.length);\n    bytes = hashfunc(bytes);\n    bytes[6] = bytes[6] & 0x0f | version;\n    bytes[8] = bytes[8] & 0x3f | 0x80;\n\n    if (buf) {\n      offset = offset || 0;\n\n      for (let i = 0; i < 16; ++i) {\n        buf[offset + i] = bytes[i];\n      }\n\n      return buf;\n    }\n\n    return Object(_stringify_js__WEBPACK_IMPORTED_MODULE_0__[\"default\"])(bytes);\n  } // Function#name is not settable on some platforms (#270)\n\n\n  try {\n    generateUUID.name = name; // eslint-disable-next-line no-empty\n  } catch (err) {} // For CommonJS default export support\n\n\n  generateUUID.DNS = DNS;\n  generateUUID.URL = URL;\n  return generateUUID;\n});\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/v35.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/v4.js":
/*!***********************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/v4.js ***!
  \***********************************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _rng_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./rng.js */ \"./node_modules/uuid/dist/esm-node/rng.js\");\n/* harmony import */ var _stringify_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./stringify.js */ \"./node_modules/uuid/dist/esm-node/stringify.js\");\n\n\n\nfunction v4(options, buf, offset) {\n  options = options || {};\n  const rnds = options.random || (options.rng || _rng_js__WEBPACK_IMPORTED_MODULE_0__[\"default\"])(); // Per 4.4, set bits for version and `clock_seq_hi_and_reserved`\n\n  rnds[6] = rnds[6] & 0x0f | 0x40;\n  rnds[8] = rnds[8] & 0x3f | 0x80; // Copy bytes to buffer, if provided\n\n  if (buf) {\n    offset = offset || 0;\n\n    for (let i = 0; i < 16; ++i) {\n      buf[offset + i] = rnds[i];\n    }\n\n    return buf;\n  }\n\n  return Object(_stringify_js__WEBPACK_IMPORTED_MODULE_1__[\"default\"])(rnds);\n}\n\n/* harmony default export */ __webpack_exports__[\"default\"] = (v4);\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/v4.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/v5.js":
/*!***********************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/v5.js ***!
  \***********************************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _v35_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./v35.js */ \"./node_modules/uuid/dist/esm-node/v35.js\");\n/* harmony import */ var _sha1_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./sha1.js */ \"./node_modules/uuid/dist/esm-node/sha1.js\");\n\n\nconst v5 = Object(_v35_js__WEBPACK_IMPORTED_MODULE_0__[\"default\"])('v5', 0x50, _sha1_js__WEBPACK_IMPORTED_MODULE_1__[\"default\"]);\n/* harmony default export */ __webpack_exports__[\"default\"] = (v5);\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/v5.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/validate.js":
/*!*****************************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/validate.js ***!
  \*****************************************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _regex_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./regex.js */ \"./node_modules/uuid/dist/esm-node/regex.js\");\n\n\nfunction validate(uuid) {\n  return typeof uuid === 'string' && _regex_js__WEBPACK_IMPORTED_MODULE_0__[\"default\"].test(uuid);\n}\n\n/* harmony default export */ __webpack_exports__[\"default\"] = (validate);\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/validate.js?");

/***/ }),

/***/ "./node_modules/uuid/dist/esm-node/version.js":
/*!****************************************************!*\
  !*** ./node_modules/uuid/dist/esm-node/version.js ***!
  \****************************************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _validate_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./validate.js */ \"./node_modules/uuid/dist/esm-node/validate.js\");\n\n\nfunction version(uuid) {\n  if (!Object(_validate_js__WEBPACK_IMPORTED_MODULE_0__[\"default\"])(uuid)) {\n    throw TypeError('Invalid UUID');\n  }\n\n  return parseInt(uuid.substr(14, 1), 16);\n}\n\n/* harmony default export */ __webpack_exports__[\"default\"] = (version);\n\n//# sourceURL=webpack:///./node_modules/uuid/dist/esm-node/version.js?");

/***/ }),

/***/ "./src/ChartClass.ts":
/*!***************************!*\
  !*** ./src/ChartClass.ts ***!
  \***************************/
/*! exports provided: ChartClass */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"ChartClass\", function() { return ChartClass; });\n/* harmony import */ var _ChartShape__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./ChartShape */ \"./src/ChartShape.ts\");\n/* harmony import */ var _PageProperty__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./PageProperty */ \"./src/PageProperty.ts\");\n/* harmony import */ var uuid__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! uuid */ \"./node_modules/uuid/dist/esm-node/index.js\");\n/* harmony import */ var _infinity__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./infinity */ \"./src/infinity.ts\");\n\n\n\n\nfunction arrayEqual(array1, array2) {\n    return array1.length === array2.length &&\n        array1.every((array1_i, i) => array1_i === array2[i]);\n}\nclass ChartClass {\n    constructor(kwargs) {\n        this._valid = true;\n        this._x_offset = 0;\n        this._y_offset = 0;\n        this._displayed = false;\n        this.edges = new Set();\n        if (!kwargs.degree) {\n            throw new TypeError(`Mandatory constructor argument \"degree\" is missing.`);\n        }\n        this.degree = kwargs.degree;\n        if (kwargs.type && kwargs.type !== this.constructor.name) {\n            throw Error(`Internal error: bad value for parameter \"type\"`);\n        }\n        this.idx = kwargs.idx;\n        this.uuid = kwargs.uuid || Object(uuid__WEBPACK_IMPORTED_MODULE_2__[\"v4\"])();\n        this.max_page = kwargs.max_page || _infinity__WEBPACK_IMPORTED_MODULE_3__[\"INFINITY\"];\n        let errorContext = \" in constructor for ChartClass.\";\n        this.name = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_1__[\"initialPagePropertyValue\"])(kwargs.name, \"\", \"name\", errorContext);\n        this.node = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_1__[\"initialPagePropertyValue\"])(kwargs.node, _ChartShape__WEBPACK_IMPORTED_MODULE_0__[\"DefaultNode\"], \"shape\", errorContext);\n        this.scale = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_1__[\"initialPagePropertyValue\"])(kwargs.scale, 1, \"scale\", errorContext);\n        this.visible = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_1__[\"initialPagePropertyValue\"])(kwargs.visible, true, \"visible\", errorContext);\n        this.x_nudge = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_1__[\"initialPagePropertyValue\"])(kwargs.x_nudge, 0, \"x_nudge\", errorContext);\n        this.y_nudge = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_1__[\"initialPagePropertyValue\"])(kwargs.y_nudge, 0, \"y_nudge\", errorContext);\n        this.dom_content = kwargs.dom_content || new Map();\n        this.user_data = kwargs.user_data || new Map();\n    }\n    _updateProjection() {\n        if (!this._sseq) {\n            throw Error(\"Undefined _sseq.\");\n        }\n        let x = 0;\n        let y = 0;\n        for (let i = 0; i < this._sseq.num_gradings; i++) {\n            x += this._sseq.x_projection[i] * this.degree[i];\n            y += this._sseq.y_projection[i] * this.degree[i];\n        }\n        this.x = x;\n        this.y = y;\n    }\n    update(kwargs) {\n        // TODO: new utils function that ensures no \"_\" fields present, raises error \"bad serialized class\".\n        if (kwargs.degree) {\n            if (!arrayEqual(this.degree, kwargs.degree)) {\n                throw TypeError(`Inconsistent values for \"degree\".`);\n            }\n        }\n        if (kwargs.type) {\n            if (kwargs.type !== this.constructor.name) {\n                throw TypeError(`Invalid value for \"type\"`);\n            }\n        }\n        if (kwargs.uuid) {\n            if (this.uuid !== kwargs.uuid) {\n                throw TypeError(`Inconsistent values for \"uuid\".`);\n            }\n        }\n        if (kwargs.idx) {\n            this.idx = kwargs.idx;\n        }\n        if (kwargs.name) {\n            this.name = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_1__[\"PagePropertyOrValueToPageProperty\"])(kwargs.name);\n        }\n        if (kwargs.max_page) {\n            this.max_page = kwargs.max_page;\n        }\n        if (kwargs.node) {\n            this.node = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_1__[\"PagePropertyOrValueToPageProperty\"])(kwargs.node);\n        }\n        if (kwargs.scale) {\n            this.scale = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_1__[\"PagePropertyOrValueToPageProperty\"])(kwargs.scale);\n        }\n        if (kwargs.visible) {\n            this.visible = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_1__[\"PagePropertyOrValueToPageProperty\"])(kwargs.visible);\n        }\n        if (kwargs.x_nudge) {\n            this.x_nudge = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_1__[\"PagePropertyOrValueToPageProperty\"])(kwargs.x_nudge);\n        }\n        if (kwargs.y_nudge) {\n            this.y_nudge = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_1__[\"PagePropertyOrValueToPageProperty\"])(kwargs.y_nudge);\n        }\n        if (kwargs.dom_content) {\n            this.dom_content = kwargs.dom_content;\n        }\n    }\n    delete() {\n        for (let e of this.edges) {\n            this._sseq.edges.delete(e.uuid);\n        }\n        this._sseq.edges.delete(this.uuid);\n    }\n    toJSON() {\n        return {\n            type: this.constructor.name,\n            degree: this.degree,\n            idx: this.idx,\n            uuid: this.uuid,\n            name: this.name,\n            max_page: this.max_page,\n            node: this.node,\n            scale: this.scale,\n            visible: this.visible,\n            x_nudge: this.x_nudge,\n            y_nudge: this.y_nudge,\n            dom_content: this.dom_content,\n            user_data: this.user_data\n        };\n    }\n    static fromJSON(obj) {\n        return new ChartClass(obj);\n    }\n    setPosition(x, y, size) {\n        if (isNaN(x) || isNaN(y) || isNaN(size)) {\n            console.error(this, x, y, size);\n            throw new TypeError(\"class.setPosition called with bad argument.\");\n        }\n        this._canvas_x = x;\n        this._canvas_y = y;\n        this._size = size;\n    }\n    drawOnPageQ(page) {\n        return page <= this.max_page && this.visible[page];\n    }\n    inRangeQ(xmin, xmax, ymin, ymax) {\n        // TODO: maybe remove the need for this guard?\n        if (this.x === undefined || this.y === undefined) {\n            throw TypeError(\"Undefined field x or y\");\n        }\n        return xmin <= this.x && this.x <= xmax && ymin <= this.y && this.y <= ymax;\n    }\n    getNameCoord(page) {\n        let tooltip = \"\";\n        let name = this.name.constructor === _PageProperty__WEBPACK_IMPORTED_MODULE_1__[\"PageProperty\"] ? this.name[page] : this.name;\n        if (name !== \"\") {\n            tooltip = `\\\\(\\\\large ${name}\\\\)&nbsp;&mdash;&nbsp;`;\n        }\n        tooltip += `(${this.x}, ${this.y})`;\n        return tooltip;\n    }\n    getTooltip(page) {\n        let tooltip = this.getNameCoord(page);\n        if (this.extra_tooltip) {\n            tooltip += this.extra_tooltip;\n        }\n        return tooltip;\n    }\n    getXOffset(page) {\n        let x_offset;\n        let classes = this._sseq.classes_in_degree(...this.degree);\n        let num_classes = classes.length;\n        if (this.idx === undefined) {\n            throw TypeError(\"Class has undefined index.\");\n        }\n        let idx = this.idx;\n        let out = (idx - (num_classes - 1) / 2) * this._sseq.offset_size;\n        if (isNaN(out)) {\n            console.error(\"Invalid offset for class:\", this);\n            x_offset = 0;\n        }\n        else {\n            x_offset = out;\n        }\n        let x_nudge = this.x_nudge[page] ? this.x_nudge[page] : 0;\n        return x_offset + x_nudge;\n    }\n    getYOffset(page) {\n        let y_offset = 0;\n        let y_nudge = this.y_nudge[page] ? this.y_nudge[page] : 0;\n        return y_offset + y_nudge;\n    }\n}\n\n\n//# sourceURL=webpack:///./src/ChartClass.ts?");

/***/ }),

/***/ "./src/ChartEdge.ts":
/*!**************************!*\
  !*** ./src/ChartEdge.ts ***!
  \**************************/
/*! exports provided: ChartEdge, ChartDifferential, ChartStructline, ChartExtension */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"ChartEdge\", function() { return ChartEdge; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"ChartDifferential\", function() { return ChartDifferential; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"ChartStructline\", function() { return ChartStructline; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"ChartExtension\", function() { return ChartExtension; });\n/* harmony import */ var _PageProperty__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./PageProperty */ \"./src/PageProperty.ts\");\n/* harmony import */ var _infinity__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./infinity */ \"./src/infinity.ts\");\n/* harmony import */ var uuid__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! uuid */ \"./node_modules/uuid/dist/esm-node/index.js\");\n\n\n\nconst DefaultLineWidth = 3;\nclass ChartEdge {\n    // arrow_type : ArrowSpecifier; // TODO??\n    constructor(kwargs) {\n        if (!kwargs.source_uuid) {\n            throw TypeError(`Missing mandatory argument \"source_uuid\"`);\n        }\n        if (!kwargs.target_uuid) {\n            throw TypeError(`Missing mandatory argument \"target_uuid\"`);\n        }\n        let errorContext = \"\";\n        this.uuid = kwargs.uuid || Object(uuid__WEBPACK_IMPORTED_MODULE_2__[\"v4\"])();\n        this._source_uuid = kwargs.source_uuid;\n        this._target_uuid = kwargs.target_uuid;\n        this.user_data = kwargs.user_data || new Map();\n    }\n    update(kwargs) {\n        if (kwargs.source_uuid) {\n            if (this._source_uuid !== kwargs.source_uuid) {\n                throw TypeError(`Inconsistent values for \"source_uuid\".`);\n            }\n        }\n        if (kwargs.target_uuid) {\n            if (this._target_uuid !== kwargs.target_uuid) {\n                throw TypeError(`Inconsistent values for \"target_uuid\".`);\n            }\n        }\n        if (kwargs.uuid) {\n            if (this.uuid !== kwargs.uuid) {\n                throw TypeError(`Inconsistent values for \"uuid\".`);\n            }\n        }\n        if (kwargs.type) {\n            if (kwargs.type !== this.constructor.name) {\n                throw TypeError(`Invalid value for \"type\"`);\n            }\n        }\n        this.user_data = this.user_data || new Map();\n    }\n    delete() {\n        var _a, _b;\n        (_a = this.source) === null || _a === void 0 ? void 0 : _a.edges.delete(this);\n        (_b = this.target) === null || _b === void 0 ? void 0 : _b.edges.delete(this);\n        this._sseq.edges.delete(this.uuid);\n    }\n    _drawOnPageQ(pageRange) {\n        throw new Error(\"This should be overridden...\");\n    }\n    toJSON() {\n        return {\n            type: this.constructor.name,\n            uuid: this.uuid,\n            source_uuid: this._source_uuid,\n            target_uuid: this._target_uuid,\n        };\n    }\n}\nclass ChartDifferential extends ChartEdge {\n    constructor(kwargs) {\n        super(kwargs);\n        this.page = kwargs.page;\n        this.start_tip = kwargs.start_tip;\n        this.end_tip = kwargs.end_tip;\n        this.bend = kwargs.bend || 0;\n        this.color = kwargs.color || \"DefaultColor\";\n        this.dash_pattern = kwargs.dash_pattern || [];\n        this.line_width = kwargs.line_width || DefaultLineWidth;\n        this.visible = kwargs.visible || true;\n    }\n    _drawOnPageQ(pageRange) {\n        return pageRange[0] === 0 || (pageRange[0] <= this.page && this.page <= pageRange[1]);\n    }\n    toJSON() {\n        let result = super.toJSON();\n        result.page = this.page;\n        return result;\n    }\n    static fromJSON(obj) {\n        return new ChartDifferential(obj);\n    }\n}\nclass ChartStructline extends ChartEdge {\n    constructor(kwargs) {\n        super(kwargs);\n        let errorContext = \"\";\n        this.visible = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_0__[\"initialPagePropertyValue\"])(kwargs.visible, true, \"visible\", errorContext);\n        this.start_tip = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_0__[\"initialPagePropertyValue\"])(kwargs.start_tip, undefined, \"start_tip\", errorContext);\n        this.end_tip = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_0__[\"initialPagePropertyValue\"])(kwargs.end_tip, undefined, \"end_tip\", errorContext);\n        this.bend = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_0__[\"initialPagePropertyValue\"])(kwargs.bend, 0, \"bend\", errorContext);\n        this.color = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_0__[\"initialPagePropertyValue\"])(kwargs.color, [0, 0, 0, 1], \"color\", errorContext);\n        this.dash_pattern = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_0__[\"initialPagePropertyValue\"])(kwargs.dash_pattern, [], \"dash_pattern\", errorContext);\n        this.line_width = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_0__[\"initialPagePropertyValue\"])(kwargs.line_width, 3, \"line_width\", errorContext);\n    }\n    update(kwargs) {\n        super.update(kwargs);\n        if (kwargs.visible) {\n            this.visible = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_0__[\"PagePropertyOrValueToPageProperty\"])(kwargs.visible);\n        }\n        if (kwargs.color) {\n            this.color = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_0__[\"PagePropertyOrValueToPageProperty\"])(kwargs.color);\n        }\n        if (kwargs.dash_pattern) {\n            this.dash_pattern = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_0__[\"PagePropertyOrValueToPageProperty\"])(kwargs.dash_pattern);\n        }\n        if (kwargs.line_width) {\n            this.line_width = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_0__[\"PagePropertyOrValueToPageProperty\"])(kwargs.line_width);\n        }\n        if (kwargs.bend) {\n            this.bend = Object(_PageProperty__WEBPACK_IMPORTED_MODULE_0__[\"PagePropertyOrValueToPageProperty\"])(kwargs.bend);\n        }\n    }\n    _drawOnPageQ(pageRange) {\n        return this.visible[pageRange[0]];\n    }\n    toJSON() {\n        let result = super.toJSON();\n        Object.assign(result, {\n            visible: this.visible,\n            color: this.color,\n            dash_pattern: this.dash_pattern,\n            line_width: this.line_width,\n            bend: this.bend,\n            user_data: this.user_data\n        });\n        return result;\n    }\n    static fromJSON(obj) {\n        return new ChartStructline(obj);\n    }\n}\nclass ChartExtension extends ChartEdge {\n    constructor(kwargs) {\n        super(kwargs);\n        this.start_tip = kwargs.start_tip;\n        this.end_tip = kwargs.end_tip;\n        this.bend = kwargs.bend || 0;\n        this.color = kwargs.color || \"DefaultColor\";\n        this.dash_pattern = kwargs.dash_pattern || [];\n        this.line_width = kwargs.line_width || DefaultLineWidth;\n        this.visible = kwargs.visible || true;\n    }\n    _drawOnPageQ(pageRange) {\n        return pageRange[0] === _infinity__WEBPACK_IMPORTED_MODULE_1__[\"INFINITY\"];\n    }\n    static fromJSON(obj) {\n        return new ChartExtension(obj);\n    }\n}\n\n\n//# sourceURL=webpack:///./src/ChartEdge.ts?");

/***/ }),

/***/ "./src/ChartShape.ts":
/*!***************************!*\
  !*** ./src/ChartShape.ts ***!
  \***************************/
/*! exports provided: DefaultNode */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"DefaultNode\", function() { return DefaultNode; });\n\n;\nconst DefaultNode = \"DefaultNode\";\n\n\n//# sourceURL=webpack:///./src/ChartShape.ts?");

/***/ }),

/***/ "./src/PageProperty.ts":
/*!*****************************!*\
  !*** ./src/PageProperty.ts ***!
  \*****************************/
/*! exports provided: PageProperty, PagePropertyOrValueToPageProperty, initialPagePropertyValue */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"PageProperty\", function() { return PageProperty; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"PagePropertyOrValueToPageProperty\", function() { return PagePropertyOrValueToPageProperty; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"initialPagePropertyValue\", function() { return initialPagePropertyValue; });\n/* harmony import */ var _infinity__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./infinity */ \"./src/infinity.ts\");\n\nclass PageProperty {\n    constructor(value) {\n        if (value instanceof Array) {\n            this.values = value;\n        }\n        else {\n            this.values = [[-_infinity__WEBPACK_IMPORTED_MODULE_0__[\"INFINITY\"], value]];\n        }\n        return new Proxy(this, {\n            get: (obj, key) => {\n                // key will either be a string or a symbol.\n                // Number(key) works fine if it's a string, but if it's a symbol it throws a type error.\n                // So first use key.toString.\n                if (Number.isInteger(Number(key.toString()))) {\n                    return obj.valueOnPage(key);\n                }\n                else {\n                    //@ts-ignore\n                    return obj[key];\n                }\n            },\n            set: (obj, key, value) => {\n                const newKey = (key || '').toString()\n                    .replace(/\\s/g, '') // Remove all whitespace.\n                    .replace(/,/g, ':'); // Replace commas with colons.\n                if (/^(-?\\d+)$/.test(newKey)) {\n                    this.setItemSingle(Number(key), value);\n                    this.mergeRedundant();\n                    return true;\n                }\n                // Handle slices.\n                if (!/^(-?\\d+)?(:(-?\\d+)?)?$/.test(newKey)) {\n                    return Reflect[\"set\"](obj, key, value);\n                }\n                let [start, stop] = newKey.split(':').map(part => part.length ? Number.parseInt(part) : undefined);\n                start = start || -_infinity__WEBPACK_IMPORTED_MODULE_0__[\"INFINITY\"];\n                stop = stop || _infinity__WEBPACK_IMPORTED_MODULE_0__[\"INFINITY\"];\n                let orig_value = this.valueOnPage(stop);\n                let [start_idx, hit_start] = this.setItemSingle(start, value);\n                let [end_idx, hit_end] = this.findIndex(stop);\n                if (!hit_end && stop < _infinity__WEBPACK_IMPORTED_MODULE_0__[\"INFINITY\"]) {\n                    [end_idx,] = this.setItemSingle(stop, orig_value);\n                }\n                if (stop == _infinity__WEBPACK_IMPORTED_MODULE_0__[\"INFINITY\"]) {\n                    end_idx++;\n                }\n                this.values.splice(start_idx + 1, end_idx - start_idx - 1);\n                this.mergeRedundant();\n                return true;\n            }\n        });\n    }\n    findIndex(target_page) {\n        let result_idx;\n        for (let idx = 0; idx < this.values.length; idx++) {\n            let [page, value] = this.values[idx];\n            if (page > target_page) {\n                break;\n            }\n            result_idx = idx;\n        }\n        return [result_idx, this.values[result_idx][0] === target_page];\n    }\n    setItemSingle(page, value) {\n        let [idx, hit] = this.findIndex(page);\n        if (hit) {\n            this.values[idx][1] = value;\n        }\n        else {\n            idx++;\n            this.values.splice(idx, 0, [page, value]);\n        }\n        return [idx, hit];\n    }\n    mergeRedundant() {\n        for (let i = this.values.length - 1; i >= 1; i--) {\n            if (this.values[i][1] === this.values[i - 1][1]) {\n                this.values.splice(i, 1);\n            }\n        }\n    }\n    toJSON() {\n        return { \"type\": \"PageProperty\", \"values\": this.values };\n    }\n    static fromJSON(obj) {\n        return new PageProperty(obj.values);\n    }\n    toString() {\n        return `PageProperty(${JSON.stringify(this.values)})`;\n    }\n    valueOnPage(target_page) {\n        let result;\n        for (let [page, v] of this.values) {\n            if (page > target_page) {\n                break;\n            }\n            result = v;\n        }\n        return result;\n    }\n}\nfunction PagePropertyOrValueToPageProperty(propertyValue) {\n    if (propertyValue instanceof PageProperty) {\n        return propertyValue;\n    }\n    else {\n        return new PageProperty(propertyValue);\n    }\n}\nfunction initialPagePropertyValue(propertyValue, defaultValue, propertyName, context) {\n    if (propertyValue) {\n        return PagePropertyOrValueToPageProperty(propertyValue);\n    }\n    else if (defaultValue !== undefined) {\n        return new PageProperty(defaultValue);\n    }\n    else {\n        throw TypeError(`Missing property ${propertyName}${context}`);\n    }\n}\n\n\n//# sourceURL=webpack:///./src/PageProperty.ts?");

/***/ }),

/***/ "./src/SseqChart.ts":
/*!**************************!*\
  !*** ./src/SseqChart.ts ***!
  \**************************/
/*! exports provided: SseqChart */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"SseqChart\", function() { return SseqChart; });\n/* harmony import */ var _StringifyingMap__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./StringifyingMap */ \"./src/StringifyingMap.ts\");\n/* harmony import */ var _ChartClass__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./ChartClass */ \"./src/ChartClass.ts\");\n/* harmony import */ var _ChartEdge__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./ChartEdge */ \"./src/ChartEdge.ts\");\n/* harmony import */ var ee_ts__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ee-ts */ \"./node_modules/ee-ts/lib/ee.js\");\n/* harmony import */ var ee_ts__WEBPACK_IMPORTED_MODULE_3___default = /*#__PURE__*/__webpack_require__.n(ee_ts__WEBPACK_IMPORTED_MODULE_3__);\n/* harmony import */ var _infinity__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(/*! ./infinity */ \"./src/infinity.ts\");\n/* harmony import */ var uuid__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(/*! uuid */ \"./node_modules/uuid/dist/esm-node/index.js\");\n\n\n\n\n\n\nfunction check_argument_is_integer(name, value) {\n    if (!Number.isInteger(value)) {\n        throw TypeError(`Argument \"${name}\" is ${value} which is not an integer. \"${name}\" is expected to be an integer.`);\n    }\n}\nclass SseqChart extends ee_ts__WEBPACK_IMPORTED_MODULE_3__[\"EventEmitter\"] {\n    constructor(kwargs) {\n        super();\n        this.name = \"\";\n        this.offset_size = 8;\n        this.min_class_size = 1;\n        this.max_class_size = 3;\n        this.class_scale = 10;\n        this.highlightScale = 2;\n        this.highlightColor = \"orange\";\n        this.bidegreeDistanceScale = 1;\n        this.mouseoverScale = 2; // How much bigger should the mouseover region be than the clas itself?\n        this.defaultClassShape = { \"type\": \"circle\" };\n        this.defaultClassScale = 1;\n        this.defaultClassStrokeColor = true;\n        this.defaultClassFillColor = true;\n        this.defaultClassColor = \"black\";\n        this.initial_x_range = [0, 10];\n        this.initial_y_range = [0, 10];\n        this.x_range = [0, 10];\n        this.y_range = [0, 10];\n        this.page_list = [[2, 2], [_infinity__WEBPACK_IMPORTED_MODULE_4__[\"INFINITY\"], _infinity__WEBPACK_IMPORTED_MODULE_4__[\"INFINITY\"]]];\n        this.num_gradings = 2;\n        this.x_projection = [1, 0];\n        this.y_projection = [0, 1];\n        this._classes_by_degree = new _StringifyingMap__WEBPACK_IMPORTED_MODULE_0__[\"default\"]();\n        this.classes = new Map();\n        this.edges = new Map();\n        this.objects = new Map();\n        this.uuid = Object(uuid__WEBPACK_IMPORTED_MODULE_5__[\"v4\"])();\n        SseqChart.charts.set(this.uuid, this);\n        if (kwargs.name) {\n            this.name = kwargs.name;\n        }\n        if (kwargs.offset_size) {\n            this.offset_size = kwargs.offset_size;\n        }\n        if (kwargs.min_class_size) {\n            this.min_class_size = kwargs.min_class_size;\n        }\n        if (kwargs.max_class_size) {\n            this.max_class_size = kwargs.max_class_size;\n        }\n        if (kwargs.highlightScale) {\n            this.highlightScale = kwargs.highlightScale;\n        }\n        if (kwargs.initial_x_range) {\n            this.initial_x_range = kwargs.initial_x_range;\n        }\n        if (kwargs.initial_y_range) {\n            this.initial_y_range = kwargs.initial_y_range;\n        }\n        if (kwargs.x_range) {\n            this.x_range = kwargs.x_range;\n        }\n        if (kwargs.y_range) {\n            this.y_range = kwargs.y_range;\n        }\n        if (kwargs.page_list) {\n            this.page_list = kwargs.page_list;\n        }\n        if (kwargs.num_gradings) {\n            this.num_gradings = kwargs.num_gradings;\n        }\n        if (kwargs.classes) {\n            for (let c of kwargs.classes) {\n                this._commit_class(c);\n            }\n        }\n        if (kwargs.edges) {\n            for (let c of kwargs.edges) {\n                this._commit_edge(c);\n            }\n        }\n    }\n    static fromJSON(json) {\n        return new SseqChart(json);\n    }\n    classes_in_degree(...args) {\n        if (args.length !== this.num_gradings) {\n            throw TypeError(`Expected this.num_gradings = ${this.num_gradings} arguments to classes_in_degree.`);\n        }\n        args.forEach((v, idx) => check_argument_is_integer(`${idx}`, v));\n        if (!this._classes_by_degree.has(args)) {\n            return [];\n        }\n        return this._classes_by_degree.get(args);\n    }\n    class_by_index(...x) {\n        if (x.length !== this.num_gradings + 1) {\n            throw TypeError(`Expected this.num_gradings + 1 = ${this.num_gradings + 1} arguments to classes_in_degree.`);\n        }\n        let idx = x.pop();\n        check_argument_is_integer(\"idx\", idx);\n        let classes = this.classes_in_degree(...x);\n        if (idx >= classes.length) {\n            throw Error(`Fewer than ${idx} classes exist in degree (${x.join(\", \")}).`);\n        }\n        return classes[idx];\n    }\n    add_class(kwargs) {\n        let c = new _ChartClass__WEBPACK_IMPORTED_MODULE_1__[\"ChartClass\"](kwargs);\n        this._commit_class(c);\n        this.emit(\"class_added\", c);\n        this.emit(\"update\");\n        return c;\n    }\n    /** Common logic between add_class and deserialization of classes. **/\n    _commit_class(c) {\n        if (c.degree.length !== this.num_gradings) {\n            throw TypeError(`Wrong number of gradings: degree {c.degree} has length {len(c.degree)} but num_gradings is {self.num_gradings}`);\n        }\n        c._sseq = this;\n        let degree = c.degree;\n        this.classes.set(c.uuid, c);\n        this.objects.set(c.uuid, c);\n        if (!this._classes_by_degree.has(degree)) {\n            this._classes_by_degree.set(degree, []);\n        }\n        // filter_dictionary_of_lists<number[], ChartClass>(this._classes_by_degree, degree, (c => c._valid) as (arg : ChartClass) => boolean);\n        if (c.idx === undefined) {\n            c.idx = this._classes_by_degree.get(degree).length;\n        }\n        this._classes_by_degree.get(c.degree).push(c);\n        c._updateProjection();\n    }\n    delete_class(c) {\n        throw Error(\"Not implemented\"); // ??\n        this.classes.delete(c.uuid);\n    }\n    delete_edge(e) {\n        if (!this.edges.has(e.uuid)) {\n            console.error(\"Failed to delete edge\", e);\n            throw Error(`Failed to delete edge!`);\n        }\n        this.edges.delete(e.uuid);\n    }\n    /** Common logic between add_structline, add_differential, add_extension, and deserialization. */\n    _commit_edge(e) {\n        e._sseq = this;\n        this.edges.set(e.uuid, e);\n        this.objects.set(e.uuid, e);\n        e.source = this.classes.get(e._source_uuid);\n        e.target = this.classes.get(e._target_uuid);\n        if (!e.source) {\n            throw Error(`No class with uuid ${e._source_uuid}`);\n        }\n        if (!e.target) {\n            throw Error(`No class with uuid ${e._target_uuid}`);\n        }\n        e.source.edges.add(e);\n        e.target.edges.add(e);\n    }\n    add_differential(kwargs) {\n        let e = new _ChartEdge__WEBPACK_IMPORTED_MODULE_2__[\"ChartDifferential\"](kwargs);\n        this._commit_edge(e);\n        this.emit(\"differential_added\", e);\n        this.emit(\"edge_added\", e);\n        this.emit(\"update\");\n        return e;\n    }\n    add_structline(kwargs) {\n        let e = new _ChartEdge__WEBPACK_IMPORTED_MODULE_2__[\"ChartStructline\"](kwargs);\n        this._commit_edge(e);\n        this.emit(\"structline_added\", e);\n        this.emit(\"edge_added\", e);\n        this.emit(\"update\");\n        return e;\n    }\n    add_extension(kwargs) {\n        let e = new _ChartEdge__WEBPACK_IMPORTED_MODULE_2__[\"ChartExtension\"](kwargs);\n        this._commit_edge(e);\n        this.emit(\"extension_added\", e);\n        this.emit(\"edge_added\", e);\n        this.emit(\"update\");\n        return e;\n    }\n    /**\n     * Gets the tooltip for the current class on the given page (currently ignores the page).\n     * @param c\n     * @param page\n     * @returns {string}\n     */\n    getClassTooltip(c, page) {\n        let tooltip = c.getNameCoord(page);\n        // let extra_info = Tooltip.toTooltipString(c.extra_info, page);\n        let extra_info = \"\";\n        if (extra_info) {\n            tooltip += extra_info;\n        }\n        return tooltip;\n    }\n    toJSON() {\n        return {\n            type: this.constructor.name,\n            name: this.name,\n            initial_x_range: this.initial_x_range,\n            initial_y_range: this.initial_y_range,\n            x_range: this.x_range,\n            y_range: this.y_range,\n            page_list: this.page_list,\n            num_gradings: this.num_gradings,\n            x_projection: this.x_projection,\n            y_projection: this.y_projection,\n            classes: Array.from(this.classes.values()),\n            edges: Array.from(this.edges.values())\n            // offset_size : number = 8;\n            // min_class_size : number = 1;\n            // max_class_size : number = 3;\n            // class_scale : number = 10;\n            // highlightScale : number = 2;\n            // highlightColor = \"orange\";\n            // bidegreeDistanceScale = 1;\n            // mouseoverScale = 2; // How much bigger should the mouseover region be than the clas itself?\n            // defaultClassShape = {\"type\" : \"circle\"};\n            // defaultClassScale = 1;\n            // defaultClassStrokeColor = true;\n            // defaultClassFillColor = true;\n            // defaultClassColor = \"black\";\n        };\n    }\n    handleMessage(msg) {\n        switch (msg.command) {\n            case \"create\":\n                if (msg.type === \"SseqClass\") {\n                    this._commit_class(msg.target);\n                }\n                else {\n                    this._commit_edge(msg.target);\n                }\n                return;\n            case \"update\": {\n                if (msg.type === \"SseqChart\") {\n                    return;\n                }\n                const target = this.objects.get(msg.target_uuid);\n                // assert(target && target.constructor.name === msg.type);\n                throw Error(\"Not implemented\");\n                // target.update(msg);\n                return;\n            }\n            case \"delete\": {\n                let target = this.objects.get(msg.target_uuid);\n                // assert(target && target.constructor.name === msg.type); // TODO: get an assert function\n                target.delete();\n            }\n        }\n    }\n}\nSseqChart.charts = new Map();\n\n\n//# sourceURL=webpack:///./src/SseqChart.ts?");

/***/ }),

/***/ "./src/StringifyingMap.ts":
/*!********************************!*\
  !*** ./src/StringifyingMap.ts ***!
  \********************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"default\", function() { return StringifyingMap; });\n\nfunction stdCatToString(x) {\n    if (x === undefined) {\n        throw TypeError(\"Argument is undefined.\");\n    }\n    if (x.getStringifyingMapKey !== undefined) {\n        return x.getStringifyingMapKey();\n    }\n    else {\n        return x.toString();\n    }\n}\nclass StringifyingMap {\n    constructor(catToString) {\n        if (catToString === undefined) {\n            catToString = stdCatToString;\n        }\n        this.catToString = catToString;\n        this.m = new Map();\n        this.key_string_to_key_object = new Map();\n    }\n    set(k, v) {\n        let key_string = this.catToString(k);\n        if (key_string === undefined) {\n            throw new Error(\"Key encoding undefined.\");\n        }\n        this.key_string_to_key_object.set(key_string, k);\n        let s = this.m.set(key_string, v);\n        return s;\n    }\n    get(k) {\n        let key_string = this.catToString(k);\n        if (key_string === undefined) {\n            return undefined;\n        }\n        return this.m.get(this.catToString(k));\n    }\n    delete(k) {\n        this.key_string_to_key_object.delete(this.catToString(k));\n        return this.m.delete(this.catToString(k));\n    }\n    has(k) {\n        if (k === undefined) {\n            return false;\n        }\n        return this.m.has(this.catToString(k));\n    }\n    getOrElse(key, value) {\n        return this.has(key) ? this.get(key) : value;\n    }\n    ;\n    [Symbol.iterator]() {\n        return function* () {\n            for (let k of this.m) {\n                yield [this.key_string_to_key_object.get(k[0]), k[1]];\n            }\n        }.bind(this)();\n    }\n    keys() {\n        return this.key_string_to_key_object.values();\n    }\n    ;\n    toJSON() {\n        return [...this];\n    }\n}\n\n\n//# sourceURL=webpack:///./src/StringifyingMap.ts?");

/***/ }),

/***/ "./src/infinity.ts":
/*!*************************!*\
  !*** ./src/infinity.ts ***!
  \*************************/
/*! exports provided: INFINITY */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"INFINITY\", function() { return INFINITY; });\nconst INFINITY = 65535;\n\n\n//# sourceURL=webpack:///./src/infinity.ts?");

/***/ }),

/***/ "./src/json_utils.ts":
/*!***************************!*\
  !*** ./src/json_utils.ts ***!
  \***************************/
/*! exports provided: parse */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"parse\", function() { return parse; });\n/* harmony import */ var _ChartClass__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./ChartClass */ \"./src/ChartClass.ts\");\n/* harmony import */ var _ChartEdge__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./ChartEdge */ \"./src/ChartEdge.ts\");\n/* harmony import */ var _PageProperty__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./PageProperty */ \"./src/PageProperty.ts\");\n/* harmony import */ var _SseqChart__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./SseqChart */ \"./src/SseqChart.ts\");\n\n\n\n\nfunction parse(json) {\n    return JSON.parse(json, parseReviver);\n}\nlet jsonTypes;\nfunction getJsonTypes() {\n    if (jsonTypes) {\n        return jsonTypes;\n    }\n    jsonTypes = new Map();\n    for (let t of [\n        _SseqChart__WEBPACK_IMPORTED_MODULE_3__[\"SseqChart\"],\n        _ChartClass__WEBPACK_IMPORTED_MODULE_0__[\"ChartClass\"], _ChartEdge__WEBPACK_IMPORTED_MODULE_1__[\"ChartStructline\"], _ChartEdge__WEBPACK_IMPORTED_MODULE_1__[\"ChartDifferential\"], _ChartEdge__WEBPACK_IMPORTED_MODULE_1__[\"ChartExtension\"],\n        _PageProperty__WEBPACK_IMPORTED_MODULE_2__[\"PageProperty\"]\n    ]) {\n        jsonTypes.set(t.name, t);\n    }\n    return jsonTypes;\n}\nfunction parseReviver(key, value) {\n    if (typeof (value) !== \"object\" || value === null || !(\"type\" in value)) {\n        return value;\n    }\n    let ty = getJsonTypes().get(value.type);\n    if (!ty) {\n        throw TypeError(`Unknown type ${value.type}`);\n    }\n    return ty.fromJSON(value);\n}\n\n\n//# sourceURL=webpack:///./src/json_utils.ts?");

/***/ }),

/***/ "./src/lib.ts":
/*!********************!*\
  !*** ./src/lib.ts ***!
  \********************/
/*! exports provided: Shapes, ChartClass, ChartEdge, ChartDifferential, ChartStructline, ChartExtension, SpectralSequenceChart, parse */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _ChartShape__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./ChartShape */ \"./src/ChartShape.ts\");\n/* harmony reexport (module object) */ __webpack_require__.d(__webpack_exports__, \"Shapes\", function() { return _ChartShape__WEBPACK_IMPORTED_MODULE_0__; });\n/* harmony import */ var _ChartClass__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./ChartClass */ \"./src/ChartClass.ts\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"ChartClass\", function() { return _ChartClass__WEBPACK_IMPORTED_MODULE_1__[\"ChartClass\"]; });\n\n/* harmony import */ var _ChartEdge__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! ./ChartEdge */ \"./src/ChartEdge.ts\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"ChartEdge\", function() { return _ChartEdge__WEBPACK_IMPORTED_MODULE_2__[\"ChartEdge\"]; });\n\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"ChartDifferential\", function() { return _ChartEdge__WEBPACK_IMPORTED_MODULE_2__[\"ChartDifferential\"]; });\n\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"ChartStructline\", function() { return _ChartEdge__WEBPACK_IMPORTED_MODULE_2__[\"ChartStructline\"]; });\n\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"ChartExtension\", function() { return _ChartEdge__WEBPACK_IMPORTED_MODULE_2__[\"ChartExtension\"]; });\n\n/* harmony import */ var _SseqChart__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! ./SseqChart */ \"./src/SseqChart.ts\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"SpectralSequenceChart\", function() { return _SseqChart__WEBPACK_IMPORTED_MODULE_3__[\"SseqChart\"]; });\n\n/* harmony import */ var _json_utils__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(/*! ./json_utils */ \"./src/json_utils.ts\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"parse\", function() { return _json_utils__WEBPACK_IMPORTED_MODULE_4__[\"parse\"]; });\n\n\n\n\n\n\n\n\n\n//# sourceURL=webpack:///./src/lib.ts?");

/***/ }),

/***/ "crypto":
/*!*************************!*\
  !*** external "crypto" ***!
  \*************************/
/*! no static exports found */
/***/ (function(module, exports) {

eval("module.exports = require(\"crypto\");\n\n//# sourceURL=webpack:///external_%22crypto%22?");

/***/ })

/******/ });