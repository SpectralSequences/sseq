import pathlib
from . import repl 
import sys
from .. import config

def add_stuff_to_repl_namespace(config):
    from ..chart import SpectralSequenceChart
    import ext
    # import rust_algebra 
    config.REPL_GLOBALS["AdemAlgebra"] = ext.algebra.AdemAlgebra
    config.REPL_GLOBALS["MilnorAlgebra"] = ext.algebra.MilnorAlgebra
    config.REPL_GLOBALS["FDModule"] = ext.module.FDModule
    config.REPL_GLOBALS["Resolution"] = ext.resolution.Resolution
    config.REPL_GLOBALS["SpectralSequenceChart"] = SpectralSequenceChart

def start_repl():
    import asyncio
    from .. import utils

    add_stuff_to_repl_namespace(config)
    utils.exec_file_if_exists(config.USER_DIR / "on_repl_init.py", config.REPL_GLOBALS, config.REPL_GLOBALS)
    _handle_script_args(config)

    f = repl.make_repl(config.REPL_GLOBALS, locals(), history_filename=str(config.USER_DIR / "repl.hist"))
    task = asyncio.ensure_future(f)    
    task.add_done_callback(_handle_task_exception)

def _handle_script_args(config):
    import os
    from .. import utils

    os.chdir(config.WORKING_DIRECTORY)
    for arg in config.INPUT_FILES:
        path = pathlib.Path(arg)
        if path.is_file():
            utils.exec_file(path, config.REPL_GLOBALS, config.REPL_GLOBALS)
        else:
            utils.print_warning(f"""Cannot find file "{arg}". Ignoring it!""")

def _handle_task_exception(f):
    from .. import utils
    try:
        f.result()
    except Exception as e: 
        utils.print_error("Task exception...")
        utils.print_error(str(e)) 