from . import config

from fastapi import FastAPI, Request, WebSocket
from fastapi.responses import HTMLResponse, Response
from fastapi.templating import Jinja2Templates
# from starlette.websockets import WebSocketDisconnect

from .repl import start_repl
from spectralsequence_chart import SseqSocketChannel
from spectralsequence_chart import SseqSocketReceiver

import logging

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

@app.get("/sseq/{sseq_name}")
async def get(request: Request, sseq_name : str):
    response_data = { "request" : request, "PORT" : config.PORT, "channel_name" : sseq_name }
    if SseqSocketChannel.has_channel(sseq_name):
        return templates.TemplateResponse("sseq_chart.html", response_data )
    else:
        return templates.TemplateResponse("invalid_channel.html", response_data)

SseqSocketChannel.serving_class_to=f"localhost:{config.PORT}/sseq"
@app.websocket("/ws/sseq/{sseq_name}")
async def websocket_subscribe_sseq(websocket: WebSocket, sseq_name : str):
    channel = SseqSocketChannel.get_channel(sseq_name)
    recv = SseqSocketReceiver(websocket)
    await channel.add_child(recv)
    await recv.run()