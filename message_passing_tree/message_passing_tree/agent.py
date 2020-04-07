import asyncio
import logging 
logger = logging.getLogger(__name__)
import inspect
from typing import List, Optional
# from abc import ABC, abstractmethod
from uuid import UUID, uuid4

from . import ansi
from .utils import arguments

CmdStr = str
CmdList = List[str]
AgentID = UUID
AgentPath = List[UUID]

class Command:
    def set_str(self, cmd_str):
        self.str = cmd_str
        self.filter_list = Command.cmdstr_to_filter_list(self.str)
        self.part_list = self.str.split(".")
        return self
    
    def set_filter_list(self, filter_list):
        self.str = filter_list[0]
        self.filter_list = filter_list
        self.part_list = self.str.split(".")
        return self

    def set_part_list(self, part_list):
        self.str = ".".join(part_list)
        self.filter_list = Command.cmdstr_to_filter_list(self.str)
        self.part_list = part_list
        return self

    @staticmethod
    def cmdstr_to_filter_list(cmd):
        # We use "__" as a standin for "." in "command filter identifiers"
        # Just in case, convert any "__" back to "."
        cmd = cmd.replace("__", ".") # TODO: is this a good choice?
        result = [cmd]
        while( (idx := cmd.rfind(".")) >= 0):
            cmd = cmd[ : idx]
            result.append(cmd)
        result.append("*")
        return result

    def __repr__(self):
        return f"""Command("{self.str}")"""



class Message:
    def __init__(self, cmd, args, kwargs):
        # Don't allow top level keys sharing a name with the arguments of transformers.
        for illegal_top_level_key in ["cmd", "source_agent_path", "source_agent_id"]:
            if illegal_top_level_key in kwargs:
                raise ValueError(
                    f"""Cannot use key "{illegal_top_level_key}" in top level of message. Ignoring this message:\n""" +\
                    f"""cmd : {cmd}, args : {args}, kwargs : {kwargs}"""
                )
        self.cmd = cmd
        self.args = args
        self.kwargs = kwargs
    
    def to_json(self):
        return { "cmd" : self.cmd.filter_list, "args" : self.args, "kwargs" : self.kwargs }

    def __repr__(self):
        return f"""Message(cmd: "{self.cmd.str}", "args": {self.args}, "kwargs": {self.kwargs})"""

class Envelope:
    def __init__(self, direction, msg, *, 
        source_agent_id = None, source_agent_path = None,
        target_agent_id = None, target_agent_path = None,
    ):
        if direction not in ["in", "out"]:
            raise TypeError(
                f"""Expected argument "direction" to have value "in" or "out" not "{direction}"."""
            )
        if direction == "in" and (source_agent_path is None or source_agent_id is not None):
            raise TypeError(
                f"""Inbound envelope should have a "source_agent_path" and no "source_agent_id"."""
            )
        if direction == "out" and (source_agent_id is None or source_agent_path is not None):
            raise TypeError(
                f"""Outbound envelope should have a "source_agent_id" and no "source_agent_path"."""
            )
        if direction == "in" and target_agent_path is not None:
            raise TypeError(
                f"""Inbound envelope should not have a "target_agent_path"."""
            )
        if direction == "out" and target_agent_id is not None:
            raise TypeError(
                f"""Outbound envelope should not have a "target_agent_id"."""
            )            
        self.msg = msg
        self.source_agent_id = source_agent_id
        self.source_agent_path = source_agent_path
        self.target_agent_id = target_agent_id
        self.target_agent_path = target_agent_path

    def info(self):
        return f"""cmd: {ansi.highlight(self.msg.cmd.str)} args: {ansi.info(self.msg.args)} kwargs: {ansi.info(self.msg.kwargs)}"""

