<p align="center">
    <img src="./docs/logo.svg" height="60px" />
</p>

<h1 align="center">lunchd</h1>

<p align="center">
  <a href="https://github.com/mishamyrt/repomop/actions/workflows/ci.yml">
    <img src="https://github.com/mishamyrt/repomop/actions/workflows/ci.yml/badge.svg" />
  </a>
  <a href="https://github.com/mishamyrt/lunchd/releases/latest">
    <img src="https://img.shields.io/github/v/tag/mishamyrt/lunchd?label=version"
  </a>
</p>

Lightweight Rust library for creating and controlling macOS Launch Agents (launchd) via `launchctl`.

## Features

- Create launch agents
- Start and stop agents
- Check status

## Installation

Add `lunchd` to your `Cargo.toml`:

```toml
lunchd = "0.1"
```

or add with cargo cli:

```sh
cargo add lunchd
```

## Usage

```rust
let agent = LaunchAgent::builder("co.myrt.lunchd")
        .arg("/usr/bin/tail")
        .arg("-f")
        .arg("/dev/null")
        .keep_alive(KeepAlive::Crashed)
        .run_at_load(true)
        .build()
        .unwrap();

    // Install the agent
    agent.install()?;
    // Check if the agent is running
    println!("Is running: {}", agent.is_running()?);
    // Uninstall the agent
    agent.uninstall()?;
```

## License

MIT — see `LICENSE` for details.
