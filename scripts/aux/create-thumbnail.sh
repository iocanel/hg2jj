#!/bin/sh

VIDEO=$1
OFFSET=${2:-0}
FILE_NAME="${VIDEO%.*}"
PNG_FILE="${FILE_NAME}.png"
echo "Converting $VIDEO to $PNG_FILE"
if [ -f "$PNG_FILE" ]; then
   rm "$PNG_FILE"
fi
ffmpeg -i "$VIDEO" -ss $OFFSET -dpi 300 -vframes 1 "${PNG_FILE}" < /dev/null 2> /dev/null
