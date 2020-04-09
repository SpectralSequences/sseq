import builtins
import os
import sys
import pathlib

from . import utils

WORKING_DIRECTORY = pathlib.Path(os.environ["WORKING_DIRECTORY"])
REPOSITORY_ROOT = pathlib.Path(os.environ["REPOSITORY_ROOT"])
PACKAGE_ROOT = REPOSITORY_ROOT / "spectralsequences_webserver"
LOCAL_USER_DIR = REPOSITORY_ROOT / "user_local"
REPO_USER_DIR = REPOSITORY_ROOT / "user"
DEMO_DIR = REPOSITORY_ROOT / "demos"

MESSAGE_PASSING_REPOSITORY_ROOT = REPOSITORY_ROOT / "../message_passing_tree"
EXT_REPOSITORY_ROOT = REPOSITORY_ROOT / "../ext"
PYTHON_EXT_REPOSITORY_ROOT = REPOSITORY_ROOT / "../python_ext"

CHART_REPOSITORY_ROOT = REPOSITORY_ROOT / "../chart"
SSEQ_WEBCLIENT_JS_FILE = CHART_REPOSITORY_ROOT / "client/target/debug/sseq_webclient.js"

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
elif LOCAL_USER_DIR.is_dir():
    USER_DIR = LOCAL_USER_DIR
else:
    USER_DIR = REPO_USER_DIR
    if not USER_DIR.is_dir():
        USER_DIR.mkdir()


SAVE_DIR = USER_DIR / "save"
REPL_INIT_FILE = USER_DIR / "on_repl_init.py"

TEMPLATE_DIR = PACKAGE_ROOT / "templates"

USER_CONFIG_FILE = USER_DIR / "config.py"
utils.exec_file_if_exists(USER_CONFIG_FILE, globals(), locals())