//! Conservative frontmatter compatibility checks for complete skills.

use std::fmt;

use crate::{
    domain::{HarnessId, HarnessSet},
    skill::ValidatedSkillTree,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkillCompatibilityClass {
    Compatible,
    Warning,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillCompatibility {
    target: HarnessId,
    class: SkillCompatibilityClass,
    strict_agent_skills: bool,
    loadable: bool,
    findings: Vec<SkillCompatibilityFinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SkillCompatibilityFinding {
    InvalidUtf8,
    MissingFrontmatter,
    MissingName,
    MissingDescription,
    UnsupportedField(String),
}

impl SkillCompatibility {
    pub fn evaluate(tree: &ValidatedSkillTree, targets: &HarnessSet) -> Vec<Self> {
        let bytes = tree
            .tree()
            .files()
            .iter()
            .find(|(path, _)| path.as_str() == "SKILL.md")
            .map(|(_, bytes)| bytes)
            .expect("validated skill tree contains SKILL.md");
        targets
            .iter()
            .map(|target| evaluate_target(target.clone(), bytes))
            .collect()
    }

    pub const fn target(&self) -> &HarnessId {
        &self.target
    }
    pub const fn class(&self) -> SkillCompatibilityClass {
        self.class
    }
    pub const fn strict_agent_skills(&self) -> bool {
        self.strict_agent_skills
    }
    pub const fn loadable(&self) -> bool {
        self.loadable
    }
    pub fn findings(&self) -> &[SkillCompatibilityFinding] {
        &self.findings
    }
}

fn evaluate_target(target: HarnessId, bytes: &[u8]) -> SkillCompatibility {
    let mut findings = Vec::new();
    let Ok(text) = std::str::from_utf8(bytes) else {
        findings.push(SkillCompatibilityFinding::InvalidUtf8);
        return SkillCompatibility {
            target,
            class: SkillCompatibilityClass::Blocked,
            strict_agent_skills: false,
            loadable: false,
            findings,
        };
    };
    let mut lines = text.lines();
    let has_open = lines.next() == Some("---");
    if !has_open {
        findings.push(SkillCompatibilityFinding::MissingFrontmatter);
    }
    let mut closed = false;
    let mut name = false;
    let mut description = false;
    for line in lines {
        if line == "---" {
            closed = true;
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "name" if !value.is_empty() => name = true,
                "description" if !value.is_empty() => description = true,
                _ => {}
            }
        }
    }
    if !closed || !has_open {
        findings.push(SkillCompatibilityFinding::MissingFrontmatter);
    }
    if !name {
        findings.push(SkillCompatibilityFinding::MissingName);
    }
    if !description {
        findings.push(SkillCompatibilityFinding::MissingDescription);
    }
    let loadable = name && description;
    let strict = loadable && closed && has_open;
    let class = if !loadable {
        SkillCompatibilityClass::Blocked
    } else if !strict {
        SkillCompatibilityClass::Warning
    } else {
        SkillCompatibilityClass::Compatible
    };
    SkillCompatibility {
        target,
        class,
        strict_agent_skills: strict,
        loadable,
        findings,
    }
}

impl fmt::Display for SkillCompatibilityFinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidUtf8 => "SKILL.md is not UTF-8",
            Self::MissingFrontmatter => "SKILL.md frontmatter is missing or unterminated",
            Self::MissingName => "frontmatter name is missing",
            Self::MissingDescription => "frontmatter description is missing",
            Self::UnsupportedField(field) => {
                return write!(formatter, "unsupported field `{field}`");
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{HarnessId, HarnessSet, RelativeArtifactPath},
        runtime::{ExternalTreeEntry, ExternalTreeLimits, ExternalTreeSnapshot},
        skill::ValidatedSkillTree,
    };

    fn tree(content: &[u8]) -> ValidatedSkillTree {
        let snapshot = ExternalTreeSnapshot::new(
            [ExternalTreeEntry::file(
                RelativeArtifactPath::new("SKILL.md").unwrap(),
                content.to_vec(),
            )],
            ExternalTreeLimits::new(8, 32, 1024, 4096, 1024).unwrap(),
        )
        .unwrap();
        ValidatedSkillTree::validate(&snapshot).unwrap()
    }

    #[test]
    fn classifies_loadable_and_incomplete_frontmatter_without_rewriting_it() {
        let targets = HarnessSet::new([
            HarnessId::new("codex").unwrap(),
            HarnessId::new("claude").unwrap(),
        ])
        .unwrap();
        let result = SkillCompatibility::evaluate(
            &tree(b"---\nname: demo\ndescription: test\n---\nbody\n"),
            &targets,
        );
        assert!(
            result
                .iter()
                .all(|value| value.class() == SkillCompatibilityClass::Compatible)
        );
        let incomplete = SkillCompatibility::evaluate(&tree(b"body\n"), &targets);
        assert!(
            incomplete
                .iter()
                .all(|value| value.class() == SkillCompatibilityClass::Blocked)
        );
    }
}
