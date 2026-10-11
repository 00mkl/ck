//! `:` as vim's command line: `:q`, `:tabnew`, `:colorscheme`, `:marks`, any command by its
//! `[keys]` name (`:export`), and else a place to go (`:4chan/g`). What's typed there and
//! after `/` in a thread is kept, and up and down bring it back.

use super::{App, View};
use crate::keys::Action;

/// The commands that take a place, whose argument completes as one.
pub(super) const PLACE_COMMANDS: &[&str] = &["e", "edit", "o", "open", "tabnew", "tabe", "tabedit"];

/// Lines kept of each kind.
const KEPT: usize = 100;

/// The commands, for completion (any `[keys]` name works too).
const COMMANDS: &[&str] = &["quit", "help", "tabnew", "tabclose", "tabnext", "tabprevious", "tabfirst", "tablast", "edit", "colorscheme", "set", "nohlsearch", "marks", "delmarks"];

impl App {
    /// Run `line` as a command. False if it isn't one (then it's a place to go).
    pub(super) fn run_command(&mut self, line: &str) -> bool {
        let line = line.trim();
        let (word, arg) = line.split_once(char::is_whitespace).map_or((line, ""), |(w, a)| (w, a.trim()));
        let (word, bang) = word.strip_suffix('!').map_or((word, false), |w| (w, true));
        match word {
            "q" | "quit" | "qa" | "qall" | "quitall" | "wq" | "wqa" | "wqall" => self.quit = true,
            "help" => self.act(Action::Help),
            "tabnew" | "tabe" | "tabedit" => self.open_tab(arg),
            "tabc" | "tabclose" => self.act(Action::CloseTab),
            "tabn" | "tabnext" => match arg.parse::<usize>() {
                Ok(n) if (1..=self.tabs.len()).contains(&n) => self.show_tab(n - 1),
                Ok(n) => self.info(format!("There's no tab {n}")),
                Err(_) => self.cycle_tab(true),
            },
            "tabp" | "tabprevious" | "tabN" | "tabNext" => self.cycle_tab(false),
            "tabfir" | "tabfirst" | "tabr" | "tabrewind" => self.show_tab(0),
            "tabl" | "tablast" => self.show_tab(self.tabs.len().saturating_sub(1)),
            // Without a place, `e` and `o` are boards (`/e/`, `/o/`).
            "e" | "edit" | "o" | "open" if !arg.is_empty() => self.goto_str(arg),
            "colo" | "colorscheme" if arg.is_empty() => self.info(format!("The theme is {} (:colorscheme NAME changes it; tab completes)", self.theme_name)),
            "colo" | "colorscheme" => self.choose_theme(arg),
            "se" | "set" => self.tab.navigate(View::Settings),
            "noh" | "nohlsearch" => match self.tab.thread.as_mut().filter(|t| !t.search.is_empty()) {
                Some(t) => t.set_search(String::new()),
                None => self.info("No search to clear"),
            },
            "marks" => self.list_marks(),
            "delm" | "delmarks" => self.delete_marks(arg, bang),
            name => match Action::named(name) {
                Some(action) if action.applies_in(self.scope()) => self.run_action(action),
                Some(action) => self.info(format!(":{name} ({}) doesn't work here", action.what())),
                None => return false,
            },
        }
        true
    }

    /// `:tabnew`: a tab after this one, at `place` if one's given, else home.
    fn open_tab(&mut self, place: &str) {
        if self.tabs.len() >= super::MAX_TABS {
            self.info(format!("{} tabs is the most; :tabclose closes one", super::MAX_TABS));
            return;
        }
        let at = self.active + 1;
        self.tabs.insert(at, super::Tab::new(self.tab.site, self.clock.instant()));
        self.switch_tab(at);
        if place.is_empty() {
            self.tab.navigate(View::Sites);
        } else {
            self.goto_str(place);
        }
    }

    /// What can complete the command typed so far, if it's one: the commands and `[keys]`
    /// names that start with it, or the themes after `:colorscheme`.
    pub(super) fn command_candidates(&self, typed: &str) -> Option<(String, Vec<String>)> {
        match typed.split_once(' ') {
            Some((word @ ("colo" | "colorscheme"), partial)) => {
                let names = crate::theme::names(&self.themes).into_iter().filter(|n| n.starts_with(partial.trim_start())).collect();
                Some((format!("{word} "), names))
            }
            Some(_) => None,
            None => {
                let actions = Action::ALL.iter().filter(|a| a.applies_in(self.scope())).map(|a| a.name().to_string());
                let names: Vec<String> = COMMANDS.iter().map(|c| c.to_string()).chain(actions).filter(|c| c.starts_with(typed)).collect();
                (!names.is_empty()).then(|| (String::new(), names))
            }
        }
    }
}

/// Keep `line` as the newest of `lines` (once), dropping the oldest past `KEPT`.
pub(super) fn keep(lines: &mut Vec<String>, line: &str) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    lines.retain(|l| l != line);
    lines.push(line.to_string());
    if let Some(over) = lines.len().checked_sub(KEPT) {
        lines.drain(..over);
    }
}

/// Up (`older`) or down the `lines` kept that start with what was typed before the first up
/// (`at` keeps where it is and that): the line to show now, if any.
pub(super) fn recall(lines: &[String], at: &mut Option<(usize, String)>, typed: &str, older: bool) -> Option<String> {
    let (pos, prefix) = at.get_or_insert_with(|| (lines.len(), typed.to_string()));
    let matches = |i: &usize| lines.get(*i).is_some_and(|l| l.starts_with(prefix.as_str()));
    let next = if older { (0..*pos).rev().find(matches) } else { (*pos + 1..lines.len()).find(matches) };
    match next {
        Some(i) => {
            *pos = i;
            lines.get(i).cloned()
        }
        // Down past the newest: what was typed.
        None if !older => {
            *pos = lines.len();
            Some(prefix.clone())
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_and_down_go_through_the_lines_typed_like_this_one() {
        let mut lines = Vec::new();
        for l in ["4chan/g", "tabnew", "4chan/a", "tabnew"] {
            keep(&mut lines, l);
        }
        assert_eq!(lines, ["4chan/g", "4chan/a", "tabnew"], "once each, newest last");
        let mut at = None;
        assert_eq!(recall(&lines, &mut at, "4", true).as_deref(), Some("4chan/a"));
        assert_eq!(recall(&lines, &mut at, "", true).as_deref(), Some("4chan/g"));
        assert_eq!(recall(&lines, &mut at, "", true), None, "nothing older");
        assert_eq!(recall(&lines, &mut at, "", false).as_deref(), Some("4chan/a"));
        assert_eq!(recall(&lines, &mut at, "", false).as_deref(), Some("4"), "back to what was typed");
        for i in 0..150 {
            keep(&mut lines, &i.to_string());
        }
        assert_eq!(lines.len(), KEPT);
        assert_eq!(lines.last().map(String::as_str), Some("149"));
    }
}
