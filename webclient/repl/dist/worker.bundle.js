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
/******/ 	return __webpack_require__(__webpack_require__.s = "./src/pyodide.worker.js");
/******/ })
/************************************************************************/
/******/ ({

/***/ "./src/executor.py":
/*!*************************!*\
  !*** ./src/executor.py ***!
  \*************************/
/*! exports provided: default */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony default export */ __webpack_exports__[\"default\"] = (\"def ast_get_last_expression(code : str):\\r\\n    \\\"\\\"\\\" Modify code so that if the last statement is an \\\"Expr\\\" or \\\"Await\\\" statement, we return that into \\\"EXEC-LAST-EXPRESSION\\\" \\\"\\\"\\\"\\r\\n    from ast import (\\r\\n        fix_missing_locations, parse, \\r\\n        Assign, Await, Constant, Expr, Name, Store\\r\\n    )\\r\\n    tree = parse(code)\\r\\n    targets = [Name(\\\"EXEC-LAST-EXPRESSION\\\", ctx = Store())]\\r\\n    if isinstance(tree.body[-1], (Expr, Await)):\\r\\n        tree.body[-1] = Assign(targets, tree.body[-1].value)\\r\\n    else:\\r\\n        tree.body.append(Assign(targets, Constant(None, None)))\\r\\n    fix_missing_locations(tree)\\r\\n    return tree\\r\\n\\r\\n\\r\\nfrom textwrap import dedent\\r\\nimport ast\\r\\ndef eval_code(code, ns):\\r\\n    \\\"\\\"\\\"\\r\\n    Runs a string of code, the last part of which may be an expression.\\r\\n    \\\"\\\"\\\"\\r\\n    # handle mis-indented input from multi-line strings\\r\\n    code = dedent(code)\\r\\n\\r\\n    mod = ast.parse(code)\\r\\n    if len(mod.body) == 0:\\r\\n        return [None, None]\\r\\n\\r\\n    if isinstance(mod.body[-1], ast.Expr):\\r\\n        expr = ast.Expression(mod.body[-1].value)\\r\\n        del mod.body[-1]\\r\\n    else:\\r\\n        expr = None\\r\\n\\r\\n    if len(mod.body):\\r\\n        exec(compile(mod, '<exec>', mode='exec'), ns, ns)\\r\\n    if expr is not None:\\r\\n        result = eval(compile(expr, '<eval>', mode='eval'), ns, ns)\\r\\n        return [result, repr(result)]\\r\\n    else:\\r\\n        return [None, None]\\r\\n\\r\\n# def eval_code(code, ns):\\r\\n#     \\\"\\\"\\\"\\r\\n#     Runs a string of code, the last part of which may be an expression.\\r\\n#     \\\"\\\"\\\"\\r\\n#     from textwrap import dedent\\r\\n#     import ast\\r\\n#     # handle mis-indented input from multi-line strings\\r\\n#     code = dedent(code)\\r\\n\\r\\n#     mod = ast.parse(code)\\r\\n#     if len(mod.body) == 0:\\r\\n#         return None\\r\\n\\r\\n#     if isinstance(mod.body[-1], ast.Expr):\\r\\n#         expr = ast.Expression(mod.body[-1].value)\\r\\n#         del mod.body[-1]\\r\\n#     else:\\r\\n#         expr = None\\r\\n\\r\\n#     if len(mod.body):\\r\\n#         exec(compile(mod, '<exec>', mode='exec'), ns, ns)\\r\\n#     if expr is not None:\\r\\n#         return eval(compile(expr, '<eval>', mode='eval'), ns, ns)\\r\\n#     else:\\r\\n#         return None        \\r\\n# import pyodide\\r\\n# pyodide.eval_code = eval_code\");//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi9zcmMvZXhlY3V0b3IucHk/NGFkZiJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiQUFBQTtBQUFlLGkxRUFBa3hFIiwiZmlsZSI6Ii4vc3JjL2V4ZWN1dG9yLnB5LmpzIiwic291cmNlc0NvbnRlbnQiOlsiZXhwb3J0IGRlZmF1bHQgXCJkZWYgYXN0X2dldF9sYXN0X2V4cHJlc3Npb24oY29kZSA6IHN0cik6XFxyXFxuICAgIFxcXCJcXFwiXFxcIiBNb2RpZnkgY29kZSBzbyB0aGF0IGlmIHRoZSBsYXN0IHN0YXRlbWVudCBpcyBhbiBcXFwiRXhwclxcXCIgb3IgXFxcIkF3YWl0XFxcIiBzdGF0ZW1lbnQsIHdlIHJldHVybiB0aGF0IGludG8gXFxcIkVYRUMtTEFTVC1FWFBSRVNTSU9OXFxcIiBcXFwiXFxcIlxcXCJcXHJcXG4gICAgZnJvbSBhc3QgaW1wb3J0IChcXHJcXG4gICAgICAgIGZpeF9taXNzaW5nX2xvY2F0aW9ucywgcGFyc2UsIFxcclxcbiAgICAgICAgQXNzaWduLCBBd2FpdCwgQ29uc3RhbnQsIEV4cHIsIE5hbWUsIFN0b3JlXFxyXFxuICAgIClcXHJcXG4gICAgdHJlZSA9IHBhcnNlKGNvZGUpXFxyXFxuICAgIHRhcmdldHMgPSBbTmFtZShcXFwiRVhFQy1MQVNULUVYUFJFU1NJT05cXFwiLCBjdHggPSBTdG9yZSgpKV1cXHJcXG4gICAgaWYgaXNpbnN0YW5jZSh0cmVlLmJvZHlbLTFdLCAoRXhwciwgQXdhaXQpKTpcXHJcXG4gICAgICAgIHRyZWUuYm9keVstMV0gPSBBc3NpZ24odGFyZ2V0cywgdHJlZS5ib2R5Wy0xXS52YWx1ZSlcXHJcXG4gICAgZWxzZTpcXHJcXG4gICAgICAgIHRyZWUuYm9keS5hcHBlbmQoQXNzaWduKHRhcmdldHMsIENvbnN0YW50KE5vbmUsIE5vbmUpKSlcXHJcXG4gICAgZml4X21pc3NpbmdfbG9jYXRpb25zKHRyZWUpXFxyXFxuICAgIHJldHVybiB0cmVlXFxyXFxuXFxyXFxuXFxyXFxuZnJvbSB0ZXh0d3JhcCBpbXBvcnQgZGVkZW50XFxyXFxuaW1wb3J0IGFzdFxcclxcbmRlZiBldmFsX2NvZGUoY29kZSwgbnMpOlxcclxcbiAgICBcXFwiXFxcIlxcXCJcXHJcXG4gICAgUnVucyBhIHN0cmluZyBvZiBjb2RlLCB0aGUgbGFzdCBwYXJ0IG9mIHdoaWNoIG1heSBiZSBhbiBleHByZXNzaW9uLlxcclxcbiAgICBcXFwiXFxcIlxcXCJcXHJcXG4gICAgIyBoYW5kbGUgbWlzLWluZGVudGVkIGlucHV0IGZyb20gbXVsdGktbGluZSBzdHJpbmdzXFxyXFxuICAgIGNvZGUgPSBkZWRlbnQoY29kZSlcXHJcXG5cXHJcXG4gICAgbW9kID0gYXN0LnBhcnNlKGNvZGUpXFxyXFxuICAgIGlmIGxlbihtb2QuYm9keSkgPT0gMDpcXHJcXG4gICAgICAgIHJldHVybiBbTm9uZSwgTm9uZV1cXHJcXG5cXHJcXG4gICAgaWYgaXNpbnN0YW5jZShtb2QuYm9keVstMV0sIGFzdC5FeHByKTpcXHJcXG4gICAgICAgIGV4cHIgPSBhc3QuRXhwcmVzc2lvbihtb2QuYm9keVstMV0udmFsdWUpXFxyXFxuICAgICAgICBkZWwgbW9kLmJvZHlbLTFdXFxyXFxuICAgIGVsc2U6XFxyXFxuICAgICAgICBleHByID0gTm9uZVxcclxcblxcclxcbiAgICBpZiBsZW4obW9kLmJvZHkpOlxcclxcbiAgICAgICAgZXhlYyhjb21waWxlKG1vZCwgJzxleGVjPicsIG1vZGU9J2V4ZWMnKSwgbnMsIG5zKVxcclxcbiAgICBpZiBleHByIGlzIG5vdCBOb25lOlxcclxcbiAgICAgICAgcmVzdWx0ID0gZXZhbChjb21waWxlKGV4cHIsICc8ZXZhbD4nLCBtb2RlPSdldmFsJyksIG5zLCBucylcXHJcXG4gICAgICAgIHJldHVybiBbcmVzdWx0LCByZXByKHJlc3VsdCldXFxyXFxuICAgIGVsc2U6XFxyXFxuICAgICAgICByZXR1cm4gW05vbmUsIE5vbmVdXFxyXFxuXFxyXFxuIyBkZWYgZXZhbF9jb2RlKGNvZGUsIG5zKTpcXHJcXG4jICAgICBcXFwiXFxcIlxcXCJcXHJcXG4jICAgICBSdW5zIGEgc3RyaW5nIG9mIGNvZGUsIHRoZSBsYXN0IHBhcnQgb2Ygd2hpY2ggbWF5IGJlIGFuIGV4cHJlc3Npb24uXFxyXFxuIyAgICAgXFxcIlxcXCJcXFwiXFxyXFxuIyAgICAgZnJvbSB0ZXh0d3JhcCBpbXBvcnQgZGVkZW50XFxyXFxuIyAgICAgaW1wb3J0IGFzdFxcclxcbiMgICAgICMgaGFuZGxlIG1pcy1pbmRlbnRlZCBpbnB1dCBmcm9tIG11bHRpLWxpbmUgc3RyaW5nc1xcclxcbiMgICAgIGNvZGUgPSBkZWRlbnQoY29kZSlcXHJcXG5cXHJcXG4jICAgICBtb2QgPSBhc3QucGFyc2UoY29kZSlcXHJcXG4jICAgICBpZiBsZW4obW9kLmJvZHkpID09IDA6XFxyXFxuIyAgICAgICAgIHJldHVybiBOb25lXFxyXFxuXFxyXFxuIyAgICAgaWYgaXNpbnN0YW5jZShtb2QuYm9keVstMV0sIGFzdC5FeHByKTpcXHJcXG4jICAgICAgICAgZXhwciA9IGFzdC5FeHByZXNzaW9uKG1vZC5ib2R5Wy0xXS52YWx1ZSlcXHJcXG4jICAgICAgICAgZGVsIG1vZC5ib2R5Wy0xXVxcclxcbiMgICAgIGVsc2U6XFxyXFxuIyAgICAgICAgIGV4cHIgPSBOb25lXFxyXFxuXFxyXFxuIyAgICAgaWYgbGVuKG1vZC5ib2R5KTpcXHJcXG4jICAgICAgICAgZXhlYyhjb21waWxlKG1vZCwgJzxleGVjPicsIG1vZGU9J2V4ZWMnKSwgbnMsIG5zKVxcclxcbiMgICAgIGlmIGV4cHIgaXMgbm90IE5vbmU6XFxyXFxuIyAgICAgICAgIHJldHVybiBldmFsKGNvbXBpbGUoZXhwciwgJzxldmFsPicsIG1vZGU9J2V2YWwnKSwgbnMsIG5zKVxcclxcbiMgICAgIGVsc2U6XFxyXFxuIyAgICAgICAgIHJldHVybiBOb25lICAgICAgICBcXHJcXG4jIGltcG9ydCBweW9kaWRlXFxyXFxuIyBweW9kaWRlLmV2YWxfY29kZSA9IGV2YWxfY29kZVwiOyJdLCJzb3VyY2VSb290IjoiIn0=\n//# sourceURL=webpack-internal:///./src/executor.py\n");

/***/ }),

