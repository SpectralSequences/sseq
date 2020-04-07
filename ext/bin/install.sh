#!/bin/bash
function main(){
    local REPO_NAME="ext"

    ###
    # This next segment is the same in all install scripts
    eval "CHECK_PATH=\${SSEQ_${REPO_NAME^^}_PATH}"
    if [ -z "$CHECK_PATH" ]; then
        local BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
        local REPOSITORY_ROOT="$(dirname "$BIN")"
        local WORKING_DIRECTORY="$(pwd)"

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
}
main