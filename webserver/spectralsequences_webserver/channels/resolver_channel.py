from message_passing_tree.prelude import *
from message_passing_tree import SocketChannel
from message_passing_tree.utils import json_stringify
from message_passing_tree import ansi

from fastapi.templating import Jinja2Templates

from spectralsequence_chart import SseqSocketReceiver, SpectralSequenceChart

import os
from .. import config
templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))
print("templates", templates)

@subscribe_to("*")
@collect_transforms(inherit=True)
class ResolverChannel(SocketChannel):
    def __init__(self, resolver):
        super().__init__(resolver.name)
        self.resolver = resolver
        self.chart = SpectralSequenceChart(resolver.name)
        self.chart.data.x_range = [0, 60]
        self.chart.data.y_range = [0, 30]
        self.resolver.chart = self.chart

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
        await type(self).repl.add_child_a(self.resolver)
        await self.resolver.add_child_a(self.chart)
        await self.chart.add_child_a(self)

    async def add_subscriber_a(self, websocket):
        recv = SseqSocketReceiver(websocket)
        await self.add_child_a(recv)
        await recv.start_a()

    @classmethod
    async def get_channel_a(cls, name, repl):
        print("Resolver get_channel", name)
        if name in cls.channels:
            return cls.channels[name]

    @classmethod
    def has_channel(cls, name):
        if name in cls.channels:
            return True
    
    def save(self, name):
        save_str = json_stringify(self.chart.data)
        # iso_time = datetime.now().replace(microsecond=0).isoformat().replace(":", "-")
        out_path = config.SAVE_DIR / f"{name}.json"
        print(ansi.success("Saving to " + str(out_path)))
        out_path.write_text(save_str)

    @transform_inbound_messages
    async def consume_click_a(self, source_agent_path, cmd, x, y, chart_class=None):
        pass # IGNORED!