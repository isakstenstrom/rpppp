#!/bin/bash

rm dat/data*.txt
mkdir -p dat

app_name=$1
app=$2

cargo b --bin $app --release

function run_for_given_cores ()
{
    latency_type=$1
    shift
    latency_arg=$1
    shift
    cpus=("$@")

    echo Testing for $latency_type
    let len="${#cpus[@]} + 1"

    for ((i=1; i<$len; i++ ));
    do
        workers=${cpus[@]:0:i}
        echo Using CPUs: ${workers[@]}

        run_num=($(printf '%02d' $i))
        file_name=dat/data_${app_name}_${latency_type}_${run_num}_X.txt
        (./target/release/$app $latency_arg ${workers// /,}) > $file_name
    done
}

function run_with_all_args()
{
    CPUs=("$@")
    # No latency
    run_for_given_cores AVG 0 ${CPUs[@]}
    # Totatal latency
    run_for_given_cores TL  1 ${CPUs[@]}
    # Task-switching latency
    run_for_given_cores TSL 2 ${CPUs[@]}
}

# Avoid hyperthreading and use only one NUMA node
CPUs=($(seq 9 2 63))
run_with_all_args ${CPUs[@]}
