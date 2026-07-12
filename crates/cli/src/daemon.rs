use skilltap_core::{
    daemon::{ServiceDefinition, ServiceFile, ServicePlatform},
    domain::AbsolutePath,
    runtime::{PlatformPaths, SupportedPlatform},
};

pub fn platform(paths: &PlatformPaths) -> ServicePlatform {
    match paths.platform() {
        SupportedPlatform::MacOs => ServicePlatform::Launchd,
        SupportedPlatform::Linux => ServicePlatform::SystemdUser,
    }
}

pub fn root(paths: &PlatformPaths, platform: ServicePlatform) -> AbsolutePath {
    match platform {
        ServicePlatform::Launchd => {
            AbsolutePath::new(format!("{}/Library/LaunchAgents", paths.home().as_str()))
                .expect("launchd service root is normalized")
        }
        ServicePlatform::SystemdUser => {
            AbsolutePath::new(format!("{}/systemd/user", paths.config_home().as_str()))
                .expect("systemd user service root is normalized")
        }
    }
}

pub fn files<'a>(
    paths: &PlatformPaths,
    definition: &'a ServiceDefinition,
) -> Vec<(AbsolutePath, &'a ServiceFile)> {
    let root = root(paths, definition.platform());
    definition
        .files()
        .iter()
        .map(|file| {
            (
                AbsolutePath::new(format!("{}/{}", root.as_str(), file.name()))
                    .expect("service file path is normalized"),
                file,
            )
        })
        .collect()
}

pub fn owns(platform: ServicePlatform, contents: &[u8]) -> bool {
    let Ok(contents) = std::str::from_utf8(contents) else {
        return false;
    };
    match platform {
        ServicePlatform::Launchd => {
            contents.contains("<key>SkilltapManaged</key><string>skilltap-managed-v3</string>")
                && contents.contains("<key>Label</key><string>com.skilltap.daemon</string>")
                && contents.contains("<key>ProgramArguments</key>")
                && contents.contains("<string>daemon</string><string>run</string>")
        }
        ServicePlatform::SystemdUser => {
            if !contents.contains("# skilltap-managed-v3") {
                return false;
            }
            (contents.contains("Description=skilltap safe update cycle")
                && contents.contains("[Service]")
                && contents.contains("Type=oneshot")
                && contents.lines().any(|line| {
                    line.strip_prefix("ExecStart=")
                        .is_some_and(|value| value.ends_with(" daemon run"))
                }))
                || (contents.contains("Description=skilltap safe update timer")
                    && contents.contains("[Timer]")
                    && contents.contains("Unit=skilltap-update.service"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::daemon::ServicePlatform;

    #[test]
    fn systemd_ownership_accepts_skilltap_service_and_timer_files() {
        let service = b"# skilltap-managed-v3\n[Unit]\nDescription=skilltap safe update cycle\n[Service]\nType=oneshot\nExecStart=/bin/skilltap daemon run\n";
        let timer = b"# skilltap-managed-v3\n[Unit]\nDescription=skilltap safe update timer\n[Timer]\nUnit=skilltap-update.service\n";
        let unrelated =
            b"# skilltap-managed-v3\n[Unit]\nDescription=skilltap safe update timer\n[Timer]\nUnit=other.service\n";
        let lookalike = b"[Unit]\nDescription=skilltap safe update cycle\n[Service]\nExecStart=/tmp/evil daemon run\n";

        assert!(owns(ServicePlatform::SystemdUser, service));
        assert!(owns(ServicePlatform::SystemdUser, timer));
        assert!(!owns(ServicePlatform::SystemdUser, unrelated));
        assert!(!owns(ServicePlatform::SystemdUser, lookalike));
    }
}
