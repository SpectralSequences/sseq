import json

from message_passing_tree import SocketReceiver
from message_passing_tree.decorators import collect_handlers, subscribe_to, handle_outbound_messages
from . import utils

@subscribe_to("*")
@collect_handlers(inherit=True)
class SseqSocketReceiver(SocketReceiver):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

    @handle_outbound_messages
    async def handle__initialize__a(self, envelope, **kwargs):
        await self.send_message_to_socket_a(envelope)

    @handle_outbound_messages
    async def handle__chart__a(self, envelope, **kwargs):
        await self.send_message_to_socket_a(envelope)

    @handle_outbound_messages
    async def handle__display__a(self, envelope, **kwargs):
        await self.send_message_to_socket_a(envelope)

    @handle_outbound_messages
    async def handle__slideshow__a(self, envelope, **kwargs):
        await self.send_message_to_socket_a(envelope)    

    # def send_introduction_message(self):
    #     await self.send_message({"cmd"})
