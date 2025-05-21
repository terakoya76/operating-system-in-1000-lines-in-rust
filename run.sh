#!/usr/bin/env bash

# d="15_diskio"
# d="16_filesystem"
d="17_refactoring"

pushd "./${d}_userland/" &&\
  ./build.sh &&\
  popd &&\
  pushd "./${d}_kernel/" &&\
  (cd disk && tar cf ../disk.tar --format=ustar *.txt) &&\
  cargo run &&\
  popd
