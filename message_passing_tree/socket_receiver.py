import asyncio
import json
from starlette.websockets import WebSocketDisconnect
from websockets.exceptions import ConnectionClosedOK

from uuid import UUID, uuid4

from .exceptions import *

from . import Agent, Receiver
from .prelude import *

from .utils import (json_stringify, arguments)

@collect_transforms(inherit=True) #inherit "all" handler
@subscribe_to("*")
class SocketReceiver(Receiver):
    def __init__(self, ws):
        super().__init__()
        self.socket = ws
        # print(type(self.socket))
        self.uuid = uuid4()
        self.accepted_connection = asyncio.Event()
        self.initialized_client = asyncio.Event()
        # print("new connection")

    def get_uid(self) -> UUID:
        return self.uuid

    async def send_message_to_socket_a(self, cmd, *args, **kwargs):
        if not self.accepted_connection.is_set():
            return
        if not self.initialized_client.is_set() and cmd.part_list[0] != "initialize":
            # Try again and hope for the best?
            # Maybe we should queue these so they don't get reordered.
            # Usually this case won't happen, but when it does it might happen many times in a row.
            asyncio.ensure_future(self.send_message_to_socket_a(cmd, *args, **kwargs))
            return
        msg = { "cmd" : cmd.filter_list, "args" : args, "kwargs" : kwargs }
        try:
            await self.socket.send_text(json_stringify(msg))
        except ConnectionClosedOK:
            self.log_warning("Connection closed while trying to send message to socket.")
            self.log_warning(f"Message: {msg}")

    async def close_connection_a(self):
        pass

    async def start_a(self):
        await self.run_a()

    async def run_a(self):
        if self.accepted_connection.is_set():
            self.log_warning("Already accepted connection for some reason? This will become an error when I have time to fix it.")
            return
        else:
            await self.socket.accept()
            self.accepted_connection.set()
        continue_running = True
        consecutive_failed_passes = 0
        while continue_running:
            try:
                continue_running = await self.main_loop_a()
                consecutive_failed_passes = 0
            except Exception as e:
                consecutive_failed_passes += 1
                await self.handle_exception_a(e)
                # TODO: what's the right threshold?
                if consecutive_failed_passes > 1: 
                    return
        await self.shutdown_a()


    async def abort_a(self):
        await self.send_text_a(json.dumps({
            "cmd" : "invalid_channel"
        }))

    async def main_loop_a(self):
        try:
            json_str = await self.socket.receive_text()
        except WebSocketDisconnect:
            self.log_info("Disconnect")
            return False 
        self.log_info("User sent: " + str(json_str))
        data = json.loads(json_str)
        if "cmd" not in data:
            raise MessageMissingCommandError(data)
        elif "args" not in data:
            raise MessageMissingArgumentsError("args", data)
        elif "kwargs" not in data:
            raise MessageMissingArgumentsError("kwargs", data)
        else:
            await self.send_message_inward_a(data["cmd"], data["args"], data["kwargs"])
        return True

    @transform_inbound_messages
    async def consume_initialize__complete_a(self, source_agent_path, cmd):
        # print("Client says it is initialized.")
        self.initialized_client.set()

    @transform_outbound_messages
    async def consume_initialize_a(self, source_agent_id, cmd, **kwargs):
        await self.send_message_to_socket_a(cmd, **kwargs)

    @transform_outbound_messages
    async def consume_interact_a(self, source_agent_id, cmd, **kwargs):
        await self.send_message_to_socket_a(cmd, **kwargs)

    @transform_inbound_messages
    async def transform_debug_a(self, source_agent_path, cmd, text, orig_msg=None):
        if orig_msg is None:
            additional_info = None
        else:
            additional_info = f"""============ : Original Message : {orig_msg}"""
        cmd.insert(1, "client")
        return [cmd, *arguments(text, additional_info=additional_info)]

    @transform_inbound_messages
    async def transform_info_a(self, source_agent_path, cmd, text, orig_msg=None):
        if orig_msg is None:
            additional_info = None
        else:
            additional_info = f"""============ : Original Message : {orig_msg}"""
        cmd.insert(1, "client")
        return [cmd, *arguments(text, additional_info=additional_info)]
        

    @transform_inbound_messages
    async def transform_warning_a(self, source_agent_path, cmd, text, orig_msg=None, stack_trace=None):
        if orig_msg is None:
            additional_info = None
        else:
            additional_info = f"""============ : Original Message : {orig_msg}"""
        cmd.insert(1, "client")
        return [cmd, *arguments(text, additional_info=additional_info, stack_trace=stack_trace)]

    @transform_inbound_messages
    async def transform_error__client_a(self, source_agent_path, cmd, orig_msg, exception=None):
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
