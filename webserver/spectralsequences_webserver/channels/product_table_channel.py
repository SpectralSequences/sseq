import asyncio
from datetime import datetime
from multiprocessing import Process
import pathlib

from message_passing_tree.prelude import *
from message_passing_tree import SocketChannel
from message_passing_tree.utils import json_stringify
from message_passing_tree import ansi


from spectralsequence_chart import SseqSocketReceiver, SpectralSequenceChart

from ..repl.executor import Executor
from .. import config

from ..process_overlay import process_overlay


from fastapi.templating import Jinja2Templates
templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))


@subscribe_to("*")
@collect_transforms(inherit=True)
class InteractChannel(SocketChannel):
    def __init__(self, name, repl_agent):
        super().__init__(name)
        self.repl_agent = repl_agent
        self.executor = Executor(repl_agent.console_io)
        self.chart = SpectralSequenceChart(name)
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
        new_channel = InteractChannel(name, repl)
        await new_channel.load_from_file_a()
        await new_channel.setup_a()
        return new_channel

    @transform_inbound_messages
    async def transform__io__save__a(self, envelope):
        envelope.mark_used()
        self.save()

    def save(self):
        save_str = json_stringify(self.chart.data)
        iso_time = datetime.now().replace(microsecond=0).isoformat().replace(":", "-")
        out_path = config.SAVE_DIR / f"{self.name}_{iso_time}.json"
        self.last_save = save_str
        self.last_save_file = out_path
        print(ansi.success("Saving to " + str(out_path)))
        out_path.write_text(save_str)

    def save_over_previous_version(self):
        save_str = json_stringify(self.chart.data)
        out_path = self.last_save_file
        self.last_save = save_str
        print(ansi.success("Overwriting " + str(out_path)))
        out_path.write_text(save_str)

    @transform_inbound_messages
    async def transform__io__process_screenshot__a(self, envelope):
        envelope.mark_used()
        files = sorted(config.SCREENSHOT_DIR.glob("*.png"))
        file = files[-1]
        if file == self.last_screenshot:
            print(ansi.info("No new screenshot to process."))
            return
        self.last_screenshot = file
        print(ansi.info("Setting up screenshot processing."))
        # save_str = json_stringify(self.chart.data)
        # if save_str != self.last_save:
        #     self.save()
        self.process_screenshot(file)

    def process_screenshot(self, file):
        i = sum(1 for i in config.OVERLAY_DIR.glob(f"{self.last_save_file.stem}*"))
        outfile = config.OVERLAY_DIR / f"{self.last_save_file.stem}__overlay{i}.svg"
        self.last_overlay_outfile = outfile
        p = Process(target=process_overlay, args=(file, outfile))
        p.start()
        print(ansi.info(f"   Output file: {outfile}"))


    def set_note(self, note):
        out_file = config.OVERLAY_DIR / (self.last_overlay_outfile.stem + "__note.txt")
        out_file.write_text(note)
        

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
