def ansi_color(r, g, b):
    return "\033[38;2;" + ";".join([str(r), str(g), str(b)]) + "m"


NOCOLOR = "\033[m"
RED = "\x1b[31m"
PINK = "\033[38;5;206m"
BLUE = ansi_color(100, 100, 255)
LIME_GREEN = ansi_color(30, 220, 30)
DARK_GREEN = ansi_color(60, 150, 60)
ORANGE = ansi_color(255, 165, 0)

INFO = BLUE
MISTAKE = RED
CORRECTION = LIME_GREEN
SUCCESS = DARK_GREEN
HIGHLIGHT = ORANGE
STATE_CHANGE = ORANGE


def info(s):
    return INFO + str(s) + NOCOLOR


def mistake(s):
    return MISTAKE + str(s) + NOCOLOR


def success(s):
    return SUCCESS + str(s) + NOCOLOR


def correction(s):
    return CORRECTION + str(s) + NOCOLOR


def highlight(s):
    return HIGHLIGHT + str(s) + NOCOLOR


def state_change(s):
    return STATE_CHANGE + str(s) + NOCOLOR
