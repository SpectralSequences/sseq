from message_passing_tree import *
from message_passing_tree.decorators import (
    collect_transforms, 
    subscribe_to, 
    transform_inbound_messages
)


import logging
logger = logging.getLogger(__name__)

@subscribe_to([]) # root node.
@collect_transforms(inherit=False) # Nothing to inherit
class ReplAgent(Agent):
    def __init__(self, repl):
        super().__init__()
        self.repl = repl

    @transform_inbound_messages
    async def consume_debug(self, source_agent_path, cmd, msg):#source, cmd, msg):
        # print("consume_info", args, kwargs)
        self.repl.print_debug(".".join(cmd.part_list[1:]), msg)

    @transform_inbound_messages
    async def consume_info(self, source_agent_path, cmd, msg):
        # print("consume_info", args, kwargs)
        self.repl.print_info(".".join(cmd.part_list[1:]), msg)

    @transform_inbound_messages
    async def consume_warning(self, source_agent_path, cmd, msg):
        self.repl.print_warning(".".join(cmd.part_list[1:]), msg)

    @transform_inbound_messages
    async def consume_error__exception(self, source_agent_path, cmd, msg,  exception):
        # do something with cmd?
        self.repl.print_exception(exception)

    @transform_inbound_messages
    async def consume_error__additional_info(self, source_agent_path, cmd, msg, additional_info):
        self.repl.print_error(".".join(cmd.part_list[2:]), msg, additional_info)
