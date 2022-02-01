import sys
import pathlib
import re

r = re.compile("\n\n>>> (.|\n)*?\n\n", flags=re.MULTILINE)
text = pathlib.Path(sys.argv[1]).read_text()
groups = [m.group(0)[2:-2].split("\n") for m in r.finditer(text)]


def join_continue_lines(lines):
    lines = [line for line in lines if line[:4] in (">>> ", "... ")]
    result = []
    cur_line = ""
    for line in lines:
        if line.startswith(">>> ") and cur_line:
            result.append(cur_line[1:])
            cur_line = ""
        cur_line += f"\n{line[4:]}"
    result.append(cur_line[1:])
    return result


result = [join_continue_lines(group) for group in groups]

import json

print(json.dumps(result))
