from message_passing_tree import SocketChannel
from message_passing_tree.decorators import collect_transforms, subscribe_to

from fastapi.templating import Jinja2Templates

import os
from .. import config
templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))


@subscribe_to("*")
@collect_transforms(inherit=True)
class SseqChannel(SocketChannel):
    async def send_start_msg(self):
        await self.has_parent.wait()
        serving_to = self.serving_to()
        if serving_to is not None:
            await self.send_info(
                "channel.opened",
                f"""Started spectral sequence "<blue>{self.name}</blue>".\n""" +\
                f"""Visit "<blue>{self.serving_to()}</blue>" to view it."""
            )

    @classmethod
    def http_response(cls, channel_name, request):
        response_data = { 
            "port" : cls.port, 
            "directory" : cls.directory,
            "channel_name" : channel_name,
            "request" : request, 
        }
        if cls.has_channel(channel_name):
            return templates.TemplateResponse("sseq_chart.html", response_data)