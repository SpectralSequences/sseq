import pathlib
import utils
MODULE_ROOT = pathlib.Path(__file__).parent.absolute()
PROJECT_ROOT = MODULE_ROOT.parent
USER_DIR = PROJECT_ROOT / "usr"
USER_CONFIG_FILE = USER_DIR / "config.py"
PARENT_DIR = PROJECT_ROOT.parent
BASIC_WEBCLIENT_JS_FILE = PARENT_DIR / "basic_webclient/target/debug/sseq_basic_webclient.js"
PORT = 8000


if not USER_DIR.is_dir():
    USER_DIR.mkdir()
utils.exec_file_if_exists(USER_DIR / "config.py", globals(), locals())