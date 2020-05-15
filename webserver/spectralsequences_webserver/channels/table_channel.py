import pathlib
import json
import itertools

from message_passing_tree.prelude import *
from message_passing_tree import SocketChannel
from message_passing_tree.utils import json_stringify
from message_passing_tree import ansi


from spectralsequence_chart import SseqSocketReceiver, ChartAgent

from ..repl.executor import Executor
from .. import config

from ..process_overlay import process_overlay


from fastapi.templating import Jinja2Templates
templates = Jinja2Templates(directory=str(config.TEMPLATE_DIR))


@subscribe_to("*")
@collect_transforms(inherit=True)
class TableChannel(SocketChannel):
    def __init__(self, name, repl_agent):
        super().__init__(name)
        self.repl_agent = repl_agent
        self.executor = Executor(repl_agent)
        self.chart = ChartAgent(name)
        sseq = self.chart.sseq
        sseq.x_range = [0, 120]
        sseq.y_range = [0, 60]
        sseq.initial_x_range = [0, 60]
        sseq.initial_y_range = [0, 30]
        self.table = ProductTable()
        self.setup_executor_namespace()
        self.last_screenshot = None
        self.populate_chart()

    channels = {}
    async def send_start_msg_a(self):
        pass

    async def setup_a(self):
        await self.repl_agent.add_child_a(self.executor)
        await self.executor.add_child_a(self.chart)
        await self.chart.add_child_a(self)
        self.chart._interact_source = None
        await self.executor.load_repl_init_file_if_it_exists_a()
        
    @transform_inbound_messages
    async def transform__console__take__a(self, envelope):
        envelope.mark_used()
        self.repl_agent.set_executor(self.executor)

    @transform_inbound_messages
    async def transform__click__a(self, envelope, x, y, chart_class):
        envelope.mark_used()

    @transform_inbound_messages
    async def transform__interact__select_bidegree__a(self, envelope, bidegree):
        prod_info = self.get_product_info(bidegree)
        await self.send_message_outward_a("interact.product_info", *arguments(product_info=prod_info))
        envelope.mark_used()

    async def add_subscriber_a(self, websocket):
        recv = SseqSocketReceiver(websocket)
        await self.add_child_a(recv)
        await recv.start_a()

    def setup_executor_namespace(self):
        globals = self.executor.get_globals()
        globals["REPL"] = self.repl_agent
        globals["chart"] = self.chart
        globals["channel"] = self
        globals["table"] = self.table

    @classmethod
    async def get_channel_a(cls, name, repl):
        if name in cls.channels:
            print("Found")
            return cls.channels[name]
        print("Generating")
        new_channel = TableChannel(name, repl)
        await new_channel.setup_a()
        return new_channel


    @classmethod
    def has_channel(cls, name):
        return True #name in cls.channels or cls.get_file_path(name)

    @classmethod
    def http_response(cls, channel_name, request):
        response_data = { 
            "port" : cls.port, 
            "directory" : cls.directory,
            "channel_name" : channel_name,
            "request" : request, 
        }
        if cls.has_channel(channel_name):
            return templates.TemplateResponse("table.html", response_data)

    def populate_chart(self):
        chart = self.chart.sseq
        for (y, row) in enumerate(self.table.num_gens):
            for (x, gens) in enumerate(row):
                for _ in range(gens):
                    chart.add_class(x,y)

        for [(in1, in2), out_list] in self.table.product_table.items():
            if in2[0] < in1[0]:
                (in1,in2) = (in2, in1)
            if in1 in [(0,1,0), (1,1,0),(3,1,0)]:
                for out in out_list:
                    try:
                        chart.add_structline(chart.class_by_idx(*in2), chart.class_by_idx(*out))
                    except IndexError:
                        continue
        chart.add_structline(chart.class_by_idx(0, 0, 0), chart.class_by_idx(0, 1, 0))
        chart.add_structline(chart.class_by_idx(0, 1, 0), chart.class_by_idx(0, 2, 0))
        chart.add_structline(chart.class_by_idx(0, 2, 0), chart.class_by_idx(0, 3, 0))
        chart.add_structline(chart.class_by_idx(0, 0, 0), chart.class_by_idx(1, 1, 0))
        chart.add_structline(chart.class_by_idx(0, 0, 0), chart.class_by_idx(3, 1, 0))

        for [c,name] in self.table.class_names:
            try:
                chart.class_by_idx(*c).name = self.table.name_to_str(name)
            except IndexError:
                pass

    def get_product_info(self, bidegree):
        bidegree = tuple(bidegree)
        decompositions = self.table.decomposition_table[bidegree]
        result = []
        for (in1, in2, out) in decompositions:
            c1 = self.chart.sseq.class_by_idx(*in1)
            if c1.name:
                n1 = c1.name
            else:
                n1 = f"x_{in1[0], in1[1]}^{in1[2]}"
            c2 = self.chart.sseq.class_by_idx(*in2)
            if c2.name:
                n2 = c2.name
            else:
                n2 = f"x_{in2[0], in2[1]}^{in2[2]}"
            out_indexes = [x[-1] for x in out]
            out_vec = [0] * len(self.chart.sseq.classes_in_bidegree(*bidegree))
            for idx in out_indexes:
                out_vec[idx] = 1
            result.append((n1, n2, out_vec))
        return result

