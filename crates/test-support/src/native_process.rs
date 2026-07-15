use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::{FileBarrier, LifecycleDialect, TempRoot, VersionResponse};

const FAKE_NATIVE_EXECUTABLE: &str = env!("SKILLTAP_FAKE_NATIVE_EXECUTABLE");
const ESCAPED_PIPE_HOLDER: &str = env!("SKILLTAP_ESCAPED_PIPE_HOLDER");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PipeHolder {
    Child,
    Descendant,
    EscapedDescendant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FakeLifecycleAction {
    MarketplaceUpdate,
    PluginUpdate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FakeNativeMode {
    Exit(u8),
    VersionKnown,
    VersionUnknown,
    ProbeNarrow,
    ProbeDrift,
    MalformedJson,
    DuplicateJson,
    ExtraJsonDocument,
    Hang,
    Flood {
        stdout_bytes: u64,
        stderr_bytes: u64,
    },
    RetainPipes(PipeHolder),
}

/// Builds one neutral fake-native executable with isolated byte captures.
#[derive(Debug)]
pub struct FakeNativeBuilder {
    mode: FakeNativeMode,
    environment: Vec<String>,
    start_barrier: bool,
    version_response: Option<VersionResponse>,
    lifecycle_dialect: LifecycleDialect,
    executable_name: &'static str,
}

impl FakeNativeBuilder {
    pub fn new(mode: FakeNativeMode) -> Self {
        Self {
            mode,
            environment: Vec::new(),
            start_barrier: false,
            version_response: None,
            lifecycle_dialect: LifecycleDialect::None,
            executable_name: "fake-native",
        }
    }

    pub fn capture_environment(
        mut self,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> io::Result<Self> {
        for name in names {
            let name = name.into();
            validate_environment_name(&name)?;
            if !self.environment.contains(&name) {
                self.environment.push(name);
            }
        }
        self.environment.sort();
        Ok(self)
    }

    pub fn wait_for_release(mut self) -> Self {
        self.start_barrier = true;
        self
    }

    pub fn build(self) -> io::Result<FakeNativeProcess> {
        let stable_executable = Path::new(FAKE_NATIVE_EXECUTABLE);
        self.build_in(
            stable_executable
                .parent()
                .expect("build-time fixture has a parent"),
        )
    }

    pub(crate) fn for_harness(
        mut self,
        version_response: VersionResponse,
        lifecycle_dialect: LifecycleDialect,
        executable_name: &'static str,
    ) -> Self {
        self.version_response = Some(version_response);
        self.lifecycle_dialect = lifecycle_dialect;
        self.executable_name = executable_name;
        self
    }

    pub(crate) fn build_in(self, parent: &Path) -> io::Result<FakeNativeProcess> {
        let stable_executable = Path::new(FAKE_NATIVE_EXECUTABLE);
        let root = TempRoot::new_in(parent, "skilltap-fake-native")?;
        let captures = root.join("captures");
        fs::create_dir_all(captures.join("argv"))?;
        fs::create_dir_all(captures.join("invocations"))?;
        fs::create_dir_all(captures.join("environment"))?;
        fs::create_dir_all(captures.join("lifecycle"))?;
        for name in &self.environment {
            fs::create_dir(captures.join("environment").join(name))?;
        }

        let start_barrier = self
            .start_barrier
            .then(|| FileBarrier::new(&root.join("barriers"), "start"))
            .transpose()?;
        let hang_barrier = matches!(self.mode, FakeNativeMode::Hang)
            .then(|| FileBarrier::new(&root.join("barriers"), "hang"))
            .transpose()?;
        let pipe_holder_barrier = matches!(self.mode, FakeNativeMode::RetainPipes(_))
            .then(|| FileBarrier::new(&root.join("barriers"), "pipe-holder"))
            .transpose()?;
        let escaped_helper = matches!(
            self.mode,
            FakeNativeMode::RetainPipes(PipeHolder::EscapedDescendant)
        )
        .then(|| {
            let path = root.join("escaped-pipe-holder");
            fs::hard_link(ESCAPED_PIPE_HOLDER, &path)?;
            Ok::<_, io::Error>(path)
        })
        .transpose()?;
        let escaped_helper_done = escaped_helper
            .as_ref()
            .map(|_| root.join("barriers/pipe-holder.done"));
        let escaped_helper_pid = escaped_helper
            .as_ref()
            .map(|_| root.join("barriers/pipe-holder.pid"));
        let executable = root.join(self.executable_name);
        let behavior = render_script(
            &captures,
            &self.environment,
            self.mode,
            self.version_response.as_ref(),
            self.lifecycle_dialect,
            start_barrier.as_ref(),
            hang_barrier.as_ref(),
            pipe_holder_barrier.as_ref(),
            escaped_helper.as_deref(),
            escaped_helper_done.as_deref(),
            escaped_helper_pid.as_deref(),
        );
        fs::write(root.join("behavior"), behavior)?;
        publish_executable(stable_executable, &executable)?;

        Ok(FakeNativeProcess {
            _root: root,
            executable,
            captures,
            environment: self.environment,
            mode: self.mode,
            start_barrier,
            hang_barrier,
            pipe_holder_barrier,
            escaped_helper_done,
            escaped_helper_pid,
        })
    }
}

/// A materialized fake executable. Debug intentionally omits paths and captured values.
pub struct FakeNativeProcess {
    _root: TempRoot,
    executable: PathBuf,
    captures: PathBuf,
    environment: Vec<String>,
    mode: FakeNativeMode,
    start_barrier: Option<FileBarrier>,
    hang_barrier: Option<FileBarrier>,
    pipe_holder_barrier: Option<FileBarrier>,
    escaped_helper_done: Option<PathBuf>,
    escaped_helper_pid: Option<PathBuf>,
}

impl FakeNativeProcess {
    pub fn new(mode: FakeNativeMode) -> io::Result<Self> {
        FakeNativeBuilder::new(mode).build()
    }

    pub fn command(&self) -> Command {
        Command::new(&self.executable)
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    /// Publishes a named executable alias with the behavior companion file.
    pub fn install_alias(&self, destination: &Path, name: &str) -> io::Result<PathBuf> {
        fs::create_dir_all(destination)?;
        let executable = destination.join(name);
        fs::copy(&self.executable, &executable)?;
        let mut permissions = fs::metadata(&executable)?.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o755);
        }
        fs::set_permissions(&executable, permissions)?;
        fs::copy(self._root.join("behavior"), destination.join("behavior"))?;
        Ok(executable)
    }

    pub fn start_barrier(&self) -> Option<&FileBarrier> {
        self.start_barrier.as_ref()
    }

    pub fn hang_barrier(&self) -> Option<&FileBarrier> {
        self.hang_barrier.as_ref()
    }

    pub fn pipe_holder_barrier(&self) -> Option<&FileBarrier> {
        self.pipe_holder_barrier.as_ref()
    }

    /// Waits until an escaped pipe-holder helper has observed release and exited.
    pub fn wait_for_escaped_helper_exit(&self, timeout: Duration) -> io::Result<()> {
        let (Some(done), Some(pid_path)) = (&self.escaped_helper_done, &self.escaped_helper_pid)
        else {
            return Ok(());
        };
        let deadline = Instant::now() + timeout;
        loop {
            match fs::symlink_metadata(done) {
                Ok(_) => {
                    let pid = fs::read_to_string(pid_path)
                        .map_err(|_| {
                            io::Error::new(io::ErrorKind::InvalidData, "missing escaped helper pid")
                        })?
                        .parse::<u32>()
                        .map_err(|_| {
                            io::Error::new(io::ErrorKind::InvalidData, "invalid escaped helper pid")
                        })?;
                    let status = Command::new("/bin/kill")
                        .args(["-0", &pid.to_string()])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()?;
                    if !status.success() {
                        return Ok(());
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "escaped pipe-holder helper did not exit",
                ));
            }
            thread::sleep(Duration::from_millis(2));
        }
    }

    pub fn captured_invocation(&self) -> io::Result<CapturedInvocation> {
        match self.captured_invocations() {
            Ok(mut invocations) => invocations
                .pop()
                .map(Ok)
                .unwrap_or_else(|| self.read_captured_invocation(&self.captures)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                self.read_captured_invocation(&self.captures)
            }
            Err(error) => Err(error),
        }
    }

    pub fn captured_invocations(&self) -> io::Result<Vec<CapturedInvocation>> {
        let mut paths = fs::read_dir(self.captures.join("invocations"))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        paths
            .into_iter()
            .map(|path| self.read_captured_invocation(&path))
            .collect()
    }

    pub fn fail_lifecycle(&self, action: FakeLifecycleAction, name: &str) -> io::Result<()> {
        self.write_lifecycle_control("fail", action, name, b"1")
    }

    pub fn set_plugin_revision(&self, selector: &str, revision: &str) -> io::Result<()> {
        validate_control_name(selector)?;
        fs::write(
            self.captures
                .join("lifecycle")
                .join(format!("plugin-revision-{selector}")),
            format!("{revision}\n"),
        )
    }

    pub fn set_available_plugin_revision(&self, selector: &str, revision: &str) -> io::Result<()> {
        validate_control_name(selector)?;
        fs::write(
            self.captures
                .join("lifecycle")
                .join(format!("plugin-available-revision-{selector}")),
            format!("{revision}\n"),
        )
    }

    /// Cause the next postcondition observation for this lifecycle action to
    /// be malformed, without poisoning the pre-observation that authorizes it.
    pub fn indeterminate_lifecycle(
        &self,
        action: FakeLifecycleAction,
        name: &str,
    ) -> io::Result<()> {
        self.write_lifecycle_control("indeterminate", action, name, b"1")
    }

    fn write_lifecycle_control(
        &self,
        prefix: &str,
        action: FakeLifecycleAction,
        name: &str,
        contents: &[u8],
    ) -> io::Result<()> {
        validate_control_name(name)?;
        fs::write(
            self.captures
                .join("lifecycle")
                .join(format!("{prefix}-{}-{name}", lifecycle_action_name(action))),
            contents,
        )
    }

    fn read_captured_invocation(&self, root: &Path) -> io::Result<CapturedInvocation> {
        let count = fs::read_to_string(root.join("argument-count"))?
            .trim()
            .parse::<usize>()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid argument count"))?;
        let mut arguments = Vec::with_capacity(count);
        for index in 0..count {
            arguments.push(fs::read(root.join("argv").join(format!("{index:020}")))?);
        }
        let mut environment = BTreeMap::new();
        for name in &self.environment {
            let directory = self.captures.join("environment").join(name);
            let present = fs::read(directory.join("present"))? == b"1";
            environment.insert(
                name.clone(),
                present
                    .then(|| fs::read(directory.join("value")))
                    .transpose()?,
            );
        }
        Ok(CapturedInvocation {
            arguments,
            environment,
            working_directory: fs::read(root.join("working-directory"))?,
        })
    }
}

impl std::fmt::Debug for FakeNativeProcess {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FakeNativeProcess")
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}

#[derive(Eq, PartialEq)]
pub struct CapturedInvocation {
    arguments: Vec<Vec<u8>>,
    environment: BTreeMap<String, Option<Vec<u8>>>,
    working_directory: Vec<u8>,
}

impl CapturedInvocation {
    pub fn arguments(&self) -> &[Vec<u8>] {
        &self.arguments
    }

    pub fn environment(&self) -> &BTreeMap<String, Option<Vec<u8>>> {
        &self.environment
    }

    pub fn working_directory(&self) -> &[u8] {
        &self.working_directory
    }
}

impl std::fmt::Debug for CapturedInvocation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedInvocation")
            .field("argument_count", &self.arguments.len())
            .field("environment_count", &self.environment.len())
            .field("working_directory_bytes", &self.working_directory.len())
            .finish()
    }
}

