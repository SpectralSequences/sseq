# Configuration file for the Sphinx documentation builder.
#
# This file only contains a selection of the most common options. For a full
# list see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

# -- Path setup --------------------------------------------------------------

# If extensions (or modules to document with autodoc) are in another directory,
# add these directories to sys.path here. If the directory is relative to the
# documentation root, use os.path.abspath to make it absolute, like shown here.
#
import os
import sys
import textwrap
sys.path.insert(0, os.path.abspath('../..'))


from spectralsequence_chart import __version__

# -- Project information -----------------------------------------------------

project = 'spectralsequence_chart'
copyright = '2020, Hood Chatham'
author = 'Hood Chatham'

# The full version, including alpha/beta/rc tags
release = __version__


# -- General configuration ---------------------------------------------------

# Add any Sphinx extension module names here, as strings. They can be
# extensions coming with Sphinx (named 'sphinx.ext.*') or your custom
# ones.
extensions = [
    'sphinx.ext.napoleon', 
    'sphinx.ext.autodoc',
    'sphinx.ext.autosummary',
    'sphinx.ext.viewcode',
    'sphinx.ext.mathjax',
    # 'recommonmark',
    # 'sphinx_autodoc_typehints'
]


autosummary_generate = True
autodoc_typehints = 'signature'  # 'description'  # show type hints in doc body instead of signature
autoclass_content = "both" 
# html_show_sourcelink = False  # Remove 'view source code' from top of page (for html, not python)
# autodoc_inherit_docstrings = True  # If no class summary, inherit base class summary

# Add any paths that contain templates here, relative to this directory.
templates_path = ['_templates']

# List of patterns, relative to source directory, that match files and
# directories to ignore when looking for source files.
# This pattern also affects html_static_path and html_extra_path.
exclude_patterns = []


# -- Options for HTML output -------------------------------------------------

# The theme to use for HTML and HTML Help pages.  See the documentation for
# a list of builtin themes.
#
html_theme = 'sphinx_rtd_theme' # 'alabaster'

# Add any paths that contain custom static files (such as style sheets) here,
# relative to this directory. They are copied after the builtin static files,
# so a file named "default.css" will overwrite the builtin "default.css".
html_static_path = ['_static']


# Napoleon settings
napoleon_google_docstring = True
napoleon_numpy_docstring = False
# napoleon_include_init_with_doc = False
# napoleon_include_private_with_doc = False
# napoleon_include_special_with_doc = True
# napoleon_use_admonition_for_examples = False
# napoleon_use_admonition_for_notes = False
# napoleon_use_admonition_for_references = False
# napoleon_use_ivar = False
# napoleon_use_param = True
# napoleon_use_rtype = True
# napoleon_type_aliases = None


def setup(app):
    app.connect('autodoc-process-signature', get_signature_from_ast)
    app.connect('autodoc-process-docstring', autodoc_process_docstring)
    app.connect('source-read', source_read)
    app.connect('autodoc-skip-member', autodoc_skip_member_handler)
    app.add_css_file("table_wrap.css")
    app.add_js_file("index.js")



from spectralsequence_chart.chart import SseqChart 
from spectralsequence_chart.chart_class import (ChartClass, ChartClassArg, ChartClassStyle)
from spectralsequence_chart.chart_edge import (
    ChartEdgeStyle, ChartStructline, ChartDifferential, ChartExtension
)
from spectralsequence_chart.display_primitives import (
    ArrowTip, Color, Shape
)
from spectralsequence_chart.serialization import JSON
from spectralsequence_chart.page_property import PageProperty
from spectralsequence_chart.utils import format_monomial
chart = SseqChart


# Monkey patch signature cross reference producer
from docutils.nodes import Element, Node 
from docutils import nodes
from sphinx import addnodes

# This produces the cross references in the method signatures.
# I modified it so it doesn't put in annoying fully qualified type names.
def type_to_xref(text: str, env = None) -> addnodes.pending_xref:
    """Convert a type string to a cross reference node."""
    if text == 'None':
        reftype = 'obj'
    else:
        reftype = 'class'

    if env:
        kwargs = {'py:module': env.ref_context.get('py:module'),
                  'py:class': env.ref_context.get('py:class')}
    else:
        kwargs = {}
    link_text = text.split(".")[-1] # HC: This is my change
    return addnodes.pending_xref('', nodes.Text(link_text),
                        refdomain='py', reftype=reftype, reftarget=text, **kwargs)
