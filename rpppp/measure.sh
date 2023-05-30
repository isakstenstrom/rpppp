#!/bin/bash

./runs.sh $1 $2
./parse.sh $1

rm ${1}_data.zip
zip ${1}_data.zip $(ls dat/data*.txt)
