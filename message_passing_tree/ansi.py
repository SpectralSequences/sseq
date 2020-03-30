def ansi_color(r, g, b):
    return "\033[38;2;" +";".join([str(r), str(g), str(b)]) + "m"

NOCOLOR = "\033[m"
RED = "\x1b[31m"
BLUE = ansi_color(100, 100, 255)
GREEN = ansi_color(30, 220, 30)

INFO = BLUE
MISTAKE = RED
CORRECTION = GREEN

def info(s):
    return INFO + str(s) + NOCOLOR

def mistake(s):
    return MISTAKE + str(s) + NOCOLOR

def correction(s):
    return CORRECTION + str(s) + NOCOLOR
