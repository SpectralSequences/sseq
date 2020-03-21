#!/bin/bash
echo "Cannot find ext repository, should I clone it? [y/n]"
while true; do
    read -p ">> " yn
    case $yn in
        [Yy]* ) export CLONE_EXT=1; break;;
        [Nn]* ) export CLONE_EXT=0; break;;
        * ) echo "Please answer y/n.";;
    esac
done

if [ -n "$CLONE_EXT" ]; then
    git clone git@github.com:SpectralSequences/ext.git
    # echo "git clone git@github.com:SpectralSequences/ext.git"
else
    echo "You will need to make a symlink to the ext repository yourself."
fi