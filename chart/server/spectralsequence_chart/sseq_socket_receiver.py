import json

from message_passing_tree import SocketReceiver
from message_passing_tree.decorators import collect_transforms, subscribe_to, transform_outbound_messages
from . import utils

@subscribe_to("*")
@collect_transforms(inherit=True)
class SseqSocketReceiver(SocketReceiver):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

    @transform_outbound_messages
    async def transform__chart__a(self, envelope, **kwargs):
        envelope.mark_used()
        await self.send_message_to_socket_a(envelope)

    @transform_outbound_messages
    async def transform__display__a(self, envelope, **kwargs):
        envelope.mark_used()
        await self.send_message_to_socket_a(envelope)

    @transform_outbound_messages
    async def transform__slideshow__a(self, envelope, **kwargs):
        envelope.mark_used()
        await self.send_message_to_socket_a(envelope)    

    # def send_introduction_message(self):
    #     await self.send_message({"cmd"})
