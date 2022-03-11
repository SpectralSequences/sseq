from js import messageLookup as js_message_lookup
from js import sendMessage as js_send_message
from js.Object import fromEntries as to_js_object
from pyodide import to_js


async def send_message_a(cmd, uuid, **kwargs):
    kwargs.update(cmd=cmd, uuid=uuid)
    js_send_message(to_js(kwargs, dict_converter=to_js_object))


def get_message(message_id):
    message = js_message_lookup[message_id].to_py()
    del js_message_lookup[message_id]
    return message
