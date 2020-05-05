## This file currently operates in an exceptions black hole.
## Where do the exceptions go when there's a failure? Nobody knows.
import asyncio
from fastapi import FastAPI, Request, WebSocket
from fastapi.responses import HTMLResponse, FileResponse, Response
from fastapi.templating import Jinja2Templates

import logging
logger = logging.getLogger(__name__)
from . import config

from .repl import start_repl_a, ReplAgent
from .channels import (
    DemoChannel, 
    InteractChannel,
    PresentationChannel,
    ResolverChannel,
    SlideshowChannel,
    SseqChannel
)
from message_passing_tree import SocketReceiver, ansi
from spectralsequence_chart import SseqSocketReceiver
# from spectralsequence_chart.utils import

app = FastAPI()

def run_main(f):
    asyncio.ensure_future(f())
    return f

@run_main
async def main():
    print("""Executing user "on_repl_init" file.""")
    repl = await start_repl_a()

    print(ansi.success("Starting server"))
    channels = {}

    templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))

    class JSResponse(Response):
        media_type = "application/javascript"

    @app.get("/static/webclient", response_class=JSResponse)
    async def get_a():
        return config.SSEQ_WEBCLIENT_JS_FILE.read_text()

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


    host = "localhost"
    port = config.PORT

    SseqChannel.serve(app, repl, host, port, "sseq")
    DemoChannel.serve(app, repl, host, port, "demo")
    InteractChannel.serve(app, repl, host, port, "interact")
    SlideshowChannel.serve(app, repl, host, port, "slideshow")
    PresentationChannel.serve(app, repl, host, port, "presentation")
    ResolverChannel.serve(app, repl, host, port, "resolver")