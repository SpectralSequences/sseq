from message_passing_tree import SocketChannel
from message_passing_tree.decorators import collect_transforms, subscribe_to

@subscribe_to("*")
@collect_transforms(inherit=True)
class SseqSocketChannel(SocketChannel):
    async def send_start_msg(self):
        await self.has_parent.wait()
        serving_to = self.serving_to()
        if serving_to is not None:
            await self.send_info(
                "channel.opened",
                f"""Started spectral sequence "<blue>{self.name}</blue>".\n""" +\
                f"""Visit "<blue>{self.serving_to()}</blue>" to view it."""
            )