from ..page_property import PageProperty
from ..infinity import INFINITY
from uuid import uuid4
from .. import utils

class ChartClass:
    def __init__(self, sseq, **kwargs):
        if "uuid" in kwargs:
            self.uuid = kwargs["uuid"]
        else:
            self.uuid = str(uuid4())
        sseq.add_batched_message(self.uuid, "chart.class.add", *utils.arguments(new_class=self))
        self._sseq = sseq
        self._edges = []
        utils.copy_fields_from_kwargs(self, kwargs)

        # TODO: figure out what to do about uuids and multiple reads from same file.
        # We'd then get multiple objects with same uuid from file.
        # But if we replace all uuids on load, we will loose the capability of being able to track
        # a chart entity through multiple different save files.

        if "name" not in kwargs:
            self.name = PageProperty("")
        
        if "shape" not in kwargs:
            self.shape = PageProperty("default")

        if "color" not in kwargs:
            self.color = PageProperty("default")

        if "fill" not in kwargs:
            self.fill = PageProperty("default")

        if "stroke" not in kwargs:
            self.stroke = PageProperty("default")

        if "scale" not in kwargs:
            self.scale = PageProperty(1)

        if "opacity" not in kwargs:
            self.opacity = PageProperty(1)


        if "visible" not in kwargs:
            self.visible = PageProperty(True)

        if "degree" not in kwargs:
            raise ValueError("""Class is missing mandatory argument "degree".""")

        if "max_page" not in kwargs:
            self.max_page = INFINITY

        sseq._classes[self.uuid] = self
        if self.degree not in sseq._classes_by_bidegree:
            sseq._classes_by_bidegree[self.degree] = []

        if not hasattr(self, "idx"):
            self.idx = len(sseq._classes_by_bidegree[self.degree])
        sseq._classes_by_bidegree[self.degree].append(self)

    def needs_update(self):
        self._sseq.add_class_to_update(self)

    @staticmethod
    def from_json(sseq, json):
        return ChartClass(sseq, **json)

    def __repr__(self):
        return f"ChartClass({self.x},{self.y})"

    def to_json(self):
        result = utils.public_fields(self)
        result["type"] = "ChartClass"
        return result


    @property
    def x(self):
        return sum(a*b for (a,b) in zip(self.degree,self._sseq.x_degree))

    @property
    def y(self):
        return sum(a*b for (a,b) in zip(self.degree,self._sseq.y_degree))

    @utils.sseq_property
    def name(self, storage_name):
        self.needs_update()

    @utils.sseq_property
    def max_page(self, storage_name):
        self.needs_update()

    @utils.sseq_property
    def visible(self, storage_name):
        self.needs_update()

    @utils.sseq_property
    def x_nudge(self, storage_name):
        self.needs_update()

    @utils.sseq_property
    def y_nudge(self, storage_name):
        self.needs_update()

    def replace(self, **kwargs):
        page = self.max_page + 1
        self.max_page = INFINITY
        # if self.max_page == INFINITY:
        #     raise ValueError("???")
        for [key, value] in kwargs.items():
            getattr(self, key)[page:] = value
        self.needs_update()
        return self

    def delete(self):
        self._sseq.add_class_to_delete(self)
        del self._sseq._classes[self.uuid]
        for e in self._edges:
            e.delete()

    def __repr__(self):
        name_str = ")"
        if self.name[0] != "":
            name_str = f""", name="{self.name[0]}")""" # put paren here to prevent four quotes in a row, better way?
        return f"""Class({self.x}, {self.y}, {self.idx}{name_str}"""

