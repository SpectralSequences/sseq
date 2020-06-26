from spectralsequences_webserver.name_tools import monomial_name

@main
class KO_HFPSS(Demo):
    async def setup_a(self, *args):
        await super().setup_a(*args)
        self.sseq.initial_x_range = [-4, 12]
        self.sseq.initial_y_range = [0, 6]
        self.sseq.x_range = [-8, 32]
        self.sseq.y_range = [0, 25]
        self.sseq.min_page_idx = 1

    async def run_a(self):
        v_degree = 4
        self.max_eta = self.sseq.y_max + v_degree
        self.min_v = (self.sseq.x_min - self.sseq.y_max) // v_degree
        self.max_v = self.sseq.x_max // v_degree
        
        
        self.make_e2_page()
        await self.chart.update_a()
        await self.wait_for_user_a()
        
        self.sseq.add_differential(3, self.classes_dict[(1,1)], self.classes_dict[(0, 4)])
        await self.chart.update_a()
        await self.wait_for_user_a()

        self.add_differentials()
        await self.chart.update_a()
        await self.wait_for_user_a()

    def make_e2_page(self):
        self.classes_dict = {}
        for v in range(self.min_v, self.max_v):
            c = None
            for i in range(self.max_eta):
                name = monomial_name(["u", v], ["\\eta", i])
                last_c = c
                c = self.sseq.add_class(i+4*v, i, name=name)
                self.classes_dict[(v, i)] = c
                if last_c:
                    self.sseq.add_structline(last_c, c)
                else:
                    c.set_field("shape", "square")
                    c.set_field("scale", 1.5)
                    # print(c.node_list)
        

    def add_differentials(self):
        for v in range(-9, 20, 2):
            for i in range(27):
                if (v-1, i+3) not in self.classes_dict:
                    break
                source = self.classes_dict[(v,i)]
                target = self.classes_dict[(v-1, i+3)]
                self.sseq.add_differential(3, source, target)
                if i==0:
                    source.replace(fill="white")