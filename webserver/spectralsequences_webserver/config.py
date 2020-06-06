import builtins
import os
import sys
import pathlib

from . import utils

PORT = os.environ["PORT"]

WORKING_DIRECTORY = pathlib.Path(os.environ["WORKING_DIRECTORY"])
REPOSITORY_ROOT = pathlib.Path(os.environ["REPOSITORY_ROOT"])
PACKAGE_ROOT = REPOSITORY_ROOT / "spectralsequences_webserver"
DEMO_DIR = REPOSITORY_ROOT / "demos"

MESSAGE_PASSING_REPOSITORY_ROOT = REPOSITORY_ROOT / "../message_passing_tree"
EXT_REPOSITORY_ROOT = REPOSITORY_ROOT / "../ext"
PYTHON_EXT_REPOSITORY_ROOT = REPOSITORY_ROOT / "../python_ext"

CHART_REPOSITORY_ROOT = REPOSITORY_ROOT / "../chart"
SSEQ_WEBCLIENT_JS_FILE = CHART_REPOSITORY_ROOT / "client/dist/sseq_webclient.js"
CLIENT_DIR = REPOSITORY_ROOT / "client/dist"

sys.path.extend([str(path) for path in [
    MESSAGE_PASSING_REPOSITORY_ROOT,
    CHART_REPOSITORY_ROOT / "server",
    PYTHON_EXT_REPOSITORY_ROOT
]])


if "INPUT_FILES" in os.environ:
    import json
    INPUT_FILES = json.loads(os.environ["INPUT_FILES"])
else:
    INPUT_FILES = []

if "USER_DIR" in os.environ:
    USER_DIR = pathlib.Path(os.environ["USER_DIR"])
else:
    USER_DIR = REPOSITORY_ROOT / "user"
if not USER_DIR.is_dir():
    USER_DIR.mkdir()
    (USER_DIR / "repl.hist").write_text("")

SAVE_DIR = USER_DIR / "save"
if not SAVE_DIR.is_dir():
    SAVE_DIR.mkdir()
REPL_INIT_FILE = USER_DIR / "on_repl_init.py"

TEMPLATE_DIR = PACKAGE_ROOT / "templates"


USER_CONFIG_FILE = USER_DIR / "config.py"
utils.exec_file_if_exists(USER_CONFIG_FILE, globals(), locals())
