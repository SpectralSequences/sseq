#!/usr/bin/python3
import pathlib

# See pyodide.worker.js where this is used to copy python file tree into emscripten filesystem.

files = [file.relative_to("src") for file in pathlib.Path("src/python").glob("**/*.py")]
directories = [
    str(f.relative_to("src/python"))
    for f in pathlib.Path("src/python").glob("**/*")
    if f.is_dir()
]
imports = [
    f'import {file.stem}_{idx} from "./{file}";' for idx, file in enumerate(files)
]
export = [
    f"'{file.relative_to('python')}' : {file.stem}_{idx}"
    for idx, file in enumerate(files)
]


outlines = imports
outlines.append(f"export const directories_to_install = {directories}; ")
outlines.append(f"export const files_to_install = {{{', '.join(export)}}};")
pathlib.Path("./src/python_imports.js").write_text("\n".join(outlines))
