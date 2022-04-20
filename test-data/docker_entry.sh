#!/bin/sh

set -x
set -e

compile () {
  cairo-compile $1 > $2
  chown $USER_ID:$GROUP_ID $2
}

compile "/contracts/run_past_end.cairo" "/artifacts/run_past_end.json"
compile "/contracts/bad_stop_ptr.cairo" "/artifacts/bad_stop_ptr.json"
