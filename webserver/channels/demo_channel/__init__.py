import asyncio
import pathlib

from message_passing_tree.prelude import *
from message_passing_tree import SocketChannel
from message_passing_tree import ansi

from spectralsequences_webserver.chart import *

from spectralsequences_webserver.repl.executor import Executor
from spectralsequences_webserver import config

from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates


@subscribe_to("*")
@collect_handlers(inherit=True)
class DemoChannel(SocketChannel):
    channels = {}
    serve_as = "demo"
    CHANNEL_DIR = pathlib.Path(__file__).parent
    templates = Jinja2Templates(directory=str(CHANNEL_DIR))

    def __init__(self, name, file_path, repl_agent):
        super().__init__(name)
        self.file_path = file_path
        self.repl_agent = repl_agent
        self.subscriber_demos = {}

    async def send_start_msg_a(self):
        pass

    async def setup_a(self):
        await self.repl_agent.add_child_a(self)

    def make_demo_take_over_console(self):
        def closure(demo):
            self.repl_agent.set_executor(demo.executor)
        return closure

    async def add_subscriber_a(self, websocket):
        executor = Executor(self.repl_agent)
        
        put_main_class_here = {}
        self.setup_executor_namespace(executor, put_main_class_here)
        await executor.exec_file_a(self.file_path)

        DemoClass = put_main_class_here["main_class"]
        if DemoClass.inward_handlers is None:
            collect_handlers(inherit = True)(DemoClass)
        if DemoClass.subscriptions is None:
            subscribe_to("*")(DemoClass)
        demo = DemoClass(executor)
        executor.get_globals()["demo"] = demo

        demo.take_over_console = self.make_demo_take_over_console()
        await self.add_child_a(demo)
        await demo.setup_a(websocket)
        demo.start_socket() # We do this weirdly here because it's a hack.
        await demo.start_a()# I didn't want to bother resolving a deadlock in a better way.
        print("END")

    def setup_executor_namespace(self, executor, return_by_reference):
        """ Get the @main decorator into the """
        def main(cls):
            return_by_reference["main_class"] = cls
            return cls
        globals = executor.get_globals()
        globals["main"] = main
        globals["Demo"] = Demo
        globals["collect_handlers"] = collect_handlers
        globals["subscribe_to"] = subscribe_to
        globals["handle_inbound_messages"] = handle_inbound_messages
        globals["handle_outbound_messages"] = handle_outbound_messages
        executor.get_globals()["REPL"] = self.repl_agent


    @classmethod
    def get_file_path(cls, name):
        file_path = (config.DEMO_DIR / (name + ".py"))
        if not file_path.is_file():
            return None
        return file_path

    @classmethod
    async def get_channel_a(cls, name, repl):
        if name in cls.channels and False: # Always reload for debugging demos.
            return cls.channels[name]
        file_path = cls.get_file_path(name)
        if file_path:
            new_channel = DemoChannel(name, file_path, repl)
            await new_channel.setup_a()
            return new_channel

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
            return DemoChannel.templates.TemplateResponse("index.html", response_data)

    
    @handle_inbound_messages
    async def handle__new_user__a(self, envelope):
        demo = self.children[envelope.source_agent_path[-1]]
        await self.send_message_outward_a("initialize.chart.state", *arguments(
            state=demo.chart.sseq, display_state=demo.chart.display_state
        ))



@collect_handlers(inherit=False) # Nothing to inherit.
class GenericDemo(Agent):
    def __init__(self, executor):
        super().__init__()
        self.executor = executor
        self.breakpoint = -1
        self.named_breakpoints = {}
        self.user_next = asyncio.Event()
        self.ready_for_next_signal = asyncio.Event()
        self.next_lock = asyncio.Lock()

    async def setup_a(self, websocket):
        raise RuntimeError("You must override setup!")

    def start_socket(self):
        raise RuntimeError("You must override start_socket??!!")

    async def start_a(self):
        try:
            await self.run_a()
        except Exception as e:
            await self.handle_exception_a(e)

    async def run_a(self):
        raise RuntimeError("You must override run!")

    async def wait_for_user_a(self, name=None):
        self.user_next.clear()
        self.ready_for_next_signal.set()
        await self.user_next.wait()
    
    @handle_inbound_messages
    async def handle__demo__next__a(self, envelope):
        await self.next_a()

    @handle_inbound_messages
    async def handle__console__take__a(self, envelope):
        self.take_over_console(self)

    async def next_a(self):
        async with self.next_lock:
            await self.ready_for_next_signal.wait()
            self.ready_for_next_signal.clear()
            self.user_next.set()

class Demo(GenericDemo):
    async def setup_a(self, websocket):
        self.socket = SseqSocketReceiver(websocket)
        self.chart = ChartAgent("demo")
        self.sseq = self.chart.sseq
        await self.chart.add_child_a(self.socket)
        await self.executor.add_child_a(self.chart)
        await self.add_child_a(self.executor)
        self.setup_executor_namespace()

    def setup_executor_namespace(self):
        globals = self.executor.get_globals()
        globals["chart"] = self.chart
        globals["channel"] = self

    def start_socket(self):
        asyncio.ensure_future(self.socket.run_a())

