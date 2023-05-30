#!/bin/bash

cd ../..

ninja -C build

cd examples/eventdev_pipeline

rm dat/data*.txt

app_name=$1

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

        bitmask=""
        if (( (i) % 2 == 1 )); then
            bitmask=$bitmask"2"
        fi
        for((j = 0; j < (i)/ 2; j++)); do
            bitmask=$bitmask"A"
        done
        bitmask=$bitmask"00"
        echo Bitmask: ${bitmask}

        run_num=($(printf '%02d' $i))
        file_name=dat/data_${app_name}_${latency_type}_${run_num}_X.txt

        (sudo timeout -s SIGINT --preserve-status 60s ../../build/examples/dpdk-eventdev_pipeline --vdev event_sw0 --no-pci -l 5,7,${workers// /,} --vdev 'net_null0' -- -r80 -t80 -e20 -w $bitmask -s3 -n0 -c1 -W1000 -l $latency_arg > $file_name)

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
