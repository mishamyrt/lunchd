#![allow(clippy::print_stdout)]

use lunchctl::{LaunchAgent, KeepAlive};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let agent = LaunchAgent::builder("co.myrt.lunchctl")
        .arg("/usr/bin/tail")
        .arg("-f")
        .arg("/dev/null")
        .env("TIMESTAMP", &timestamp.to_string())
        .keep_alive(KeepAlive::Always)
        .run_at_load(true)
        .build()
        .unwrap();

    println!("Installing '{}' {}", agent.label, agent.path()?.display());
    agent.install()?;

    thread::sleep(Duration::from_millis(300));
    println!("Is running: {}", agent.is_running()?);

    println!("Uninstalling '{}'", agent.label);
    agent.uninstall()?;

    Ok(())
}
