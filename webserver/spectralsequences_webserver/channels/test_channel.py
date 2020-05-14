import asyncio
from datetime import datetime
from multiprocessing import Process
import pathlib

from message_passing_tree.prelude import *
from message_passing_tree import SocketChannel
from message_passing_tree.utils import json_stringify
from message_passing_tree import ansi


from spectralsequence_chart import SseqSocketReceiver, InteractiveChartAgent

from ..repl.executor import Executor
from .. import config

from ..process_overlay import process_overlay


from fastapi.templating import Jinja2Templates
templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))


@subscribe_to("*")
@collect_transforms(inherit=True)
class TestChannel(SocketChannel):
    def __init__(self, name, repl_agent):
        super().__init__(name)
        self.repl_agent = repl_agent
        self.executor = Executor(repl_agent)
        self.chart = InteractiveChart(name)
        self.setup_executor_namespace()
        self.last_screenshot = None

    channels = {}
    async def send_start_msg_a(self):
        pass

    async def setup_a(self):
        await self.repl_agent.add_child_a(self.executor)
        await self.executor.add_child_a(self.chart)
        await self.chart.add_child_a(self)
        self.chart._interact_source = None
        await self.executor.load_repl_init_file_if_it_exists_a()
        
    @transform_inbound_messages
    async def transform__console__take__a(self, envelope):
        envelope.mark_used()
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

    async def load_from_file_a(self):
        return await self.load_from_old_file_a(-1)

    async def load_from_old_file_a(self, idx):
        files = sorted(config.SAVE_DIR.glob(f"{self.name}_*.json"))
        if not files:
            return False
        file = files[idx]
        print(ansi.success("Loading from file " + str(file)))
        self.last_save_file = file
        self.last_save = file.read_text()
        self.chart.load_json(self.last_save)
        await self.chart.reset_state_a()
        return True

    @classmethod
    async def get_channel_a(cls, name, repl):
        if name in cls.channels:
            return cls.channels[name]
        new_channel = TestChannel(name, repl)
        await new_channel.load_from_file_a()
        await new_channel.setup_a()
        return new_channel


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
            return templates.TemplateResponse("test.html", response_data)
