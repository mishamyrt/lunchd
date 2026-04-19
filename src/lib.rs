mod launchctl;
mod plist;

use std::{env, path::PathBuf, fs};

use derive_builder::{Builder};
use thiserror::Error;

pub use derive_builder::UninitializedFieldError;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("home directory not found")]
    HomeNotFound,

    #[error("failed to write agent: {0}")]
    FailedToWrite(std::io::Error),

    #[error("failed to remove agent: {0}")]
    FailedToRemove(std::io::Error),

    #[error("agent not found")]
    NotFound,

    #[error("launchctl command failed: {0}")]
    LaunchCtlFailed(#[from] launchctl::LaunchCtlError),
}

/// Launch Agent configuration.
///
/// A Launch Agent is a macOS mechanism for automatically starting user-level processes
/// at login or in response to specific events. Launch Agents are described using plist files,
/// which are placed in the `~/Library/LaunchAgents` directory.
///
/// More information:
/// <https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html>
#[derive(Clone, Debug, Default, Builder)]
pub struct LaunchAgent {
    /// The label of the Launch Agent, used to identify it in the system.
    #[builder(setter(custom))]
    pub label: String,

    /// The program arguments to pass to the Launch Agent's program.
    /// First element is the program path.
    #[builder(setter(into))]
    pub program_arguments: Vec<String>,

    /// The environment variables to set for the Launch Agent.
    #[builder(setter(custom), default)]
    pub environment_variables: Vec<EnvironmentVariable>,

    #[builder(setter(strip_option), default)]
    /// The keep-alive policy for the Launch Agent.
    pub keep_alive: Option<KeepAlive>,

    #[builder(setter(strip_option), default)]
    /// The process type for the Launch Agent.
    pub process_type: Option<ProcessType>,

    /// Whether the Launch Agent should be run at login.
    #[builder(default = "false")]
    pub run_at_load: bool,

    /// Whether the Launch Agent should be started on mount.
    ///
    /// Note: `launchd` does not report which device has been mounted.
    #[builder(setter(strip_option), default)]
    pub start_on_mount: Option<bool>,

    /// The working directory for the Launch Agent's program.
    #[builder(setter(into, strip_option), default)]
    pub working_directory: Option<PathBuf>,

    /// The path to the file to redirect standard input to.
    #[builder(setter(into, strip_option), default)]
    pub stdin_path: Option<PathBuf>,

    /// The path to the file to redirect standard output to.
    #[builder(setter(into, strip_option), default)]
    pub stdout_path: Option<PathBuf>,

    /// The path to the file to redirect standard error to.
    #[builder(setter(into, strip_option), default)]
    pub stderr_path: Option<PathBuf>,
}

/// The process type for the Launch Agent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcessType {
    /// Background jobs are generally processes that do work that was not
    /// directly requested by the user. The resource limits applied to
    /// Background jobs are intended to prevent them from disrupting the
    /// user experience.
    Background,
    /// Standard jobs are equivalent to no `ProcessType` being set.
    Standard,
    /// Adaptive jobs move between the Background and Interactive classifications
    /// based on activity over XPC connections.
    Adaptive,
    /// Interactive jobs run with the same resource limitations as apps,
    /// that is to say, none. Interactive jobs are critical to maintaining
    /// a responsive user experience, and this key should only be used if
    /// an app's ability to be responsive depends on it, and cannot be made
    /// Adaptive.
    Interactive,
}

/// The keep-alive policy for the Launch Agent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeepAlive {
    /// Always keep the agent running.
    /// If the agent exits with any code, it will be restarted automatically.
    Always,
    /// Start agent running only until it exits successfully.
    SuccessfulExit,
    /// Start agent running only until it exits successfully, and restart if it crashes.
    Crashed,
    /// Setting this variant to true will start the job when/while any network is/becomes available.
    /// Setting this variant to false will start the job when/while all network connections are down.
    NetworkState(bool),
    /// Start agent running only while the given paths exists.
    PathExists(PathBuf),
    /// Start agent running only while the given paths does not exist.
    PathNotExists(PathBuf),
    /// Do not restart the agent automatically if it exits.
    Disabled,
}

/// A variable to set in the environment for the Launch Agent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentVariable {
    pub key: String,
    pub value: String,
}

impl LaunchAgent {
    /// Creates a new `LaunchAgent` with the given label.
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            ..LaunchAgent::default()
        }
    }

    /// Creates a new `LaunchAgent` builder with the given label.
    pub fn builder(label: &str) -> LaunchAgentBuilder {
        LaunchAgentBuilder {
            label: Some(label.to_string()),
            ..LaunchAgentBuilder::default()
        }
    }

    /// Returns the path to the launch agent plist file.
    pub fn path(&self) -> Result<PathBuf, AgentError> {
        let home_dir = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or(AgentError::HomeNotFound)?;

        Ok(home_dir
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{}.plist", self.label)))
    }

    /// Renders the launch agent plist as a string.
    pub fn as_string(&self) -> String {
        plist::render(self)
    }

    /// Installs the launch agent by writing its plist file and bootstrapping it with launchctl.
    pub fn install(&self) -> Result<(), AgentError> {
        let domain = launchctl::GuiDomain::current();
        let service_target = launchctl::ServiceTarget::new(domain, &self.label);
        let agent_path = self.path()?;

        // Remove any existing service with the same label
        let _ = launchctl::bootout(service_target);

        let agent_content = self.as_string();
        fs::write(&agent_path, &agent_content).map_err(AgentError::FailedToWrite)?;

        launchctl::bootstrap(domain, &agent_path)
            .map_err(AgentError::LaunchCtlFailed)
    }

    /// Uninstalls the launch agent by removing its plist file and bootstrapping it with launchctl.
    pub fn uninstall(&self) -> Result<(), AgentError> {
        let domain = launchctl::GuiDomain::current();
        let service_target = launchctl::ServiceTarget::new(domain, &self.label);
        let agent_path = self.path()?;

        if !agent_path.exists() {
            return Err(AgentError::NotFound);
        }

        fs::remove_file(&agent_path).map_err(AgentError::FailedToRemove)?;

        launchctl::bootout(service_target).map_err(AgentError::LaunchCtlFailed)
    }

    /// Returns `true` if the launch agent is currently running.
    pub fn is_running(&self) -> Result<bool, AgentError> {
        let domain = launchctl::GuiDomain::current();
        let service_target = launchctl::ServiceTarget::new(domain, &self.label);

        launchctl::check_is_running(service_target)
            .map_err(AgentError::LaunchCtlFailed)
    }

    /// Returns `true` if the launch agent exists on disk.
    pub fn exists(&self) -> bool {
        self.path().ok().as_ref().is_some_and(|p| p.exists())
    }
}

impl LaunchAgentBuilder {
    /// Environment variables to set for the Launch Agent.
    pub fn env(&mut self, key: &str, value: &str) -> &mut Self {
        let env = EnvironmentVariable {
            key: key.to_string(),
            value: value.to_string(),
        };
        match self.environment_variables {
            Some(ref mut vars) => vars.push(env),
            None => {
                self.environment_variables = Some(vec![env]);
            }
        }
        self
    }

    /// Program arguments to pass to the Launch Agent's program.
    pub fn arg(&mut self, arg: impl Into<String>) -> &mut Self {
        match self.program_arguments {
            Some(ref mut args) => args.push(arg.into()),
            None => {
                self.program_arguments = Some(vec![arg.into()]);
            }
        }
        self
    }
}
