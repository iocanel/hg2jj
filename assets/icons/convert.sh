#!/bin/bash

ls *.svg | while read svg_file; do
    png_file="${svg_file%.*}.png"
    convert -background none -size 64x64 ${svg_file} ${png_file}
done
