#!/bin/bash
function main(){
    local REPO_NAME="basic_webclient"

    ###
    # SAME: This next segment is the same in all install scripts
    local BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
    local REPOSITORY_ROOT="$(dirname "$BIN")"
    local WORKING_DIRECTORY="$(pwd)"

    eval "CHECK_PATH=\${SSEQ_${REPO_NAME^^}_PATH}"
    if [ -z "$CHECK_PATH" ]; then
        if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
            printf "You must source this script.\n"
            printf "Rerun as \"source $REPO_NAME/bin/install.sh\".\n"
            return 1
        fi

        if [ -z "$SSEQ_BIN" ]; then 
            printf "Cannot find \"sseq/bin\".\n"
            printf "First clone \"sseq\" and run \"source sseq/bin/install.sh\".\n"
            printf "Then run \"source $REPO_NAME/bin/install.sh\" again.\n"
            return 1
        fi

        
        eval "SSEQ_${REPO_NAME^^}_PATH=$REPOSITORY_ROOT"
        echo -e "SSEQ_${REPO_NAME^^}_PATH=$REPOSITORY_ROOT" >> "$HOME/bin/_sseq.sh"
    fi
    unset CHECK_PATH
    # END SAME
    ###

    echo "Installing wasm-bindgen-cli"
    cargo install wasm-bindgen-cli
    echo "Installing wasm-opt"
    source $BIN/install-wasm-opt.sh
    echo "Installing rustup target wasm32-unknown-unknown"
    rustup target add wasm32-unknown-unknown
}
main