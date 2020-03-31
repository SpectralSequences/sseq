## This file currently operates in an exceptions black hole.
## Where do the exceptions go when there's a failure? Nobody knows.
import asyncio
from fastapi import FastAPI, Request, WebSocket
from fastapi.responses import HTMLResponse, Response
from fastapi.templating import Jinja2Templates

from . import config


from .repl import start_repl
from .channels import (DemoChannel, SseqChannel)
from spectralsequence_chart import SseqSocketReceiver
# from spectralsequence_chart.utils import

start_repl()
app = FastAPI()

print("Starting server")
channels = {}

templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))

class JSResponse(Response):
    media_type = "application/javascript"

@app.get("/static/webclient", response_class=JSResponse)
async def get():
    return config.SSEQ_WEBCLIENT_JS_FILE.read_text()


def serve_channel(app, channel_cls, cls_dir):
    channel_cls.serve_channel_to("localhost",config.PORT, cls_dir)

    @app.get(f"/{cls_dir}/{{channel_name}}")
    async def get_html(request: Request, channel_name : str):
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

    @app.websocket(f"/ws/{cls_dir}/{{channel_name}}")
    async def websocket_subscribe(websocket: WebSocket, channel_name : str):
        print("ws:", channel_name)
        channel = channel_cls.get_channel(channel_name)
        print("ws:", channel)
        if channel is None:
            pass
            return # TODO: Reject connection request.
        print("???")
        sock_recv = SseqSocketReceiver(websocket)
        await channel.add_subscriber(sock_recv)
        await sock_recv.run()



serve_channel(app, SseqChannel, "sseq")
serve_channel(app, DemoChannel, "demo")

