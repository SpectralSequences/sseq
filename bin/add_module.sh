function main() {
    if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
        printf "You must source this script.\n"
        printf "Rerun as \"source sseq/bin/add_module.sh\".\n"
        return 1
    fi

    if [ -z "$SSEQ" ]; then
        echo "Run bin/install.sh first!"
        exit 1
    fi
    git clone git@github.com:SpectralSequences/$1.git
    source ./$1/bin/install.sh
}
main

