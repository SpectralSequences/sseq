import asyncio
from . import utils
from .agent import Agent
from .socket_receiver import SocketReceiver
from .prelude import *

from . import socket_close_codes

from fastapi import FastAPI, Request, WebSocket
from fastapi.responses import HTMLResponse, FileResponse, Response


import logging
logger = logging.getLogger(__name__)

@subscribe_to("*")
@collect_handlers(inherit = False)
class SocketChannel(Agent):
    channels = {}
    serving_class_to = None

    @classmethod
    def has_channel(cls, name):
        return name in cls.channels

    @classmethod
    def serve(cls, app, repl, host, port, cls_dir):
        num_routes = len(app.routes)
        cls.set_serving_info(host, port, cls_dir)
        cls.set_repl(repl)
        @app.get(f"/{cls_dir}/{{channel_name}}")
        async def get_html_a(request: Request, channel_name : str):
            logger.debug(f"get: {cls_dir}/{channel_name}")
            try:
                response_data = { 
                    "port" : port, 
                    "directory" : cls_dir,
                    "channel_name" : channel_name,
                    "request" : request, 
                }
                response = cls.http_response(channel_name, request)
                if response is None:
                    return cls.templates.TemplateResponse("invalid_channel.html", response_data)
                else:
                    return response
            except Exception as e:
                repl.console_io._handle_exception(e)

        @app.websocket(f"/ws/{cls_dir}/{{channel_name}}")
        async def websocket_subscribe_a(websocket: WebSocket, channel_name : str):
            logger.debug(f"ws: {cls_dir}/{channel_name}")
            try:
                channel = await cls.get_channel_a(channel_name, repl)
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

        cls.serve_extra(app, host, port, cls_dir)
        cls.routes = app.routes[num_routes:]

    @classmethod
    def serve_extra(cls, app, host, port, cls_dir):
        pass

    @classmethod
    def remove_routes(cls, app):
        cls.serving_class_to = ""
        for route in cls.routes:
            app.routes.remove(route)

    @classmethod
    async def close_all_channels_a(cls, code):
        tasks = []
        for channel in cls.channels.values():
            tasks.append(channel.schedule_coroutine(channel.close_channel_a(code)))
        return await asyncio.gather(*tasks)

    async def close_channel_a(self, code):
        del type(self).channels[self.name]
        await self.close_connections_a(code)

    async def close_connections_a(self, code):
        tasks = []
        for receiver in SocketChannel.get_receivers(self):
            print("  Closing connection")
            tasks.append(self.schedule_coroutine(receiver.close_a(code)))
            tasks.append(self.schedule_coroutine(self.remove_subscriber_a(receiver)))
        return await asyncio.gather(*tasks)
    
    @staticmethod 
    def get_receivers(agent):
        for child in agent.children.values():
            if isinstance(child, SocketReceiver):
                yield child
            else:
                yield from SocketChannel.get_receivers(child)

    @classmethod
    def set_serving_info(cls, host, port, directory):
        cls.host = host
        cls.port = port
        cls.directory = directory
        cls.serving_class_to = f"{host}:{port}/{directory}"
        cls.initialize_channel()

    @classmethod
    def set_repl(cls, repl):
        cls.repl = repl

    @classmethod
    def initialize_channel(cls):
        """ Override me """
        pass

    @classmethod
    def http_response(cls, channel_name, request):
        """ Override me. 
            If I return `None` we reject the request.
            Otherwise, probably use Jinja2Templates.
        """
        pass    

    def __init__(self, name):
        super().__init__()
        self.name = name
        type(self).channels[name] = self
        asyncio.ensure_future(self.send_start_msg_a())

    async def send_start_msg_a(self):
        print(f"Started {self.name}")

    def serving_to(self):
        if self.serving_class_to is None:
            return None
        else:
            return self.serving_class_to + f"/{self.name}"

    # async def handle_leaked_envelope_a(self, envelope):
    #     self.log_info(f"""Leaked envelope self: {self.info()}  envelope: {envelope.info()}""")


    async def add_subscriber_a(self, sock_recv):
        """ For overriding by subclasses. 
            Will be called by server when it gets a request to /ws/class_directory/channel_name.
            Channels are in charge of assembling the connection to the SocketReceiver themselves.
        """
        await self.add_child_a(sock_recv)

    @handle_inbound_messages
    async def handle__socket__close__a(self, envelope):
        recv = self
        for id in reversed(envelope.source_agent_path):
            recv = recv.children[id]
        await self.remove_subscriber_a(recv)

    async def remove_subscriber_a(self, sock_recv):
        child = sock_recv
        while child.parent is not self:
            child = child.parent
        await self.remove_child_a(child)

