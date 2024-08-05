# History file format

This file documents the format for storing a spectral sequence calculation to
be read by `display/`. Such a file will be known as a history file.

A spectral sequence calculation is defined to be an ordered list of spectral
sequences with the same E_2 page, each of which is called a "step". These
represent partially computed spectral sequences. Each spectral sequence is
annotated with a list of "actions", which should be thought of as the actions
performed to get from the previous spectral sequence to the current spectral
sequence.

A history file starts with a header, followed by some data. The data is a
concatenation of zlib'ed strings, each of which is called a segment.

The header is a 0-terminated array of 32-bit unsigned little-endian integers,
and lists the lengths of each segment in bytes. This list of lengths is then
used to read the data.

The (decompressed) segments in the history file are as follows:

* The first segment contains "permanent data". This is data of the spectral
  sequence that remains the same throughout the calculation, e.g. names of
  classes. This is stored as a stringified JSON.

* The second segment is a list of actions. It is stored as a newline-separated
  list of stringified JSON, and the ith entry of the list parses to the list
  of actions of the ith step.

* Afterwards, every segment contains data about a step of the calculation.
  These data are known as "changing data", as the data changes with each step.
  For example, this includes the list of differentials. The segments are
  listed in the same order as the steps.

At the moment, the JS library does not work correctly with big-endian machines,
since it always uses the system endianness. (That said, files produced by the
JS library on a big endian machine will work correctly on a big endian machine,
but will not work correctly with the rust compressor)
