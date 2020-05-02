import logging
logger = logging.getLogger(__name__)

from . import Agent, AgentID, AgentPath, CmdList
from .decorators import collect_transforms, transform_outbound_messages

@collect_transforms(inherit=False) # nothing to inherit 
class Receiver(Agent):
    async def add_child_a(self, recv : Agent):
        raise RuntimeError("Receiver cannot have children.")
        
    async def remove_child_a(self, rec : Agent):
        raise RuntimeError("Receiver cannot have children.")

    @transform_outbound_messages
    async def transform__all__a(self, envelope, *args, **kwargs ):
        raise RuntimeError(
            f"""Receiver failed to consume outbound message.\n""" +\
            f""" cmd : {cmd}, args : {args}, kwargs : {kwargs}"""
        )
