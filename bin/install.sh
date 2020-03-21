#!/bin/bash
function main(){
    local REPO_NAME="webserver"

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
    
    source "$SSEQ_BIN/_make_repository_link.sh" $REPOSITORY_ROOT ext || return 1
    source "$SSEQ_BIN/_make_repository_link.sh" $REPOSITORY_ROOT basic_webclient || return 1
}
main


# export BASIC_WEBCLIENT_REPOSITORY=$($BIN/_find_ext_repository $1)
# if [ -z "$EXT_REPOSITORY" ]; then
#     ./_query_clone_repository.sh basic_webclient
# else
#     ln -s $EXT_REPOSITORY $REPOSITORY_ROOT/ext
# fi

# virtualenv -p /usr/bin/python3.8 $BIN/virtualenv
# source $BIN/virtualenv/bin/activate

# pip install fastapi
# pip install jinja2
# pip install maturin
# pip install pathlib
# pip install ptpython
# pip install uvicorn
# pip install websockets

# cd $EXT_REPOSITORY
# rustup override set nightly-2020-02-29
# $BIN/build_rust_ext.sh