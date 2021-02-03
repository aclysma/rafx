# Building for iOS

The broad, general steps are:
 * Create a rust crate
     * Make it produce a static C library instead of an executable
 * Get SDL2 (I like to clone https://github.com/spurious/SDL-mirror.git)
 * Create an xcode project that static links your rust code and SDL2

This doc also includes instructions for using assets on-device.

## Create a rust crate

```
cargo new --lib ios-rust-sdl2
```

### Make it produce a static C library instead of an executable

Add this to the Cargo.toml

```
[lib]
crate-type = ["lib", "staticlib"]
```

### Add Rust dependencies

Also, add some additional dependencies. Most likely you'll want to update the versions used here
You can use path = "../rafx/rafx" instead of version if you prefer

```
[dependencies]
# use the metal backend of rafx
rafx = { version = "0.1", features = ["rafx-metal", "assets"] }

# use SDL2, raw-window-handle is required to use rafx
sdl2 = { version = "0.34", features = ["static-link", "raw-window-handle"] }

# Interop with iOS
objc = { version = "0.2.4", features = ["exception"] }
objc-foundation = "0.1"
cocoa-foundation = "0.1.0"
objc_id = "0.1"
libc = "0.2"

# Logging
env_logger = "0.6"
log="0.4"
```

### Patch sdl2-rs crate to not link static libraries

We are going to manually build/link SDL2 ourselves, but I didn't see a way to get sdl2-rs to NOT try to link its own
SDL2. So unfortunately we need to modify sdl2-rs.

 * Clone the rust sdl2 crate (https://github.com/Rust-SDL2/rust-sdl2)
 * In sdl2-sys/build.rs, make this edit:

```rust
if cfg!(feature = "bundled")
    || (cfg!(feature = "use-pkgconfig") == false && cfg!(feature = "use-vcpkg") == false)
{
    // COMMENT THESE LINES OUT
    // println!("cargo:rustc-link-lib=static=SDL2main");
    // println!("cargo:rustc-link-lib=static=SDL2");
}
```

 * Update your Cargo.toml to use this local version instead of crates-io

```
[patch.crates-io]
sdl2-sys = { path = "../rust-sdl2/sdl2-sys" }
sdl2 = { path = "../rust-sdl2" }
```

## Get SDL2

This will set it up as a git submodule

```
git submodule add --name SDL2 https://github.com/spurious/SDL-mirror.git SDL2
```

We will add a script to xcode that will force it to build so don't worry about that now

## Create an xcode project

We can use [xcodegen](https://github.com/yonaskolb/XcodeGen) to make this a lot easier. Fastest way to install is 
`brew install xcodegen`.

Create a folder xcode in the root of your rust crate (next to Cargo.toml)

### xcode/project.yml (Read the comments, modify as needed!)

```yaml

# This assumes a directory structure like this:
# root
#   SDL2 # git clone of https://github.com/spurious/SDL-mirror.git
#   xcode
#     project.yml # THIS FILE
#     src
#       main.cpp
#  Cargo.toml #rust lib, with [lib] crate-type = ["lib", "staticlib"]
#  src (rust code)

name: RustSdl2App

options:
  bundleIdPrefix: # <-- Provide your own prefix like com.yourcompanyname
  createIntermediateGroups: true
  usesTabs: false
  indentWidth: 4
  tabWidth: 4
  deploymentTarget:
    iOS: "12.0"

settings:
  CLANG_CXX_LANGUAGE_STANDARD: c++11
  CLANG_CXX_LIBRARY: libc++
  GCC_C_LANGUAGE_STANDARD: c11
  CLANG_WARN_DOCUMENTATION_COMMENTS: false

targets:
  RustSdl2App:
    type: application
    platform: iOS
    info:
      path: Generated/Info.plist
      properties:
        LSRequiresIPhoneOS: true
        UIRequiredDeviceCapabilities: [arm64]
        UIRequiresFullScreen: true
        UIStatusBarHidden: true
        UISupportedInterfaceOrientations: [UIInterfaceOrientationLandscapeLeft, UIInterfaceOrientationLandscapeRight].  # <-- Landscape only
        UILaunchStoryboardName: LaunchScreen # <-- Maybe this can be removed?
    entitlements:
      path: Generated/app.entitlements
    sources:
      - src # <-- This just holds a single main.cpp that calls into rust
      # You can add more data to deploy to device like this
      # - path: ../custom_data_file.txt
      #   type: data
      #   buildPhase: resources
      # - path: assets
      #   type: folder
      #   buildPhase: resources
    settings:
      # DEVELOPMENT_TEAM: XXXXXXXXXX <-- Provide your own ID or remove this line and do it manually in the UI.
      # (If you don't know it, do it manually in the UI and open your .xcproj in a text editor to find it)
      ENABLE_BITCODE: false
      HEADER_SEARCH_PATHS: $(PROJECT_DIR)/../SDL2/include
      LIBRARY_SEARCH_PATHS:
        - $(inherited)
        - $(PROJECT_DIR)/../target
        - $(PROJECT_DIR)/sdl_build/Debug-iphoneos # This example is building debug
    dependencies:
      - sdk: Metal.framework
      - framework: Libs/libSDL2.a
        embed: false
      - framework: libios-rust-sdl2.a # <-- Update to "lib[YOUR_CRATE_NAME].lib
        embed: false
      - sdk: CoreServices.framework
      - sdk: CoreMotion.framework
      - sdk: CoreGraphics.framework
      - sdk: AudioToolbox.framework
      - sdk: CoreAudio.framework
      - sdk: QuartzCore.framework
      - sdk: GameController.framework
      - sdk: Foundation.framework
      - sdk: OpenGLES.framework
      - sdk: UIKit.framework
      - sdk: AVFoundation.framework
      - sdk: ImageIO.framework
      - sdk: Security.framework
      - sdk: CoreHaptics.framework
    preBuildScripts:
      - name: Build Rust
        #path: ../build_rush.sh <-- alternative to inlining the script here
        # THIS ASSUMES THE PROJECT STRUCTURE AS EXPLAINED ABOVE. Update "ios-rust-sdl2" to match [YOUR_CRATE_NAME]
        script: |
          cd ${SRCROOT}/..
          cargo build --package ios-rust-sdl2 --target aarch64-apple-ios
          cp ${SRCROOT}/../target/aarch64-apple-ios/debug/libios-rust-sdl2.a ${SRCROOT}/../target/libios-rust-sdl2.a
          xcodebuild -project ${SRCROOT}/../SDL2/Xcode/SDL/SDL.xcodeproj -scheme "Static Library-iOS" build SYMROOT="${SRCROOT}/sdl_build"
```

Add a src folder in the xcode folder (so [root]/src/main.cpp)

### xcode/src/main.cpp

```C
#include "SDL2/SDL.h"
#include <stdio.h>

extern "C" void run_the_game();

extern "C" int main(int argc, char *argv[])
{
    run_the_game();
    printf("run_the_game returned");
    return 0;
}
```

### Run xcodegen

Assuming you already did `brew install xcodegen` or otherwise have xcodegen on your path:

```
cd xcode
xcodegen
```

This will produce a .xcodeproj that you can double click to open.

## Using Assets

Since a shader is necessary to do anything interesting with rafx, we will need to support bundling asset data with the
app. There are two approaches:
 * Generate a pack file using distill - that way we're just dealing with a single file. This is what you would do in a
   shipping build.
 * It's also possible to stream assets over a local network.

### Packfile Method

#### Generate a pack file

Generate the pack file like this, and place it in the root of your crate

```
run --bin cli -- pack out.pack
```

#### Modify project.yml to include it

```
# Read comments in project.yml above
# but it's something like this
targets:
  RustSdl2App:
    sources:
    - src
    - path: ../out.pack
      type: data
      buildPhase: resources
```

#### Read the file from your app

Accessing a file on iOS is unfortunately not as straightforward as a posix call. Here's a snippet you can use. This will
give you a path which you can use with standard file access APIs in rust

```rust
pub fn find_path_to_bundle_resource(file_name: &str, file_extension: &str) -> Option<String> {
    use objc::runtime::{Class, Object};
    use objc_id::Id;
    use objc_foundation::{INSString, NSString};
    use cocoa_foundation::base::nil;
    use objc::msg_send;
    use objc::sel;
    use objc::sel_impl;

    unsafe {
        let ns_bundle_class = Class::get("NSBundle").unwrap();
        let bundle: *mut Object = msg_send![ns_bundle_class, mainBundle];
        let path_for_resource = NSString::from_str(file_name).share();
        let resource_type = NSString::from_str(file_extension).share();
        let path: *mut Object = msg_send![bundle, pathForResource:path_for_resource ofType:resource_type];
        let cstr: *const libc::c_char = msg_send![path, UTF8String];
        if cstr != std::ptr::null() {
            let rstr = std::ffi::CStr::from_ptr(cstr).to_string_lossy().into_owned();
            return Some(rstr);
        }

        return None;
    }
}
```

Use it like this:

```rust
let packfile_path = find_path_to_bundle_resource("out", "pack");
```

#### Set up distill to read the packfile like this:

```rust
let mut asset_resource = {
    let packfile = std::fs::File::open(packfile_path).unwrap();
    let packfile_loader = rafx::distill::loader::PackfileReader::new(packfile).unwrap();
    let loader = Loader::new(Box::new(packfile_loader));
    let resolver = Box::new(DefaultIndirectionResolver);

    AssetResource::new(loader, resolver)
};
```

### Streaming via Network Method:

Run this on your PC:

```
run --bin cli -- host-daemon
```

Set up distill to stream assets over the network like this:

```rust
let connect_string = "192.168.0.X:9999"; // This should be your asset daemon host
let mut asset_resource = {
    let rpc_loader = RpcIO::new(connect_string.to_string()).unwrap();
    let loader = Loader::new(Box::new(rpc_loader));
    let resolver = Box::new(DefaultIndirectionResolver);
};
```
