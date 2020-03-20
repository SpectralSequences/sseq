import pathlib

from . import utils
MODULE_ROOT = pathlib.Path(__file__).parent.absolute()
PROJECT_ROOT = MODULE_ROOT.parent
USER_DIR = PROJECT_ROOT / "usr"
USER_CONFIG_FILE = USER_DIR / "config.py"
PARENT_DIR = PROJECT_ROOT.parent
TEMPLATE_DIR = MODULE_ROOT / "templates"


if not USER_DIR.is_dir():
    USER_DIR.mkdir()
utils.exec_file_if_exists(USER_CONFIG_FILE, globals(), locals())
print(BASIC_WEBCLIENT_JS_FILE)