#!/bin/bash

#
# The script will markdown links from relative to absolute
# The script uses the intermediate literal ~space~ to escape spaces.
#

MD_FILE=$1
DIR=`dirname "$MD_FILE"`
ESCAPED_PATH=`echo $DIR | sed -s 's| |~space~|g'`

ABSOLUTE_LINES=`cat "$MD_FILE" | grep -E "\(.*/.*\)"`

if [ -n "$ABSOLUTE_LINES" ];then
    echo "File contains absolute links"
else
    sed -i -E 's|\((.*)\)|('$ESCAPED_PATH'/\1)|' "$MD_FILE" 
    sed -i -E 's|~space~| |g' "$MD_FILE"
fi
