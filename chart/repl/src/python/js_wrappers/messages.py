from js import (
    sendMessage as js_send_message, 
    messageLookup as js_message_lookup,
)

async def send_message_a(cmd, uuid, **kwargs):
    kwargs.update(cmd=cmd, uuid=uuid)
    js_send_message(kwargs)

def get_message(message_id):
    message = dict(js_message_lookup[message_id])
    del js_message_lookup[message_id]
    return message