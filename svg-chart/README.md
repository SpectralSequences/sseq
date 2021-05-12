# svg-chart

This is an experimental minimalist chart display library. The input to the
library is an SVG containing the chart (or more precisely, an SVG group element
containing the chart). Data at bidegree (x, y) should be placed at SVG
coordinates (x, -y). The library then adds grids, axes, and pan/zoom
functionality to this SVG.

The input to the library can be obtained in multiple ways. For example, one can
construct the SVG using javascript at runtime, or obtain it via applying
pdf2svg to a pdf chart. Of course, one can also write an SVG by hand or using a
compile-time script.

# Outline
The entire library is contained in `chart.js`. The other files form an example
of how one may use the library. The library depends on d3-zoom and
d3-selection, which must be loaded into the d3 object of the global scope.
