#!/bin/bash


#
# The script will markdown links from relative to absolute
# The script uses the intermediate literal ~space~ to escape spaces.
#

MD_FILE=$1
DIR=`dirname "$MD_FILE"`
ESCAPED_PATH=`echo $DIR | sed -s 's| |~space~|g'`

LINKS=`cat "$MD_FILE" | grep -E "\[\[.*]\]" | wc -l`
RELATIVE_LINKS=`cat "$MD_FILE" | grep -E "\[\[\./.*]\]" | wc -l`

echo "Links: $LINKS Relative: $RELATIVE_LINKS"
if [ "$RELATIVE_LINKS" -lt "$LINKS" ];then
    echo "File contains absolute links (relative: $RELATIVE_LINKS < links: $LINKS)"
else
#    sed -i  's|\[\[\.\/\(.*\)\]\]|[['$ESCAPED_PATH'/\1]]|' "$MD_FILE" 
    sed -i -E 's|~space~| |g' "$MD_FILE"
fi
