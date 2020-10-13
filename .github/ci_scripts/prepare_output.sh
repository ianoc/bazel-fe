set -e

#!/usr/bin/env bash


ARTIFACT_NAME=$1
OUTPUT_PATH=$2

BINARY=target/release/bazel-runner


GENERATED_SHA_256=$(shasum -a 256 $BINARY | awk '{print $1}')

mkdir $OUTPUT_PATH

mv $BINARY $OUTPUT_PATH/${ARTIFACT_NAME}
echo $GENERATED_SHA_256 > $OUTPUT_PATH/${ARTIFACT_NAME}.sha256
