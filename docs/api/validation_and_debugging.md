# Validation and Debugging

Graphics APIs are complex and it's easy to get something wrong. This can often lead to frustrating and time-consuming
debugging sessions.

Most graphics APIs provide some mechanism to turn on additional runtime checking

## Enabling Validation

### Metal

Set the environment variable METAL_DEVICE_WRAPPER_TYPE=1

### Vulkan

Set `RafxApiDef::validation_mode` to the desired setting.

* NOTE: This may move to `RafxApiDefVulkan` in the future

## GPU Debugging

Most APIs have tools in their ecosystem to help diagnose problems

### Metal

XCode includes a metal debugger. To use it with a rust program, create a dummy project and manually change the "scheme"
to run your binary in the correct current directory. Unfortunately xcode may crash when trying to do a capture, but when
it works, it has a very complete feature set including shader debugging.

### Vulkan

Renderdoc is a good tool for windows and linux. Other tools from nvidia and AMD can be useful too.