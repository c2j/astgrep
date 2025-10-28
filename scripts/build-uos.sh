#!/bin/bash
# Build script for Linux targets using vcpkg with fallback support

# 如果当前目录下有Dockerfile.nos，则使用Dockerfile.nos
if [ -f "Dockerfile.uos" ]; then
    echo "Using Dockerfile.uos"
    DOCKERFILE="Dockerfile.uos"
else
    cd ..
fi  


# docker build -t seeu-desktop-linux-builder -f Dockerfile.uos .
# docker create --name seeu-temp-container seeu-desktop-linux-builder
# docker cp seeu-temp-container:/output/seeu_desktop dist/linux/
# docker rm seeu-temp-container

mkdir -p dist/linux

docker build -t astgrep-linux-builder -f Dockerfile.uos .
docker create --name astgrep-container astgrep-linux-builder
docker cp astgrep-container:/output/astgrep dist/linux/
docker cp astgrep-container:/output/astgrep-cli dist/linux/
docker cp astgrep-container:/output/astgrep-web dist/linux/
docker cp astgrep-container:/output/astgrep-gui dist/linux/
docker rm astgrep-container
