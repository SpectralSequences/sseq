import asyncio
import json
from starlette.websockets import WebSocketDisconnect
from websockets.exceptions import ConnectionClosedOK

from uuid import UUID, uuid4

from .exceptions import *

from . import Agent, Receiver
from .prelude import *

from .utils import (json_stringify, arguments)

@collect_handlers(inherit=True) #inherit "all" handler
@subscribe_to("*")
class SocketReceiver(Receiver):
    def __init__(self, ws):
        super().__init__()
        self.socket = ws
        self.uuid = uuid4()
        self.accepted_connection = asyncio.Event()
        self.initialized_client = asyncio.Event()
        # print("new connection")

    def get_uid(self) -> UUID:
        return self.uuid

    async def send_message_to_socket_a(self, envelope):
        if not self.accepted_connection.is_set():
            return
        cmd = envelope.msg.cmd
        if not self.initialized_client.is_set() and cmd.part_list[0] != "initialize":
            # Try again and hope for the best?
            # Maybe we should queue these so they don't get reordered.
            # Usually this case won't happen, but when it does it might happen many times in a row.
            asyncio.ensure_future(self.send_message_to_socket_a(envelope))
            # self.schedule_coroutine(self.send_message_to_socket_a(envelope))
            return
        msg = { "cmd" : cmd.filter_list, "args" : envelope.msg.args, "kwargs" : envelope.msg.kwargs }
        try:
            await self.socket.send_text(json_stringify(msg))
        except ConnectionClosedOK:
            pass
            # self.log_warning("Connection closed while trying to send message to socket.")
            # self.log_warning(f"Message: {msg}")
        except RuntimeError as e:
            # Annoyingly the ASGI server only throws RuntimeError so we have to inspect the message text to decide
            # what sort of error it is.
            if e.args[0].find("websocket.close"):
                await self.shutdown_a()
            else:
                raise

    async def close_a(self, close_code):
        await self.socket.close(close_code)

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
            except TypeError as e:
                # TODO: what's the right threshold?
                consecutive_failed_passes += 1
                if e.args[0] != "An asyncio.Future, a coroutine or an awaitable is required":
                    await self.handle_exception_a(e)               
                if consecutive_failed_passes > 2: 
                    print("Too many errors, quitting!")
                    return                    
            except Exception as e:
                await self.handle_exception_a(e)
                # TODO: what's the right threshold?
                consecutive_failed_passes += 1
                if consecutive_failed_passes > 2: 
                    print("Too many errors, quitting!")
                    return
        await self.shutdown_a()


    # # TODO: Should this be here?
    async def shutdown_a(self):
        await self.send_message_inward_a("socket.close", *arguments())

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

    @handle_inbound_messages
    async def handle__initialize__complete__a(self, envelope):
        # print("Client says it is initialized.")'
        envelope.mark_used()
        self.initialized_client.set()

    @handle_outbound_messages
    async def handle__initialize__a(self, envelope, **kwargs):
        envelope.mark_used()
        await self.send_message_to_socket_a(envelope)

    @handle_outbound_messages
    async def handle__interact__a(self, envelope, **kwargs):
        envelope.mark_used()
        await self.send_message_to_socket_a(envelope)

    @handle_inbound_messages
    async def handle__debug__a(self, envelope, text, orig_msg=None):
        if orig_msg is None:
            additional_info = None
        else:
            additional_info = f"""============ : Original Message : {orig_msg}"""
        part_list = envelope.msg.cmd.part_list
        part_list.insert(1, "client")
        envelope.msg.cmd.set_part_list(part_list)
        envelope.msg.update_arguments(additional_info=additional_info)

    @handle_inbound_messages
    async def handle__info__a(self, envelope, text, orig_msg=None):
        if orig_msg is None:
            additional_info = None
        else:
            additional_info = f"""============ : Original Message : {orig_msg}"""
        part_list = envelope.msg.cmd.part_list
        part_list.insert(1, "client")
        envelope.msg.cmd.set_part_list(part_list)
        envelope.msg.update_arguments(additional_info=additional_info)
        

    @handle_inbound_messages
    async def handle__warning__a(self, envelope, text, orig_msg=None, stack_trace=None):
        if orig_msg is None:
            additional_info = None
        else:
            additional_info = f"""============ : Original Message : {orig_msg}"""
        part_list = envelope.msg.cmd.part_list
        part_list.insert(1, "client")
        envelope.msg.cmd.set_part_list(part_list)
        envelope.msg.update_arguments(additional_info=additional_info)

    @handle_inbound_messages
    async def handle__error__client__a(self, envelope, orig_msg, exception=None):
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
        part_list = envelope.msg.cmd.part_list
        part_list.insert(1, "additional_info")
        envelope.msg.cmd.set_part_list(part_list)        
        envelope.msg.update_arguments(msg=exception["msg"], additional_info=additional_info)
        envelope.msg.del_arguments(["exception", "orig_msg"])