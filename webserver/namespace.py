import json
import pathlib

from spectralsequence_chart import ChartAgent, SseqChart
from channels import ResolverChannel
import ext

def read_file(path):
    return pathlib.Path(path).read_text()

def read_json_file(path):
    return json.loads(read_file(path))

@namespace
def get_namespace():
    return [
        read_file, read_json_file,
        ext, ext.algebra, ext.module, ext.fp, ext.fp.FpVector,
        ext.algebra.AdemAlgebra, ext.algebra.MilnorAlgebra,
        ext.module.FDModule,
        ext.resolution.Resolver,
        ChartAgent, SseqChart,
        ResolverChannel,
        pathlib, config
    ]