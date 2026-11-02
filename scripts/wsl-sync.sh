#!/bin/bash
# Copie le depot vers ~/wnbuild (WSL) sans target/.git, pour compiler sur le disque Linux (rapide).
set -e
mkdir -p ~/wnbuild
cd "$(dirname "$0")/.."
tar --exclude=./target --exclude=./.git --exclude=./vendor/_old -cf - . | tar -xf - -C ~/wnbuild
