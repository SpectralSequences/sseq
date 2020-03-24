import asyncio

from .decorators import handler_class, handler
from . import utils

@handler_class
class Channel:
    dict = {}

    @staticmethod
    def is_channel(name):
        return name in Channel.dict

    def __init__(self, name):
        self.name = name
        self.users = {}
        self.next_user_id = 0
        self.task = None
        Channel.dict[self.name] = self
        self.print_started_msg()

    def print_started_msg(self):
        pass

    def add_user(self, user):
        user.id = self.next_user_id
        self.next_user_id += 1
        self.users[user.id] = user

    def remove_user(self, user):
        del self.users[user.id]

    async def send_message_to_user(self, uid, msg):
        await self.users[uid].send_text(utils.json_stringify(msg))

    async def broadcast_message(self, msg):
        # msg = utils.json_stringify({"cmd" : cmd, "data" : data})
        for user in self.users.values():
            await user.websocket.send_text(utils.json_stringify(msg))

    async def broadcast_command(self, cmd, **kwargs):
        msg = {"cmd" : cmd, **kwargs}
        await self.broadcast_message(msg)
        
    async def send_command_to_user(self, uid, cmd, **kwargs):
        msg = {"cmd" : cmd, **kwargs}
        await self.send_message_to_user(uid, msg)
