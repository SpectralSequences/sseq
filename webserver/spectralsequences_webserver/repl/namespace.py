import json
import pathlib
from .. import config


from spectralsequence_chart import ChartAgent, ChartData
from ..channels import ResolverChannel
import ext

def read_file(path):
    return pathlib.Path(path).read_text()

def read_json_file(path):
    return json.loads(read_file(path))

default_namespace = [
    read_file, read_json_file,
    ext, ext.algebra, ext.module, ext.fp, ext.fp.FpVector,
    ext.algebra.AdemAlgebra, ext.algebra.MilnorAlgebra,
    ext.module.FDModule,
    # ext.resolution.Resolution,
    ChartAgent, ChartData,
    ResolverChannel,
    pathlib, config
]


def add_to_namespace(namespace, obj):
    name = obj.__name__.split(".")[-1]
    namespace[name] = obj

def add_stuff_to_namespace(namespace, to_add):
    for name in to_add:
        add_to_namespace(namespace, name)