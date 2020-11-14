from .chart_types import Color

class Shape:
    def __init__(self, character=None, font=None):
        if character:
            self.dict = dict(
                ty="character",
                font=font or "stix",
                char=character,
                whole_shape=True
            )
        else:
            self.dict = dict(ty="empty")
    
    def circled(self, padding : float, num_circles : int = 1, circle_gap : float = 0, include_background : bool = True) -> "Shape":
        if "whole_shape" in self.dict:
            self.dict["whole_shape"] = False
        self.dict = dict(
            ty = "composed",
            operation="circled",
            padding=padding,
            num_circles=num_circles,
            circle_gap=circle_gap,
            include_background=include_background,
            innerShape=self.dict
        )
        return self

    def boxed(self, padding : float, include_background : bool = True) -> "Shape":
        if "whole_shape" in self.dict:
            self.dict["whole_shape"] = False
        self.dict = dict(
            ty = "composed",
            operation="boxed",
            padding=padding,
            include_background=include_background,
            innerShape=self.dict
        )
        return self

    def to_json(self):
        return self.dict

    def __repr__(self):
        return f"Shape({repr(self.to_json())})"
