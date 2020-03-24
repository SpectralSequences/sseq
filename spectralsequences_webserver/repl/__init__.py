import asyncio
import os
import pathlib
import sys

from . import repl 
from .. import utils
from .. import config

def add_stuff_to_repl_namespace(repl):
    from ..chart import SpectralSequenceChart
    import ext
    repl_globals = repl.get_globals()
    repl_globals["ext"] = ext
    repl_globals["algebra"] = ext.algebra
    repl_globals["module"] = ext.module
    repl_globals["AdemAlgebra"] = ext.algebra.AdemAlgebra
    repl_globals["MilnorAlgebra"] = ext.algebra.MilnorAlgebra
    repl_globals["FDModule"] = ext.module.FDModule
    repl_globals["Resolution"] = ext.resolution.Resolution
    repl_globals["SpectralSequenceChart"] = SpectralSequenceChart

def start_repl():
    f = repl.make_repl(
        globals(),
        locals(),
        history_filename=str(config.USER_DIR / "repl.hist"),
        configure=configure_repl
    )
    task = asyncio.ensure_future(f)    
    task.add_done_callback(_handle_task_exception)

def configure_repl(r):
    add_stuff_to_repl_namespace(r)
    exec_file_if_exists(r, config.USER_DIR / "on_repl_init.py")
    _handle_script_args(r, config)

def _handle_script_args(r, config):
    os.chdir(config.WORKING_DIRECTORY)
    for arg in config.INPUT_FILES:
        path = pathlib.Path(arg)
        if path.is_file():
            exec_file(r, path)
        else:
            utils.print_warning(f"""Cannot find file "{arg}". Ignoring it!""")

def exec_file(r, path : pathlib.PosixPath):
    f = asyncio.ensure_future(r.exec_file(path))
    f.add_done_callback(_handle_input_file_exception)

def exec_file_if_exists(r, path):
    if path.is_file():
        exec_file(r, path)

def _handle_input_file_exception(f):
    try:
        f.result()
    except Exception as e: 
        utils.print_error("Exception while processing input file:")
        utils.print_error(str(e))

def _handle_task_exception(f):
    try:
        f.result()
    except Exception as e: 
        utils.print_error("Task exception...")
        utils.print_error(str(e)) 