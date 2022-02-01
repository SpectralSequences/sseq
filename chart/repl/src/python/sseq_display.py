from js import location, console

from js_wrappers.async_js import Fetcher
from js_wrappers.filesystem import FileHandle
from asyncio import ensure_future


import json
import pathlib

from spectralsequence_chart import SseqChart
from spectralsequence_chart.serialization import JSON
from working_directory import get_working_directory_a, set_working_directory_a
from functools import wraps
from repl.handler_decorator import collect_handlers, handle

fetcher = Fetcher("api/")


def create_display(name):
    disp = SseqDisplay(name)
    print(f"Creating display at {disp.url}")
    return disp.chart


async def load_display_a(name):
    disp = SseqDisplay(name)
    await disp.load_a()
    return disp.chart


@collect_handlers("message_handlers")
class SseqDisplay:
    """A Spectral Sequence display. This contains the logic to communicate between the SseqChart and the browser.
    All of the data is contained in the field SseqDisplay.chart which is the SseqChart object that is being displayed.
    You may want to store the chart into a variable and use it directly.
    """

    #
    displays = {}

    def __init__(self, name, chart=None):
        self.name = name
        self.chart = None
        self.save_file_handle = FileHandle()
        self.autosave = False
        chart = chart or SseqChart(name)
        self.set_sseq(chart)
        self.subscribers = {}
        SseqDisplay.displays[name] = self
        from repl.executor import Executor

        self.executor = Executor.executor
        self._started = False
        ensure_future(self.start_a())

    def __repr__(self):
        if self._started:
            return f'{type(self).__name__}(name="{self.name}", url="{self.url}", chart={self.chart})'
        return f"""{type(self).__name__}(name="{self.name}", state="Not started, run 'await display.start_a()' to start.")"""

    # def __dir__(self):
    #     """ getattr and dir have to be set up carefully to allow jedi to provide good docs for the SseqChart functions. """
    #     result = self.chart.__dir__()
    #     result.extend(self.__dict__.keys())
    #     return sorted(set(result))

    # def __getattr__(self, name):
    #     """ getattr and dir have to be set up carefully to allow jedi to provide good docs for the SseqChart functions. """
    #     if not hasattr(self.chart, name):
    #         raise AttributeError(f'Instance of {self.__class__.__name__} has no attribute {name}')
    #     return getattr(self.chart, name)

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
        return f"{location.protocol}//{location.host}{directory}charts/{self.name}"

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
            self.chart._clear_batched_messages()
        await self.send_message_a("chart.state.reset", state=self.chart.to_json())
        await self.maybe_autosave_a()

    def update(self):
        ensure_future(self.update_a())

    async def update_a(self):
        await self.chart.update_a()

    async def send_batched_messages_a(self, messages):
        console.log("Sending batched messages:", messages)
        await self.send_message_a("chart.update", messages=messages)
        await self.maybe_autosave_a()

    async def maybe_autosave_a(self):
        if self.autosave and self.save_file_handle.is_open():
            await self.save_a()

    async def save_a(self):
        await self.save_file_handle.ensure_open_a(modify=True)
        await self.save_file_handle.write_text_a(JSON.stringify(self.chart))

    async def save_as_a(self, path=None):
        if path:
            working_directory = await get_working_directory_a()
            if not working_directory:
                raise RuntimeError("...")
            self.save_file_handle = await working_directory.path(
                path
            ).resolve_file_handle_a(create=True)
        else:
            self.save_file_handle = FileHandle()
        await self.save_a()

    async def load_a(self, path=None):
        if path:
            working_directory = await get_working_directory_a()
            if not working_directory:
                raise RuntimeError("...")
            self.save_file_handle = await working_directory.path(
                path
            ).resolve_file_handle_a()
        else:
            self.save_file_handle = FileHandle()
            await self.save_file_handle.open_a()
        self.set_sseq(JSON.parse(await self.save_file_handle.read_text_a()))
        await self.reset_state_a()

    @staticmethod
    async def dispatch_message(obj):
        message = json.loads(obj["message"])
        del obj["message"]
        message.update(obj)
        chart_name = message["chart_name"]
        del message["chart_name"]
        display = SseqDisplay.displays[chart_name]
        await display.handle_message(**message)

    async def handle_message(self, cmd, args, port, client_id, uuid, kwargs):
        kwargs = dict(kwargs)
        console.log(f"SseqDisplay.handle_message({cmd}, {JSON.stringify(kwargs)})")
        await self.message_handlers[cmd](
            self, uuid=uuid, port=port, client_id=client_id, **kwargs
        )

    @staticmethod
    def _create_message(cmd, **kwargs):
        return JSON.stringify(dict(cmd=cmd, args=[], kwargs=kwargs))

    async def send_message_a(self, cmd, **kwargs):
        message = SseqDisplay._create_message(cmd, **kwargs)
        for port in self.subscribers.values():
            port.postMessage(message)

    async def send_message_to_target_client_a(self, port, cmd, uuid, **kwargs):
        port.postMessage(
            JSON.stringify(dict(cmd=cmd, uuid=uuid, args=[], kwargs=kwargs))
        )

    @handle("new_user")
    async def new_user__a(self, uuid, port, client_id):
        print("Handling new user...")
        # Might as well make sure that we don't have other charts that are out of date.
        # So let's send an update to the existing charts first.
        await self.update_a()
        self.subscribers[client_id] = port
        # "initialize" command sets chart range and page in addition to setting the chart.
        # "initialize" does a superset of what "reset" does.
        port.postMessage(
            SseqDisplay._create_message(
                "chart.state.initialize", state=self.chart.to_json()
            )
        )

    @handle("initialize.complete")
    async def initialize__complete__a(self, uuid, port, client_id):
        print("initialize.complete")


def _wrap_chart_func(func):
    @wraps(func)
    def wrap(self, *args, **kwargs):
        return func(self.chart, *args, **kwargs)

    return wrap


def _bind_chart_attribute(name):
    func = getattr(SseqChart, name)
    func_type_name = type(func).__name__
    if func_type_name == "function":
        wrapped = _wrap_chart_func(func)
    elif func_type_name == "property":
        wrapped_fget = None
        wrapped_fset = None
        wrapped_fdel = None
        if func.fget:
            wrapped_fget = _wrap_chart_func(func.fget)
        if func.fset:
            wrapped_fset = _wrap_chart_func(func.fset)
        if func.fdel:
            wrapped_fdel = _wrap_chart_func(func.fdel)
        wrapped = property(wrapped_fget, wrapped_fset, wrapped_fdel)
    else:
        raise AssertionError()
    setattr(SseqDisplay, name, wrapped)


# for a in dir(SseqChart):
#     if a.startswith("_") or a in dir(SseqDisplay):
#         continue
#     # The __getattr__ and __dir__ methods above aren't enough to get docs for properties.
#     # For properties, we copy a wrapper from SseqChart to SseqDisplay.
#     # Note that if we do this for methods too, it screws up jedi get_signatures.
#     # So __dir__ / __getattr__ work only for methods and this works only for properties...
#     if type(getattr(SseqChart, a)) is property:
#         _bind_chart_attribute(a)
