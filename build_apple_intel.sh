# If you are running this on an apple silicon machine you need to first install this toolchain.
# command: rustup target add x86_64-apple-darwin

SDKROOT=$(xcrun -sdk macosx --show-sdk-path) MACOSX_DEPLOYMENT_TARGET=$(xcrun -sdk macosx --show-sdk-platform-version) cargo build --target=x86_64-apple-darwin --release
