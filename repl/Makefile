.PHONY: dist/index.html

dist/index.html: monaco.json node_modules
	npx webpack

monaco.json dist/editor.worker.js: library.webpack.config.js src/monaco.js node_modules
	npx webpack --config library.webpack.config.js

node_modules: package.json
	npm i
	touch -m node_modules

.PHONY: clean

clean:
	rm -rf dist monaco.json src/python_imports.js node_modules
