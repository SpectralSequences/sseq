from typing import Union, List

UUID_str = str
DashPattern = List[int]
LineWidth = float

from .css_colors import CSS_COLORS_JSON


class Color:
    """ Represents a color in RGBA colorspace. Each channel should be an integer from 0 to 255, values outside of this range will be clipped.

    """ 
    CSS_COLORS = None
    def __init__(self, r : int, g : int, b : int, a : int = 255):
        """
            Args:
                r (float): The red color channel.
                
                g (float): The green color channel.
                
                b (float): The blue color channel.
                
                a (float): The alpha / transparency color channel.
        """
        self._name = None
        self._color = tuple(min(max(int(s), 0),255) for s in (r, g, b, a))

    @staticmethod
    def from_string(color : str) -> "Color":
        if color.startswith("#"):
            return Color.from_hex(color)
        if color in Color.CSS_COLORS:
            return Color.CSS_COLORS[color]
        raise ValueError(f"Unrecognized color '{color}'")

    @staticmethod
    def from_hex(hex_str : str) -> "Color":
        assert hex_str.startswith("#")
        assert len(hex_str) == 7 or len(hex_str) == 9
        parts = [hex_str[1:3], hex_str[3:5], hex_str[5:7]]
        if len(hex_str) == 9:
            parts.append(hex_str[7:])        
        parts = [int(s, 16) for s in parts]
        return Color(*parts)
    
    def to_hex(self) -> str:
        return "#" + "".join([hex(s)[2:].zfill(2) for s in self._color])

    
    def lerp(self, other : "Color", t : float) -> "Color":
        """ Linearly interpolate between two colors.

            Returns:
                t * self + (1-t) * other.
        """
        return Color(*(self._color[i] * t + other[i] * (1 - t) for i in range(4)))

    def to_json(self):
        result = dict(
            type=type(self).__name__,
            color= self.to_hex()
        )
        if self._name:
            result["name"] = self._name
        return result
    @classmethod
    def from_json(cls, json):
        assert json["type"] == cls.__name__
        result = Color.from_hex(json["color"])
        result._name = json.get("name")
        return result

    def __repr__(self):
        if self._name:
            return f'Color("{self._name}")'
        return f'Color("{self.to_hex()}")'

Color.CSS_COLORS = {}
for (name, value) in CSS_COLORS_JSON.items():
    c = Color.from_hex(value)
    c._name = name
    Color.CSS_COLORS[name] = c

Color.CSS_COLORS["transparent"] = Color(0,0,0,0)
Color.CSS_COLORS["transparent"]._name = "transparent"

class ArrowTip:
    """ An ArrowTip. Curently the only possible arrow tip is the standard one. 
        TODO: support for hook, some parameters.
    """
    def __init__(self, tip="standard"):
        self._tip = tip

    # @property
    # def tip(self):
    #     return self._tip
    
    def to_json(self):
        return dict(
            type=type(self).__name__,
            tip = self._tip,
        )

    @staticmethod
    def from_json(json):
        assert json.pop("type") == ArrowTip.__name__
        return ArrowTip(**json)

    def __repr__(self):
        return f"ArrowTip('{self._tip}')"

from copy import deepcopy

class Shape:
    """ A Shape. A Shape has three components: a background, a foreground, and a border (some of these may be empty).
        The shape can be iteratively built up by starting with a string to be drawn at the center and wrapping it 
        with accents and border shapes.

        If the whole shape is a single character, then the character will be rendered as the "background"
        and the "border" will outline the border of the character.
        If the characters are wrapped in a circle or rectangle, then the characters will be drawn in the "foreground" component,
        the "background" component will consist of the interior of the bounding circle / rectangle, and the border will be the border 
        of the circle / rectangle.
    """
    def __init__(self, character : str = None, font : str = None):
        """
            TODO: Link to StixTwoMath.
            
            Args:
                character (str): The characters to render at the center of the shape. 
                font (str): The font to render the characters in. Currently the only supported font is "stix".                  
        """
        self._name = None
        if character:
            self.dict = dict(
                ty="character",
                font=font or "stix",
                char=character,
                whole_shape=True
            )
        else:
            self.dict = dict(ty="empty")

    @staticmethod
    def square(size : float):
        return Shape().boxed(size)
    
    @staticmethod
    def circle(size : float):
        return Shape().circled(size)

    def circled(self, padding : float, num_circles : int = 1, circle_gap : float = 0, include_background : bool = True) -> "Shape":
        """ Circle the existing shape with one or more circles. 

            Args:
                padding (float): How much space to leave between the circle and the shape we are circling.
                
                num_circles (int): How many concentric circles to draw. Because the padding is computed based on a bounding box,
                    repeatedly using `Shape.circled` leads to inconsistent spacing between the circles.
                
                circle_gap (int): If num_circles > 1, how much space to leave between circles. If num_circles == 1, has no effect.

                include_background (bool): If True, the background of the circle goes in the background component, if False, 
                    the new circle makes no contribution to the background component.

        """
        copy_dict = deepcopy(self.dict)
        if "whole_shape" in copy_dict:
            copy_dict["whole_shape"] = False
        result = Shape()
        result.dict = dict(
            ty = "composed",
            operation="circled",
            padding=padding,
            num_circles=num_circles,
            circle_gap=circle_gap,
            include_background=include_background,
            innerShape=copy_dict
        )
        return result

    def boxed(self, padding : float, include_background : bool = True) -> "Shape":
        """ Box the existing shape. 

            Args:
                padding (float): How much space to leave between the box and the shape we are boxing.
                
                include_background (bool): If True, the background of the box goes in the background component, if False, 
                    the new box makes no contribution to the background component.
        """        
        copy_dict = deepcopy(self.dict)
        if "whole_shape" in copy_dict:
            copy_dict["whole_shape"] = False
        result = Shape()
        result.dict = dict(
            ty = "composed",
            operation="boxed",
            padding=padding,
            include_background=include_background,
            innerShape=copy_dict
        )
        return result

    def to_json(self):
        result = {"type" : type(self).__name__}
        result.update(self.dict)
        if self._name:
            result["name"] = self._name
        return result
    
    @staticmethod
    def from_json(json):
        assert json.pop("type") == Shape.__name__
        result = Shape()
        if "name" in json:
            result._name = json.pop("name")
        result.dict = json
        return result

    def __repr__(self):
        if self._name:
            return f'Shape("{self._name}")'
        return f"Shape({repr(self.dict)})"