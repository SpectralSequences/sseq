#/bin/bash
DATE=`date --iso-8601=seconds | sed "s/\(.*\)-.*/\1/" | sed "s/\:/-/g"`
old="$IFS"
IFS="-"
args_str=`echo $* | sed "s/_//g" | sed "s/ /-/g"`
IFS=$old
FILENAME="cachegrind.out.${DATE}_$1_${args_str}"
nice -n +14 valgrind --tool=callgrind  --cache-sim=yes --branch-sim=yes --dump-instr=yes --collect-jumps=yes --callgrind-out-file=$FILENAME target/release/ext $2 $3
