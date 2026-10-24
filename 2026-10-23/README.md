# Night Signal

Keep a small emergency radio station on the air through a 14-night blackout. Match each district's need with a broadcast, route station power, and switch on relay towers to build resonance. The city is a network: matching a powered neighborhood compounds its reach. Rumors earn quick listeners and burn trust; verified bulletins and open lines take a steadier path.

## Run

```sh
cargo run --release
```

## Controls

- `1`–`4`: choose a broadcast program.
- Click a district: switch its relay on or off (the first two relays are permanently powered).
- Click the power routing buttons: route power to transmitter, studio, verifier, archive, and tower.
- `Space` or click **Broadcast Now**: commit the night's broadcast.
- `U`: buy a station upgrade for six funds.
- `M`: mute (reserved for the audio setting).
- `R`: begin another campaign after the ending; `Esc`: quit.

A synthesized radio score swells for the campaign finale, with broadcast stingers. Press `M` to mute. Each night consumes power and advances district needs. The blackout periodically cuts audience and trust. Keep the station's resources visible, meet the campaign's final audience and trust targets, and avoid running out of transmitter power.

## Checks and screenshots

```sh
cargo test --release
cargo run --release -- --selftest
cargo run --release -- --screenshot ./screenshots
```

The engine-free simulation and software renderer are in `src/lib.rs`; `src/main.rs` adapts the simulation to minifb and writes PNG screenshots.
