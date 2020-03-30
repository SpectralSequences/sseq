import asyncio
from . import utils
from .agent import Agent
from .decorators import subscribe_to

@subscribe_to("*")
class SocketChannel(Agent):
    channels = {}
    serving_class_to = None
    @classmethod
    def has_channel(cls, name):
        return name in cls.channels

    @classmethod
    def get_channel(cls, name):
        return cls.channels[name]

    def __init__(self, name):
        super().__init__()
        self.name = name
        type(self).channels[name] = self
        asyncio.ensure_future(self.send_start_msg())

    def serving_to(self):
        if self.serving_class_to is None:
            return None
        else:
            return self.serving_class_to + f"/{self.name}"

    async def send_start_msg(self):
        pass # await self.info("")

    async def receiver_error(self, data):
        self.error(data) # TODO: Fix me.
    
    async def handle_leaked_envelope(self, envelope):
        self.log_info(f"""Leaked envelope self: {self.info()}  envelope: {envelope.info()}""")