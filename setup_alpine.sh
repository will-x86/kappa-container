#!/bin/sh

if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
else
    echo ".env file not found."
    exit 1
fi

mkdir -p "$ALPINE_PATH"

docker pull alpine:latest

container_id=$(docker create alpine:latest)

docker export $container_id > "$ALPINE_PATH/alpine-rootfs.tar"

tar -xf "$ALPINE_PATH/alpine-rootfs.tar" -C "$ALPINE_PATH"

docker rm $container_id
rm "$ALPINE_PATH/alpine-rootfs.tar"

echo "Alpine filesystem extracted to $ALPINE_PATH"
