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
            contents.contains("com.skilltap.daemon")
                && contents.contains("<string>daemon</string><string>run</string>")
        }
        ServicePlatform::SystemdUser => {
            contents.contains("skilltap safe update") && contents.contains("daemon run")
        }
    }
}
