This file documents the format for storing a spectral sequence calculation to
be read by `display/`.

A spectral sequence calculation is defined to be an ordered list of spectral
sequences with the same E_2 page, each of which is called a "step". These
represent partially computed spectral sequences. Each spectral sequence is
annotated with a list of "actions", which should be thought of as the actions
performed to get from the previous spectral sequence to the current spectral
sequence.

First of all, the data is separated into two parts --- permanent data and
changing data. Permanent data is data that remains the same throughout the
calculation, e.g. names of classes, while changing data changes with each step,
e.g. the list of differentials. The list of such fields are defined in
`interface/sseq.js`. TODO: Document these.

Each calculation file stores one set of permanent data, and many sets of
changing data, one for each step. Each piece of datum is a JSON object. Which
we stringify and then compress with zlib.

The calculation file starts with a header. The header is a 0-terminated array
of 32-bit unsigned little-endian integers, and lists the lengths of each piece
of (zlib'ed) data in bytes. After the header, the (zlib'ed) data is listed
consecutively, starting with the permanent data, then the changing data in the
order the steps are to be displayed.

At the moment, the JS library does not work correctly with big-endian machines,
since it always uses the system endianness. (That said, files produced by the
JS library on a big endian machine will work correctly on a big endian machine,
but will not work correctly with the rust compressor)
