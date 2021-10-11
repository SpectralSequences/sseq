SPECTRALSEQUENCE_CHART_VERSION = 0.0.28
SPECTRALSEQUENCE_CHART_WHEEL = python/dist/spectralsequence_chart-$(SPECTRALSEQUENCE_VERSION)-py3-none-any.whl

DISPLAY_BACKEND = display_backend/pkg/sseq_display_backend.js
DISPLAY = display/dist/sseq_chart.js
JAVASCRIPT = javascript/dist/sseq_chart.js

.PHONY: all display $(DISPLAY) $(DISPLAY_BACKEND) $(SPECTRALSEQUENCE_CHART_WHEEL) $(JAVASCRIPT)

all: $(SPECTRALSEQUENCE_CHART_WHEEL) display

display: $(DISPLAY)

$(SPECTRALSEQUENCE_CHART_WHEEL):
	cd python; python3 ./setup.py bdist_wheel

$(DISPLAY_BACKEND):
	cd display_backend; wasm-pack build

$(DISPLAY): $(DISPLAY_BACKEND) $(JAVASCRIPT)
	cd display; npm i && npm run build

$(JAVASCRIPT):
	cd javascript; npm i && npm run build

.PHONY: clean clean_sseq_chart clean_display_backend clean_display clean_javascript
clean_sseq_chart:
	rm -rf python/{dist,build,spectralsequence_chart.egg-info}

clean_display_backend:
	rm -rf display_backend/{pkg,target}

clean_display:
	rm -rf display/{node_modules,dist}

clean_javascript:
	rm -rf javascript/{node_modules,dist}

clean: clean_sseq_chart clean_display_backend clean_display clean_javascript
