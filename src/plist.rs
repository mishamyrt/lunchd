use std::path::PathBuf;

use crate::{KeepAlive, LaunchAgent, ProcessType};

pub(crate) fn render(agent: &LaunchAgent) -> String {
    let mut plist = PlistBuilder::new();
    plist.open_dict();
    plist.push_key("Label");
    plist.push_string(&agent.label);
    plist.push_key("RunAtLoad");
    plist.push_bool(agent.run_at_load);

    render_program_arguments(&mut plist, &agent.program_arguments);
    maybe_render_path(
        &mut plist,
        "WorkingDirectory",
        agent.working_directory.as_ref(),
    );
    maybe_render_path(&mut plist, "StandardInPath", agent.stdin_path.as_ref());
    maybe_render_path(&mut plist, "StandardOutPath", agent.stdout_path.as_ref());
    maybe_render_path(&mut plist, "StandardErrorPath", agent.stderr_path.as_ref());
    maybe_render_keep_alive(&mut plist, agent.keep_alive.as_ref());
    maybe_render_process_type(&mut plist, agent.process_type.as_ref());

    plist.close_dict();
    plist.finish()
}

fn render_program_arguments(
    plist: &mut PlistBuilder,
    program_arguments: &Vec<String>,
) {
    plist.push_key("ProgramArguments");
    plist.open_array();
    for arg in program_arguments {
        plist.push_string(arg);
    }
    plist.close_array();
}

fn maybe_render_keep_alive(
    plist: &mut PlistBuilder,
    keep_alive: Option<&KeepAlive>,
) {
    let Some(keep_alive) = keep_alive else {
        return;
    };
    plist.push_key("KeepAlive");
    match keep_alive {
        KeepAlive::Always => {
            plist.push_bool(true);
        }
        KeepAlive::Disabled => {
            plist.push_bool(false);
        }
        KeepAlive::SuccessfulExit => {
            plist.open_dict();
            plist.push_key("SuccessfulExit");
            plist.push_bool(true);
            plist.close_dict();
        }
        KeepAlive::Crashed => {
            plist.open_dict();
            plist.push_key("Crashed");
            plist.push_bool(true);
            plist.close_dict();
        }
        KeepAlive::NetworkState(enabled) => {
            plist.open_dict();
            plist.push_key("NetworkState");
            plist.push_bool(*enabled);
            plist.close_dict();
        }
        KeepAlive::PathExists(path) | KeepAlive::PathNotExists(path) => {
            plist.open_dict();
            plist.push_key("PathState");
            plist.open_dict();
            plist.push_key(path.to_str().expect("incorrect path"));
            plist.push_bool(matches!(keep_alive, KeepAlive::PathExists(_)));
            plist.close_dict();
            plist.close_dict();
        }
    }
}

fn maybe_render_process_type(
    plist: &mut PlistBuilder,
    process_type: Option<&ProcessType>,
) {
    let Some(process_type) = process_type else {
        return;
    };
    plist.push_key("ProcessType");
    match process_type {
        ProcessType::Standard => {
            plist.push_string("Standard");
        }
        ProcessType::Interactive => {
            plist.push_string("Interactive");
        }
        ProcessType::Background => {
            plist.push_string("Background");
        }
        ProcessType::Adaptive => {
            plist.push_string("Adaptive");
        }
    }
}

fn maybe_render_path(plist: &mut PlistBuilder, key: &str, path: Option<&PathBuf>) {
    if let Some(path) = path {
        plist.push_key(key);
        plist.push_string(&path.display().to_string());
    }
}

struct PlistBuilder {
    xml: String,
    depth: usize,
}

impl PlistBuilder {
    fn new() -> Self {
        let mut xml = String::with_capacity(512);
        xml.push_str(concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n",
            "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\"\n",
            "    \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n",
            "<plist version=\"1.0\">\n"
        ));

