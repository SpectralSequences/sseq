#!/usr/bin/env python3

# This script converts a module defined in Bruner's format into one in our
# format. This only works with finite dimensional modules at the prime 2. This
# does not verify that the module file defines a well-defined Steenrod module.

import sys
import json
import os.path

if len(sys.argv) != 2:
    sys.exit("Run command with input filename as unique argument")

try:
    f = open(sys.argv[1], "r")
except FileNotFoundError:
    sys.exit("File not found: %", sys.argv[1])

obj = {
        "type": "finite dimensional module",
        "p": 2,
        "generic": False,
        "file_name": os.path.basename(sys.argv[1]),
        "algebra": ["milnor"],
        "gens": {},
        "actions": []
    }

def isPowerOfTwo(n):
    return (n != 0) and (n & (n - 1) == 0)

f = filter(lambda x: x != "", map(str.strip, iter(f)))

try:
    dim = int(next(f))
except ValueError:
    sys.exit("Invalid file format: First line must be dimension of module")

try:
    dims = [int(x) for x in next(f).split(" ")]
except ValueError:
    sys.exit("Invalid file format: Second line must be list of generators")

if len(dims) != dim:
    sys.exit("Invalid module specification: Incorrect number of generators specified")

for i, dim in enumerate(dims):
    obj["gens"]["x{}".format(i)] = dim

for line in f:
    try:
        data = [int(x) for x in line.split(" ")]
    except ValueError:
        print("Invalid action specification: " + line)
        print("This must be a space-separated list of integers")
        sys.exit()

    if len(data) < 3:
        print("Invalid action specifiction: " + line)
        print("Each line must have at least 3 entries")
        sys.exit()

    if len(data) != data[2] + 3:
        print("Invalid action specifiction: " + line)
        print("The third entry of the line is the number of terms in the sum")
        sys.exit()

    if not isPowerOfTwo(data[1]):
        continue
    
    obj["actions"].append("Sq{} x{} = {}".format(data[1], data[0], " + ".join("x{}".format(x) for x in data[3:])))

print(json.dumps(obj))
