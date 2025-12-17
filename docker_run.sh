#!/bin/bash

docker run \
  --rm \
  -p 8000:8000 \
  --cap-add SYS_ADMIN \
  --security-opt seccomp=unconfined \
  --security-opt apparmor=unconfined \
  executor