/***/ "./src/pyodide.worker.js":
/*!*******************************!*\
  !*** ./src/pyodide.worker.js ***!
  \*******************************/
/*! no exports provided */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _executor_py__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./executor.py */ \"./src/executor.py\");\nself.languagePluginUrl = '/pyodide-build-0.15.0/'\r\nimportScripts('/pyodide-build-0.15.0/pyodide.js')\r\n\r\n\r\nasync function startup(){\r\n    await languagePluginLoader;\r\n    await Promise.all([\r\n        async function(){\r\n            await pyodide.loadPackage(\"micropip\");\r\n            await self.pyodide.runPythonAsync(`\r\n                import micropip\r\n                micropip.install(\"spectralsequence_chart\")\r\n                \r\n            `)\r\n        }(),\r\n        self.pyodide.runPythonAsync(_executor_py__WEBPACK_IMPORTED_MODULE_0__[\"default\"])\r\n    ]);\r\n}\r\n\r\nlet startup_promise = startup();\r\n\r\nself.addEventListener(\"message\", async function(e) { // eslint-disable-line no-unused-vars\r\n    await startup_promise;\r\n    const data = e.data;\r\n    let id = data.id;\r\n    delete data.id;\r\n    const keys = Object.keys(data);\r\n    for (let key of keys) {\r\n      if (key !== 'python') {\r\n        // Keys other than python must be arguments for the python script.\r\n        // Set them on self, so that `from js import key` works.\r\n        self[key] = data[key];\r\n      }\r\n    }\r\n\r\n    try {\r\n        // In order to quote our input string in the most robust possible way,\r\n        // we insert it as a global variable and then read it from globals().\r\n        pyodide.globals[id] = data.python; \r\n        let [result, result_repr] = await self.pyodide.runPythonAsync(`\r\n            result = eval_code(globals()[\"${id}\"], globals())\r\n            globals().pop(\"${id}\")\r\n            result\r\n        `);\r\n        self.postMessage({result_repr, id});\r\n    } catch(err) {\r\n        console.log(err);\r\n        self.postMessage({error : err.message, id});\r\n    }\r\n        // .catch((err) => {\r\n          // if you prefer messages with the error\r\n          \r\n          // if you prefer onerror events\r\n          // setTimeout(() => { throw err; });\r\n        // });\r\n});//# sourceURL=[module]\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIndlYnBhY2s6Ly8vLi9zcmMvcHlvZGlkZS53b3JrZXIuanM/ODRkYSJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiQUFBQTtBQUFBO0FBQUE7QUFDQTtBQUMwQzs7QUFFMUM7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTs7QUFFQTtBQUNBLFNBQVM7QUFDVCxvQ0FBb0Msb0RBQWE7QUFDakQ7QUFDQTs7QUFFQTs7QUFFQSxvREFBb0Q7QUFDcEQ7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBO0FBQ0E7QUFDQTtBQUNBOztBQUVBO0FBQ0E7QUFDQTtBQUNBLDBDO0FBQ0E7QUFDQSw0Q0FBNEMsR0FBRztBQUMvQyw2QkFBNkIsR0FBRztBQUNoQztBQUNBO0FBQ0EsMEJBQTBCLGdCQUFnQjtBQUMxQyxLQUFLO0FBQ0w7QUFDQSwwQkFBMEIsd0JBQXdCO0FBQ2xEO0FBQ0E7QUFDQTs7QUFFQTtBQUNBLCtCQUErQixXQUFXLEVBQUU7QUFDNUMsWUFBWTtBQUNaLENBQUMiLCJmaWxlIjoiLi9zcmMvcHlvZGlkZS53b3JrZXIuanMuanMiLCJzb3VyY2VzQ29udGVudCI6WyJzZWxmLmxhbmd1YWdlUGx1Z2luVXJsID0gJy9weW9kaWRlLWJ1aWxkLTAuMTUuMC8nXHJcbmltcG9ydFNjcmlwdHMoJy9weW9kaWRlLWJ1aWxkLTAuMTUuMC9weW9kaWRlLmpzJylcclxuaW1wb3J0IGV4ZWN1dG9yX3RleHQgZnJvbSBcIi4vZXhlY3V0b3IucHlcIjtcclxuXHJcbmFzeW5jIGZ1bmN0aW9uIHN0YXJ0dXAoKXtcclxuICAgIGF3YWl0IGxhbmd1YWdlUGx1Z2luTG9hZGVyO1xyXG4gICAgYXdhaXQgUHJvbWlzZS5hbGwoW1xyXG4gICAgICAgIGFzeW5jIGZ1bmN0aW9uKCl7XHJcbiAgICAgICAgICAgIGF3YWl0IHB5b2RpZGUubG9hZFBhY2thZ2UoXCJtaWNyb3BpcFwiKTtcclxuICAgICAgICAgICAgYXdhaXQgc2VsZi5weW9kaWRlLnJ1blB5dGhvbkFzeW5jKGBcclxuICAgICAgICAgICAgICAgIGltcG9ydCBtaWNyb3BpcFxyXG4gICAgICAgICAgICAgICAgbWljcm9waXAuaW5zdGFsbChcInNwZWN0cmFsc2VxdWVuY2VfY2hhcnRcIilcclxuICAgICAgICAgICAgICAgIFxyXG4gICAgICAgICAgICBgKVxyXG4gICAgICAgIH0oKSxcclxuICAgICAgICBzZWxmLnB5b2RpZGUucnVuUHl0aG9uQXN5bmMoZXhlY3V0b3JfdGV4dClcclxuICAgIF0pO1xyXG59XHJcblxyXG5sZXQgc3RhcnR1cF9wcm9taXNlID0gc3RhcnR1cCgpO1xyXG5cclxuc2VsZi5hZGRFdmVudExpc3RlbmVyKFwibWVzc2FnZVwiLCBhc3luYyBmdW5jdGlvbihlKSB7IC8vIGVzbGludC1kaXNhYmxlLWxpbmUgbm8tdW51c2VkLXZhcnNcclxuICAgIGF3YWl0IHN0YXJ0dXBfcHJvbWlzZTtcclxuICAgIGNvbnN0IGRhdGEgPSBlLmRhdGE7XHJcbiAgICBsZXQgaWQgPSBkYXRhLmlkO1xyXG4gICAgZGVsZXRlIGRhdGEuaWQ7XHJcbiAgICBjb25zdCBrZXlzID0gT2JqZWN0LmtleXMoZGF0YSk7XHJcbiAgICBmb3IgKGxldCBrZXkgb2Yga2V5cykge1xyXG4gICAgICBpZiAoa2V5ICE9PSAncHl0aG9uJykge1xyXG4gICAgICAgIC8vIEtleXMgb3RoZXIgdGhhbiBweXRob24gbXVzdCBiZSBhcmd1bWVudHMgZm9yIHRoZSBweXRob24gc2NyaXB0LlxyXG4gICAgICAgIC8vIFNldCB0aGVtIG9uIHNlbGYsIHNvIHRoYXQgYGZyb20ganMgaW1wb3J0IGtleWAgd29ya3MuXHJcbiAgICAgICAgc2VsZltrZXldID0gZGF0YVtrZXldO1xyXG4gICAgICB9XHJcbiAgICB9XHJcblxyXG4gICAgdHJ5IHtcclxuICAgICAgICAvLyBJbiBvcmRlciB0byBxdW90ZSBvdXIgaW5wdXQgc3RyaW5nIGluIHRoZSBtb3N0IHJvYnVzdCBwb3NzaWJsZSB3YXksXHJcbiAgICAgICAgLy8gd2UgaW5zZXJ0IGl0IGFzIGEgZ2xvYmFsIHZhcmlhYmxlIGFuZCB0aGVuIHJlYWQgaXQgZnJvbSBnbG9iYWxzKCkuXHJcbiAgICAgICAgcHlvZGlkZS5nbG9iYWxzW2lkXSA9IGRhdGEucHl0aG9uOyBcclxuICAgICAgICBsZXQgW3Jlc3VsdCwgcmVzdWx0X3JlcHJdID0gYXdhaXQgc2VsZi5weW9kaWRlLnJ1blB5dGhvbkFzeW5jKGBcclxuICAgICAgICAgICAgcmVzdWx0ID0gZXZhbF9jb2RlKGdsb2JhbHMoKVtcIiR7aWR9XCJdLCBnbG9iYWxzKCkpXHJcbiAgICAgICAgICAgIGdsb2JhbHMoKS5wb3AoXCIke2lkfVwiKVxyXG4gICAgICAgICAgICByZXN1bHRcclxuICAgICAgICBgKTtcclxuICAgICAgICBzZWxmLnBvc3RNZXNzYWdlKHtyZXN1bHRfcmVwciwgaWR9KTtcclxuICAgIH0gY2F0Y2goZXJyKSB7XHJcbiAgICAgICAgY29uc29sZS5sb2coZXJyKTtcclxuICAgICAgICBzZWxmLnBvc3RNZXNzYWdlKHtlcnJvciA6IGVyci5tZXNzYWdlLCBpZH0pO1xyXG4gICAgfVxyXG4gICAgICAgIC8vIC5jYXRjaCgoZXJyKSA9PiB7XHJcbiAgICAgICAgICAvLyBpZiB5b3UgcHJlZmVyIG1lc3NhZ2VzIHdpdGggdGhlIGVycm9yXHJcbiAgICAgICAgICBcclxuICAgICAgICAgIC8vIGlmIHlvdSBwcmVmZXIgb25lcnJvciBldmVudHNcclxuICAgICAgICAgIC8vIHNldFRpbWVvdXQoKCkgPT4geyB0aHJvdyBlcnI7IH0pO1xyXG4gICAgICAgIC8vIH0pO1xyXG59KTsiXSwic291cmNlUm9vdCI6IiJ9\n//# sourceURL=webpack-internal:///./src/pyodide.worker.js\n");

/***/ })

/******/ });