fn lifecycle_action_name(action: FakeLifecycleAction) -> &'static str {
    match action {
        FakeLifecycleAction::MarketplaceUpdate => "marketplace-update",
        FakeLifecycleAction::PluginUpdate => "plugin-update",
    }
}

fn validate_control_name(name: &str) -> io::Result<()> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "lifecycle control name is not a safe path component",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_script(
    captures: &Path,
    environment: &[String],
    mode: FakeNativeMode,
    version_response: Option<&VersionResponse>,
    lifecycle_dialect: LifecycleDialect,
    start_barrier: Option<&FileBarrier>,
    hang_barrier: Option<&FileBarrier>,
    pipe_holder_barrier: Option<&FileBarrier>,
    escaped_helper: Option<&Path>,
    escaped_helper_done: Option<&Path>,
    escaped_helper_pid: Option<&Path>,
) -> String {
    let mut script = String::from("umask 077\n");
    script.push_str(&format!(
        "capture={}\nprintf '%s' \"$PWD\" > \"$capture/working-directory\"\ninvocation_dir=\"$capture/invocations/$$\"\nmkdir -p \"$invocation_dir/argv\"\n",
        shell_quote(captures)
    ));
    script.push_str(
        "index=0\nfor argument do\n  name=$(printf '%020u' \"$index\")\n  printf '%s' \"$argument\" > \"$capture/argv/$name\"\n  printf '%s' \"$argument\" > \"$invocation_dir/argv/$name\"\n  index=$((index + 1))\ndone\nprintf '%s' \"$index\" > \"$capture/argument-count\"\nprintf '%s\\n' \"$index\" > \"$invocation_dir/argument-count\"\nprintf '%s' \"$PWD\" > \"$invocation_dir/working-directory\"\n",
    );
    for name in environment {
        let directory = captures.join("environment").join(name);
        script.push_str(&format!(
            "if [ \"${{{name}+x}}\" = x ]; then\n  printf 1 > {present}\n  printf '%s' \"${{{name}}}\" > {value}\nelse\n  printf 0 > {present}\nfi\n",
            present = shell_quote(&directory.join("present")),
            value = shell_quote(&directory.join("value")),
        ));
    }
    if let Some(barrier) = start_barrier {
        script.push_str(&barrier_script(barrier, ""));
    }
    // A profile's dialect owns lifecycle semantics independently of its
    // process behavior; the unprofiled version mode retains legacy coverage.
    if lifecycle_dialect != LifecycleDialect::None
        || (version_response.is_none() && matches!(mode, FakeNativeMode::VersionKnown))
    {
        if lifecycle_dialect == LifecycleDialect::Qwen {
            script.push_str(&format!(
                r#"lifecycle={lifecycle}
if [ "${{1-}} ${{2-}} ${{3-}}" = "extensions sources list" ]; then
  printf 'Configured extension sources:\n'
  for path in "$lifecycle"/qwen-source-*; do
    [ -f "$path" ] || continue
    name=${{path##*/}}
    name=${{name#qwen-source-}}
    printf '  %s\n    Source: local\n    Type: local\n' "$name"
  done
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "extensions sources add" ]; then
  : > "$lifecycle/qwen-source-team"
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "extensions sources remove" ]; then
  /bin/rm -f "$lifecycle/qwen-source-${{3-}}"
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "extensions sources update" ]; then exit 0; fi
if [ "${{1-}} ${{2-}}" = "extensions list" ]; then
  scope=${{4-}}
  found=0
  for path in "$lifecycle"/qwen-extension-*; do
    [ -f "$path" ] || continue
    name=${{path##*/}}
    name=${{name#qwen-extension-}}
    enabled=0
    [ -f "$lifecycle/qwen-enabled-${{scope}}-$name" ] && enabled=1
    [ "$enabled" = 1 ] || continue
    found=1
    printf 'Installed extensions:\n  %s\n    Version: 1.0.0\n    Source: local\n    Type: local\n    Enabled (User): %s\n    Enabled (Workspace): %s\n    Skills:\n      - %s\n' "$name" "$([ "$scope" = user ] && echo true || echo false)" "$([ "$scope" = workspace ] && echo true || echo false)" "$name"
  done
  [ "$found" = 1 ] || printf 'Installed extensions:\n  none\n'
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "extensions install" ] || [ "${{1-}} ${{2-}}" = "extensions uninstall" ]; then
  name=${{3-}}
  scope=${{5-}}
  safe=$(printf '%s' "$name" | tr / _)
  if [ "${{2-}}" = install ]; then
    : > "$lifecycle/qwen-extension-$safe"
    : > "$lifecycle/qwen-enabled-$scope-$safe"
  else
    /bin/rm -f "$lifecycle/qwen-extension-$safe" "$lifecycle/qwen-enabled-user-$safe" "$lifecycle/qwen-enabled-workspace-$safe"
  fi
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "extensions enable" ] || [ "${{1-}} ${{2-}}" = "extensions disable" ]; then
  name=${{3-}}
  scope=${{5-}}
  safe=$(printf '%s' "$name" | tr / _)
  if [ "${{2-}}" = enable ]; then : > "$lifecycle/qwen-enabled-$scope-$safe"; else /bin/rm -f "$lifecycle/qwen-enabled-$scope-$safe"; fi
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "extensions update" ]; then exit 0; fi
if [ "${{1-}} ${{2-}}" = "mcp list" ]; then
  printf 'No MCP servers configured.\n'
  exit 0
fi
"#,
                lifecycle = shell_quote(&captures.join("lifecycle")),
            ));
        }
        if lifecycle_dialect == LifecycleDialect::Factory {
            script.push_str(&format!(
                r#"lifecycle={lifecycle}
if [ "${{1-}} ${{2-}}" = "plugin list" ] && [ "${{3-}}" = "--scope" ]; then
  scope=${{4-}}
  if [ "$scope" != "user" ] && [ "$scope" != "project" ]; then exit 2; fi
  found=0
  for path in "$lifecycle"/plugin-${{scope}}-*; do
    [ -f "$path" ] || continue
    found=1
  done
  if [ "$found" = 0 ]; then
    printf 'No plugins installed in %s scope.\n' "$scope"
    exit 0
  fi
  printf 'Installed plugins:\nActive:\n'
  for path in "$lifecycle"/plugin-${{scope}}-*; do
    [ -f "$path" ] || continue
    name=${{path##*/}}
    name=${{name#plugin-${{scope}}-}}
    case "$name" in revision-*|available-revision-*|fail-*|indeterminate-*|pending-*) continue ;; esac
    revision=e8801fa
    if [ -f "$lifecycle/plugin-revision-${{scope}}-$name" ]; then read revision < "$lifecycle/plugin-revision-${{scope}}-$name"; fi
    printf '  %s  [%s]  %s\n' "$name" "$scope" "$revision"
  done
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "plugin install" ] || [ "${{1-}} ${{2-}}" = "plugin uninstall" ]; then
  scope=${{5-}}
  name=${{3-}}
  if [ "${{2-}}" = "install" ]; then : > "$lifecycle/plugin-${{scope}}-$name"; else /bin/rm -f "$lifecycle/plugin-${{scope}}-$name"; fi
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "plugin update" ]; then
  scope=${{5-}}
  name=${{3-}}
  if [ -f "$lifecycle/fail-plugin-update-$name" ]; then exit 23; fi
  if [ -f "$lifecycle/plugin-available-revision-$name" ]; then cp "$lifecycle/plugin-available-revision-$name" "$lifecycle/plugin-revision-${{scope}}-$name"; fi
  exit 0
fi
"#,
                lifecycle = shell_quote(&captures.join("lifecycle")),
            ));
        }
        script.push_str(&format!(
            r#"lifecycle={lifecycle}
if [ "${{1-}} ${{2-}} ${{3-}}" = "plugin marketplace list" ]; then
  if [ -f "$lifecycle/pending-indeterminate-marketplace-list" ]; then
    /bin/rm -f "$lifecycle/pending-indeterminate-marketplace-list"
    printf '%s' '{{malformed'
    exit 0
  fi
  first=1
  printf '%s' '{{"marketplaces":['
  for path in "$lifecycle"/marketplace-*; do
    [ -f "$path" ] || continue
    name=${{path##*/}}
    name=${{name#marketplace-}}
    case "$name" in fail-*|indeterminate-*|pending-*) continue ;; esac
    if [ "$first" = 0 ]; then printf ','; fi
    printf '{{"name":"%s","scope":"user"}},{{"name":"%s","scope":"local"}}' "$name" "$name"
    first=0
  done
  printf '%s' ']}}'
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "plugin list" ]; then
  if [ -f "$lifecycle/pending-indeterminate-plugin-list" ]; then
    /bin/rm -f "$lifecycle/pending-indeterminate-plugin-list"
    printf '%s' '{{malformed'
    exit 0
  fi
  first=1
  printf '%s' '{{"plugins":['
  for path in "$lifecycle"/plugin-*; do
    [ -f "$path" ] || continue
    name=${{path##*/}}
    name=${{name#plugin-}}
    case "$name" in revision-*|available-revision-*|fail-*|indeterminate-*|pending-*) continue ;; esac
    revision=
    if [ -f "$lifecycle/plugin-revision-$name" ]; then read revision < "$lifecycle/plugin-revision-$name"; fi
    if [ "$first" = 0 ]; then printf ','; fi
    if [ -n "$revision" ]; then
      printf '{{"name":"%s","scope":"user","version":"%s"}},{{"name":"%s","scope":"local","version":"%s"}}' "$name" "$revision" "$name" "$revision"
    else
      printf '{{"name":"%s","scope":"user"}},{{"name":"%s","scope":"local"}}' "$name" "$name"
    fi
    first=0
  done
  printf '%s' ']}}'
  exit 0
fi
if [ "${{1-}} ${{2-}} ${{3-}}" = "plugin marketplace add" ]; then
  : > "$lifecycle/marketplace-team"
  : > "$lifecycle/marketplace-other"
  exit 0
fi
if [ "${{1-}} ${{2-}} ${{3-}}" = "plugin marketplace remove" ]; then
  /bin/rm -f "$lifecycle/marketplace-${{4-}}"
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "plugin add" ] || [ "${{1-}} ${{2-}}" = "plugin install" ]; then
  : > "$lifecycle/plugin-${{3-}}"
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "plugin remove" ] || [ "${{1-}} ${{2-}}" = "plugin uninstall" ]; then
  /bin/rm -f "$lifecycle/plugin-${{3-}}"
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "plugin update" ]; then
  name=${{3-}}
  if [ -f "$lifecycle/fail-plugin-update-$name" ]; then exit 23; fi
  if [ -f "$lifecycle/plugin-available-revision-$name" ]; then cp "$lifecycle/plugin-available-revision-$name" "$lifecycle/plugin-revision-$name"; fi
  if [ -f "$lifecycle/indeterminate-plugin-update-$name" ]; then : > "$lifecycle/pending-indeterminate-plugin-list"; fi
  exit 0
fi
if [ "${{1-}} ${{2-}} ${{3-}}" = "plugin marketplace update" ] || [ "${{1-}} ${{2-}} ${{3-}}" = "plugin marketplace upgrade" ]; then
  name=${{4-}}
  if [ -f "$lifecycle/fail-marketplace-update-$name" ]; then exit 23; fi
  if [ -f "$lifecycle/indeterminate-marketplace-update-$name" ]; then : > "$lifecycle/pending-indeterminate-marketplace-list"; fi
  exit 0
fi
"#,
            lifecycle = shell_quote(&captures.join("lifecycle")),
        ));
    }
    match mode {
        FakeNativeMode::Exit(code) => script.push_str(&format!("exit {code}\n")),
        FakeNativeMode::VersionKnown => {
            if let Some(response) = version_response {
                script.push_str(&format!(
                    "printf '%s' {}\nexit 0\n",
                    shell_quote_value(&response.render())
                ));
            } else {
                script.push_str("printf '%s' '{\"version\":\"0.144.1\"}'\nexit 0\n");
            }
        }
        FakeNativeMode::VersionUnknown => {
            script.push_str("printf '%s' '{\"version\":\"99.0.0\"}'\nexit 0\n");
        }
        FakeNativeMode::ProbeNarrow => {
            script.push_str(
                "printf '%s' '{\"scope\":\"project\",\"capabilities\":{\"plugin.install\":\"unsupported\"}}'\nexit 0\n",
            );
        }
        FakeNativeMode::ProbeDrift => {
            script.push_str(
                "printf '%s' '{\"scope\":\"project\",\"capabilities\":{\"future.capability\":\"supported\"}}'\nexit 0\n",
            );
        }
        FakeNativeMode::MalformedJson => {
            script.push_str("printf '%s' '{malformed'\nexit 0\n");
        }
        FakeNativeMode::DuplicateJson => {
            script
                .push_str("printf '%s' '{\"version\":\"3.0.0\",\"version\":\"3.0.1\"}'\nexit 0\n");
        }
        FakeNativeMode::ExtraJsonDocument => {
            script
                .push_str("printf '%s' '{\"version\":\"3.0.0\"}{\"version\":\"3.0.1\"}'\nexit 0\n");
        }
        FakeNativeMode::Hang => {
            let barrier = hang_barrier.expect("hang mode has a readiness barrier");
            script.push_str(&format!(
                ": > {}\nexec /bin/sleep 3600\n",
                shell_quote(barrier.ready_path())
            ));
        }
        FakeNativeMode::Flood {
            stdout_bytes,
            stderr_bytes,
        } => {
            script.push_str(&format!(
                "x_chunk='xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'\ny_chunk='yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy'\nflood_stdout() {{ remaining={stdout_bytes}; while [ \"$remaining\" -ge 256 ]; do printf '%s' \"$x_chunk\"; remaining=$((remaining - 256)); done; while [ \"$remaining\" -gt 0 ]; do printf x; remaining=$((remaining - 1)); done; }}\nflood_stderr() {{ remaining={stderr_bytes}; while [ \"$remaining\" -ge 256 ]; do printf '%s' \"$y_chunk\" >&2; remaining=$((remaining - 256)); done; while [ \"$remaining\" -gt 0 ]; do printf y >&2; remaining=$((remaining - 1)); done; }}\nflood_stdout &\nflood_stderr &\nwait\n"
            ));
        }
        FakeNativeMode::RetainPipes(holder) => {
            let barrier = pipe_holder_barrier.expect("pipe-holder mode has a barrier");
            match holder {
                PipeHolder::Child => {
                    script.push_str("(\n");
                    script.push_str(&barrier_script(barrier, "  "));
                    script.push_str(") &\nexit 0\n");
                }
                PipeHolder::Descendant => {
                    script.push_str("(\n  (\n");
                    script.push_str(&barrier_script(barrier, "    "));
                    script.push_str("  ) &\n  wait\n) &\nexit 0\n");
                }
                PipeHolder::EscapedDescendant => {
                    script.push_str(&format!(
                        "{} {} {} {} {} &\nwait\nexit 0\n",
                        shell_quote(escaped_helper.expect("escaped mode has helper")),
                        shell_quote(barrier.ready_path()),
                        shell_quote(barrier.release_path()),
                        shell_quote(escaped_helper_done.expect("escaped mode has exit marker")),
                        shell_quote(escaped_helper_pid.expect("escaped mode has pid marker")),
                    ));
                }
            }
        }
    }
    if matches!(mode, FakeNativeMode::Flood { .. }) {
        normalize_flood_chunk(&mut script, "x_chunk", 'x');
        normalize_flood_chunk(&mut script, "y_chunk", 'y');
    }
    script
}

fn publish_executable(source: &Path, destination: &Path) -> io::Result<()> {
    match fs::hard_link(source, destination) {
        Ok(()) => Ok(()),
        // Caller-owned roots can be on a different filesystem than Cargo's
        // OUT_DIR. A symbolic link keeps the sealed build artifact as the
        // executed inode, avoiding a copy-then-exec writer race entirely.
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            std::os::unix::fs::symlink(source, destination)
        }
        Err(error) => Err(error),
    }
}

fn normalize_flood_chunk(script: &mut String, variable: &str, byte: char) {
    let marker = format!("{variable}='");
    let start = script.find(&marker).expect("flood chunk is rendered") + marker.len();
    let end = start
        + script[start..]
            .find('\'')
            .expect("flood chunk is single quoted");
    script.replace_range(start..end, &byte.to_string().repeat(256));
}

fn barrier_script(barrier: &FileBarrier, indent: &str) -> String {
    format!(
        "{indent}: > {ready}\n{indent}while [ ! -e {release} ]; do :; done\n",
        ready = shell_quote(barrier.ready_path()),
        release = shell_quote(barrier.release_path()),
    )
}

fn shell_quote(path: &Path) -> String {
    let value = path
        .to_str()
        .expect("fixture roots are constructed from UTF-8 names");
    shell_quote_value(value)
}

fn shell_quote_value(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn validate_environment_name(name: &str) -> io::Result<()> {
    let mut bytes = name.bytes();
    let valid = bytes
        .next()
        .is_some_and(|byte| byte == b'_' || byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric());
    if valid {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "environment capture name is not portable",
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Read, process::Stdio, thread, time::Duration};

    use super::*;

    #[test]
    fn capture_preserves_exact_arguments_environment_and_working_directory() {
        const SECRET: &str = "secret-native-capture-canary";
        let working = TempRoot::new("skilltap-native-cwd").unwrap();
        let native = FakeNativeBuilder::new(FakeNativeMode::Exit(0))
            .capture_environment(["CAPTURED", "ABSENT"])
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(
            native.executable().canonicalize().unwrap(),
            native.executable()
        );
        let status = native
            .command()
            .args(["", "two words", "line\nbreak"])
            .current_dir(&working)
            .env_clear()
            .env("CAPTURED", SECRET)
            .status()
            .unwrap();
        assert!(status.success());

        let capture = native.captured_invocation().unwrap();
        assert_eq!(
            capture.arguments(),
            [b"".as_slice(), b"two words", b"line\nbreak"]
        );
        assert_eq!(
            capture.environment().get("CAPTURED").unwrap().as_deref(),
            Some(SECRET.as_bytes())
        );
        assert_eq!(capture.environment().get("ABSENT"), Some(&None));
        assert_eq!(
            capture.working_directory(),
            working
                .canonicalize()
                .unwrap()
                .as_os_str()
                .as_encoded_bytes()
        );
        assert!(!format!("{capture:?}").contains(SECRET));
        assert!(!format!("{native:?}").contains(SECRET));
    }

    #[test]
    fn nonzero_and_both_stream_flood_modes_are_exact() {
        let nonzero = FakeNativeProcess::new(FakeNativeMode::Exit(23)).unwrap();
        assert_eq!(nonzero.command().status().unwrap().code(), Some(23));

        let flood = FakeNativeProcess::new(FakeNativeMode::Flood {
            stdout_bytes: 4097,
            stderr_bytes: 3073,
        })
        .unwrap();
        let output = flood.command().output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, vec![b'x'; 4097]);
        assert_eq!(output.stderr, vec![b'y'; 3073]);
    }

    #[test]
    fn detection_payload_modes_are_exact_and_deterministic() {
        for (mode, expected) in [
            (
                FakeNativeMode::VersionKnown,
                b"{\"version\":\"0.144.1\"}".as_slice(),
            ),
            (
                FakeNativeMode::VersionUnknown,
                b"{\"version\":\"99.0.0\"}".as_slice(),
            ),
            (
                FakeNativeMode::ProbeNarrow,
                b"{\"scope\":\"project\",\"capabilities\":{\"plugin.install\":\"unsupported\"}}"
                    .as_slice(),
            ),
            (
                FakeNativeMode::ProbeDrift,
                b"{\"scope\":\"project\",\"capabilities\":{\"future.capability\":\"supported\"}}"
                    .as_slice(),
            ),
            (FakeNativeMode::MalformedJson, b"{malformed".as_slice()),
            (
                FakeNativeMode::DuplicateJson,
                b"{\"version\":\"3.0.0\",\"version\":\"3.0.1\"}".as_slice(),
            ),
            (
                FakeNativeMode::ExtraJsonDocument,
                b"{\"version\":\"3.0.0\"}{\"version\":\"3.0.1\"}".as_slice(),
            ),
        ] {
            let native = FakeNativeProcess::new(mode).unwrap();
            let output = native.command().output().unwrap();
            assert!(output.status.success());
            assert_eq!(output.stdout, expected);
            assert!(output.stderr.is_empty());
            if matches!(mode, FakeNativeMode::VersionKnown) {
                let output = native.command().arg("--version").output().unwrap();
                assert!(
                    output.status.success(),
                    "version stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(output.stdout, expected);
                assert!(output.stderr.is_empty());
            }
        }
    }

    #[test]
    fn start_and_pipe_holder_barriers_are_deterministic() {
        let gated = FakeNativeBuilder::new(FakeNativeMode::Exit(0))
            .wait_for_release()
            .build()
            .unwrap();
        let mut child = gated.command().spawn().unwrap();
        let barrier = gated.start_barrier().unwrap();
        barrier.wait_until_ready(Duration::from_secs(1)).unwrap();
        assert!(child.try_wait().unwrap().is_none());
        barrier.release().unwrap();
        assert!(child.wait().unwrap().success());

        for holder in [
            PipeHolder::Child,
            PipeHolder::Descendant,
            PipeHolder::EscapedDescendant,
        ] {
            let native = FakeNativeProcess::new(FakeNativeMode::RetainPipes(holder)).unwrap();
            let mut child = native
                .command()
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let barrier = native.pipe_holder_barrier().unwrap();
            barrier.wait_until_ready(Duration::from_secs(1)).unwrap();
            assert!(child.wait().unwrap().success());
            barrier.release().unwrap();
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            child
                .stdout
                .take()
                .unwrap()
                .read_to_end(&mut stdout)
                .unwrap();
            child
                .stderr
                .take()
                .unwrap()
                .read_to_end(&mut stderr)
                .unwrap();
            assert!(stdout.is_empty());
            assert!(stderr.is_empty());
        }
    }

    #[test]
    fn invalid_environment_names_fail_before_materialization() {
        assert_eq!(
            FakeNativeBuilder::new(FakeNativeMode::Exit(0))
                .capture_environment(["NOT-PORTABLE"])
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn hang_stays_alive_until_deterministically_killed_and_reaped() {
        let native = FakeNativeProcess::new(FakeNativeMode::Hang).unwrap();
        let mut child = native.command().spawn().unwrap();
        let process_id = child.id();
        native
            .hang_barrier()
            .unwrap()
            .wait_until_ready(Duration::from_secs(1))
            .unwrap();
        assert!(child.try_wait().unwrap().is_none());

        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        let probe = Command::new("/bin/kill")
            .args(["-0", &process_id.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(!probe.success(), "reaped fixture process must not remain");
    }

    #[test]
    fn executable_publication_survives_parallel_fixture_churn() {
        thread::scope(|scope| {
            let workers = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        for _ in 0..32 {
                            let native = FakeNativeProcess::new(FakeNativeMode::Exit(0)).unwrap();
                            assert!(native.command().status().unwrap().success());
                        }
                    })
                })
                .collect::<Vec<_>>();
            for worker in workers {
                worker.join().unwrap();
            }
        });
    }
}
