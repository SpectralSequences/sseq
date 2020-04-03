## This file currently operates in an exceptions black hole.
## Where do the exceptions go when there's a failure? Nobody knows.
import asyncio
from fastapi import FastAPI, Request, WebSocket
from fastapi.responses import HTMLResponse, Response
from fastapi.templating import Jinja2Templates

import logging
logger = logging.getLogger(__name__)
from . import config

from .repl import start_repl_a, ReplAgent
from .channels import (
    DemoChannel, 
    InteractChannel,
    SseqChannel
)
from . import socket_close_codes
from message_passing_tree import SocketReceiver
from spectralsequence_chart import SseqSocketReceiver
# from spectralsequence_chart.utils import
print("Starting server")

app = FastAPI()

def run_main(f):
    asyncio.ensure_future(f())
    return f

@run_main
async def main():
    repl = await start_repl_a()
    channels = {}

    templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))

    class JSResponse(Response):
        media_type = "application/javascript"

    @app.get("/static/webclient", response_class=JSResponse)
    async def get_a():
        return config.SSEQ_WEBCLIENT_JS_FILE.read_text()

    @app.get("/anss-S0.html")
    async def get_a():
        return HTMLResponse((config.TEMPLATE_DIR / "anss-S0.html").read_text())

    @app.get("/anss-S0.json")
    async def get_a():
        return (config.USER_DIR / "anss-S0_2020-04-03T15-43-48.json").read_text()


    def serve_channel(app, channel_cls, cls_dir):
        channel_cls.serve_channel_to("localhost",config.PORT, cls_dir)

        @app.get(f"/{cls_dir}/{{channel_name}}")
        async def get_html_a(request: Request, channel_name : str):
            logger.debug(f"get: {cls_dir}/{channel_name}")
            try:
                response_data = { 
                    "port" : config.PORT, 
                    "directory" : cls_dir,
                    "channel_name" : channel_name,
                    "request" : request, 
                }
                response = channel_cls.http_response(channel_name, request)
                if response is None:
                    return templates.TemplateResponse("invalid_channel.html", response_data)
                else:
                    return response
            except Exception as e:
                repl._handle_exception(e)

        @app.websocket(f"/ws/{cls_dir}/{{channel_name}}")
        async def websocket_subscribe_a(websocket: WebSocket, channel_name : str):
            logger.debug(f"ws: {cls_dir}/{channel_name}")
            try:
                channel = await channel_cls.get_channel_a(channel_name, repl)
                if channel is None:
                    # TODO: is this the best way to handle this?
                    # One reasonable reason we could end up here is if the channel closed between the
                    # get request and now...
                    # In that case we should respond with GOING_AWAY rather than INTERNAL_ERROR.
                    raise RuntimeError(f"""No channel available named "{cls_dir}/{channel_name}".""")
                await channel.add_subscriber_a(websocket)
            except Exception as e:
                await websocket.close(socket_close_codes.INTERNAL_ERROR)
                repl.console_io._handle_exception(e)


    serve_channel(app, SseqChannel, "sseq")
    serve_channel(app, DemoChannel, "demo")
    serve_channel(app, InteractChannel, "interact")