        Self { xml, depth: 0 }
    }

    fn open_dict(&mut self) {
        self.push_line("<dict>");
        self.depth += 1;
    }

    fn close_dict(&mut self) {
        self.depth -= 1;
        self.push_line("</dict>");
    }

    fn open_array(&mut self) {
        self.push_line("<array>");
        self.depth += 1;
    }

    fn close_array(&mut self) {
        self.depth -= 1;
        self.push_line("</array>");
    }

    fn push_key(&mut self, key: &str) {
        self.push_tagged_value("key", key);
    }

    fn push_string(&mut self, value: &str) {
        self.push_tagged_value("string", value);
    }

    fn push_bool(&mut self, value: bool) {
        if value {
            self.push_line("<true/>");
        } else {
            self.push_line("<false/>");
        }
    }

    fn finish(mut self) -> String {
        self.xml.push_str("</plist>\n");
        self.xml
    }

    fn push_tagged_value(&mut self, tag: &str, value: &str) {
        self.push_indent();
        self.xml.push('<');
        self.xml.push_str(tag);
        self.xml.push('>');
        escape_xml_into(value, &mut self.xml);
        self.xml.push_str("</");
        self.xml.push_str(tag);
        self.xml.push_str(">\n");
    }

    fn push_line(&mut self, line: &str) {
        self.push_indent();
        self.xml.push_str(line);
        self.xml.push('\n');
    }

    fn push_indent(&mut self) {
        for _ in 0..self.depth {
            self.xml.push_str("  ");
        }
    }
}

fn escape_xml_into(value: &str, output: &mut String) {
    for ch in value.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{KeepAlive, ProcessType, plist::render};

    use super::LaunchAgent;

    const LABEL: &str = "co.myrt.nanomiddleclick";

    #[test]
    fn render_uses_expected_label() {
        let agent = LaunchAgent::new(LABEL);
        let plist = render(&agent);

        assert!(plist.contains("<string>co.myrt.nanomiddleclick</string>"));
    }

    #[test]
    fn render_uses_expected_paths() {
        let agent = LaunchAgent::builder(LABEL)
            .program_arguments(vec![])
            .working_directory("/tmp/")
            .stdin_path("/tmp/stdin.log")
            .stdout_path("/tmp/stdout.log")
            .stderr_path("/tmp/stderr.log")
            .build()
            .unwrap();
        let plist = render(&agent);

        assert!(plist.contains("<key>WorkingDirectory</key>"));
        assert!(plist.contains("<string>/tmp/</string>"));
        assert!(plist.contains("<key>StandardInPath</key>"));
        assert!(plist.contains("<string>/tmp/stdin.log</string>"));
        assert!(plist.contains("<key>StandardOutPath</key>"));
        assert!(plist.contains("<string>/tmp/stdout.log</string>"));
        assert!(plist.contains("<key>StandardErrorPath</key>"));
        assert!(plist.contains("<string>/tmp/stderr.log</string>"));
    }

    #[test]
    fn render_escapes_xml_sensitive_characters() {
        let agent = LaunchAgent::builder(LABEL)
            .arg("/tmp/co<myrt>.lunch&ctl")
            .stdout_path("/tmp/stdout\".log")
            .stderr_path("/tmp/stderr'.log")
            .build()
            .unwrap();
        let plist = render(&agent);

        assert!(plist.contains("/tmp/co&lt;myrt&gt;.lunch&amp;ctl"));
        assert!(plist.contains("/tmp/stdout&quot;.log"));
        assert!(plist.contains("/tmp/stderr&apos;.log"));
    }

    #[test]
    fn render_uses_expected_process_type_and_keep_alive() {
        let agent = LaunchAgent::builder(LABEL)
            .arg("/tmp/co.myrt.lunchctl")
            .process_type(ProcessType::Interactive)
            .keep_alive(KeepAlive::SuccessfulExit)
            .build()
            .unwrap();
        let plist = render(&agent);

        assert!(plist.contains("<key>ProcessType</key>"));
        assert!(plist.contains("<string>Interactive</string>"));

        assert!(plist.contains("<key>KeepAlive</key>"));
        assert!(plist.contains("SuccessfulExit"));
    }
}
