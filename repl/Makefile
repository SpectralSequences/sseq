dist/index.html: src/python_imports.js
	npm i && npm run build

src/python_imports.js:
	python3 scripts/bundle_python_sources.py

.PHONY: clean

clean:
	rm -r dist monaco.json src/python_imports.js
