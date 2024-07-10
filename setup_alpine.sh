#!/bin/sh

if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
else
    echo ".env file not found."
    exit 1
fi

mkdir -p "$CONTAINER_PATH"

docker pull alpine:latest

container_id=$(docker create alpine:latest)

docker export $container_id > "$CONTAINER_PATH/alpine-rootfs.tar"

tar -xf "$CONTAINER_PATH/alpine-rootfs.tar" -C "$CONTAINER_PATH"

docker rm $container_id
rm "$CONTAINER_PATH/alpine-rootfs.tar"

echo "Alpine filesystem extracted to $CONTAINER_PATH"
