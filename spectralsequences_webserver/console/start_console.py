import asyncio
import os
import pathlib


from .. import config
from . import repl 
from .. import utils
from .spectral_sequence import SpectralSequenceChart
from . import resolution
import rust_algebra 
from . import rust_algebra_wrappers


def add_stuff_to_console_namespace():
    config.REPL_GLOBALS["AdemAlgebra"] = rust_algebra_wrappers.AdemAlgebra
    config.REPL_GLOBALS["MilnorAlgebra"] = rust_algebra_wrappers.MilnorAlgebra
    config.REPL_GLOBALS["FDModule"] = rust_algebra.algebra.FDModule
    config.REPL_GLOBALS["Resolution"] = resolution.Resolution
    config.REPL_GLOBALS["SpectralSequenceChart"] = SpectralSequenceChart

def main():
    add_stuff_to_console_namespace()
    utils.exec_file_if_exists(config.USER_DIR / "on_console_init.py", config.REPL_GLOBALS, config.REPL_GLOBALS)
    os.chdir(config.WORKING_DIRECTORY)
    for arg in config.SCRIPT_ARGS.split():
        path = pathlib.Path(arg)
        if path.is_file():
            utils.exec_file(path, config.REPL_GLOBALS, config.REPL_GLOBALS)
        else:
            utils.print_warning(f"""Cannot find file "{arg}". Ignoring it!""")
    f = repl.make_repl(config.REPL_GLOBALS, locals(), history_filename=str(config.USER_DIR / "console.hist"))
    task = asyncio.ensure_future(f) 
    task.add_done_callback(handle_task_exception)
    add_stuff_to_console_namespace()



        
def handle_task_exception(f):
    try:
        f.result()
    except Exception as e: 
        utils.print_error("Task exception...")
        utils.print_error(str(e))
 

from .resolution import Resolution
if __name__ == "__main__": 
 
    from .spectral_sequence import SpectralSequenceChart
    from . import basic_spectral_sequence 
    from .resolution import Resolution
    main()

1