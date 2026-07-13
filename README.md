# lite-render-2d

lite-render-2d is a 2d renderer I wrote in Rust. The main idea is that it batches everything you draw into one buffer and sends it to the gpu in a single draw call, so you can throw a hundred thousand sprites at it and it stays fast and barely uses any memory.

![showcase](showcase.gif)

It does a fair bit more than draw shapes. It has rectangles, circles, lines and dashed lines, a path tessellator for arbitrary shapes and an svg path parser on top of it. It has sprites, sprite sheets and a texture atlas, sdf and bitmap fonts with a rich text layer, particles, trails, tilemaps, a camera with pan and zoom, a transform stack, some post processing, input handling and a bit of audio. There are two backends behind the same api, one on opengl and one on wgpu, so your code does not care which one it runs on.

On a laptop gpu it does around 100 fps at a hundred thousand sprites in one draw call, and it holds 60 fps up to about 150 thousand. Instanced draws go out in zero draw calls. The benchmark and an example for each feature are in the examples folder.

This is still in progress, and it is not on crates.io.
