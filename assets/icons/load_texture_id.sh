#!/bin/bash

ls *.png | while read file; do
    id="${file%.*}"
    echo "self.icons.insert(\"${id}\", load_texture_id(frame, Path::new(\"assets/icons/${file}\")).unwrap());"
done
