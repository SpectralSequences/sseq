import asyncio

from message_passing_tree import SocketChannel
from message_passing_tree.prelude import *

from spectralsequence_chart import SpectralSequenceChart

from ..repl.executor import Executor
from .. import config

from fastapi.templating import Jinja2Templates
templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))


@subscribe_to("*")
@collect_transforms(inherit=True)
class SseqDemoChannel(SocketChannel):
    def __init__(self, name, file_path):
        super().__init__(name)
        self.file_path = file_path
        self.user_next = asyncio.Event()

    channels = {}
    async def send_start_msg(self):
        await self.has_parent.wait()
        serving_to = self.serving_to()
        if serving_to is not None:
            await self.send_info(
                "channel.opened",
                f"""Started demo spectral sequence "<blue>{self.name}</blue>".\n""" +\
                f"""Visit "<blue>{self.serving_to()}</blue>" to view it."""
            )

    async def add_subscriber(self, sock_recv):
        chart = SpectralSequenceChart("demo")
        executor = Executor()
        executor.get_globals()["chart"] = chart
        executor.get_globals()["wait_for_user"] = self.wait_for_user
        await chart.add_child(sock_recv)
        await executor.add_child(chart)
        await self.add_child(chart)
        asyncio.ensure_future(executor.exec_file(self.file_path))

    async def wait_for_user(self):
        await self.user_next.wait()
        self.user_next.clear()

    @classmethod
    def get_file_path(cls, name):
        file_path = (config.DEMO_DIR / (name + ".py"))
        if not file_path.is_file():
            return None
        return file_path

    @classmethod
    def get_channel(cls, name):
        if name in cls.channels:
            return cls.channels[name]
        file_path = cls.get_file_path(name)
        if file_path:
            return SseqDemoChannel(name, file_path)
    
    @classmethod
    def has_channel(cls, name):
        return name in cls.channels or cls.get_file_path(name)

    @classmethod
    def http_response(cls, channel_name, request):
        response_data = { 
            "port" : cls.port, 
            "directory" : cls.directory,
            "channel_name" : channel_name,
            "request" : request, 
        }
        if cls.has_channel(channel_name):
            return templates.TemplateResponse("demo.html", response_data)


    @transform_inbound_messages
    async def consume_demo__next(self, source_agent_path, cmd):
        print("next!")
        self.user_next.set()