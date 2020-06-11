from message_passing_tree.prelude import *
from message_passing_tree import SocketChannel

from fastapi.templating import Jinja2Templates

import os
from .. import config
templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))


@subscribe_to("*")
@collect_handlers(inherit=True)
class SseqChannel(SocketChannel):
    def __init__(self, name):
        super().__init__(name)

    async def send_start_msg_a(self):
        await self.has_parent.wait()
        serving_to = self.serving_to()
        if serving_to is not None:
            await self.send_info_a(
                "channel.opened",
                f"""Started spectral sequence "<blue>{self.name}</blue>".\n""" +\
                f"""Visit "<blue>{self.serving_to()}</blue>" to view it."""
            )

    @classmethod
    def http_response(cls, channel_name, request):
        response_data = { 
            "port" : cls.port, 
            "directory" : cls.directory,
            "channel_name" : channel_name,
            "request" : request, 
        }
        if cls.has_channel(channel_name):
            return templates.TemplateResponse("sseq_chart.html", response_data)

    async def setup_a(self):
        await type(self).repl.add_child_a(self.chart)
        await self.chart.add_child_a(self)

    async def add_subscriber_a(self, websocket):
        recv = SseqSocketReceiver(websocket)
        await self.add_child_a(recv)
        await recv.start_a()

    @classmethod
    async def get_channel_a(cls, name, repl):
        if name in cls.channels:
            return cls.channels[name]

    @classmethod
    def has_channel(cls, name):
        if name in cls.channels:
            return True

    @handle_inbound_messages
    async def handle__click__a(self, envelope, x, y, chart_class=None):
        envelope.mark_used()
        pass # IGNORED!