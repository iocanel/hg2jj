#!/bin/sh

TMP_DIR=`mktemp -d /tmp/scale-720-XXXXX`
ffmpeg -i "$1" -vf scale=-1:720 -c:v libx264 -crf 23 -maxrate 1M -bufsize 2M -preset veryslow -c:a copy ${TMP_DIR}/720p.mp4 2> /dev/null
cp "$1" "$1.bkp"
cp ${TMP_DIR}/720p.mp4 "$1"
