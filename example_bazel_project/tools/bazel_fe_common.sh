#!/bin/bash

BAZEL_FE_VERSION=v0.1-25
BUILDOZER_VERSION=3.5.0
export INDEX_INPUT_LOCATION=/tmp/bazelfe_current_index

TMPDIR="${TMPDIR:-/tmp}"
RND_UID="${USER}_$(date "+%s")_${RANDOM}"

if [ "$(uname -s)" == "Linux" ]; then
  export BAZEL_FE_PLATFORM_NAME='linux'
  export BUILDIFIER_PLATFORM_SUFFIX=""
elif [ "$(uname -s)" == "Darwin" ]; then
  export BAZEL_FE_PLATFORM_NAME='macos'
  export BUILDIFIER_PLATFORM_SUFFIX=".mac"
else
  "Your platform $(uname -s) is unsupported, sorry"
  exit 1
fi

if [ -z "$BAZEL_FE_TOOLS" ]; then
  BAZEL_FE_TOOLS=~/.bazelfe_tools
fi
mkdir -p "$BAZEL_FE_TOOLS"


BAZEL_RUNNER_URL=https://github.com/ianoc/bazel-fe/releases/download/${BAZEL_FE_VERSION}/bazel-runner-${BAZEL_FE_PLATFORM_NAME}
BAZEL_RUNNER_SHA_URL=https://github.com/ianoc/bazel-fe/releases/download/${BAZEL_FE_VERSION}/bazel-runner-${BAZEL_FE_PLATFORM_NAME}.sha256
BAZEL_RUNNER_LOCAL_PATH="${BAZEL_FE_TOOLS}/bazel-runner-${BAZEL_FE_VERSION}"


JVM_INDEXER_URL=https://github.com/ianoc/bazel-fe/releases/download/${BAZEL_FE_VERSION}/jvm-indexer-${BAZEL_FE_PLATFORM_NAME}
JVM_INDEXER_SHA_URL=https://github.com/ianoc/bazel-fe/releases/download/${BAZEL_FE_VERSION}/jvm-indexer-${BAZEL_FE_PLATFORM_NAME}.sha256
JVM_INDEXER_LOCAL_PATH="${BAZEL_FE_TOOLS}/jvm-indexer-${BAZEL_FE_VERSION}"

BUILDOZER_URL=https://github.com/bazelbuild/buildtools/releases/download/${BUILDOZER_VERSION}/buildozer${BUILDIFIER_PLATFORM_SUFFIX}
BUILDOZER_SHA_URL=""
BUILDOZER_LOCAL_PATH="${BAZEL_FE_TOOLS}/buildozer-${BUILDOZER_VERSION}"


export BUILD_DIR=${TMPDIR}/bazel_b_${RND_UID}
mkdir -p $BUILD_DIR


function fetch_binary() {
  TARGET_PATH="$1"
  FETCH_URL="$2"
  URL_SHA="$3"
  set +e
  which shasum &> /dev/null
  HAVE_SHASUM=$?
  set -e
  if [ ! -f $TARGET_PATH ]; then
    echo "Need to fetch new copy of tool, fetching... ${FETCH_URL}"
    ( # Opens a subshell
      set -e
      cd $BUILD_DIR

      echo $PWD
      curl -o tmp_download_file -L $FETCH_URL
      chmod +x tmp_download_file


      if [ "$HAVE_SHASUM" == "0" ]; then
        if [ -n "$URL_SHA" ]; then
          curl -o tmp_download_file_SHA -L $URL_SHA
          GENERATED_SHA_256=$(shasum -a 256 tmp_download_file | awk '{print $1}')

          if [ "$GENERATED_SHA_256" != "$(cat tmp_download_file_SHA)" ]; then
            echo "Sha 256 does not match, expected: $(cat tmp_download_file_SHA)"
            echo "But found $GENERATED_SHA_256"
            echo "Recommend you:  update the sha to the expected"
            echo "and then re-run this script"
            exit 1
          fi
        fi
      fi

      mv tmp_download_file "$TARGET_PATH"
    )
    rm -rf $BUILD_DIR
  fi
}
