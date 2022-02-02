#!/bin/sh

set -x
set -e

compile () {
  cairo-compile $1 | jq > $2
  chown $USER_ID:$GROUP_ID $2
}

apt-get update
apt-get install -y jq

compile "/contracts/run_past_end.cairo" "/artifacts/run_past_end.json"
