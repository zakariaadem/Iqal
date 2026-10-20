//! Unsaved edits to one block: the draft both apps edit, and turning the
//! rule lists typed as text back into rules.

use iqal_core::{AppRule, BlockSet, WebRule};
use iqal_i18n::ta;

/// The rule lists edited as text, one rule per line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ListKey {
    WebBlock,
    WebAllow,
    AppBlock,
    AppAllow,
}

impl ListKey {
    pub const ALL: [ListKey; 4] = [ListKey::WebBlock, ListKey::WebAllow, ListKey::AppBlock, ListKey::AppAllow];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn is_web(self) -> bool {
        matches!(self, ListKey::WebBlock | ListKey::WebAllow)
    }

    /// The saved rules of this list, one per line.
    pub fn text_of(self, set: &BlockSet) -> String {
        let lines: Vec<String> = match self {
            ListKey::WebBlock => set.web.block.iter().map(ToString::to_string).collect(),
            ListKey::WebAllow => set.web.allow.iter().map(ToString::to_string).collect(),
            ListKey::AppBlock => set.apps.block.iter().map(ToString::to_string).collect(),
            ListKey::AppAllow => set.apps.allow.iter().map(ToString::to_string).collect(),
        };
        lines.join("\n")
    }

    /// Checks one line; the error is already translated.
    pub fn check(self, line: &str) -> Result<(), String> {
        if self.is_web() {
            WebRule::parse(line).map(drop).map_err(|e| e.to_string())
        } else {
            AppRule::parse(line).map(drop).map_err(|e| e.to_string())
        }
    }

    /// Parses the text into the set. Returns the bad lines (1-based number,
    /// line, problem) and leaves the set unchanged if there are any.
    pub fn apply(self, text: &str, set: &mut BlockSet) -> Vec<(usize, String, String)> {
        let mut bad = Vec::new();
        let mut web = Vec::new();
        let mut apps = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if self.is_web() {
                match WebRule::parse(line) {
                    Ok(r) if !web.contains(&r) => web.push(r),
                    Ok(_) => {}
                    Err(e) => bad.push((n + 1, line.to_string(), e.to_string())),
                }
            } else {
                match AppRule::parse(line) {
                    Ok(r) if !apps.contains(&r) => apps.push(r),
                    Ok(_) => {}
                    Err(e) => bad.push((n + 1, line.to_string(), e.to_string())),
                }
            }
        }
        if bad.is_empty() {
            match self {
                ListKey::WebBlock => set.web.block = web,
                ListKey::WebAllow => set.web.allow = web,
                ListKey::AppBlock => set.apps.block = apps,
                ListKey::AppAllow => set.apps.allow = apps,
            }
        }
        bad
    }
}

/// Lines that mean something: trimmed, without blanks.
pub fn meaningful(text: &str) -> Vec<&str> {
    text.lines().map(str::trim).filter(|l| !l.is_empty()).collect()
}

/// Unsaved edits to one block.
pub struct Draft {
    /// The block as the service last reported it.
    pub base: BlockSet,
    /// The edited block. Its rule lists are not used: `texts` hold them.
    pub set: BlockSet,
    pub texts: [String; 4],
}

impl Draft {
    pub fn new(base: BlockSet) -> Self {
        let texts = ListKey::ALL.map(|k| k.text_of(&base));
        Draft { set: base.clone(), base, texts }
    }

    /// The edited block with the fields the editor never changes (lists,
    /// lock) taken from the saved one, for comparing.
    fn comparable(&self) -> BlockSet {
        let mut s = self.set.clone();
        s.web.block = self.base.web.block.clone();
        s.web.allow = self.base.web.allow.clone();
        s.apps.block = self.base.apps.block.clone();
        s.apps.allow = self.base.apps.allow.clone();
        s.lock = self.base.lock.clone();
        s.pending_unlock_at = self.base.pending_unlock_at;
        s
    }

    pub fn dirty(&self) -> bool {
        self.comparable() != self.base
            || ListKey::ALL.iter().any(|k| meaningful(&self.texts[k.index()]) != meaningful(&k.text_of(&self.base)))
    }

    /// The block to save, or the lines that cannot be used.
    pub fn finish(&self) -> Result<BlockSet, String> {
        let mut set = self.set.clone();
        set.lock = self.base.lock.clone();
        set.pending_unlock_at = self.base.pending_unlock_at;
        let mut problems = Vec::new();
        for k in ListKey::ALL {
            for (n, line, problem) in k.apply(&self.texts[k.index()], &mut set) {
                problems.push(ta(
                    "line-invalid",
                    &[("line", (n as i64).into()), ("rule", line.into()), ("problem", problem.into())],
                ));
            }
        }
        if problems.is_empty() {
            Ok(set)
        } else {
            problems.truncate(8);
            Err(problems.join("\n"))
        }
    }
}

