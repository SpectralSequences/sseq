import asyncio
from spectralsequence_chart import SseqSocketReceiver, SpectralSequenceChart
from spectralsequences_webserver.demo_utils import monomial_name

@main
@collect_transforms(inherit = True)
@subscribe_to("*")
class KO_HFPSS(Demo):
    async def setup_a(self, *args):
        await super().setup_a(*args)
        await self.chart.set_initial_x_range_a(0, 16)
        await self.chart.set_initial_y_range_a(0, 10)
        # self.chart.data.min_page_idx = 1

    async def run_a(self):
        await self.chart.set_x_range_a(-8, 32)
        await self.chart.set_y_range_a(0, 25)
        v_degree = 4
        self.max_eta = self.chart.y_max + v_degree
        self.min_v = (self.chart.x_min - self.chart.y_max) // v_degree
        self.max_v = self.chart.x_max // v_degree
        await self.wait_for_user_a()
        await self.make_e2_page()
        await self.wait_for_user_a()
        await self.chart.add_differential_a(3, self.classes_dict[(1,1)], self.classes_dict[(0, 4)])
        await self.wait_for_user_a()
        await self.add_differentials()

    async def make_e2_page(self):
        self.classes_dict = {}
        for v in range(self.min_v, self.max_v):
            c = None
            for i in range(self.max_eta):
                name = monomial_name(["u", v], ["\\eta", i])
                last_c = c
                c = await self.chart.add_class_a(i+4*v, i, name=name)
                self.classes_dict[(v, i)] = c
                if last_c:
                    await self.chart.add_structline_a(last_c, c)
                else:
                    c.set_field("shape", "square")
                    c.set_field("scale", 1.5)
                    await self.chart.update_a()
                    # print(c.node_list)
        

    async def add_differentials(self):
        for v in range(-9, 20, 2):
            for i in range(27):
                if (v-1, i+3) not in self.classes_dict:
                    break
                source = self.classes_dict[(v,i)]
                target = self.classes_dict[(v-1, i+3)]
                await self.chart.add_differential_a(3, source, target)
                if i==0:
                    source.replace(fill="white")


        

    


# 