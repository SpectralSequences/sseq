import asyncio
import os
import pathlib
import sys
import traceback

from message_passing_tree.agent import Agent

from .console_io import ConsoleIO
from .executor import Executor
from .repl_agent import ReplAgent
from .namespace import add_stuff_to_repl_namespace
from .. import utils
from .. import config

async def start_repl_a():
    r = ReplAgent(
        title = "Test",
        history_filename=str(config.USER_DIR / "repl.hist"),
    )
    REPL_NAMESPACE = globals()
    executor = Executor(REPL_NAMESPACE)
    r.set_executor(executor)

    REPL_NAMESPACE["REPL"] = r
    add_stuff_to_repl_namespace(REPL_NAMESPACE)

    set_double_fault_handler(r)
    await read_input_files_a(r)
    start_repl(r)
    return r

def set_double_fault_handler(r):
    # TODO: Is this the right logic for double_fault_handler?
    # We should probably climb to the root of our tree and use that handler?
    def double_fault_handler(self, exception):
        r.console_io.print_exception(exception)

    Agent.double_fault_handler = double_fault_handler


async def read_input_files_a(r):
    r.console_io.turn_on_buffered_stdout()
    await exec_file_if_exists_a(r.executor, config.USER_DIR / "on_repl_init.py", working_directory=config.USER_DIR)
    await handle_script_args_a(r.executor, config)
    r.console_io.turn_off_buffered_stdout()


def start_repl(r):
    def handle_task_exception(f):
        try:
            f.result()
        except Exception as e: 
            r.console_io.print_exception(e)

    task = asyncio.ensure_future(r.start_a())    
    task.add_done_callback(handle_task_exception)



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
