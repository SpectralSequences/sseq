.PHONY: all chart repl

all: chart repl

chart:
	make -C chart

repl: chart
	make -C repl

.PHONY: clean clean_chart clean_repl

clean: clean_chart clean_repl

clean_chart:
	make -C chart clean

clean_repl:
	make -C repl clean
