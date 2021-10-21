import asyncio
from spectralsequence_chart import SseqSocketReceiver, SpectralSequenceChart


@main
@collect_handlers(inherit = True)
@subscribe_to("*")
class TestDemo(Demo):
    async def setup_a(self, websocket):
        self.socket = SseqSocketReceiver(websocket)
        self.chart = SpectralSequenceChart("demo")
        await self.chart.add_child_a(self.socket)
        await self.executor.add_child_a(self.chart)
        await self.add_child_a(self.executor)
        asyncio.ensure_future(self.socket.run_a())

    def get_socket(self):
        return self.socket

    async def run_a(self):
        wait_for_user_a = self.wait_for_user_a
        chart = self.chart
        print("started test.py")
        await wait_for_user_a()

        c = await chart.add_class_a(0,0)

        await wait_for_user_a()

        d = await chart.add_class_a(1,1)
        await chart.add_structline_a(c, d)

        await wait_for_user_a()

        c = await chart.add_class_a(2,2)
        await chart.add_structline_a(d, c)
# 
