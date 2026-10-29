#!/bin/sh
# Собирает статические SDL2 и SDL2_ttf (со встроенным FreeType) в PREFIX,
# чтобы релизный бинарник не зависел от SDL в системе.
#   scripts/build-sdl-static.sh <папка сборки> <PREFIX>
set -eu
SDL_VERSION=2.32.10
TTF_VERSION=2.24.0
WORK=$(realpath -m "$1")
PREFIX=$(realpath -m "$2")
mkdir -p "$WORK" "$PREFIX"
cd "$WORK"

[ -d "SDL2-$SDL_VERSION" ] || curl -sSL "https://github.com/libsdl-org/SDL/releases/download/release-$SDL_VERSION/SDL2-$SDL_VERSION.tar.gz" | tar xz
[ -d "SDL2_ttf-$TTF_VERSION" ] || curl -sSL "https://github.com/libsdl-org/SDL_ttf/releases/download/release-$TTF_VERSION/SDL2_ttf-$TTF_VERSION.tar.gz" | tar xz

cmake -S "SDL2-$SDL_VERSION" -B build-sdl -G Ninja -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$PREFIX" -DSDL_SHARED=OFF -DSDL_STATIC=ON -DSDL_STATIC_PIC=ON -DSDL_TEST=OFF
cmake --build build-sdl
cmake --install build-sdl

cmake -S "SDL2_ttf-$TTF_VERSION" -B build-ttf -G Ninja -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$PREFIX" -DCMAKE_PREFIX_PATH="$PREFIX" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    -DCMAKE_POLICY_VERSION_MINIMUM=3.5 -DBUILD_SHARED_LIBS=OFF -DSDL2TTF_VENDORED=ON -DSDL2TTF_HARFBUZZ=OFF -DSDL2TTF_SAMPLES=OFF
cmake --build build-ttf
cmake --install build-ttf
