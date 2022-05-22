#!/bin/sh

# Deterministically generate contract artifacts

docker run -it --rm \
    -v "$(pwd)/artifacts:/artifacts" \
    -v "$(pwd)/contracts:/contracts:ro" \
    -v "$(pwd)/docker_entry.sh:/entry.sh:ro" \
    --env "USER_ID=$(id -u)" \
    --env "GROUP_ID=$(id -g)" \
    --entrypoint "/entry.sh" \
    shardlabs/cairo-cli:0.8.2.1

# Using prettier instead of `jq` due to known issue:
#   https://github.com/xJonathanLEI/starknet-rs/issues/76#issuecomment-1058153538
docker run -it --rm \
    -v "$(pwd)/artifacts:/work" \
    --user root \
    tmknom/prettier:2.6.2 \
    --write .
