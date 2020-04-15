#!/bin/bash
date --iso-8601=seconds | sed "s/\:/-/g" | sed "s/\(.*\)-.*-.*/\1/"