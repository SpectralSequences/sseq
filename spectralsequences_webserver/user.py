import json
from starlette.websockets import WebSocketDisconnect

from .decorators import handler_class, handler
from . import utils
from .channel import Channel

import logging

logger = logging.getLogger("mine")

class User:
    def __init__(self, channel_name, websocket):
        self.channel = Channel.dict[channel_name]
        self.websocket = websocket
        self.id = -1

    async def run(self):
        await self.websocket.accept() 
        if self.channel is None:
            await self.abort()
            return
        else:
            #await
            self.channel.add_user(self)
        while await self.main_loop():
            pass
        
    async def abort(self):
        await self.send_text(json.dumps({
            "cmd" : "invalid_channel"
        }))

    async def main_loop(self):
        try:
            try:
                json_str = await self.websocket.receive_text()
            except WebSocketDisconnect:
                await self.cleanup()
                return False 
            logger.info("User sent: " + str(json_str))
            data = json.loads(json_str)
            data["user_id"] = self.id
            if "cmd" not in data:
                utils.print_error(f"""MissingCommandError: Client sent message missing "cmd" key.""")
            elif self.channel.has_handler(data["cmd"]):
                await self.channel.handle_message(data["cmd"], data)
            else:
                utils.print_error(f"""UnknownCommandError: Client sent unrecognized command "{data["cmd"]}".""")
            return True
        except Exception as e:
            print(e)
            await self.cleanup()
            raise

    async def cleanup(self):
        self.channel.remove_user(self)

    async def close(self, code=1000):
        await self.websocket.close(code=code) # what does "code=1000" do? 

    async def send_text(self, text):
        await self.websocket.send_text(text)

    async def dispatch_text_to_channel(self, text):
        await self.channel.dispatch_text(text)