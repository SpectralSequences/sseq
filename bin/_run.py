import argparse
import json
import os
from pathlib import Path
import sys
import uvicorn

sys.path.append(os.environ["REPOSITORY_ROOT"])

from spectralsequences_webserver.utils import print_error

def main():
    parser = argparse.ArgumentParser(description='Spectral sequence webserver')
    parser.add_argument("files", nargs='*')
    args = parser.parse_args()
    files = [str(Path(file).absolute()) for file in args.files]
    for file in files:
        if not Path(file).is_file():
            print_error(f"""Input file "{file}" not found.\nQuitting.""")
            return
    os.environ["INPUT_FILES"] = json.dumps(files)

    uvicorn.run("_main:app", host="127.0.0.1", port=8000, log_level="warning")

main()