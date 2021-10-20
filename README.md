The Spectral Sequences Project
==============================

This is a monorepo containing various projects:

1. `ext`
A general library to work with Ext over an Fp algebra. More generally, it
allows us to work compute in the derived category of said algebra. The primary
purpose is to compute the classical Adams E2 page by computing Ext over the
Steenrod algebra.

2. `web_ext`
Web interfaces to `ext`. There are two subprojects at the moment:

 - `sseq_gui`: A GUI to work with the Adams spectral sequence. Given a
   Steenrod module, this computes its Ext and displays the associated Adams
   spectral sequence. The user can then interactively input differentials and
   the program can propagate differentials via the Leibniz rule.

   This can be tried out at https://spectralsequences.github.io/sseq/ which
   does not require installation.

 - `steenrod_calculator`: This is a simple user interface to compute sums and
   products in the Steenrod algebra and express the result in either the Adem
   or Milnor basis.

   This is available at
   https://spectralsequences.github.io/steenrod_calculator/ .

3. `python_ext`
WIP python bindings for the `ext` library.

4. `chart`

A general spectral sequence web interface, with a python-based repl for
programmatic interaction.
