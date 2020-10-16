from .chart_types import Color

class Shape:
    def __init__(self, character=None, font=None):
        if character:
            self.dict = dict(
                ty="character",
                font=font or "stix",
                char=character
            )
        else:
            self.dict = dict(ty="empty")
    
    def circled(self, padding : float):
        self.dict = dict(
            ty = "composed",
            operation="circled",
            padding=padding,
            innerShape=self.dict
        )

    def boxed(self, padding : float):
        self.dict = dict(
            ty = "composed",
            operation="boxed",
            padding=padding,
            innerShape=self.dict
        )

    def to_json(self):
        return self.dict

class Node:
    def __init__(self, 
        shape : Shape, 
        foreground : Color = (0, 0, 0, 0), 
        stroke : Color = (0, 0, 0, 0),
        fill : Color = (0, 0, 0, 0)
    ):
        self.shape = shape
        self.foreground = foreground
        self.stroke = stroke
        self.fill = fill

    def to_json(self):
        return dict(shape=self.shape, foreground=self.foreground, stroke=self.stroke, fill=self.fill)
