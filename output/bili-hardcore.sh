#!/bin/bash
# bili-hardcore 启动脚本
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)
"$SCRIPT_DIR/bili-hardcore_bin" "$@"
