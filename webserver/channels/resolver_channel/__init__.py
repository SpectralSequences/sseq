import asyncio
import pathlib

from message_passing_tree.prelude import *
from message_passing_tree import SocketChannel
from message_passing_tree.utils import json_stringify
from message_passing_tree import ansi

from fastapi.templating import Jinja2Templates

from spectralsequence_chart import SseqSocketReceiver, ChartAgent, SseqChart

from spectralsequences_webserver import config
from spectralsequences_webserver.repl.executor import Executor


import os
from fastapi.staticfiles import StaticFiles

@subscribe_to("*")
@collect_handlers(inherit=True)
class ResolverChannel(SocketChannel):
    serve_as = "resolver"
    CHANNEL_DIR = pathlib.Path(__file__).parent
    templates = Jinja2Templates(directory=str(CHANNEL_DIR))

    def __init__(self, resolver, repl_agent):
        super().__init__(resolver.name)
        self.repl_agent = repl_agent
        self.resolver = resolver
        self.chart = ChartAgent(resolver.name)
        self.chart.sseq.x_range = [0, 60]
        self.chart.sseq.y_range = [0, 30]
        self.executor = Executor(repl_agent)
        self.resolver.chart = self.chart
        self.setup_executor_namespace()
        self.ready = False
        self.setup_lock = asyncio.Lock()

    def setup_executor_namespace(self):
        globals = self.executor.get_globals()
        globals["REPL"] = self.repl_agent
        globals["chart"] = self.chart
        globals["channel"] = self

    @handle_inbound_messages
    async def handle__console__take__a(self, envelope):
        self.repl_agent.set_executor(self.executor)

    @handle_inbound_messages
    async def handle__new_user__a(self, envelope):
        await self.send_message_outward_a("initialize.chart.state", *arguments(
            state=self.chart.sseq, display_state=self.chart.display_state
        ))

    async def send_start_msg_a(self):
        await self.has_parent.wait()
        serving_to = self.serving_to()
        if serving_to is not None:
            await self.send_info_a(
                "channel.opened",
                f"""Started spectral sequence "<blue>{self.name}</blue>".\n""" +\
                f"""Visit "<blue>{serving_to}</blue>" to view it."""
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
            return ResolverChannel.templates.TemplateResponse("index.html", response_data)

    async def setup_a(self):
        if not self.ready:
            async with self.setup_lock:
                if not self.ready:
                    await self.setup_main_a()
                    self.ready = True

    async def setup_main_a(self):
        await self.repl_agent.add_child_a(self.executor)
        await self.executor.add_child_a(self.resolver)
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
            await cls.channels[name].setup_a()
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

    @handle_inbound_messages
    async def handle__click__a(self, envelope, x, y, chart_class=None):
        pass # IGNORED!