from js import sendMessage as js_send_message

def send_message(cmd, uuid, **kwargs):
    kwargs.update(cmd=cmd, uuid=uuid)
    js_send_message(kwargs)