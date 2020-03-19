import config

import asyncio
from fastapi import FastAPI, Request, WebSocket
from fastapi.responses import HTMLResponse, Response
from fastapi.templating import Jinja2Templates
import json
from starlette.websockets import WebSocketDisconnect

from console import start_console
from channel import Channel
from user import User

from console.spectral_sequence import SpectralSequenceChart
from decorators import *

import utils

start_console.main()

app = FastAPI()
templates = Jinja2Templates(directory="templates")

class JSResponse(Response):
    media_type = "application/javascript"

@app.get("/sseq/{sseq_name}")
async def get(request: Request, sseq_name : str):
    if Channel.is_channel(sseq_name):
        return templates.TemplateResponse("sseq_chart.html", { "request" : request, "PORT" : config.PORT, "channel_name" : sseq_name })
    else:
        return templates.TemplateResponse("invalid_channel.html", { "request" : request, "PORT" : config.PORT, "channel_name" : sseq_name })

@app.get("/static/basic_webclient", response_class=JSResponse)
async def get():
    return config.BASIC_WEBCLIENT_JS_FILE.read_text()

@app.websocket("/subscribe_sseq/{channel_name}")
async def websocket_subscribe_sseq(websocket: WebSocket, channel_name : str):
    user = User(channel_name, websocket)
    await user.run()

@app.websocket("/publish_sseq/{channel_name}")
async def websocket_publish_sseq(websocket: WebSocket, channel_name : str):
    channel = Channel(channel_name, websocket)
    await channel.run()



    


