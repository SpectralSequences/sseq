import asyncio
import os
import pathlib
import sys

from . import repl 
from .namespace import add_stuff_to_repl_namespace
from .. import utils
from .. import config

def start_repl():
    f = repl.make_repl(
        globals(),
        locals(),
        history_filename=str(config.USER_DIR / "repl.hist"),
        configure=configure_repl
    )
    task = asyncio.ensure_future(f)    
    task.add_done_callback(_handle_task_exception)

REPL=None
def get_repl():
    return REPL

def configure_repl(r):
    global REPL
    REPL = r
    add_stuff_to_repl_namespace(r.get_globals())
    asyncio.ensure_future(turn_on_buffered_stdout())
    exec_file_if_exists(r, config.USER_DIR / "on_repl_init.py", working_directory=config.USER_DIR)
    _handle_script_args(r, config)
    asyncio.ensure_future(turn_off_buffered_stdout())

async def turn_on_buffered_stdout():
    REPL.BUFFER_STDOUT=True

async def turn_off_buffered_stdout():
    REPL.BUFFER_STDOUT=False

def _handle_script_args(r, config):
    os.chdir(config.WORKING_DIRECTORY)
    for arg in config.INPUT_FILES:
        path = pathlib.Path(arg)
        if path.is_file():
            exec_file(r, path)
        else:
            utils.print_warning(f"""Cannot find file "{arg}". Ignoring it!""")

def exec_file(r, path : pathlib.Path, working_directory=None):
    f = asyncio.ensure_future(r.exec_file(path, working_directory))
    f.add_done_callback(_handle_input_file_exception)

def exec_file_if_exists(r, path : pathlib.Path, working_directory=None):
    if path.is_file():
        exec_file(r, path, working_directory)

def _handle_input_file_exception(f):
    try:
        f.result()
    except Exception as e: 
        REPL.print_error("Exception while processing input file:")
        REPL.print_error(str(e))

def _handle_task_exception(f):
    try:
        f.result()
    except Exception as e: 
        REPL.print_error("Task exception...")
        REPL.print_error(str(e))