import sphinx.domains.python
sphinx.domains.python.type_to_xref = type_to_xref


# Some Jedi helper stuff so we can look up full names and types of identifiers
import jedi
def get_inferences(qual):
    return jedi.Interpreter(qual, [globals()]).infer()

def get_inference_full_name(inf):
    if inf.name == "property":
        prop_fget = inf._name._value.access_handle.access._obj.fget
        return f"{prop_fget.__module__}.{prop_fget.__qualname__}"
    return inf.full_name


def get_inference_sphinx_type(inf):
    if inf.name == "property":
        return "attr"

    if inf.type == "function":
        if inf.parent().type == "class":
            sphinx_type = "meth"
        else:
            sphinx_type = "func"
    else:
        sphinx_type = inf.type
    return sphinx_type


import re

# Don't evaluate default options in signature
# So it will say things like "Color.BLACK" instead of "Color(0,0,0,1)"

import ast
import inspect
from astunparse import unparse
from textwrap import dedent
type_annotation_re = re.compile(": ([^=,)]*)")
def get_signature_from_ast(app, what, name, obj, options, signature,
                           return_annotation):
    if what in ('class', 'exception', 'function', 'method'):
        return signature, return_annotation

    remove_args = 0
    if what == 'method':
        remove_args += 1  # Remove self from instance methods.
    while True:
        if inspect.isclass(obj):
            obj = obj.__init__
            remove_args += 1  # Remove self from instance methods.
        elif inspect.ismethod(obj):
            remove_args += 1  # Remove self from instance methods.
            obj = obj.__func__
        elif hasattr(obj, '__wrapped__'):
            obj = obj.__wrapped__
        else:
            break
    try:
        n = ast.parse(dedent(inspect.getsource(obj)))
    except TypeError:
        return signature, return_annotation
    n = n.body[0]
    if not isinstance(n, ast.FunctionDef):
        return signature, return_annotation
    n.args.args = n.args.args[remove_args:]
    for arg in n.args.args:
        if not arg.annotation:
            continue
        for node in ast.walk(arg.annotation):
            if not isinstance(node, ast.Name):
                continue
            infs = get_inferences(node.id)
            if not infs:
                continue
            inf = infs[0]
            if not inf.module_name.startswith(project):
                continue
            node.id = inf.full_name

    signature = '(' + unparse(n.args).strip() + ')'
    if n.returns:
        return_annotation = unparse(n.returns)
    return signature, return_annotation



# Subsitute `SseqChart.add_class` ==> :meth:`SseqChart.add_class`
# Substitute `chart.add_class <SseqChart.add_class>` ==> :meth:`chart.add_class <SseqChart.add_class>`
import re


def source_read(app, docname, source):
    if docname.startswith("_autosummary"):
        return
    lines = source[0].split("\n")
    make_sphinx_links(lines)
    source[0] = "\n".join(lines)

def autodoc_process_docstring(app, what, name, obj, options, lines):
    make_sphinx_links(lines)

link_regex = re.compile(r"(?<!:)`([^<>`]*)(?: <([_.A-Za-z]*)>)?`")
def link_replacer(match):
    # print(f"---- {match.group(0)}")
    text = match.group(1)
    qual = match.group(2) or text
    # print(f"--- qual: {qual}")
    inferences = get_inferences(qual)
    if not inferences:
        return match.group(0)
    full_name = get_inference_full_name(inferences[0])
    sphinx_type = get_inference_sphinx_type(inferences[0])

    # print(f" === :{sphinx_type}:`{text} <{full_name}>`")
    return f":{sphinx_type}:`{text} <{full_name}>`"

inline_math_regex = re.compile(r"(?<!\\)\$([^$]*)(?<!\\)\$")
def inline_math_replacer(match):
    math = match.group(1)
    return f":math:`{math}`"

display_math_regex = re.compile(r"\\\[([^$]*)\\\]")
def display_math_replacer(match):
    math = match.group(1)
    math = textwrap.indent(math, "   ")
    return f".. math::\n\n`{math}`"

def make_sphinx_links(lines):
    for i in range(len(lines)):
        lines[i] = link_regex.sub(link_replacer, lines[i])
        lines[i] = inline_math_regex.sub(inline_math_replacer, lines[i])
        lines[i] = display_math_regex.sub(display_math_replacer, lines[i])

def autodoc_skip_member_handler(app, what, name, obj, skip, options):
    return name.endswith("from_json") or name.endswith("to_json") or name.startswith("_")