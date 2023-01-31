# Triton - Text-to-SVG diagrams

Triton takes a textual description of diagrams and turns them into SVG images.
Inspired by [mermaid.js](https://mermaid.js.org) it supports (part of) their syntax.

In contrast to mermaid it comes with a CLI inbuilt. Written it rust it natively compiles to both
 _the command line_ and _the browser_.

## Building

The project is a rust [workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) with three projects.

- **core**: contains the share logic that does most of the lifting
- **cli**: Generated the executable for the command line
- **browser**: Compiles to WASM to generate SVG in the browser


## `browser`

To build the WASM fragment:

- follow https://rustwasm.github.io/docs/book/game-of-life/setup.html
- execute `wasm-pack build`