class Agent:
    outward_transformers = None
    inward_transformers = None
    subscriptions = None

    @classmethod
    def get_transformer(cls, transform_dict, cmd):
        for cmd_filter in cmd.filter_list:
            if cmd_filter in transform_dict:
                return transform_dict[cmd_filter]
        return None

    def __init__(self):
        if type(self).inward_transformers is None:
            raise RuntimeError(f"""You forgot to use "@collect_transforms" on {type(self).__name__}.""")
        if type(self).subscriptions is None:
            raise RuntimeError(f"""You forgot to use "@subscribe_to(...)" on {type(self).__name__}.""")
        self.parent = None
        self.has_parent = asyncio.Event()
        self.uuid = uuid4()
        logger.debug(f"new {type(self).__name__} uuuid: {self.uuid}")
        self.cached_path = None
        self.children = {}
        self.outward_transformers = {}
        self.inward_transformers = {}
        # self.inward_responses_expected = []
        # self.inward_responses_expected_lock = asyncio.Lock()
        # self.outward_responses_expected = []
        # self.outward_responses_expected_lock = asyncio.Lock()        

    @classmethod
    def log_debug(cls, msg):
        logging.getLogger(cls.__module__).debug(msg)

    @classmethod
    def log_info(cls, msg):
        logging.getLogger(cls.__module__).info(msg)

    @classmethod
    def log_warning(cls, msg):
        logging.getLogger(cls.__module__).warning(msg)

    @classmethod
    def log_error(cls, msg):
        logging.getLogger(cls.__module__).error(msg)

    def info(self):
        return f"mytype: {ansi.highlight(type(self).__name__)}"

    def log_envelope_task(self, name, envelope):
        self.log_debug(self.envelope_task_info(name, envelope))

    def envelope_task_info(self, name, envelope):
        return f"""Task: {ansi.highlight(name)}  self: {self.info()}  envelope: {envelope.info()}"""

    def handle_leaked_envelope(self, envelope):
        raise RuntimeWarning(f"""Leaked envelope self: {self.info()}  envelope: {envelope.info()}""")

    def get_uuid(self) -> UUID:
        return self.uuid

    def add_transformer(self, cmd : str, direction):
        if not hasattr(self, f"{direction}ward_transformers"):
            raise ValueError(f"""Direction should be "in" or "out" not "{direction}".""")
        def helper(transformer):
            getattr(self, f"{direction}ward_transformers")[cmd] = transformer
            return transformer
        return helper

    async def add_child_a(self, recv):
        logger.debug(f"Adding child {type(recv).__name__} to {type(self).__name__}")
        recv.has_parent.clear()
        self.children[recv.get_uuid()] = recv
        old_parent = recv.parent
        recv.parent = self
        await recv.new_parent_a(old_parent)
        recv.has_parent.set()

    async def remove_child_a(self, recv):
        recv.has_parent.clear()
        recv.parent = None
        del self.children[recv.get_uuid()]

    async def new_parent_a(self, old_parent):
        pass

    # TODO: Should this be here?
    async def run_a(self):
        pass

    # TODO: Should this be here?
    async def shutdown_a(self):
        # print("shutdown")
        if self.parent:
            await self.parent.remove_child_a(self)

    def is_subscribed_to(self, cmd):
        for subcmd in reversed(cmd):
            if subcmd in self.subscriptions:
                return True
        return False

    async def transform_outbound_envelope_a(self, envelope : Envelope):
        self.log_envelope_task("transform_outbound_envelope", envelope)
        transform_a = self.get_transformer(self.outward_transformers, envelope.msg.cmd)
        if transform_a is None:
            transform_a = self.get_transformer(type(self).outward_transformers, envelope.msg.cmd)
        if transform_a is None:
            return False
        return await transform_a(self, envelope)

    async def transform_inbound_envelope_a(self, envelope):
        # async with self.inward_responses_expected_lock:
            # for (i, (cmd_filter, evt)) in enumerate(self.inward_responses_expected):
                # if cmd_filter in envelope.msg.cmd.filter_list:
                    # return True
        transform_a = Agent.get_transformer(self.inward_transformers, envelope.msg.cmd)
        if transform_a is None:
            transform_a = Agent.get_transformer(type(self).inward_transformers, envelope.msg.cmd)
        if transform_a is None:
            return False
        return await transform_a(self, envelope)

    async def pass_envelope_inward_a(self, envelope):
        self.log_envelope_task("pass_envelope_inward", envelope)
        consume = await self.transform_inbound_envelope_a(envelope)
        if consume:
            return
        if envelope.target_agent_id == self.uuid:
            raise RuntimeError(f"""Unconsumed message with command "{envelope.msg.cmd.str}" targeted to me.""")
        if self.parent is None:
            raise RuntimeError(f"""Unconsumed message with command "{envelope.msg.cmd.str}" hit root node.""")
        envelope.source_agent_path.append(self.uuid)
        await self.parent.pass_envelope_inward_a(envelope)        
        
    async def pass_envelope_outward_a(self, envelope):
        self.log_envelope_task("pass_envelope_outward", envelope)
        consume = await self.transform_outbound_envelope_a(envelope)
        if consume:
            return  
        children_to_pass_to = self.pass_envelope_outward_get_children_to_pass_to(envelope)
        if not children_to_pass_to:
            # TODO: extra logging info about subscriber filters!
            await self.handle_leaked_envelope_a(envelope)
            # raise RuntimeWarning("Leaked message") # TODO: should be a warning.
        for recv in children_to_pass_to:
            await recv.pass_envelope_outward_a(envelope)        
   
    def pass_envelope_outward_get_children_to_pass_to(self,  envelope):
        if envelope.target_agent_path is None or len(envelope.target_agent_path) == 0:
            # TODO: extra logging info about subscriber filters!
            return [recv for recv in self.children.values() if recv.is_subscribed_to(envelope.msg.cmd.filter_list)]
        child_uuid = envelope.target_agent_path.pop()
        if child_uuid not in self.children:
            raise RuntimeError(f"""I don't have a child with id "{child_uuid}".""")
        return [self.children[child_uuid]]


    async def handle_leaked_envelope_a(self, envelope):
        self.log_warning(f"""Leaked envelope self: {self.info()}  envelope: {envelope.info()}""")
        self.log_warning(f"""=== Leaked envelope {self.children}""")


    async def send_message_inward_a(self, 
        cmd_str, args, kwargs,
        target_agent_id : Optional[AgentID] = None
    ):
        cmd = Command().set_str(cmd_str)
        message = Message(cmd, args, kwargs)
        envelope = Envelope("in", message, source_agent_path = [], target_agent_id = target_agent_id)
        self.log_envelope_task("send_message_inward", envelope)
        await self.pass_envelope_inward_a(envelope)

    async def send_message_outward_a(self, 
        cmd_str, args, kwargs, *,
        target_agent_path : Optional[AgentPath] = None
    ):
        self.log_debug(f"smo {cmd_str}")
        cmd = Command().set_str(cmd_str)
        message = Message(cmd, args, kwargs)
        envelope = Envelope("out", message, source_agent_id = self.uuid, target_agent_path = target_agent_path)
        self.log_envelope_task("send_message_outward", envelope)
        await self.pass_envelope_outward_a(envelope)
    
    async def broadcast_a(self, 
        cmd : CmdStr,
        args, kwargs
    ):
        await self.send_message_outward_a(cmd, args, kwargs)

    # async def get_response_a(self, cmd_filter):
        # async with self.inward_responses_expected_lock:
            # event = asyncio.Event()
            # self.inward_responses_expected.append((cmd_filter, event))
        # return event.wait()


    async def send_debug_a(self, msg_type, msg):
        cmd = "debug"
        if msg_type != "":
            cmd = f"{cmd}.{msg_type}"
        await self.send_message_inward_a(cmd, *arguments(msg=msg))

    async def send_info_a(self, msg_type, msg):
        cmd = "info"
        if msg_type != "":
            cmd = f"{cmd}.{msg_type}"        
        await self.send_message_inward_a(cmd, *arguments(msg=msg))

    async def send_warning_a(self, msg_type, msg):
        cmd = "warning"
        if msg_type != "":
            cmd = f"{cmd}.{msg_type}"        
        await self.send_message_inward_a(cmd, *arguments(msg=msg))

    async def send_error_a(self, msg_type, msg=None, exception=None):
        cmd = "error"
        if msg_type != "":
            cmd = f"{cmd}.{msg_type}"
        await self.send_message_inward_a(cmd, *arguments(msg=msg, exception=exception))

    async def handle_exception_a(self, exception):
        print("handle_exception_a:", exception)
        try:
            # raise RuntimeError("Double fault test")
            await self.parent.send_error_a("exception." + type(exception).__name__, exception=exception)
        except Exception as double_fault:
            if hasattr(self, "double_fault_handler"):
                self.double_fault_handler(double_fault)
            else:
                raise