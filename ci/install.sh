#!/bin/bash

# `install` phase: install stuff needed for the `script` phase

set -ex

. $(dirname $0)/utils.sh

install_javascript_stuff() {
  curl -o- https://raw.githubusercontent.com/creationix/nvm/v0.31.1/install.sh | bash
  source ~/.nvm/nvm.sh

  nvm install 5.0.0

  pushd client
  npm install
  popd
}

main() {
    install_javascript_stuff
}

main
