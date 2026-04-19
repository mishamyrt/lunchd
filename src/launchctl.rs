use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use thiserror::Error;

unsafe extern "C" {
    /// Returns the effective user ID of the current process.
    fn geteuid() -> u32;
}

#[derive(Debug, Error)]
pub enum LaunchCtlError {
    #[error("launchctl command failed: {0}")]
    CommandExecutionFailed(#[from] std::io::Error),

    #[error("launchctl failed: {0}")]
    CommandFailed(String),
}

type Result<T> = std::result::Result<T, LaunchCtlError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GuiDomain {
    uid: u32,
}

impl GuiDomain {
    /// Creates a new GUI domain with the given user ID.
    const fn new(uid: u32) -> Self {
        Self { uid }
    }

    /// Returns the current GUI domain.
    pub(crate) fn current() -> Self {
        Self::new(unsafe { geteuid() })
    }

    /// Formats this GUI domain as an argument for launchctl commands.
    fn as_argument(self) -> String {
        format!("gui/{}/", self.uid)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ServiceTarget<'a> {
    domain: GuiDomain,
    label: &'a str,
}

impl<'a> ServiceTarget<'a> {
    /// Creates a new service target with the given GUI domain and label.
    pub(crate) fn new(domain: GuiDomain, label: &'a str) -> Self {
        Self { domain, label }
    }

    /// Formats this service target as an argument for launchctl commands.
    fn as_argument(self) -> String {
        format!("gui/{}/{}", self.domain.uid, self.label)
    }
}

pub(crate) fn bootstrap(domain: GuiDomain, plist_path: &Path) -> Result<()> {
    let args = bootstrap_args(domain, plist_path);
    run_launchctl(&args).map(|_| ())
}

pub(crate) fn bootout(service_target: ServiceTarget<'_>) -> Result<()> {
    let args = bootout_args(service_target);
    run_launchctl(&args).map(|_| ())
}

pub(crate) fn check_is_running(service_target: ServiceTarget<'_>) -> Result<bool> {
    let args = print_args(service_target);
    let output = run_launchctl(&args)?;
    Ok(is_running(&output))
}

/// Returns `true` if the output contains running flag.
fn is_running(output: &str) -> bool {
    output.contains("state = running")
}

fn run_launchctl<const N: usize>(args: &[OsString; N]) -> Result<String> {
    let output = Command::new("launchctl")
        .args(args)
        .output()
        .map_err(LaunchCtlError::CommandExecutionFailed)?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        return Ok(stdout);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(LaunchCtlError::CommandFailed(stderr))
}

fn bootstrap_args(domain: GuiDomain, plist_path: &Path) -> [OsString; 3] {
    [
        OsString::from("bootstrap"),
        OsString::from(domain.as_argument()),
        plist_path.as_os_str().to_owned(),
    ]
}

fn bootout_args(service_target: ServiceTarget<'_>) -> [OsString; 2] {
    [
        OsString::from("bootout"),
        OsString::from(service_target.as_argument()),
    ]
}

fn print_args(service_target: ServiceTarget<'_>) -> [OsString; 2] {
    [
        OsString::from("print"),
        OsString::from(service_target.as_argument()),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        GuiDomain, ServiceTarget, bootout_args, bootstrap_args, print_args,
        is_running,
    };
    use std::ffi::OsString;
    use std::path::Path;

    fn stringify_args<const N: usize>(args: &[OsString; N]) -> Vec<String> {
        args.iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn gui_domain_formats_domain_target() {
        assert_eq!(GuiDomain::new(501).as_argument(), "gui/501/");
    }

    #[test]
    fn service_target_formats_service_target() {
        let domain = GuiDomain::new(501);
        let service_target = ServiceTarget::new(domain, "co.myrt.nanomiddleclick");

        assert_eq!(
            service_target.as_argument(),
            "gui/501/co.myrt.nanomiddleclick"
        );
    }

    #[test]
    fn bootstrap_uses_expected_arguments() {
        let args = bootstrap_args(
            GuiDomain::new(501),
            Path::new("/tmp/co.myrt.nanomiddleclick.plist"),
        );

        assert_eq!(
            stringify_args(&args),
            [
                "bootstrap",
                "gui/501/",
                "/tmp/co.myrt.nanomiddleclick.plist",
            ]
        );
    }

    #[test]
    fn bootout_uses_expected_arguments() {
        let args = bootout_args(ServiceTarget::new(
            GuiDomain::new(501),
            "co.myrt.nanomiddleclick",
        ));

        assert_eq!(
            stringify_args(&args),
            ["bootout", "gui/501/co.myrt.nanomiddleclick"]
        );
    }

    #[test]
    fn print_uses_expected_arguments() {
        let args = print_args(ServiceTarget::new(
            GuiDomain::new(501),
            "co.myrt.nanomiddleclick",
        ));

        assert_eq!(
            stringify_args(&args),
            ["print", "gui/501/co.myrt.nanomiddleclick"]
        );
    }

    #[test]
    fn is_running_validates_output() {
        let output = "
{
        domain = gui/501 [100003]
        asid = 100003

        jetsam memory limit (active) = (unlimited)
        jetsam memory limit (inactive) = (unlimited)
        jetsamproperties category = daemon
        jetsam thread limit = 32
        cpumon = default
        job state = running
        probabilistic guard malloc policy = {
                activation rate = 1/1000
                sample rate = 1/0
        }

        properties = keepalive | runatload | inferred program | managed LWCR | has LWCR
}
        ";
        assert!(is_running(output));

        let output = "
        {
            domain = gui/501 [100003]
            asid = 100003
        }
        ";
        assert!(!is_running(output));
    }
}
