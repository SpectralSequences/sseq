import asyncio
import os
import pathlib
import sys
import traceback

from message_passing_tree.agent import Agent

from . import repl
from .repl_agent import ReplAgent
from .namespace import add_stuff_to_repl_namespace
from .. import utils
from .. import config

put_repl_here = {}
repl_configured_event = asyncio.Event()
async def start_repl_a():
    f = repl.make_repl_a(
        globals(),
        locals(),
        history_filename=str(config.USER_DIR / "repl.hist"),
        configure_a=configure_repl_a
    )
    task = asyncio.ensure_future(f)
    task.add_done_callback(handle_task_exception)
    await repl_configured_event.wait()
    return put_repl_here["REPL"]

async def configure_repl_a(r):
    put_repl_here["REPL"] = r
    REPL_NAMESPACE = r.get_globals()
    REPL_AGENT = ReplAgent(r)
    REPL_NAMESPACE["REPL_AGENT"] = REPL_AGENT
    REPL_NAMESPACE["REPL"] = r
    add_stuff_to_repl_namespace(REPL_NAMESPACE)
    r.turn_on_buffered_stdout()
    await exec_file_if_exists_a(r, config.USER_DIR / "on_repl_init.py", working_directory=config.USER_DIR)
    await handle_script_args_a(r, config)
    repl_configured_event.set()

def get_repl():
    return put_repl_here["REPL"]

# TODO: Is this the right logic for double_fault_handler?
# We should probably climb to the root of our tree and use that handler?
def double_fault_handler(self, exception):
    get_repl().print_exception(exception)

Agent.double_fault_handler = double_fault_handler

    
    # r.turn_off_buffered_stdout()

async def handle_script_args_a(r, config):
    os.chdir(config.WORKING_DIRECTORY)
    for arg in config.INPUT_FILES:
        path = pathlib.Path(arg)
        if path.is_file():
            await exec_file(r, path)
        else:
            utils.print_warning(f"""Cannot find file "{arg}". Ignoring it!""")

async def exec_file_a(r, path : pathlib.Path, working_directory=None):
    await r.exec_file_a(path, working_directory)

async def exec_file_if_exists_a(r, path : pathlib.Path, working_directory=None):
    if path.is_file():
        await exec_file_a(r, path, working_directory)


def handle_task_exception(f):
    try:
        f.result()
    except Exception as e: 
        REPL.print_exception(e)