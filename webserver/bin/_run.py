from argparse import ArgumentParser
from os import environ
from pathlib import Path
from sys import path
import uvicorn


def find_free_port(start_number):
    import socket
    for port in range(start_number, start_number + 1000):
        try:
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.bind(('127.0.0.1', port))
            s.listen(0)
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            return s.getsockname()[1]
        except socket.error as e:
            pass

def find_free_port2():
    import socket
    from contextlib import closing
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as s:
        s.bind(('', 0))
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        return s.getsockname()[1]

path.append(environ["REPOSITORY_ROOT"])

def main():
    parser = ArgumentParser(description='Spectral sequence webserver')
    parser.add_argument("files", nargs='*')
    parser.add_argument("-p", dest="port", default=0)
    args = parser.parse_args()
    for file in args.files:
        if not Path(file).is_file():
            from prompt_toolkit import HTML, print_formatted_text
            print_formatted_text(HTML(f"""<red>Input file "{file}" not found, quitting!</red>"""))
            return
    files = [str(Path(file).absolute()) for file in args.files]
    # Make a JSON list out of the files. We don't import json in order to speed up
    # startup time.
    port = args.port
    if port == 0:
        port = find_free_port(8000)
    environ["INPUT_FILES"] = "[" + ", ".join([f'"{file}"' for file in files]) + "]"
    environ["PORT"] = str(port)

    uvicorn.run("_main:app", host="127.0.0.1", port=port, log_level="warning")

main()