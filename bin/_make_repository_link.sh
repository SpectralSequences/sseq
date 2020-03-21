#!/bin/bash
# Look for t

function check_repository(){
    if [ ! -d "$2" ]; then
        return 1
    fi

    cd "$2"
    local ORIGIN=$(git remote get-url origin 2> /dev/null)
    local WHAT_ORIGIN_SHOULD_BE="git@github.com:SpectralSequences/$1.git"
    cd "$WORKING_DIRECTORY"

    if [ "$ORIGIN" = "$WHAT_ORIGIN_SHOULD_BE" ]; then
        # echo "Repo $1 is in $2"
        return 0
    else
        # echo "Repo $1 is not in $2"
        return 1
    fi
}

function main(){
    local LINK_TARGET="$1/$2"
    local REPO_NAME="$2"
    local WORKING_DIRECTORY=$(pwd)
    eval "local QUOTED_REPOSITORY_PATH_ENV=\${SSEQ_${REPO_NAME^^}_PATH}"

    check_repository "$REPO_NAME" "$LINK_TARGET"
    if [ "$?" -eq 0 ]; then
        return 0
    fi

    check_repository "$REPO_NAME" "$QUOTED_REPOSITORY_PATH_ENV"
    if [ "$?" -eq 0 ]; then
        ln -s "$QUOTED_REPOSITORY_PATH_ENV" "$LINK_TARGET"
        return 0
    fi

    unset QUOTED_REPOSITORY_PATH_ENV

    echo -e "Cannot find \"$REPO_NAME\" repository."
    echo -e "Clone \"$REPO_NAME\" and then run \"source $REPO_NAME/bin/install.sh\"."
    return 1
}
main "$1" "$2"
