#!/bin/bash

#
# we need to prevent subshell or we are going to get our scene_idx reset for each file
#
shopt -s lastpipe

#
# Script requires `grab`. To install it:
# curl -s https://raw.githubusercontent.com/shellib/grab/master/install.sh | bash
#

source $(grab github.com/shellib/cli)

INDEX_FILE=${1:-index.org}
M3U_FILE=${2:-playlist.m3u}

ARTIST=$(or $(readopt --artist "$@") "Unknwon")
INDEX_FILE=$(or $(readopt --index "$@") "index.org")
M3U_FILE=$(or $(readopt --m3u-file "$@") "playlist.m3u")
SPLIT=$(hasflag --split "$@")
SCALE=$(hasflag --scale "$@")
ARGS=$(filteropts 5 --index=_ --m3u-file=_ --artist=_ --split --scale "$@")


echo "Creating playlist: $M3U_FILE using index: $INDEX_FILE"
echo "#EXTM3U" > $M3U_FILE
echo "#EXT-X-VERSION:6" >> $M3U_FILE

function timestamp_to_time () {
#Convert timestamp to time using the HH:mm:ss format
    local ts=${1%.*}
    if [ -z "$ts" ]; then
        ts="0"
    fi
    # Hours
    local hours="00"
    if [ $ts -gt 3600 ]; then
        hours=`expr $ts / 3600`
    fi

    #Minutes
    local minutes="00"
    local remainder=`expr $ts % 3600` 
    if [ $remainder -gt 60 ]; then
        minutes=`expr $remainder / 60`
        if [ $minutes -lt 10 ]; then
            minutes="0$minutes"
        fi
    else 
        minutes="00"
    fi

    #Seconds
    local seconds=`expr $ts % 60`
    if [ $seconds -eq 10 ]; then
        seconds="11"
    elif [ $seconds -lt 9 ]; then
        seconds="0$((seconds+1))"
    fi
    echo "$hours:$minutes:$seconds"
}

scene_idx=1
cat $INDEX_FILE | grep -n ":video:" | while read line; do
    start_line_number=`echo $line | awk -F':' '{print $1}'`
    end_line_number=`expr $start_line_number + 7`
    title=`sed "${start_line_number}q;d" $INDEX_FILE | awk '{$1=""; $NF=""; print $0}' |  sed -s 's/^ //;s/[ \n]*$//'`
    start_timestamp=`sed -n "${start_line_number},${end_line_number}p" $INDEX_FILE | grep START_TIMESTAMP | head -n 1 | awk '{$1=""; print $0}' | sed -s 's/^ //;s/[ \n]*$//'`
    end_timestamp=`sed -n "${start_line_number},${end_line_number}p" $INDEX_FILE | grep END_TIMESTAMP | head -n 1 | awk '{$1=""; print $0}' | sed -s 's/^ //;s/[ \n]*$//'`
    if [ -z "$start_timestamp" ];then
       start_timestamp=0
    fi
    printf "%03d. $title - %s - %s." $scene_idx $start_timestamp $end_timestamp
    echo ""
    filename=`sed -n "${start_line_number},${end_line_number}p" $INDEX_FILE | grep FILE_OR_URL | head -n 1 | awk '{$1=""; print $0}' | sed -s 's/^ //;s/ $//'`
    filename=`realpath --relative-to=./ "$filename"`
    if [ "$SPLIT" == "true" ]; then
        extension="${filename##*.}"
        start=`timestamp_to_time ${start_timestamp%.*}`
        if [ -n "$end_timestamp" ];then
           end=`timestamp_to_time ${end_timestamp%.*}`
        else
            end=""
        fi
        split_filename=`printf "%03d. ${title}.${extension}" $scene_idx | sed -s 's/\//-/'`
        scaled_filename="${filename%.*}-720p.${extension}"

        if [ "$SCALE" == "true" ]; then
            if [ ! -f "$scaled_filename" ]; then
                echo "ffmpeg -y -i "$filename" -vf scale=-1:720 -c:v libx264 -crf 23 -maxrate 1M -bufsize 2M -preset veryslow -c:a copy "${scaled_filename}" < /dev/null 2>> m3u.log"
                ffmpeg -y -i "$filename" -vf scale=-1:720 -c:v libx264 -crf 23 -maxrate 1M -bufsize 2M -preset veryslow -c:a copy "${scaled_filename}" < /dev/null 2>> m3u.log
            fi
            filename="$scaled_filename"
        fi

        if [ ! -f "$split_filename" ]; then
            if [ -z "$end" ]; then
                echo "ffmpeg -y -i \"filename\" -ss $start \"${split_filename}\" < /dev/null"
                ffmpeg -y -i "$filename" -ss $start "${split_filename}" < /dev/null 2>> m3u.log
            else
                echo "ffmpeg -y -i \"$filename\" -ss $start -to $end \"${split_filename}\" < /dev/null"
                ffmpeg -y -i "$filename" -ss $start -to $end "${split_filename}" < /dev/null 2>> m3u.log
            fi
        fi

        if [ -n "$end" ]; then
            duration=`expr ${end_timestamp%.*} - ${start_timestamp%.*}`
        else
            duration=`ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 $filename < /dev/null 2> /dev/null` 
        fi
        echo "#EXTINF: $duration, $title" >> $M3U_FILE
        echo ${split_filename} >> $M3U_FILE
    else
        echo "#EXTINF: $duration, $title" >> $M3U_FILE
        echo "#EXT-X-START:TIME-OFFSET=$start_timestamp" >> $M3U_FILE
        echo $filename >> $M3U_FILE
    fi
    scene_idx=$((scene_idx+1))
done
