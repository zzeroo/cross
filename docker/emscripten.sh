set -ex

main() {
    local version=$1

    local dependencies=(
        ca-certificates
        cmake
        curl
        g++
        ninja-build
        python
    )

    apt-get update
    apt-get install --no-install-recommends -y ${dependencies[@]}

    local td=$(mktemp -d)

    mkdir $td/{build,fastcomp}

    curl -L https://github.com/kripken/emscripten-fastcomp/archive/$version.tar.gz |
        tar --strip-components=1 -C $td/fastcomp -xz

    mkdir $td/fastcomp/tools/clang
    curl -L https://github.com/kripken/emscripten-fastcomp-clang/archive/$version.tar.gz |
        tar --strip-components=1 -C $td/fastcomp/tools/clang -xz

    pushd $td
    cmake \
        -G Ninja \
        -DCLANG_INCLUDE_TESTS=OFF \
        -DCMAKE_BUILD_TYPE=Release \
        -DLLVM_INCLUDE_EXAMPLES=OFF \
        -DLLVM_INCLUDE_TESTS=OFF \
        -DLLVM_TARGETS_TO_BUILD="X86;JSBackend" \
        $td/fastcomp

    nice ninja
    ninja install

    mkdir /emscripten
    curl -L https://github.com/kripken/emscripten/archive/$version.tar.gz |
        tar --strip-components=1 -C /emscripten -xz

    # TODO build tools/optimizer. I have no idea if `rustc` calls `emcc` in
    # a way that makes uses of that optimizer though.

    # Put `emcc` in `$PATH`
    ln -s /emscripten/emcc /usr/local/bin

    # Cleanup
    apt-get purge --auto-remove -y ${dependencies[@]}

    popd

    rm -rf $td
    rm $0
}

main "${@}"
