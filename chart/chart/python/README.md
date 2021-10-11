# spectralsequence_chart
This project is a Python implementation of the spectralsequence chart API.
The Python chart API is a work in progress, and in particular there is currently no published spec.
Hopefully I will make one soon.

The script packager.py packages the library into an emscripten virtual file system, reverse engineered from [https://github.com/emscripten-core/emscripten/blob/master/tools/file_packager.py](https://github.com/emscripten-core/emscripten/blob/master/tools/file_packager.py)

Changelog:
==========
## [0.0.4]
### Fixed:
- Repr of ChartClassArg

## [0.0.4]
### Fixed:
- reprs of SseqChart, ChartClass, ChartEdge

## [0.0.3]
### Fixed:
- Deserialization didn't work correctly.

### Added:
- Doc strings for a lot of the main functions.

### Changed:
- Many private methods in SseqChart have had underscores added in front of their names.

## [0.0.2]
### Fixed: 
- Removed whisker operator for compatibility with Python versions before 3.8

## [0.0.1] (2020-07-15)
