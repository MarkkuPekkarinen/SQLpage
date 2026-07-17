#!/bin/bash
set -euo pipefail

source /tmp/build-env.sh

PROFILE="${CARGO_PROFILE:-superoptimized}"
OUTPUT_DIR="$PROFILE"
if [ "$PROFILE" = "dev" ]; then
    OUTPUT_DIR="debug"
fi
echo "Building project for target: $TARGET (profile: $PROFILE)"

cargo build \
    --target "$TARGET" \
    --config "target.$TARGET.linker=\"$LINKER\"" \
    --features odbc-static \
    --profile "$PROFILE"

mv "target/$TARGET/$OUTPUT_DIR/sqlpage" sqlpage.bin
