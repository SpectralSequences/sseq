SSEQ_CHART_WHL = spectralsequence_chart-0.0.28-py3-none-any.whl

# List of files to be copied from src/ to dist/
COPY = dist/index.html dist/charts/chart.html dist/charts/nonexistent-chart.html
.PHONY: dist/index.bundle.js

all: dist/index.bundle.js dist/editor.worker.js $(COPY) dist/$(SSEQ_CHART_WHL)

dist/index.bundle.js: monaco.json node_modules src/python_imports.js
	npx webpack

monaco.json dist/editor.worker.js: library.webpack.config.js src/monaco.js node_modules
	npx webpack --config library.webpack.config.js

node_modules: package.json
	npm i
	touch -m node_modules

# This does pattern matching over the items in $(COPY). $< is the source file
# and $@ is the target file.
$(COPY): dist/%: src/%
	cp $< $@

dist/$(SSEQ_CHART_WHL): ../chart/python/dist/$(SSEQ_CHART_WHL)
	cp $< $@

src/python_imports.js: scripts/bundle_python_sources.py $(wildcard src/python/**/*)
	python3 scripts/bundle_python_sources.py

.PHONY: clean

clean:
	rm -rf dist monaco.json src/python_imports.js node_modules
