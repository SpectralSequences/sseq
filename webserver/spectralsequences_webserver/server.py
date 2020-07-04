## This file currently operates in an exceptions black hole.
## Where do the exceptions go when there's a failure? Nobody knows.
## TODO: Does the above ^^ still apply? If so, fix it.
import asyncio
from fastapi import FastAPI

import sys

import logging
logger = logging.getLogger(__name__)
from . import config
from . import utils

from .repl import start_repl_a, Executor, ConsoleIO
from message_passing_tree import ansi
import time

def run_server(Server):
    global server
    server = Server()
    task = asyncio.ensure_future(server.main())
    def done_callback(e):
        exc = e.exception()
        if exc is not None:
            if server.repl:
                server.repl.exit()
            sys.exit(1)
    task.add_done_callback(done_callback)
    return Server

# should define "startup"
utils.exec_file(config.SERVER_STARTUP_FILE, globals(), locals())


@run_server
class Server:

    def __init__(self):
        self.app = FastAPI()
        self.repl = None
        self.host = "localhost"
        self.port = config.PORT
        self.served_channels = {} 

    async def main(self):
        try:           
            self.repl = await start_repl_a()
            self.startup = utils.bind(self, startup)
            print(ansi.success(f"""Starting server. Listening on port {self.port}. Visit "localhost:{self.port}/<channel_name>/<file_name>" to use."""))
            Executor.add_to_global_namespace(self.serve)
            Executor.add_to_global_namespace("app", self.app)
            self.startup()
        except Exception as e:
            self.critical_error(e)
            raise e

    def critical_error(self, e):
        console_io = ConsoleIO()
        console_io.print_critical_error("Startup failed", "Critical error in server startup!", "")
        console_io.print_exception(e)

    
    def serve(self, channel, name = None):
        if name is None:
            name = channel.serve_as
        if name in self.served_channels:
            self.served_channels[name].remove_routes(app)
        self.served_channels[name] = channel
        print(f"""Serving {channel.__name__} as "{name}".""")
        channel.serve(self.app, self.repl, config, self.host, self.port, name)