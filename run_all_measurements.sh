#!/bin/bash

SECONDS=0
cd rpppp
./revgen_measure.sh

./reventdev_measure.sh
cd ..

# cd PATH/TO/EVENTDEV_PIPELINE
# ./evgen_measure.sh
# cd ..

# cd PATH/TO/EVENTDEV_PIPELINE
# ./eventdev_measure.sh
# cd ../../..

duration=$SECONDS
echo "It took $(($duration / 3600)):$((($duration % 3600) / 60)):$(($duration % 60)) to run"
