import os
import pathlib
from . import utils
REPOSITORY_ROOT = pathlib.Path(os.environ["REPOSITORY_ROOT"])
utils.exec_file(REPOSITORY_ROOT / "config.py", globals(), locals())