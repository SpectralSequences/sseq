import sys
from parso import cache
import asyncio


from js import loadingMessage
from namespace import get_namespace
from sseq_display import SseqDisplay
from repl import Executor
from js_wrappers.async_js import WebLoop
from js_wrappers.messages import send_message_a

asyncio.set_event_loop(WebLoop())
namespace = get_namespace()
loadingMessage("Initializing Jedi completion engine")

import jedi # This is slow but better to do it up front.

# Prevent a common source of stack overflows
def dummied_save_to_file_system(a, b, c, cache_path):
    pass
cache._save_to_file_system = dummied_save_to_file_system
jedi.Interpreter("SseqDisplay", [namespace]).completions() # Maybe this will reduce Jedi initialization time?



executor = Executor(send_message_a,  namespace)
# from working_directory import get_working_directory_a
# async def temp():
#     d = await get_working_directory_a()
#     if not d:
#         return
#     init_path = d.path("repl_init.py")
#     if await init_path.exists_a():
#         await executor.run_a(await init_path.read_text_a(), "repl_init.py")
# executor.loop.call_soon(temp())


from js_wrappers.messages import get_message

from pyodide_interrupts import check_interrupts

def check_for_interrupt(interrupt_buffer):
    def helper():
        if interrupt_buffer() == 0:
            return
        raise KeyboardInterrupt()
    return helper

def handle_message(uuid):
    sys.setrecursionlimit(135)
    msg = get_message(uuid)
    interrupt_buffer = msg.pop("interrupt_buffer")
    with check_interrupts(check_for_interrupt(interrupt_buffer), 10_000):
        executor.handle_message(**msg)