#!/bin/bash
function main() {
    local REPO_NAME="sseq"
    local BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
    local ROOT="$(dirname "$BIN")"

    if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
        printf "You must source this script.\n"
        printf "Rerun as \"source $REPO_NAME/bin/install.sh\".\n"
        return 1
    fi

    if [ -z "$SSEQ_BIN" ]; then
        SSEQ_BIN=$BIN
        SSEQ_ROOT=$ROOT
        if [ ! -d $HOME/bin ]; then
            mkdir $HOME/bin 
            PATH="$HOME/bin:$PATH"
        fi
        cp "$BIN/_sseq.sh" "$HOME/bin"
        echo -e "SSEQ_BIN=$SSEQ_BIN" >> "$HOME/bin/_sseq.sh"
        echo -e "SSEQ_ROOT=$SSEQ_ROOT" >> "$HOME/bin/_sseq.sh"
        echo -e "source \"$HOME/bin/_sseq.sh\"" >> "$HOME/.profile"
    fi
}
main