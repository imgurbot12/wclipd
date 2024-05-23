#!/bin/sh

SELF=`realpath $0`
NAME=`basename $(dirname $SELF) | cut -d- -f1`
CONFIG="$HOME/.config/$NAME"

sudo cp -vf bin/* /usr/local/bin/.
mkdir -p "$CONFIG"
cp -vf config.yaml "$CONFIG/."
