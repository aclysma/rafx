# Rafx Blender Addon

Blender addon for automating common operations and exporting data to a format that is compatible with rafx

## Installation Instructions

The addon can be packaged into a zip, just zip the rafx-blender-addon folder (you can remove git-related
files and `__pycache__` if it exists).

## Usage Instructions

Documentation not yet available. Currently there are two primary features, exporting, and scene managment.

In both cases, a `.rafx_project` file is required in the root of your project. It should look something
like this:

```
{
    "art_dir": "art",
    "assets_dir": "assets"
}
```

The blender files in the art dir will be exported to the assets dir, 1:1. The export UI is in the
properties panels and the "N" menu of the 3d viewport, shader node editor, and image editor.

## Development Setup

Suggested steps using VS code
 * pip install fake-bpy-module-2.92
 * install the "Blender Development" extension ("jacqueslucke.blender-development")

Use the "Blender: Build and Start" command to launch blender with the addon installed
Use the "Blender: Reload Addons" command to reload the addon

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## External Code

There is some code derived from the GLTF importer/exporter in this addon. It is available under 
Apache 2.0 license here: https://github.com/KhronosGroup/glTF-Blender-IO

## Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT).
