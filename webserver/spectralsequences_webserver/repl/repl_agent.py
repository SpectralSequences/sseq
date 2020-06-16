from message_passing_tree.prelude import *
from .console_io import ConsoleIO
from typing import Optional
from prompt_toolkit import HTML
from prompt_toolkit.patch_stdout import patch_stdout as patch_stdout_context

import pathlib
import logging
logger = logging.getLogger(__name__)

@subscribe_to([]) # root node.
@collect_handlers(inherit=False) # Nothing to inherit
class ReplAgent(Agent):
    def __init__(self,
        vi_mode: bool = False,
        history_filename: Optional[str] = None,
        title: Optional[str] = None
    ):
        super().__init__()
        self.executor = None
        def get_globals():
            return self.executor.get_globals()

        def get_locals():
            return self.executor.get_locals()

        # Create REPL.
        self.console_io = ConsoleIO(
            get_globals=get_globals,
            get_locals=get_locals,
            vi_mode=vi_mode,
            history_filename=history_filename,
        )
        if title:
            self.console_io.terminal_title = title
        self.patch_context : ContextManager = patch_stdout_context()

    async def start_a(self):
        with self.patch_context:
            await self.console_io.run_a()

    async def load_a(self, file):
        await self.executor.exec_file_a(pathlib.Path(file))

    def set_executor(self, executor):
        if self.console_io.executor:
            self.console_io.print_formatted_text(HTML(
                "<orange>Switching executor!!</orange>"
            ), buffered=True)
        self.executor = executor
        self.console_io.executor = executor


    @handle_inbound_messages
    async def handle__debug__a(self, envelope, msg):#source, cmd, msg):
        self.console_io.print_debug(".".join(envelope.msg.cmd.part_list[1:]), msg)

    @handle_inbound_messages
    async def handle__info__a(self, envelope, msg):
        # print("consume_info", args, kwargs)
        self.console_io.print_info(".".join(envelope.msg.cmd.part_list[1:]), msg)

    @handle_inbound_messages
    async def handle__warning__a(self, envelope, msg):
        self.console_io.print_warning(".".join(envelope.msg.cmd.part_list[1:]), msg)

    @handle_inbound_messages
    async def handle__error__exception__a(self, envelope, msg,  exception):
        # do something with cmd?
        self.console_io.print_exception(exception)

    @handle_inbound_messages
    async def handle__error__additional_info__a(self, envelope, msg, additional_info):
        self.console_io.print_error(".".join(envelope.msg.cmd.part_list[2:]), msg, additional_info)
