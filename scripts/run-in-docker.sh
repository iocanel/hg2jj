#!/bin/sh

# This command optionall accepts the following arguments:
# -version:	The version of the image.
# 
# If you want to use these options you need to install `readopt` and `fitleropts`. 
# They can be found in the shell module and can be installed using `make helpers_install`.
#
# USERNAME is the user folder where your home directory will be mounted

USERNAME="hg2jj"
VERSION="latest"

xhost local:root
docker run \
         -v ~/.Xauthority:/home/$USERNAME/.Xauthority \
         -v ~/:/home/$USERNAME \
    	   -v /mnt/bjj:/mnt/bjj \
    	   -v /mnt/downloads:/mnt/downloads\
         -v ~/.cache/hg2jj:/opt/hg2jj/.cache \
    	   -v /tmp/.X11-unix:/tmp/.X11-unix \
         -v /dev/:/dev/ \
         -v /var/run/user/$(id -u)/:/var/run/user/$(id -u)/:ro \
         -v /var/run/dbus/:/var/run/dbus \
         -v /var/lib/dbus/:/var/lib/dbus \
         -v /etc/machine-id/:/etc/machine-id \
         -e DISPLAY=$DISPLAY \
         -e PULSE_SERVER=unix:/run/user/1000/pulse/native \
         -e HG2JJ_DIR=/opt/hg2jj/ \
         --device=/dev/snd \
         --device=/dev/dri \
         --privileged \
         --net=host \
         iocanel/hg2jj:$VERSION
