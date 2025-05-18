#!/usr/bin/env bash

# d="15_diskio"
d="16_filesystem"

pushd "./${d}_userland/" &&\
  ./build.sh &&\
  popd &&\
  pushd "./${d}_kernel/" &&\
  (cd disk && tar cf ../disk.tar --format=ustar *.txt) &&\
  rustup run nightly cargo run &&\
  popd
