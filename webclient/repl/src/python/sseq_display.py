from js import (
    location, messageLookup as js_message_lookup
)
import json
import pathlib

from spectralsequence_chart import SseqChart
from spectralsequence_chart.utils import JSON, arguments

from .async_js import Fetcher
from .handler_decorator import collect_handlers, handle
fetcher = Fetcher("api/")

@collect_handlers("message_handlers")
class SseqDisplay:
    """ A Spectral Sequence display. This contains the logic to communicate between the SseqChart and the browser.
        All of the data is contained in the field SseqDisplay.chart which is the SseqChart object that is being displayed.
        You may want to store the chart into a variable and use it directly.
    """
    # 
    displays = {}    
    def __init__(self, name, chart=None):
        self.name = name
        self.chart = None
        chart = chart or SseqChart(name)
        self.set_sseq(chart)
        self.subscribers = {}
        SseqDisplay.displays[name] = self
        from .executor import PyodideExecutor
        self.executor = PyodideExecutor.executor
        self._started = False
    
    def __repr__(self):
        if self._started:
            return f'{type(self).__name__}(name="{self.name}", url="{self.url}", chart={self.chart})'
        return f"""{type(self).__name__}(name="{self.name}", state="Not started, run 'await display.start_a()' to start.")"""


    def load_json(self, json_obj):
        if type(json_obj) is str:
            json_obj = json.loads(json_obj)
        self.set_sseq(SseqChart.from_json(json_obj))

    def set_sseq(self, chart):
        if self.chart is not None:
            self.chart._agent = None
        self.chart = chart
        self.chart._agent = self

    @property
    def url(self):
        directory = str(pathlib.Path(location.pathname).parent)
        return f"{location.protocol}//{location.host}{directory}/charts/{self.name}"

    async def start_a(self):
        if self._started:
            return
        self._started = True
        response = await fetcher.put(f"charts/{self.name}", {})
        if response.status >= 400:
            raise Exception(f"Failed to create chart: {response.status_text}")
        body = await response.json()
        print(f'Display started. Visit "{self.url}" to view.')

    async def reset_state_a(self):
        with self.chart._batched_messages_lock:
            self.chart._batched_messages = []
        await self.send_message_a("chart.state.reset", state = self.chart.to_json())

    async def update_a(self):
        await self.chart.update_a()

    async def send_batched_messages_a(self, messages):
        print("Sending batched messages...")
        print("Messages:", messages)
        await self.send_message_a("chart.batched", messages = messages)

    @staticmethod
    def dispatch_message(message_id):
        obj = js_message_lookup[message_id]
        message = json.loads(obj["message"])
        del obj["message"]
        message.update(obj)        
        del js_message_lookup[message_id]
        chart_name = message["chart_name"]
        del message["chart_name"]
        display = SseqDisplay.displays[chart_name]
        display.handle_message(**message)

    def handle_message(self, cmd, args, port, client_id, uuid, kwargs):
        kwargs = dict(kwargs)
        print(f"SseqDisplay.handle_message({cmd}, {JSON.stringify(kwargs)})")
        self.executor.loop.call_soon(self.message_handlers[cmd](
            self, uuid=uuid, port=port, client_id=client_id, **kwargs
        ))

    async def send_message_a(self, cmd, **kwargs):
        kwargs.update(cmd=cmd)
        message = JSON.stringify(dict(cmd=cmd, args=[], kwargs=kwargs))
        for port in self.subscribers.values():
            port.postMessage(message)

    async def send_message_to_target_client_a(self, port, cmd, uuid, **kwargs):
        port.postMessage(JSON.stringify(dict(
            cmd=cmd, uuid=uuid,
            args=[], kwargs=kwargs
        )))

    @handle("new_user")
    async def new_user__a(self, uuid, port, client_id):
        print("Handling new user...")
        self.subscribers[client_id] = port
        await self.reset_state_a()

    @handle("initialize.complete")
    async def initialize__complete__a(self, uuid, port, client_id):
        pass