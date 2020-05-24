## This file currently operates in an exceptions black hole.
## Where do the exceptions go when there's a failure? Nobody knows.
import asyncio
from fastapi import FastAPI, Request, WebSocket
from fastapi.responses import HTMLResponse, FileResponse, Response
from fastapi.templating import Jinja2Templates
from fastapi.staticfiles import StaticFiles

import logging
logger = logging.getLogger(__name__)
from . import config

from .repl import start_repl_a, Executor
from .channels import (
    DemoChannel, 
    InteractChannel,
    PresentationChannel,
    ResolverChannel,
    SlideshowChannel,
    SseqChannel,
    TestChannel,
    TableChannel
)
from message_passing_tree import SocketReceiver, ansi
from spectralsequence_chart import SseqSocketReceiver
# from spectralsequence_chart.utils import

app = FastAPI()

def run_main(f):
    asyncio.ensure_future(f())
    return f

repl = None
host = "localhost"
port = config.PORT

served_channels = {}
def serve(channel, name = None):
    if name is None:
        name = channel.serve_as
    if name in served_channels:
        served_channels[name].remove_routes(app)
    served_channels[name] = channel
    print("Serving", channel.__repr__(0), hash(channel), "as", name)
    channel.serve(app, repl, host, port, name)

@run_main
async def main():
    print(ansi.success("Starting server"))
    channels = {}

    templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))

    class JSResponse(Response):
        media_type = "application/javascript"

    app.mount("/static/client", StaticFiles(directory=config.CLIENT_DIR), name="client")

    @app.get("/static/webclient", response_class=JSResponse)
    async def get_a():
        return config.SSEQ_WEBCLIENT_JS_FILE.read_text()


    Executor.add_to_global_namespace(serve)
    Executor.add_to_global_namespace("app", app)
    global repl
    repl = await start_repl_a()
    

    # serve(SseqChannel, "sseq")
    # serve(DemoChannel, "demo")
    # serve(InteractChannel, "interact")
    # serve(SlideshowChannel, "slideshow")
    # serve(PresentationChannel, "presentation")
    # serve(ResolverChannel, "resolver")
    # serve(TestChannel, "test")
    serve(TableChannel)

    print("""Executing user "on_repl_init" file.""")



    # TODO: clean this crap up.


    @app.get("/anss-S0.html")
    async def get_anss_S0():
        return FileResponse(config.TEMPLATE_DIR / "anss-S0.html")

    @app.get("/anss-S0.json")
    async def get_anss_S0_json():
        return FileResponse(config.USER_DIR / "anss-S0_2020-04-03T15-43-48.json")

    @app.get("/anss-S0-with-J.html")
    async def get_S0_with_J_html():
        return FileResponse(config.TEMPLATE_DIR / "anss-S0-with-J.html")

    @app.get("/anss-S0-with-J.json")
    async def get_S0_with_J_json():
        return FileResponse(config.USER_DIR / "anss-S0-with-J_2020-04-03T20-09-21.json")

    @app.get("/overlay-test.svg")
    async def get_test_overlay():
        return FileResponse(config.USER_DIR / "anss-labels-white.svg")
    
    @app.get("/overlay/{file_name}")
    async def get_overlay(request: Request, file_name : str):
        return FileResponse(config.OVERLAY_DIR / file_name);