#!/bin/bash
while [[ $# -gt 0 ]]; do
    case $1 in
      --absolute)
        ABSOLUTE="true"
        shift # past argument
        shift # past value
        ;;
      --help)
        echo "create-readme"
        echo "Arguyments:"
        echo "<directorry>:         The instructional directory, or current if missing"
        echo "Options:"
        echo "  --absolute:         Use absolute paths in links"
        echo "  --with-properties:  Include properties in each heading"
        exit 0
        ;;
      -*|--*)
        echo "Unknown option $1"
        exit 1
        ;;
      *)
        POSITIONAL_ARGS+=("$1") # save positional arg
        shift # past argument
        ;;
    esac
  done

MP4_COUNT=`ls | grep -E "^[0-9]+.*mp4$" | wc -l`
MKV_COUNT=`ls | grep -E "^[0-9]+.*mkv$" | wc -l`
AVI_COUNT=`ls | grep -E "^[0-9]+.*avi$" | wc -l`

if [ "$MP4_COUNT" -gt "0" ]; then
    EXTENSION=${1:-mp4}
elif [ "$MKV_COUNT" -gt "0" ]; then
    EXTENSION=${1:-mkv}
elif [ "$AVI_COUNT" -gt "0" ]; then
    EXTENSION=${1:-avi}
fi

FALLBACK_DIR=`pwd`
DIR=${1:-$FALLBACK_DIR}
ABSOLUTE_DIR=`realpath "$DIR"`
MD_FILE="$ABSOLUTE_DIR/readme.md"
DIR_NAME=`basename "$ABSOLUTE_DIR"`
if [ -z "$CREATOR" ]; then
    CREATOR=`echo $DIR_NAME | cut -d'-' -f1 | xargs`
fi
if [ -z "$TITLE" ]; then
    TITLE=`echo $DIR_NAME | cut -d'-' -f2- | xargs`
fi
INSTRUCTIONAL=`basename "$DIR"`
echo "" > "$MD_FILE"

echo "# $CREATOR" >> "$MD_FILE"
echo " # $TITLE" >> "$MD_FILE"

ls *.${EXTENSION} | grep -E "^[0-9]+." | while read video; do
    title=`echo ${video%.*} | awk '{$1=""; print $0}' | sed -s 's/^ //'`
    if [ -z "$ABSOLUTE" ];then
        path="./$video"
    else
        path="$DIR/$video"
    fi
    echo "  ### $title" >> "$MD_FILE"
    echo "   - ![$title](${path})" >> "$MD_FILE"
    echo "" >> "$MD_FILE"
done
