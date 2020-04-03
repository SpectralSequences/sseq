import asyncio
from datetime import datetime
import pathlib

from message_passing_tree.prelude import *
from message_passing_tree import SocketChannel
from message_passing_tree.utils import json_stringify
from message_passing_tree import ansi


from spectralsequence_chart import SseqSocketReceiver, InteractiveChart

from ..repl.executor import Executor
from .. import config

from fastapi.templating import Jinja2Templates
templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))


@subscribe_to("*")
@collect_transforms(inherit=True)
class InteractChannel(SocketChannel):
    def __init__(self, name, repl_agent):
        super().__init__(name)
        self.repl_agent = repl_agent
        self.executor = Executor()
        self.chart = InteractiveChart(name)
        self.setup_executor_namespace()

    channels = {}
    async def send_start_msg_a(self):
        pass

    async def setup_a(self):
        await self.repl_agent.add_child_a(self.executor)
        await self.executor.add_child_a(self.chart)
        await self.chart.add_child_a(self)
        self.chart._interact_source = None
        await self.executor.load_repl_init_file_if_it_exists()
        
    @transform_inbound_messages
    async def consume_console__take_a(self, source_agent_path, cmd):
        self.repl_agent.set_executor(self.executor)

    async def add_subscriber_a(self, websocket):
        recv = SseqSocketReceiver(websocket)
        await self.add_child_a(recv)
        await recv.start_a()

    def setup_executor_namespace(self):
        globals = self.executor.get_globals()
        globals["REPL"] = self.repl_agent
        globals["chart"] = self.chart
        globals["channel"] = self

    async def load_from_file(self):
        files = sorted(config.SAVE_DIR.glob(f"{self.name}_*.json"))
        file = files[-1]
        print(ansi.success("Loading from file " + str(file)))
        self.chart.load_json(file.read_text())
        await self.chart.update_a()

    @classmethod
    async def get_channel_a(cls, name, repl):
        if name in cls.channels:
            return cls.channels[name]
        new_channel = InteractChannel(name, repl)
        await new_channel.load_from_file()
        await new_channel.setup_a()
        return new_channel

    @transform_inbound_messages
    async def consume_io__save_a(self, source_agent_path, cmd):
        save_str = json_stringify(self.chart.data)
        iso_time = datetime.now().replace(microsecond=0).isoformat().replace(":", "-")
        out_path = config.SAVE_DIR / f"{self.name}_{iso_time}.json"
        print(ansi.success("Saving to " + str(out_path)))
        out_path.write_text(save_str)

    @classmethod
    def has_channel(cls, name):
        return True #name in cls.channels or cls.get_file_path(name)

    @classmethod
    def http_response(cls, channel_name, request):
        response_data = { 
            "port" : cls.port, 
            "directory" : cls.directory,
            "channel_name" : channel_name,
            "request" : request, 
        }
        if cls.has_channel(channel_name):
            return templates.TemplateResponse("interact.html", response_data)
