# Windowing and Swapchains

Rafx uses raw-window-handle to avoid taking a direct dependency on any particular windowing solution. This means rafx
is compatible with `winit`, `sdl2`, and most other windowing APIs in the rust ecosystem.

In addition to a basic swapchain API (`RafxSwapchain`), the API includes `RafxSwapchainHelper`. It adds a few 
conveniences:
 * Automatic detection and rebuilding of the swapchain (for example on a window resize)
 * GPU synchronization with fences/semaphores to ensure that we wait until enough progress in previous frames has been
   made to safely reuse resources
 * An API that eases moving rendering to a separate thread.
     * Call `RafxSwapchainHelper::acquire_next_image(...)`. This will rebuild the swapchain if necessary and wait until it
       is safe to use the next swapchain image.
     * Use the returned `RafxPresentableFrame` to access the swapchain image
     * When rendering is complete, call `RafxPresentableFrame::present()`. This call can occur from a separate render 
       thread.