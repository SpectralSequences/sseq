import os
import sys
os.environ["GET_CONFIG_VARS"] = "true"
sys.path.append(os.environ["REPOSITORY_ROOT"])
from spectralsequences_webserver import config
for arg in sys.argv[1:]:
    print(getattr(config, arg))