//! Behaviour that only appears over the network, so the in-process mock cannot reach it: what
//! minima actually puts on the wire, and what it does when a gateway serves no model list.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

/// Kills the fake endpoint however the test ends, including on a failed assertion.
struct Fixture {
    child: Child,
    port: u16,
    dir: tempdir::Dir,
}

impl Fixture {
    /// Text only: the tests that use this are about the request minima sends, not about tools.
    fn start(mode: &str) -> Self {
        Self::spawn(mode, "chat", true)
    }

    fn start_with(mode: &str, dialect: &str) -> Self {
        Self::spawn(mode, dialect, false)
    }

    fn spawn(mode: &str, dialect: &str, reply_only: bool) -> Self {
        let dir = tempdir::Dir::new();
        let mut child = Command::new("python3")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/fake_provider.py"
            ))
            .args(["--mode", mode, "--dialect", dialect])
            .args(if reply_only {
                &["--reply-only"][..]
            } else {
                &[][..]
            })
            .arg("--capture")
            .arg(dir.path().join("request.json"))
            .arg("--gets")
            .arg(dir.path().join("gets.txt"))
            .stdout(Stdio::piped())
            .spawn()
            .expect("python3 is required for the live-path tests");

        // The fixture prints its ephemeral port once listening, so there is nothing to poll.
        let stdout = child.stdout.take().expect("fixture stdout");
        let mut first = String::new();
        BufReader::new(stdout)
            .read_line(&mut first)
            .expect("fixture never reported a port");
        let port = first
            .trim()
            .strip_prefix("PORT ")
            .and_then(|p| p.parse().ok())
            .unwrap_or_else(|| panic!("unexpected fixture greeting: {first:?}"));

        Self { child, port, dir }
    }

    fn run(&self, extra_env: &[(&str, &str)], prompt: &str) -> std::process::Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_minima"));
        // Tools the fixture streams run here, never in the repo.
        cmd.current_dir(self.dir.path())
            .env("XDG_CONFIG_HOME", self.dir.path())
            .env(
                "MINIMA_BASE_URL",
                format!("http://127.0.0.1:{}/v1", self.port),
            )
            .env("MINIMA_API_KEY", "test")
            .env_remove("MINIMA_MODEL")
            .env_remove("MINIMA_CONTEXT")
            // A bare key does not say which provider it belongs to, so these name one.
            .args(["--provider", "openrouter", "-p", prompt]);
        for (k, v) in extra_env {
            cmd.env(k, v);
        }
        cmd.output().expect("running minima")
    }

    fn run_as(&self, provider: &str, prompt: &str) -> std::process::Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_minima"));
        cmd.current_dir(self.dir.path())
            .env("XDG_CONFIG_HOME", self.dir.path())
            .env(
                "MINIMA_BASE_URL",
                format!("http://127.0.0.1:{}/v1", self.port),
            )
            .env("MINIMA_API_KEY", "test")
            .env_remove("MINIMA_PROVIDER")
            .env_remove("MINIMA_CONTEXT")
            .args([
                "--provider",
                provider,
                "--model",
                "m",
                "--max-turns",
                "1",
                "-p",
                prompt,
            ]);
        cmd.output().expect("running minima")
    }

    fn captured_request(&self) -> serde_json::Value {
        let raw = std::fs::read(self.dir.path().join("request.json")).expect("no request captured");
        serde_json::from_slice(&raw).expect("captured request was not JSON")
    }

    fn get_paths(&self) -> Vec<String> {
        std::fs::read_to_string(self.dir.path().join("gets.txt"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn request_carries_the_cache_key_and_every_tool() {
    let fixture = Fixture::start("full");
    let out = fixture.run(&[], "hello");
    assert!(
        out.status.success(),
        "minima failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("live path works"));

    let body = fixture.captured_request();
    let key = body["prompt_cache_key"]
        .as_str()
        .expect("prompt_cache_key missing");
    assert!(key.starts_with("minima-"), "unexpected cache key: {key}");

    // A mock cannot catch a tool that is never advertised; only the real body can.
    let advertised: Vec<&str> = body["tools"]
        .as_array()
        .expect("no tools advertised")
        .iter()
        .map(|t| t["function"]["name"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(advertised, ["read", "write", "edit", "bash"]);

    // The model and its context window come from the cached /models listing.
    assert_eq!(body["model"], "fake-model");
}

#[test]
fn instructions_and_skills_reach_the_system_prompt_user_file_first() {
    let fixture = Fixture::start("full");
    let dir = fixture.dir.path();
    let skill = dir.join("minima/skills/audit");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(dir.join("minima/AGENTS.md"), "Prefer stdlib.\n").unwrap();
    std::fs::write(
        skill.join("SKILL.md"),
        "---\ndescription: Check deps.\n---\n",
    )
    .unwrap();
    std::fs::write(dir.join("AGENTS.md"), "Run make check.\n").unwrap();
    let out = fixture.run(&[], "hello");
    assert!(
        out.status.success(),
        "minima failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let system = &fixture.captured_request()["messages"][0];
    assert_eq!(system["role"], "system");
    let content = system["content"].as_str().expect("system content");
    let user = dir.join("minima/AGENTS.md");
    let tail = content
        .split_once(&format!("\n# {}\n\nPrefer stdlib.\n", user.display()))
        .unwrap_or_else(|| panic!("no user instructions: {content}"))
        .1;
    let skill_section = format!(
        "\n## {}\n\ndescription: Check deps.\n",
        skill.join("SKILL.md").display()
    );
    assert!(
        tail.starts_with("\n# AGENTS.md\n\nRun make check.\n\n# Skills\n\n"),
        "{content}"
    );
    assert!(tail.ends_with(&skill_section), "{content}");
}

#[test]
fn a_gateway_without_a_model_list_still_runs_when_the_model_is_named() {
    let fixture = Fixture::start("no-models");
    let out = fixture.run(&[("MINIMA_MODEL", "m"), ("MINIMA_CONTEXT", "8000")], "hi");

    assert!(
        out.status.success(),
        "minima failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("live path works"));
    // Nothing was missing, so nothing justified the round-trip.
    assert!(
        fixture.get_paths().is_empty(),
        "unexpected GETs: {:?}",
        fixture.get_paths()
    );
}

#[test]
fn a_gateway_without_a_model_list_fails_clearly_when_the_model_is_not_named() {
    let fixture = Fixture::start("no-models");
    let out = fixture.run(&[], "hi");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("/v1/models"), "unhelpful failure: {stderr}");
}

/// Anthropic's list is paged and authenticated like its completions. A bearer token was refused by
/// any gateway that checks `x-api-key`, and only the first page was ever read.
#[test]
fn the_messages_model_list_uses_its_own_auth_and_follows_the_cursor() {
    let fixture = Fixture::start_with("full", "messages");
    let out = Command::new(env!("CARGO_BIN_EXE_minima"))
        .current_dir(fixture.dir.path())
        .env("XDG_CONFIG_HOME", fixture.dir.path())
        .env(
            "MINIMA_BASE_URL",
            format!("http://127.0.0.1:{}/v1", fixture.port),
        )
        .env("MINIMA_API_KEY", "test")
        .env_remove("MINIMA_CONTEXT")
        // A failed list is fatal only when a refresh is asked for.
        .args(["--provider", "anthropic", "--model", "fake-model"])
        .args(["--refresh-models", "--max-turns", "1", "-p", "hi"])
        .output()
        .expect("running minima");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("live path works"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert_eq!(
        fixture.get_paths(),
        [
            "/v1/models?limit=1000",
            "/v1/models?limit=1000&after_id=fake-model"
        ]
    );
    let cache = std::fs::read_dir(fixture.dir.path().join("minima"))
        .expect("config directory")
        .map(|e| e.expect("entry").path())
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("models-"))
        })
        .expect("a model cache");
    let cache: serde_json::Value =
        serde_json::from_slice(&std::fs::read(cache).expect("reading the cache")).expect("JSON");
    assert_eq!(
        cache["entries"],
        serde_json::json!([
            {"id": "fake-model", "context_length": 32000},
            {"id": "other-model"},
        ])
    );
}

/// A scratch directory that removes itself, so the fixtures never collide or leak.
mod tempdir {
    use std::path::{Path, PathBuf};

    pub struct Dir(PathBuf);

    impl Dir {
        pub fn new() -> Self {
            let unique = format!(
                "minima-test-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            );
            let path = std::env::temp_dir().join(unique.replace(['(', ')', ' '], ""));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("creating the scratch directory");
            Self(path)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

/// Each dialect puts the same conversation on the wire in a different shape, and each fragments
/// tool arguments differently. Only the built binary against a real socket covers both at once.
#[test]
fn every_dialect_sends_its_own_shape_and_reassembles_fragments() {
    struct Case {
        dialect: &'static str,
        provider: &'static str,
        present: &'static [&'static str],
        absent: &'static [&'static str],
        tool_schema_key: &'static str,
    }

    const CASES: &[Case] = &[
        Case {
            dialect: "chat",
            provider: "openrouter",
            present: &["messages", "tools"],
            absent: &["input", "instructions", "system", "max_tokens"],
            tool_schema_key: "/function/parameters",
        },
        Case {
            dialect: "responses",
            provider: "openai",
            present: &["input", "instructions", "store"],
            absent: &["messages", "system", "max_tokens"],
            tool_schema_key: "/parameters",
        },
        Case {
            dialect: "messages",
            provider: "anthropic",
            present: &["messages", "system", "max_tokens"],
            absent: &["input", "instructions", "store"],
            tool_schema_key: "/input_schema",
        },
    ];

    for case in CASES {
        let fixture = Fixture::start_with("full", case.dialect);
        let out = fixture.run_as(case.provider, "hi");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("live path works"),
            "{}: no reply, stderr {}",
            case.dialect,
            String::from_utf8_lossy(&out.stderr)
        );

        // The fixture splits the arguments across two frames. The call is described by its path
        // only if the joined arguments parse, so the dialect's grouping key survived reassembly.
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.lines().any(|line| line == "[tool] read notes.txt"),
            "{}: fragments did not reassemble, stderr {stderr}",
            case.dialect
        );

        let body = fixture.captured_request();
        for key in case.present {
            assert!(body.get(key).is_some(), "{} lacks {key}", case.dialect);
        }
        for key in case.absent {
            assert!(body.get(key).is_none(), "{} leaked {key}", case.dialect);
        }
        let schema = body["tools"][0].pointer(case.tool_schema_key);
        assert_eq!(
            schema.and_then(|s| s["type"].as_str()),
            Some("object"),
            "{}: tool schema missing at {}",
            case.dialect,
            case.tool_schema_key
        );
    }
}

/// With no --provider, the first key variable that is set decides. Only the built binary shows
/// that the registry order and the environment actually meet.
#[test]
fn an_unset_provider_is_chosen_from_the_environment() {
    let fixture = Fixture::start("full");
    let endpoint = format!("http://127.0.0.1:{}/v1", fixture.port);

    let run_with = |var: &str| {
        Command::new(env!("CARGO_BIN_EXE_minima"))
            .current_dir(fixture.dir.path())
            .env("XDG_CONFIG_HOME", fixture.dir.path())
            .env("MINIMA_BASE_URL", &endpoint)
            .env_remove("MINIMA_PROVIDER")
            .env_remove("MINIMA_API_KEY")
            .env_remove("MINIMA_MODEL")
            .env_remove("MINIMA_CONTEXT")
            .env_remove("OPENROUTER_API_KEY")
            .env_remove("ANTHROPIC_API_KEY")
            .env_remove("OPENAI_API_KEY")
            .env(var, "test-key")
            .args(["--model", "m", "--max-turns", "1", "-p", "hi"])
            .output()
            .expect("running minima")
    };

    // The captured request shows which dialect, and so which entry, each key selected.
    let chosen = run_with("OPENROUTER_API_KEY");
    assert!(
        chosen.status.success(),
        "openrouter should have been selected: {}",
        String::from_utf8_lossy(&chosen.stderr)
    );

    assert!(fixture.captured_request().get("messages").is_some());
    assert!(fixture.captured_request().get("max_tokens").is_none());

    let other = run_with("ANTHROPIC_API_KEY");
    let stderr = String::from_utf8_lossy(&other.stderr);
    assert!(
        !stderr.contains("no provider key found"),
        "a set key should still select a provider: {stderr}"
    );
    assert!(
        fixture.captured_request().get("max_tokens").is_some(),
        "ANTHROPIC_API_KEY did not select the messages dialect"
    );
}

#[test]
fn no_key_anywhere_names_every_variable_it_looked_at() {
    let dir = tempdir::Dir::new();
    let out = Command::new(env!("CARGO_BIN_EXE_minima"))
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", dir.path())
        .env_remove("MINIMA_PROVIDER")
        .env_remove("MINIMA_API_KEY")
        .env_remove("OPENROUTER_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .args(["--model", "m", "-p", "hi"])
        .output()
        .expect("running minima");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    for var in ["OPENROUTER_API_KEY", "ANTHROPIC_API_KEY", "OPENAI_API_KEY"] {
        assert!(stderr.contains(var), "{var} missing from: {stderr}");
    }
}

/// Everything under the config directory is owner-only. The model cache is public data, but it
/// sits beside prompt history and, later, saved sessions; a directory with one lax file in it
/// invites the next one.
#[cfg(unix)]
#[test]
fn everything_written_to_the_config_directory_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::start("full");
    let out = fixture.run(&[], "hi");
    assert!(
        out.status.success(),
        "minima failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let dir = fixture.dir.path().join("minima");
    let mode = |p: &std::path::Path| {
        std::fs::metadata(p)
            .unwrap_or_else(|e| panic!("{}: {e}", p.display()))
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(mode(&dir), 0o700, "config directory");

    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("reading the config directory") {
        let path = entry.expect("a directory entry").path();
        assert_eq!(mode(&path), 0o600, "{}", path.display());
        checked += 1;
    }
    assert!(checked > 0, "the run wrote nothing to check");
}
