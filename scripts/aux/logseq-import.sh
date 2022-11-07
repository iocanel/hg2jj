#!/bin/bash


#
# Utility to import files into logseq
# 
# The target files are either org or md files indexing
# the instructional content.
# 
# By default they target `readme.md` but both name and extension
# are customizable.

INSTRUCTIONAL_DIR=$1
LOGSEQ_BASE_DIR="$HOME/Documents/logseq/BJJ/pages"
LINK=""
FILE_NAME="readme"
FILE_EXT="md"
FULL_NAME="${FILE_NAME}.${FILE_EXT}"

while [[ $# -gt 0 ]]; do
  case $1 in
    --link)
      LINK="true"
      shift # past argument
      shift # past value
      ;;
    --logseq-dir)
      LOGSEQ_BASE_DIR="$2"
      shift # past argument
      shift # past value
      ;;
    --file)
      FILE_NAME="$2"
      FULL_NAME="${FILE_NAME}.${FILE_EXT}"
      shift # past argument
      shift # past value
      ;;
    --ext)
      FILE_EXT="$2"
      FULL_NAME="${FILE_NAME}.${FILE_EXT}"
      shift # past argument
      shift # past value
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


pushd "$INSTRUCTIONAL_DIR"
INSTRUCTIONAL_NAME=$(basename "$INSTRUCTIONAL_DIR")
echo "Importing: $INSTRUCTIONAL_DIR/${FILE_NAME}.${FILE_EXT}"
if [ -n "$LINK" ]; then
    ln -s "$INSTRUCTIONAL_DIR/${FILE_NAME}.${FILE_EXT}" "$LOGSEQ_BASE_DIR/${INSTRUCTIONAL_NAME}.${FILE_EXT}"
else    
    cp "$INSTRUCTIONAL_DIR/${FILE_NAME}.${FILE_EXT}" "$LOGSEQ_BASE_DIR/${INSTRUCTIONAL_NAME}.${FILE_EXT}"
fi
popd
