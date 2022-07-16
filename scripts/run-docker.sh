#!/bin/sh

# This command optionall accepts the following arguments:
# -repository:	The docker repository
# -user:	The user under which the container is run (should match the one specified on build).
# -version:	The version of the image.
# 
# If you want to use these options you need to install `readopt` and `fitleropts`. 
# They can be found in the shell module and can be installed using `make helpers_install`.
#
U=`readopt -user "$@" 2> /dev/null`
V=`readopt -version "$@" 2> /dev/null`
A=`filteropts 2 -user -version "$@" 2> /dev/null`

USERNAME=${U:-"iocanel"}
VERSION=${V:-"latest"}
ARGS=${A:-"$@"}

xhost local:root
docker run -v ~/:/home/$USERNAME \
    	   -v /tmp/.X11-unix:/tmp/.X11-unix \
           -e DISPLAY=$DISPLAY \
           --device=/dev/dri \
           iocanel/hg2jj:$VERSION $ARGS
