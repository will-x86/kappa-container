# Kappa-Container

A lightweight container tool built in Rust, inspired by Docker but with a minimalist approach.

## What is this?

Kappa Container is a simple containerization tool that allows you to run commands in isolated environments. It's perfect for developers who want to understand the basics of container technology or experiment with a bare-bones implementation.

## Features

- Create isolated environments using Linux namespaces
- Run commands within these containers
- Minimal dependencies and straightforward code

## Prerequisites

- Rust (latest stable version)
- Linux environment (this tool uses Linux-specific system calls)
- Root privileges (required for namespace operations)
- Docker (used to obtain the Alpine Linux rootfs)

## Installation

1. Clone this repository:
   ```
   git clone https://github.com/seal/kappa-container.git
   cd kappa-container
   ```

2. Build the project:
   ```
   cargo build --release
   ```

3. Set up the Alpine Linux rootfs:
   - Create a `.env` file in the project root with the following content:
     ```
     ALPINE_PATH=/path/to/your/desired/alpine/rootfs
     ```
   - Run the provided shell script to set up the Alpine Linux rootfs:
     ```
     chmod +x setup_alpine.sh
     ./setup_alpine.sh
     ```

   This script will:
   - Pull the latest Alpine Linux image
   - Extract its filesystem
   - Clean up temporary files

## Usage

The primary command to run a shell inside the container is:

```
sudo ./target/release/kappa-container run /bin/sh
```

This will start a new shell inside the container environment.

You can also run other commands. For example:

```
sudo ./target/release/kappa-container run /bin/ls -l 
```

## How it works

Kappa Container uses Linux namespaces to create isolated environments. It:

1. Creates new UTS, PID, and Mount namespaces
2. Forks a child process
3. Sets up the container environment (hostname, filesystem, mounts)
4. Executes the specified command inside this environment

## The setup_alpine.sh script

This script automates the process of setting up the Alpine Linux rootfs:

```bash
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
```

This script pulls the latest Alpine Linux image, extracts its filesystem to the specified `ALPINE_PATH`, and cleans up temporary files and containers.

## Limitations

- This is a basic implementation and lacks many features of full-fledged container runtimes
- It requires root privileges to run
- Only tested on Linux systems

## Contributing

Feel free to open issues or submit pull requests if you have ideas for improvements or find any bugs!

## Inspiration 

Liz Rice's 'Containers from Scratch' here:
https://www.youtube.com/watch?v=8fi7uSYlOdc
## License

MIT License 

