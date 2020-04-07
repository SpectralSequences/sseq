import argparse
import json
import os
from pathlib import Path
from prompt_toolkit import HTML, print_formatted_text
import sys
import uvicorn

sys.path.append(os.environ["REPOSITORY_ROOT"])

def main():
    parser = argparse.ArgumentParser(description='Spectral sequence webserver')
    parser.add_argument("files", nargs='*')
    args = parser.parse_args()
    files = [str(Path(file).absolute()) for file in args.files]
    for file in files:
        if not Path(file).is_file():
            print_formatted_text(HTML("<red>" + str(err) + "</red>"))
            return
    os.environ["INPUT_FILES"] = json.dumps(files)

    uvicorn.run("_main:app", host="127.0.0.1", port=8000, log_level="warning")

main()