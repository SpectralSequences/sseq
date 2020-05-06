import asyncio
import os
import pathlib
import sys
import traceback

from message_passing_tree.agent import Agent

from .console_io import ConsoleIO
from .executor import Executor
from .repl_agent import ReplAgent
from .. import utils
from .. import config

async def start_repl_a():
    repl = ReplAgent(
        title = "Test",
        history_filename=str(config.USER_DIR / "repl.hist"),
    )
    executor = Executor(repl)
    repl.set_executor(executor)

    set_double_fault_handler(repl)
    await read_input_files_a(repl)
    start_repl(repl)
    return repl

def set_double_fault_handler(r):
    # TODO: Is this the right logic for double_fault_handler?
    # We should probably climb to the root of our tree and use that handler?
    def double_fault_handler(self, exception):
        r.console_io.print_exception(exception)

    Agent.double_fault_handler = double_fault_handler


async def read_input_files_a(r):
    r.console_io.turn_on_buffered_stdout()
    await r.executor.load_repl_init_file_if_it_exists_a()
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
            await r.exec_file_a(path)
        else:
            utils.print_warning(f"""Cannot find file "{arg}". Ignoring it!""")
