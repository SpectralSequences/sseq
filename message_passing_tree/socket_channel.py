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
        """ This is used by server.py to look up what channel a websocket should
            subscribe to. If server receives a requet to /ws/class_directory/{name}
            it will call this function. Return value should be a SocketChannel 
            instance.
        """
        if name in cls.channels:
            return cls.channels[name]

    @classmethod
    def serve_channel_to(cls, host, port, directory):
        cls.host = host
        cls.port = port
        cls.directory = directory
        cls.serving_class_to = f"{host}:{port}/{directory}"
        cls.initialize_channel()

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
        asyncio.ensure_future(self.send_start_msg())

    def serving_to(self):
        if self.serving_class_to is None:
            return None
        else:
            return self.serving_class_to + f"/{self.name}"

    async def handle_leaked_envelope(self, envelope):
        self.log_info(f"""Leaked envelope self: {self.info()}  envelope: {envelope.info()}""")


    async def add_subscriber(self, sock_recv):
        """ For overriding by subclasses. 
            Will be called by server when it gets a request to /ws/class_directory/channel_name.
            Channels are in charge of assembling the connection to the SocketReceiver themselves.
        """
        await self.add_child(sock_recv)