from ..name_tools import monomial_name


class ProductTable:
    def __init__(self):
        self.setup_numgens()
        self.load_product_table()
        self.generate_decomposition_table()
        self.setup_class_names()

    def s_idx_to_x_idx(self, s, idx):
        if idx == 0:
            return (0, s, 0)
        prev_gens = 0
        for [i, [gens, x]] in enumerate(self.s_indexes[s]):
            if idx <= gens:
                return (x, s, idx - prev_gens - 1)
            prev_gens = gens

    def setup_numgens(self):
        num_gens = json.loads(pathlib.Path(config.USER_DIR / "S_2-dims.json").read_text())
        self.num_gens = [[x or 0 for x in row] for row in num_gens]
        self.s_totals = [[x - 1 for x in itertools.accumulate(row)] for row in num_gens]
        self.s_indexes = [[] for _ in range(120)]
        for (i, row) in enumerate(self.s_totals):
            for (j, entry) in enumerate(row):
                if entry > row[j-1]:
                    self.s_indexes[i].append([entry, j])

    def load_product_table(self):
        product_table_json = json.loads(pathlib.Path(config.USER_DIR / "product_table.json").read_text())
        self.product_table = dict([tuple(tuple(l) for l in key), value] for [key, value] in product_table_json)

    def generate_decomposition_table(self):
        self.decomposition_table = {}
        for ((in1, in2), out) in self.product_table.items():
            key = (in1[0] + in2[0], in1[1] + in2[1])
            if key not in self.decomposition_table:
                self.decomposition_table[key] = []
            self.decomposition_table[key].append((in1, in2, out))

    def generate_product_table(self):
        all_JR = pathlib.Path(config.USER_DIR / "all.JR.txt").read_text()
        self.ajr_lines = [l for l in all_JR.splitlines() if l]
        product_table = {}
        self.product_table = product_table
        for s in self.ajr_lines:
            l = s.replace("(", "").split()
            try:
                output = [int(x) for x in l[:2]]
                in1 = [int(x) for x in l[2:4]]
                in2 = [int(x) for x in l[-1].split("_")]
            except ValueError:
                print(l)
                break
            [output, in1, in2] = [self.s_idx_to_x_idx(*x) for x in (output, in1, in2)]
            if None in [output, in1, in2]:
                continue
            if (in1, in2) not in product_table:
                product_table[(in1,in2)] = []
            product_table[(in1,in2)].append(output)
            

    def setup_class_names(self):
        self.class_names = json.loads(pathlib.Path(config.USER_DIR / "class_names_parsed.json").read_text())
        self.class_name_table = dict([tuple(rest,),name] for [rest, name] in self.class_names)
        self.gen_degs = {}
        for [t, n] in self.class_names:
            if len(n) == 1 and n[0][1] == 1:
                self.gen_degs[n[0][0]] = t
        self.gen_degs["P"] = [0,0,0]
    
    def name_to_str(self, name):
        return monomial_name(*sorted(name, key=lambda x : self.gen_degs[x[0]] if x[0] in self.gen_degs else [10000, 10000]))
