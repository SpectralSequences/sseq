import json

from message_passing_tree import SocketReceiver
from message_passing_tree.decorators import collect_transforms, subscribe_to, transform_outbound_messages
from . import utils

@subscribe_to("*")
@collect_transforms(inherit=True)
class SseqSocketReceiver(SocketReceiver):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

    # @transform_outbound_messages
    # async def consume_chart(self, source_agent_id, cmd, state):
    #     # utils.replace_keys(state, [
    #     #     ["x_range", "xRange"], 
    #     #     ["y_range", "yRange"],
    #     #     ["initial_x_range", "initialxRange"],
    #     #     ["initial_y_range", "initialyRange"]
    #     # ])
    #     await self.send_message_to_socket_a(cmd, state=state)

    @transform_outbound_messages
    async def consume_chart_a(self, source_agent_id, cmd, **kwargs):
        await self.send_message_to_socket_a(cmd, **kwargs)

    @transform_outbound_messages
    async def consume_display_a(self, source_agent_id, cmd, **kwargs):
        await self.send_message_to_socket_a(cmd, **kwargs)

    # def send_introduction_message(self):
    #     await self.send_message({"cmd"})
