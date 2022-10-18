# Simple DeepZoom Whole Slide Image viewer

Based on OpenSlide and the DeepZoom protocol. Uses OpenSeaDragon as a frontend browser-based viewer. Written in Rust.

# Why?

Most pathology slide viewers are desktop apps that read slides from the local filesystem. My slides usually reside on a research HPC cluster of some kind. As these slides are usually multi-GB files, it can be quite annoying to have to download a full slide whenever I have to inspect one. This tool spins up a webserver that can serve any OpenSlide-compatible slide using the DeepZoom protocol for quick browser-based inspections of slides.

# Warning
The state of this project: its just a quick hack to solve a small problem. As the many `TODO` comments in the code suggest, this project is not what decent programmers would consider 'production quality'.

# Building

## Install the OpenSlide dependency
This tool uses forked code from these OpenSlide rust bindings: https://crates.io/crates/openslide.

See https://openslide.org/download/ for instructions on how to install OpenSlide 3.4.1.

On MacOS (ARM), do not forget to add OpenSlide to your LIBRARY_PATH.:
```sh
brew install openslide
export LIBRARY_PATH=$LIBRARY_PATH:/opt/homebrew/Cellar/openslide/3.4.1_7/lib
```

## Build and run the example

We use the CMU-1 slide, which is one of the example tissue slides provided by Carnegie Mellon University (licensed CC0 1.0).
```bash
cargo run --release ./assets/CMU-1-Small-Region.svs
```
Then open `127.0.0.1:8080` in the browser.

# Benchmarks

A single benchmark is provided for the `get_tile()` function. Run it using:

```bash
cargo bench
```

# Profiling using flamegraph
Requires https://crates.io/crates/flamegraph.

Note that `flamegraph` requires `sudo` privileges to run on most machines.

Example command:

```bash
sudo flamegraph -o flamegraph.svg ./target/release/deps/slidestream-28140b6ab18609c3
```

