import sys
import os
sys.path.append(os.environ["REPOSITORY_ROOT"])
from spectralsequences_webserver.server import server
app = server.app