//! The system prompt: the machine, then instruction files, then the skills a model can load.

use std::path::{Path, PathBuf};

const SYSTEM: &str = "You are minima, a coding agent. Use the tools to inspect and change files. \
Be terse. State what you did; do not narrate what you are about to do.";
/// Instructions for coding agents, by the cross-tool convention (https://agents.md).
const AGENTS_MD: &str = "AGENTS.md";
/// A skill's entry point, by the Agent Skills convention (https://agentskills.io/specification).
const SKILL_MD: &str = "SKILL.md";
const SKILLS: &str = "A skill is a SKILL.md file of instructions for one kind of task. Each \
section below is a skill's path and YAML frontmatter. When a task matches a skill's description, \
read its SKILL.md with the read tool before any other action, and follow it. Relative paths in it \
are relative to its directory.";
/// The spec's bounded fields total about 1.6 KB; the rest is room for `license` and `metadata`.
const MAX_FRONTMATTER: usize = 4096;

/// Facts about the machine, so the model does not have to guess at it. Without this it reaches
/// for GNU flags on a BSD userland and burns a turn discovering the mistake.
///
/// Three fields, not hax's six: the working directory, the platform, and the shell. Home
/// directory and model name are not worth their tokens, and a git root needs a walk up the tree.
///
/// The user's AGENTS.md precedes the working directory's, so project instructions come last.
/// Skills are listed by frontmatter only; the model reads a SKILL.md when a task needs it.
pub fn system_prompt() -> String {
    let mut prompt = String::from(SYSTEM);
    prompt.push_str("\n\n# Environment\n\n");

    if let Ok(cwd) = std::env::current_dir() {
        prompt.push_str(&format!("- Working directory: {}\n", cwd.display()));
    }
    prompt.push_str(&format!(
        "- Operating system: {} ({})\n",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    // The shell the bash tool runs, not the user's $SHELL: the model writes syntax for this one.
    prompt.push_str("- Command shell: bash\n");

    let user = crate::config::config_dir();
    let files = user.iter().map(|d| d.join(AGENTS_MD));
    for path in files.chain([PathBuf::from(AGENTS_MD)]) {
        if let Some(text) = read(&path) {
            prompt.push_str(&format!("\n# {}\n\n{text}\n", path.display()));
        }
    }

    let skills = user.map(|d| skills(&d.join("skills"))).unwrap_or_default();
    if !skills.is_empty() {
        prompt.push_str(&format!("\n# Skills\n\n{SKILLS}\n"));
        for skill in skills {
            prompt.push_str(&format!(
                "\n## {}\n\n{}\n",
                skill.path.display(),
                skill.frontmatter
            ));
        }
    }
    prompt
}

/// The file's trimmed text, or `None` if it is absent, blank or unreadable.
fn read(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Some(text.trim().to_owned()).filter(|t| !t.is_empty()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            tracing::warn!("{} not loaded: {e}", path.display());
            None
        }
    }
}

#[derive(Debug, PartialEq)]
struct Skill {
    path: PathBuf,
    frontmatter: String,
}

/// Every `<dir>/<name>/SKILL.md` whose frontmatter has a `description` and fits the cap, sorted
/// by path. The frontmatter is passed on unparsed: the model reads YAML, and minima needs no field.
fn skills(dir: &Path) -> Vec<Skill> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            tracing::warn!("skills in {} not loaded: {e}", dir.display());
            return Vec::new();
        }
    };
    let mut skills: Vec<_> = entries
        .filter_map(|entry| {
            let dir = entry.ok()?.path();
            // is_dir follows symlinks, so a linked skill directory counts.
            if !dir.is_dir() {
                return None;
            }
            let path = dir.join(SKILL_MD);
            let text = read(&path)?;
            let skipped = match frontmatter(&text) {
                None => "no frontmatter",
                Some(f) if f.len() > MAX_FRONTMATTER => "frontmatter over 4096 bytes",
                Some(f) if !f.lines().any(|l| l.starts_with("description:")) => "no description",
                Some(f) => {
                    return Some(Skill {
                        frontmatter: f.to_owned(),
                        path,
                    });
                }
            };
            tracing::warn!("{} not loaded: {skipped}", path.display());
            None
        })
        .collect();
    skills.sort_by(|a, b| a.path.cmp(&b.path));
    skills
}

