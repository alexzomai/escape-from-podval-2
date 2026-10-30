# AGENTS.md

Guidance for AI coding agents working in this repository.

## What this is

A Rust port of the Python/pygame raycaster [escape-from-podval](https://github.com/alexzomai/escape-from-podval). It is branded as the sequel "Escape From Podval 2".

The hard requirement is **pixel- and state-exact parity with the original**: the same logic, the same assets and the same frames. The only intended differences are:

- uncapped FPS;
- interpolated rendering between logic ticks;
- WASD works on any keyboard layout.

Any change to game logic or rendering must keep `tests/parity.rs` passing.

Comments, README and release notes are written in Russian. Keep that convention.

## Commands

On Linux, development needs SDL2, SDL2_ttf and ALSA from the system.

```sh
cargo run --release                     # play
cargo run --release -- --bench 5        # 5 s without menu/sound, prints average FPS
cargo test --release                    # unit tests + parity tests (release is much faster)
cargo test --release --test parity gameplay_matches_original   # single test
cargo test --lib rect::                 # unit tests of one module
cargo clippy --all-targets
cargo run --release --example bench     # per-stage render timings
```

- If a parity test fails, it writes PNG diffs to `target/parity/`.
- `PARITY_DIR=<dir>` points the test at a different reference set.

### Releases

Pushing a `v*` tag runs `.github/workflows/release.yml`. It builds `EscapeFromPodval2.exe` and `EscapeFromPodval2.sh` and attaches them to the GitHub release.

Both builds link SDL statically:

- **Linux:** cargo feature `static-sdl`. SDL comes from `scripts/build-sdl-static.sh`, used through pkg-config with `PKG_CONFIG_ALL_STATIC=1`. It is built on ubuntu-22.04 for an old glibc. `scripts/make-sh.sh` wraps the binary into a self-extracting `.sh` that unpacks to `~/.cache/escape-from-podval-2/`.
- **Windows:** cargo feature `static-sdl-vcpkg`. SDL comes from vcpkg `sdl2-ttf:x64-windows-static`, built with `+crt-static`.
- **Windows CI runs only `cargo test --lib`.** The parity reference was recorded on Linux, and the Windows CRT's libm may differ in the last bits.

## Architecture

### Module mapping

The modules mirror the original's Python modules:

| Rust | Python |
|---|---|
| `settings` | `settings.py` |
| `map` | `mapdata.py` |
| `player` | `player.py` |
| `raycast` | `ray_casting.py` |
| `sprites` | `sprites.py` |
| `game` | `interactions.py` plus the game loop of `main.py` |
| `render` | `render.py` |

The remaining modules:

- `gfx`: the software framebuffer and pygame-compatible drawing primitives.
- `assets`: image decoding and the SDL_ttf text cache.
- `audio`: rodio.
- `main.rs`: the SDL window, menu, game loop and end screen.

### Assets

`build.rs` embeds everything under `resources/` and `maps/` into the binary. The generated table is read with `embedded::file(path)`, and nothing is read from disk at runtime.

Adding or replacing an asset means rebuilding. These directories must stay byte-identical to the original's.

### Fixed-step logic, uncapped rendering

All of the original's mechanics are expressed per frame at 60 FPS, for example 6 px/frame movement and 0.1 damage/frame.

- **Logic:** `Game::tick` runs at a fixed 60 Hz from an accumulator in `main.rs`. It follows `main.py`'s per-frame order exactly. Reordering steps breaks parity.
- **Rendering:** runs every loop iteration. `Game::view(alpha, pending_mouse_dx)` interpolates the player and sprite positions. The interpolation `lerp` is written so that it is exact at alpha=1, and mouse turning is applied immediately.

### Software rendering

Frames are drawn into a `u32` buffer (`gfx::Canvas`) and uploaded to a streaming SDL texture with vsync off.

Walls and background are drawn in one row-major pass. Sprites are drawn after them, far to near, masked per column by wall depth: the sprite pixel is hidden if the wall is nearer, and the sprite wins a tie. This reproduces the original's painter's order exactly.

### Reproducing pygame/Python quirks

Parity depends on these behaviours. Do not "clean them up".

- **`Rect`:**
  - the constructor and `move` truncate;
  - the `center` setter rounds half away from zero;
  - blit positions truncate.
- **`transform.scale`:** uses pygame's Bresenham stretch, in `gfx::stretch_indices`.
- **Two alpha formulas:**
  - per-pixel alpha uses pygame's `ALPHA_BLEND_COMP`;
  - surface alpha (the damage overlay) uses SDL's blend formula.
- **Python `x**2`:** it is libm `pow`, not `x*x`. Use `sprites::py_square` (`powf` plus `black_box`).
- **Python `%` and `//`:** use `player::py_mod`.
- **`map.txt`:** it is CRLF, and the map width is computed exactly as the original does.
- **Original state bugs:** these are preserved on purpose. Examples are `npc_action_trigger` being truthy initially, and the raycaster's texture variables persisting across rays.
- **Text:** uses SDL_ttf, the same library as `pygame.font`. `antialias=True` maps to blended rendering; `antialias=False` maps to solid rendering with a colorkey.

### Parity test data

`tests/data/parity/` was produced by `tests/reference/harness.py`. The harness runs the original Python game with a bot (BFS navigation and aiming) for 7200 frames, using dummy SDL drivers, and records:

- the per-frame input;
- the state every 10 frames;
- a CRC32 of every frame;
- sample screenshots;
- the menu, win and lose screens.

`tests/parity.rs` replays that input through `Game::tick`. It compares the state bitwise and every rendered frame by CRC.

To regenerate the data, run `python tests/reference/harness.py <original repo checkout> <out dir> [frames]`. It needs pygame and Pillow.
