#!/bin/bash

cd dat
rm *_out.txt

DATAS=$(ls data*X.txt)
for FILE in $DATAS
do
    type=$(sed -nE 's/^.*_(.*)_[0-9][0-9]_X.txt$/\1/p' <<< $FILE)
    sed "1,/^# $type/d" -i $FILE

    if [ $type == "AVG" ]
    then
        sed '/^#/,$d' $FILE >> data_${1}_AVGS_out.txt
    elif [ $type == "TL" ]
    then
        sed '/^#/,$d' $FILE > ${FILE/_X/_out}
    elif [ $type == "TSL" ]
    then
        i=0
        while [ -s $FILE ]
        do
            sed '/^#/,$d' $FILE > ${FILE/X/stage${i}_out}
            ((i++))
            sed '1,/^#/d' -i $FILE
        done
    else
        echo "The type '$type' is not a valid type"
        echo "The file was $FILE"
    fi

    rm $FILE
    # mv 1$FILE $FILE
done

cd ..