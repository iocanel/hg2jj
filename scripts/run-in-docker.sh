#!/bin/sh

# This command optionall accepts the following arguments:
# -version:	The version of the image.
# 
# If you want to use these options you need to install `readopt` and `fitleropts`. 
# They can be found in the shell module and can be installed using `make helpers_install`.
#
V=`readopt -version "$@" 2> /dev/null`
A=`filteropts 1 -version "$@" 2> /dev/null`

# USERNAME is the user folder where your home directory will be mounted
USERNAME="hg2jj"
VERSION=${V:-"latest"}
ARGS=${A:-"$@"}

xhost local:root
docker run -v ~/:/home/$USERNAME \
    	   -v /tmp/.X11-unix:/tmp/.X11-unix \
           -e DISPLAY=$DISPLAY \
           --device=/dev/dri \ 
           iocanel/hg2jj:$VERSION $ARGS

