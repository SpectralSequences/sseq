import pathlib
import config

import asyncio
import logging
import functools
import signal

import server

from console import repl 
import utils

from console import resolution

logger = logging.getLogger("temp")
logger.setLevel(logging.INFO)

def main():
    # asyncio.get_event_loop().set_exception_handler(handle_loop_exception)
    utils.exec_file_if_exists(config.USER_DIR / "initialize_console.py", globals(), locals()) 
    f = repl.make_repl(globals(), locals(), history_filename=str(config.USER_DIR / "console.hist"))
    task = asyncio.ensure_future(f) 
    task.add_done_callback(handle_task_exception)
        
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