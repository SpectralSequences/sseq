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

if LOCAL_USER_DIR.is_dir():
    USER_DIR = LOCAL_USER_DIR
else:
    USER_DIR = REPO_USER_DIR
    if not USER_DIR.is_dir():
        USER_DIR.mkdir()

USER_CONFIG_FILE = USER_DIR / "config.py"
TEMPLATE_DIR = PACKAGE_ROOT / "templates"

utils.exec_file_if_exists(USER_CONFIG_FILE, globals(), locals())

if "GET_CONFIG_VARS" not in os.environ:
    if "INPUT_FILES" in os.environ and "GET_CONFIG_VARS" not in os.environ:
        import json
        INPUT_FILES = json.loads(os.environ["INPUT_FILES"])
    else:
        INPUT_FILES = []
    # Somehow we need to ensure that "ext" package is on the path.
    # Maybe we should allow it to be locally installed?
    # We could try importing it and if that fails try EXT_REPOSITORY flag
    # and if that fails, quit with an error.
    try:
        import ext
    except ImportError:
        try:
            EXT_PYTHON = EXT_REPOSITORY / "python"
            sys.path.append(str(EXT_PYTHON.absolute()))
            import ext
        except NameError:
            utils.print_error("""Could not import ext.""")
            utils.print_error("""Add "EXT_REPOSITORY = 'path/to/ext_repository'" to user/config.py""")
            utils.print_error("""Quitting.""")
            sys.exit()
        except ImportError:
            utils.print_error("""Could not import ext.""")
            utils.print_error("""Check the path "EXT_REPOSITORY = 'path/to/ext_repository'" in user/config.py""")
            utils.print_error("""Quitting.""")
            sys.exit()



    REPL_GLOBALS =  {
        "__name__": "__main__",
        "__package__": None,
        "__doc__": None,
        "__builtins__": builtins,
    }