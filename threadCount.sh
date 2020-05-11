#!/bin/bash


LOGFILE=${1:-threads.txt}

printf "date\ttime\tThreadCount\n" > $LOGFILE
while true
do

  ls /proc/$(ps -e | grep my-onvif | awk '{print $1}')/task | wc | awk '{print $2}' | xargs -L 1 | xargs -L 1 bash  -c 'printf "%s\t%s\t%s\n" "$(date +%Y-%m-%d)" "$(date +%H:%M:%S)" "$*" ' bash >> $LOGFILE
  sleep 5

done