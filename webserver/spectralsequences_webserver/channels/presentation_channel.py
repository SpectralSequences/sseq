import asyncio
import pathlib

import logging

from message_passing_tree.prelude import *
from message_passing_tree import SocketChannel
from message_passing_tree.utils import json_stringify
from message_passing_tree import ansi

import spectralsequence_chart
from spectralsequence_chart import SseqSocketReceiver, SpectralSequenceChart


from ..repl.executor import Executor
from .. import config

from fastapi.templating import Jinja2Templates
templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))

# The same as slideshow except that different tabs share same view.
@subscribe_to("*")
@collect_transforms(inherit=True)
class PresentationChannel(SocketChannel):
    def __init__(self, name, chart_source_files, repl_agent):
        super().__init__(name)
        self.repl_agent = repl_agent
        self.executor = Executor(repl_agent.console_io)
        self.chart = SpectralSequenceChart("A presentation")
        self.setup_executor_namespace()
        self.last_screenshot = None
        self.chart_source_files = chart_source_files
        self.chart_idx = 0
        self.overlay_idx = 0
        self.overlay_lists = []
        self.lock = asyncio.Lock() # This is to handle the case where the user holds down n or b.
        print(ansi.state_change("I AM PRESENTATIONCHANNEL AT YOUR SERVICE"))

    async def setup_a(self):
        await self.repl_agent.add_child_a(self.executor)
        await self.executor.add_child_a(self.chart)
        await self.chart.add_child_a(self)
        await self.executor.load_repl_init_file_if_it_exists_a()
        await self.update_chart_a()
    
    async def reset_a(self):
        self.chart_source_files = PresentationChannel.get_chart_files(self.name)
        self.chart_idx = 0
        self.overlay_idx = 0
        self.overlay_lists = []
        self.load_chart()
        await self.send_message_outward_a("slideshow.reset",*arguments())

    @property
    def current_chart_source(self):
        return self.chart_source_files[self.chart_idx]

    @property
    def current_overlay_source(self):
        return self.overlay_lists[self.chart_idx][self.overlay_idx]

    def set_note(self, note):
        out_file = config.OVERLAY_DIR / (self.current_overlay_source.stem + "__note.txt")
        out_file.write_text(note)

    @transform_inbound_messages
    async def transform__slideshow__reset__a(self, envelope):
        envelope.mark_used()
        print("Reset presentation. Refresh page!")
        await self.reset_a()

    async def add_subscriber_a(self, websocket):
        recv = SseqSocketReceiver(websocket)
        await self.add_child_a(recv)        
        await recv.start_a()

    def setup_executor_namespace(self):
        globals = self.executor.get_globals()
        globals["REPL"] = self.repl_agent
        globals["chart"] = self.chart
        globals["channel"] = self

    @staticmethod
    def get_chart_files(name):
        return sorted(config.SAVE_DIR.glob(f"{name}_*.json"))

    @classmethod
    async def get_channel_a(cls, name, repl):
        if name in cls.channels:
            return cls.channels[name]
        files = PresentationChannel.get_chart_files(name)
        if not files:
            return None
        new_channel = PresentationChannel(name, files, repl)
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
            return templates.TemplateResponse("presentation.html", response_data)

    @transform_inbound_messages
    async def transform__click__a(self, envelope, x, y, chart_class=None):
        envelope.mark_used()
        pass # IGNORED!

    @transform_inbound_messages
    async def transform__slideshow__chart__initialize__a(self, envelope):
        envelope.mark_used()
        await self.send_message_outward_a(
            "slideshow.initialize",
            *arguments(chart_idx = self.chart_idx, overlay_idx = self.overlay_idx),
            target_agent_path = source_agent_path
        )
        # await self.prepare_source_file_a()
        # self.chart_idx += 1
        # await self.load_source_file_a()
        # await self.prepare_source_file_a()

    def get_overlays(self, chart_idx):
        if chart_idx < len(self.overlay_lists):
            return self.overlay_lists[chart_idx]
        for i in range(len(self.overlay_lists), chart_idx + 1):
            file = self.chart_source_files[i]
            self.overlay_lists.append(list(config.OVERLAY_DIR.glob(f"{file.stem}*.svg")))
        return self.overlay_lists[chart_idx]

    @transform_inbound_messages
    async def transform__slideshow__overlay__request_batch__a(self, envelope, chart_idx):
        envelope.mark_used()
        if chart_idx >= len(self.chart_source_files):
            self.log_error(f"Client requested chart number {chart_idx} but I only have {len(self.chart_source_files)} charts.") 
            # TODO: how do we handle the end of the slideshow?
            return
        overlays = self.get_overlays(chart_idx)
        request_list = [f"/overlay/{overlay.name}" for overlay in overlays]
        # Let's send the client the list of source files so it can http get them.
        # TODO: target the particular receiver...
        await self.send_message_outward_a(
            "slideshow.overlay.load_batch", 
            *arguments(file_list = request_list, chart_idx = chart_idx),
            target_agent_path = source_agent_path
        )

    def load_chart(self):
        file = self.chart_source_files[self.chart_idx]
        json = file.read_text()
        self.chart.load_json(json)

    async def update_chart_a(self, change_direction=None):
        print(ansi.info(f"Loading source file {self.chart_idx}"))
        print(f" == {self.chart_source_files[self.chart_idx]}")
        self.load_chart()
        overlays = self.get_overlays(self.chart_idx)
        # print(f"  Loading overlay {overlays[self.overlay_idx]}")
        await self.send_message_outward_a("slideshow.chart.switch",
            *arguments(
                state = self.chart.data,
                chart_idx = self.chart_idx, 
                overlay_idx = self.overlay_idx, change_direction = change_direction,
            )
        )


    def check_chart_idx(self, chart_idx):
        if chart_idx != self.chart_idx:
            self.log_error(
                f"Inconsistent source files: client thinks current chart is {chart_idx} "  +\
                f"but I think the current chart is {self.chart_idx}"
            )
            return            
    
    def check_overlay_idx(self, overlay_idx):
        if overlay_idx != self.overlay_idx:
            self.log_error(
                f"Inconsistent overlay indexes: client thinks current overlay is {overlay_idx} "  +\
                f"but I think the current overlay is {self.overlay_idx}"
            )
            return

    @transform_inbound_messages
    async def transform__slideshow__next__a(self, envelope, chart_idx, overlay_idx):
        envelope.mark_used()
        if self.lock.locked():
            return
        await self.lock.acquire() # This lock prevents the program from exploding if the user continuously holds n.
        try:
            self.check_chart_idx(chart_idx)
            self.check_overlay_idx(overlay_idx)
            del chart_idx # Avoid bugs where we update self.chart_idx then use chart_idx.
            del overlay_idx    
            overlays = self.get_overlays(self.chart_idx)
            if(self.overlay_idx + 1 < len(overlays)):
                self.overlay_idx += 1
                print(f"""  {ansi.info("Loading next overlay")} {overlays[self.overlay_idx]}""")
                await self.send_message_outward_a("slideshow.overlay.switch",
                    *arguments(chart_idx = self.chart_idx, overlay_idx = self.overlay_idx, change_direction = 1 )
                )
            else:
                if self.chart_idx + 1 == len(self.chart_source_files):
                    # We are out of slides.
                    print(ansi.info("At end, out of slides..."))
                    return
                self.chart_idx += 1
                self.overlay_idx = 0
                await self.update_chart_a(1)
        finally: # Wait a bit before releasing lock to advance at an orderly pace when user holds n.
            asyncio.ensure_future(self.release_lock())

    async def release_lock(self):
        await asyncio.sleep(0.1)
        self.lock.release()


    @transform_inbound_messages
    async def transform__slideshow__previous__a(self, envelope, chart_idx, overlay_idx):
        envelope.mark_used()
        if self.lock.locked():
            return
        await self.lock.acquire()
        try:
            self.check_chart_idx(chart_idx)
            self.check_overlay_idx(overlay_idx)
            del chart_idx # Avoid bugs where we update self.chart_idx then use chart_idx.
            del overlay_idx 
            change_direction = -1
            overlays = self.get_overlays(self.chart_idx)
            if(self.overlay_idx > 0):
                self.overlay_idx -= 1
                print(f"""  {ansi.info("Loading previous overlay")} {overlays[self.overlay_idx]}""")
                await self.send_message_outward_a("slideshow.overlay.switch",
                    *arguments(chart_idx = self.chart_idx, overlay_idx = self.overlay_idx, change_direction = -1 )
                )
            else:
                # TODO: handle beginning of presentation
                if self.chart_idx == 0:
                    # already at beginning
                    print(ansi.info("At beginning"))
                    return
                self.chart_idx -= 1
                self.overlay_idx = len(self.get_overlays(self.chart_idx)) - 1
                await self.update_chart_a(-1)
        finally:
            asyncio.ensure_future(self.release_lock())
        
        
    @transform_inbound_messages
    async def transform__console__take__a(self, envelope):
        envelope.mark_used()
        self.repl_agent.set_executor(self.executor)
