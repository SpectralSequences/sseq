#!/usr/bin/python3
import pathlib

files = [file.stem for file in pathlib.Path("./src/python").glob("*")]

outlines = []
for file in files:
    outlines.append(f'import {file} from "./python/{file}.py"')

outlines.append("export const files_to_install = {" + ",".join(files) + "}")
pathlib.Path("./src/python_imports.js").write_text("\n".join(outlines))