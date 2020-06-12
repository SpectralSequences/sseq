import asyncio
import pathlib

import logging

from message_passing_tree.prelude import *
from message_passing_tree import SocketChannel
from message_passing_tree.utils import json_stringify
from message_passing_tree import ansi

import spectralsequence_chart
from spectralsequence_chart import SseqSocketReceiver, ChartAgent


from ..repl.executor import Executor
from .. import config

from fastapi.templating import Jinja2Templates
templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))

@subscribe_to("*")
@collect_handlers(inherit=True)
class SlideshowChannel(SocketChannel):
    def __init__(self, name, source_files, repl_agent):
        super().__init__(name)
        self.repl_agent = repl_agent
        self.last_screenshot = None
        self.source_files = source_files

    async def setup_a(self):
        await self.repl_agent.add_child_a(self)

    async def add_subscriber_a(self, websocket):
        recv = SseqSocketReceiver(websocket)
        recv.current_source_file = -1
        recv.max_source_file = -1
        recv.chart = SpectralSequenceChart("A slideshow")
        recv.executor = Executor(self.repl_agent)
        recv.executor.recv = recv  
        self.setup_executor_namespace(recv)
        await recv.executor.load_repl_init_file_if_it_exists_a()
        await recv.chart.add_child_a(recv)
        await recv.executor.add_child_a(recv.chart)
        await self.add_child_a(recv.executor)
        await recv.run_a()

    def setup_executor_namespace(self, recv):
        globals = recv.executor.get_globals()
        globals["REPL"] = self.repl_agent
        globals["chart"] = recv.chart
        globals["channel"] = self
        globals["receiver"] = recv

    @classmethod
    async def get_channel_a(cls, name, repl):
        if name in cls.channels:
            return cls.channels[name]
        files = sorted(config.SAVE_DIR.glob(f"{name}_*.json"))
        if not files:
            return None
        new_channel = SlideshowChannel(name, files, repl)
        await new_channel.setup_a()
        return new_channel

    @classmethod
    def has_channel(cls, name):
        if name in cls.channels:
            return True
        files = sorted(config.SAVE_DIR.glob(f"{name}_*.json"))
        return not not files

    @classmethod
    def http_response(cls, channel_name, request):
        response_data = { 
            "port" : cls.port, 
            "directory" : cls.directory,
            "channel_name" : channel_name,
            "request" : request, 
        }
        if cls.has_channel(channel_name):
            return templates.TemplateResponse("slideshow.html", response_data)

    @handle_inbound_messages
    async def handle__click__a(self, envelope, x, y, chart_class=None):
        pass # IGNORED!

    @handle_inbound_messages
    async def handle__slideshow__initialize_source_files__a(self, envelope):
        recv = self.look_up_recv_by_path(source_agent_path)
        self.log_debug("ready to prepare source file")
        await self.prepare_source_file_a(recv)
        recv.current_source_file += 1
        await self.load_source_file_a(recv)
        await self.prepare_source_file_a(recv)

    async def prepare_source_file_a(self, recv):
        idx = recv.current_source_file + 1
        if idx >= len(self.source_files):
            # TODO: how do we handle the end of the slideshow?
            return
        file = self.source_files[idx]
        overlays = list(config.OVERLAY_DIR.glob(f"{file.stem}*.svg"))
        file_list = [f"/overlay/{overlay.name}" for overlay in overlays]
        # Let's send the client the list of source files so it can http get them.
        # TODO: target the particular receiver...
        await self.send_message_outward_a(
            "slideshow.load_overlays", 
            *arguments(file_list = file_list),
            target_agent_path = [recv.executor.uuid]
        )

    async def load_source_file_a(self, recv):
        print(f"Loading source file {recv.current_source_file}")
        file = self.source_files[recv.current_source_file]
        json = file.read_text()
        recv.chart.load_json(json)
        print("x_range:", recv.chart.data.x_range)
        print("initial_x_range:", recv.chart.data.initial_x_range)
        # await recv.chart.reset_state_a()
        print(recv.chart.data)       
        await self.send_message_outward_a(
            "slideshow.updated_chart",
            *arguments(state = recv.chart.data),
            target_agent_path = [recv.executor.uuid]
        )

    @handle_inbound_messages
    async def handle__slideshow__next_file__a(self, envelope, file_idx):
        recv = self.look_up_recv_by_path(source_agent_path)
        print("slideshow.next_file", recv.current_source_file)
        if file_idx != recv.current_source_file + 1:
            self.log_error(
                f"Inconsistent source files: client requested next file {file_idx} "  +\
                f"but I think the next file is {recv.current_source_file + 1}"
            )
            return
        recv.current_source_file += 1
        await self.load_source_file_a(recv)
        if recv.current_source_file > recv.max_source_file:
            recv.max_source_file = recv.current_source_file
            await self.prepare_source_file_a(recv)

    @handle_inbound_messages
    async def handle__slideshow__previous_file__a(self, envelope, file_idx):
        recv = self.look_up_recv_by_path(source_agent_path)
        print("slideshow.previous_file", recv.current_source_file)
        if file_idx != recv.current_source_file - 1:
            self.log_error(
                f"Inconsistent source files: client requested previous file {file_idx} "  +\
                f"but I think the previous file is {recv.current_source_file - 1}"
            )
            return
        recv.current_source_file -= 1
        await self.load_source_file_a(recv)


    def look_up_recv_by_path(self, source_agent_path):
        # My children are executors.
        executor_id = source_agent_path[-1]
        for [uuid, child] in self.children.items():
            if uuid == executor_id:
                return child.recv
        raise RuntimeError("Didn't find a child with the appropriate ID.")


    @handle_inbound_messages
    async def handle__console__take__a(self, envelope):
        recv = self.look_up_recv_by_path(source_agent_path)
        self.repl_agent.set_executor(recv.executor)
