import builtins
import os
import pathlib

from . import utils

WORKING_DIRECTORY = os.environ["WORKING_DIRECTORY"]
SCRIPT_ARGS = os.environ["SCRIPT_ARGS"]

MODULE_ROOT = pathlib.Path(__file__).parent.absolute()
PROJECT_ROOT = MODULE_ROOT.parent
LOCAL_USER_DIR = PROJECT_ROOT / "user_local"
REPO_USER_DIR = PROJECT_ROOT / "user"
if LOCAL_USER_DIR.is_dir():
    USER_DIR = LOCAL_USER_DIR
else:
    USER_DIR = REPO_USER_DIR
    if not USER_DIR.is_dir():
        USER_DIR.mkdir()


REPL_GLOBALS =  {
    "__name__": "__main__",
    "__package__": None,
    "__doc__": None,
    "__builtins__": builtins,
}

USER_CONFIG_FILE = USER_DIR / "config.py"
PARENT_DIR = PROJECT_ROOT.parent
TEMPLATE_DIR = MODULE_ROOT / "templates"

utils.exec_file_if_exists(USER_CONFIG_FILE, globals(), locals())