/// The text between the opening `---` line and the closing one, unparsed.
fn frontmatter(text: &str) -> Option<&str> {
    let mut lines = text.split_inclusive('\n');
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    let start = text.find('\n')? + 1;
    let mut end = start;
    for line in lines {
        if line.trim_end() == "---" {
            return Some(text[start..end].trim_end());
        }
        end += line.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Scratch;

    #[test]
    fn the_system_prompt_states_the_platform_and_place() {
        let prompt = system_prompt();
        assert!(prompt.starts_with("You are minima"));
        assert!(prompt.contains("# Environment"));
        assert!(prompt.contains(std::env::consts::OS));
        assert!(prompt.contains(std::env::consts::ARCH));

        let cwd = std::env::current_dir().expect("a working directory");
        assert!(prompt.contains(&cwd.display().to_string()));
    }

    #[test]
    fn the_system_prompt_names_the_shell_the_tool_runs() {
        assert!(system_prompt().contains("- Command shell: bash\n"));
    }

    #[test]
    fn instructions_are_trimmed_and_skipped_when_absent_or_blank() {
        let dir = Scratch::new("prompt-agents-md");
        let path = dir.file(AGENTS_MD);
        let path = Path::new(&path);
        assert_eq!(read(path), None);

        std::fs::write(path, " \n\n").unwrap();
        assert_eq!(read(path), None);

        std::fs::write(path, "\nUse make test.\n\n").unwrap();
        assert_eq!(read(path).as_deref(), Some("Use make test."));
    }

    /// A directory by that name is unreadable as a file; the session still starts.
    #[test]
    fn unreadable_instructions_are_skipped() {
        let dir = Scratch::new("prompt-agents-md-dir");
        let path = dir.file(AGENTS_MD);
        std::fs::create_dir(&path).unwrap();
        assert_eq!(read(Path::new(&path)), None);
    }

    #[test]
    fn frontmatter_is_the_raw_text_between_the_fences() {
        let yaml = "name: pdf\ndescription: >\n  Fill forms.\nmetadata:\n  version: \"1.0\"";
        let text = format!("---\n{yaml}\n---\n# Body\n\n---\nnot frontmatter\n");
        assert_eq!(frontmatter(&text), Some(yaml));

        let crlf = "---\r\ndescription: a\r\n---\r\nbody";
        assert_eq!(frontmatter(crlf), Some("description: a"));
    }

    #[test]
    fn no_frontmatter_without_both_fences() {
        assert_eq!(frontmatter("description: a\n---\n"), None);
        assert_eq!(frontmatter("---\ndescription: a\n"), None);
        assert_eq!(frontmatter("# Title\n---\ndescription: a\n---\n"), None);
    }

    #[test]
    fn skills_are_sorted_and_invalid_ones_are_skipped() {
        let dir = Scratch::new("prompt-skills");
        let skill = |name: &str, text: &str| {
            std::fs::create_dir(dir.file(name)).unwrap();
            std::fs::write(dir.file(&format!("{name}/{SKILL_MD}")), text).unwrap();
        };
        skill("zip", "---\nname: zip\ndescription: Pack files.\n---\n");
        skill("audit", "---\ndescription: Check deps.\n---\nbody");
        skill("bare", "no frontmatter");
        skill("nameless", "---\nname: nameless\n---\n");
        let long = "x".repeat(MAX_FRONTMATTER);
        skill("huge", &format!("---\ndescription: {long}\n---\n"));
        std::fs::create_dir(dir.file("empty")).unwrap();
        std::fs::write(dir.file("notes.txt"), "not a skill").unwrap();

        let found = skills(Path::new(&dir.file("")));
        let names: Vec<_> = found
            .iter()
            .map(|s| s.path.parent().unwrap().file_name().unwrap())
            .collect();
        assert_eq!(names, ["audit", "zip"]);
        assert_eq!(found[0].frontmatter, "description: Check deps.");
        assert!(found[0].path.ends_with("audit/SKILL.md"));

        assert!(skills(Path::new(&dir.file("missing"))).is_empty());
    }
}
