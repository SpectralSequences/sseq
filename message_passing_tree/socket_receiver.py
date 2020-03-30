import json
from starlette.websockets import WebSocketDisconnect
from uuid import UUID, uuid4
import logging
logger = logging.getLogger(__name__)

from .exceptions import *

from . import Agent, Receiver
from .agent import Command
from .decorators import subscribe_to, collect_transforms, transform_inbound_messages
from .utils import (json_stringify, arguments)

@collect_transforms(inherit=True) #inherit "all" handler
@subscribe_to("*")
class SocketReceiver(Receiver):
    def __init__(self, ws):
        super().__init__()
        self.socket = ws
        self.uuid = uuid4()
        print("new connection")

    def get_uid(self) -> UUID:
        return self.uuid

    async def send_message_to_socket(self, cmd, *args, **kwargs):
        msg = { "cmd" : cmd.filter_list, "args" : args, "kwargs" : kwargs }
        await self.socket.send_text(json_stringify(msg))

    async def close_connection(self):
        pass

    async def run(self): 
        await self.socket.accept()
        continue_running = True
        while continue_running:
            try:
                continue_running = await self.main_loop()
            except Exception as e:
                await self.handle_exception(e)
        await self.shutdown()


    async def abort(self):
        await self.send_text(json.dumps({
            "cmd" : "invalid_channel"
        }))

    async def main_loop(self):
        try:
            json_str = await self.socket.receive_text()
        except WebSocketDisconnect:
            logger.debug("Disconnect")
            return False 
        logger.info("User sent: " + str(json_str))
        data = json.loads(json_str)
        if "cmd" not in data:
            raise MessageMissingCommandError(data)
        elif "args" not in data:
            raise MessageMissingArgumentsError("args", data)
        elif "kwargs" not in data:
            raise MessageMissingArgumentsError("kwargs", data)
        else:
            await self.send_message_inward(data["cmd"], data["args"], data["kwargs"])
        return True

    @transform_inbound_messages
    async def transform_debug(self, source_agent_path, cmd, text, orig_msg=None):
        if orig_msg is None:
            additional_info = None
        else:
            additional_info = f"""============ : Original Message : {orig_msg}"""
        cmd.insert(1, "client")
        return [cmd, *arguments(text, additional_info=additional_info)]

    @transform_inbound_messages
    async def transform_info(self, source_agent_path, cmd, text, orig_msg=None):
        if orig_msg is None:
            additional_info = None
        else:
            additional_info = f"""============ : Original Message : {orig_msg}"""
        cmd.insert(1, "client")
        return [cmd, *arguments(text, additional_info=additional_info)]
        

    @transform_inbound_messages
    async def transform_warning(self, source_agent_path, cmd, text, orig_msg=None, stack_trace=None):
        if orig_msg is None:
            additional_info = None
        else:
            additional_info = f"""============ : Original Message : {orig_msg}"""
        cmd.insert(1, "client")
        return [cmd, *arguments(text, additional_info=additional_info, stack_trace=stack_trace)]

    @transform_inbound_messages
    async def transform_error__client(self, source_agent_path, cmd, orig_msg, exception=None):
        # raise RuntimeError("Test error")
        if orig_msg is None:
            additional_info = ""
        else:
            orig_msg_args = json.dumps(orig_msg["kwargs"])
            if len(orig_msg_args) > 240:
                orig_msg_args = orig_msg_args[:240] + "... <<truncated output>>"
            additional_info = f"""== Original Message : cmd: {orig_msg["cmd"][0]} kwargs: {orig_msg_args}\n"""
        if "stack" in exception:
            additional_info += "== Javascript stacktrace: \n"
            additional_info += exception["stack"]
        cmd.part_list.insert(1, "additional_info")
        cmd.set_part_list(cmd.part_list)
        return [cmd, *arguments(msg=exception["msg"], additional_info=additional_info)]
