# If you are running this on an apple intel machine you need to first install this toolchain.
# command: rustup target add aarch64-apple-darwin

SDKROOT=$(xcrun -sdk macosx --show-sdk-path) MACOSX_DEPLOYMENT_TARGET=$(xcrun -sdk macosx --show-sdk-platform-version) cargo build --target=aarch64-apple-darwin --release
