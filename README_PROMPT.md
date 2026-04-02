# README prompt for Claude Code

Read the full codebase first. Then write a README.md that makes people want to use this crate.

## tone and style
- Write like a developer who built this because they were frustrated, not like a marketing team. The motivation is real: wgpu uses 200MB of RAM to draw a rectangle. This fixes that.
- Short sentences. No fluff. No "blazingly fast" or "zero-cost abstractions" or any other overused rust buzzwords.
- Confidence without arrogance. Let the numbers do the talking.
- Casual but competent. Like a senior dev explaining their side project at a meetup.

## structure (follow this order)

### 1. opening hook (2-3 sentences max)
What this is and why it exists. Lead with the problem, not the solution. Something like "wgpu allocated 200MB before I drew my first sprite. This crate does 3000 objects in 20MB."

### 2. a real code example
Show the simplest possible working example — open a window, draw a shape, done. Under 20 lines. No boilerplate comments, no "// import the prelude" noise. Just the code.

### 3. numbers table
A comparison table showing RAM usage and object throughput. Compare against wgpu (raw), macroquad, and lite-render-2d. Use the real benchmarks from the codebase if they exist, otherwise use these approximate numbers:
- lite-render-2d: ~20MB at 3000 objects, 60fps
- wgpu raw: ~200MB baseline + objects
- macroquad: ~35-50MB at 3000 objects, 60fps

### 4. what it does (short bullet list)
The actual features. Sprites, shapes, batched draw calls, camera, backend-swappable. One line per feature, no sub-bullets.

### 5. what it doesn't do
Be honest. No 3D. No text rendering (yet). No audio. No ECS. No game loop. This is a renderer, not a framework. This section builds trust.

### 6. how it compares
One paragraph positioning against miniquad (lower level, you write your own batching), macroquad (game framework with more overhead), and femtovg (vector graphics, not sprites). The pitch: "macroquad's rendering, miniquad's footprint, none of the game framework."

### 7. installation
```toml cargo add``` one-liner plus the feature flags for backend selection (glow vs wgpu).

### 8. backends
Short explanation of glow (default, lightweight) vs wgpu (fallback, heavier). How to switch via feature flags.

### 9. examples
List the examples in the repo with one-line descriptions and how to run them.

### 10. license
Whatever license is in the repo. If none exists, use MIT.

## formatting rules
- No emojis anywhere
- No badges (build status, crates.io, etc) — add those later when they actually exist
- Headers use ## not #
- Code blocks specify the language (rust, toml, bash)
- Keep the whole thing under 200 lines
- No "Table of Contents" section — the readme should be short enough to not need one

## things to avoid
- Don't start with "# lite-render-2d" and then immediately repeat the name in the first sentence
- Don't use the word "blazingly"
- Don't use the phrase "batteries included"
- Don't open with a quote or a philosophical statement
- Don't include a massive feature matrix or comparison chart — keep it lean
- Don't explain what Rust is or what OpenGL is
- Don't say "contributions welcome" with a paragraph about how to contribute — just link to CONTRIBUTING.md (even if it doesn't exist yet)
