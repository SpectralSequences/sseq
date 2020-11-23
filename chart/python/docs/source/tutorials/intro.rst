Introduction to the spectralsequences Repl
==========================================

Basic Example
_____________

First create a ``SseqDisplay``:

>>> disp = SseqDisplay("my_chart")
SseqDisplay(name="my_chart", state="Not started, run 'await display.start_a()' to start.")

We can start the display:

>>> await disp.start_a()
Display started. Visit "http://localhost:8100/dist/charts/my_chart" to view.

