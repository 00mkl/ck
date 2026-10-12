//! Vim's marks: `ma` marks the selected post as `a`, `'a` goes back to it. A lowercase mark
//! is the thread's own; an uppercase one is one of its kind, found from anywhere. Kept
//! between runs; `ctrl-o` goes back from where `'` went. And `ctrl-^`, vim's alternate file:
//! the thread read before this one.

use super::{App, Opening, View};
use crate::store::Mark;

/// Lowercase marks kept, across threads: the oldest go past this.
const MOST: usize = 1000;

impl App {
    /// `m` and a letter: mark the selected post.
    pub(super) fn set_mark(&mut self, name: char) {
        let here = self.tab.thread.as_ref().filter(|_| self.tab.view() == View::Thread).and_then(|t| Some((t.key().clone(), t.current()?.no)));
        let Some((key, post)) = here else {
            self.note("Marks are set on a thread's posts");
            return;
        };
        let marks = &mut self.store.recall.marks;
        marks.retain(|m| m.name != name || (name.is_ascii_lowercase() && m.key != key));
        marks.push(Mark { name, key, post });
        // Past the most kept, the oldest lowercase marks go (the first listed).
        let mut over = marks.iter().filter(|m| m.name.is_ascii_lowercase()).count().saturating_sub(MOST);
        marks.retain(|m| {
            let drop = over > 0 && m.name.is_ascii_lowercase();
            over -= usize::from(drop);
            !drop
        });
        self.save();
        self.info(format!("No.{post} marked as {name} ('{name} comes back to it)"));
    }

    /// `'` and a letter: back to that mark, from wherever it's still found.
    pub(super) fn go_to_mark(&mut self, name: char) {
        let key = self.tab.thread.as_ref().filter(|_| self.tab.view() == View::Thread).map(|t| t.key().clone());
        let mark = self.store.recall.marks.iter().find(|m| m.name == name && (name.is_ascii_uppercase() || Some(&m.key) == key.as_ref())).cloned();
        let Some(mark) = mark else {
            let msg = match (name.is_ascii_lowercase(), key.is_some()) {
                (true, false) => format!("Mark {name} is a thread's: open the thread, or use an uppercase mark (m{})", name.to_ascii_uppercase()),
                _ => format!("No mark {name}"),
            };
            self.note(msg);
            return;
        };
        if key.as_ref() == Some(&mark.key) {
            if let Some(t) = &mut self.tab.thread
                && !t.jump_to(mark.post)
            {
                self.note(format!("No.{} (mark {name}) isn't in the thread any more", mark.post));
            }
            return;
        }
        // Another thread: `ctrl-o` comes back here.
        let back = Some(self.tab.place_view());
        if key.is_some() {
            self.leave_trail();
        }
        if self.open_thread_key(&mark.key, Opening::at(Some(mark.post))) {
            self.tab.return_to = back;
        }
    }

    /// `ctrl-^`: the thread read before this one (the newest in History that isn't it), where
    /// you left it; again, back.
    pub(super) fn alternate(&mut self) {
        let here = self.tab.thread.as_ref().filter(|_| self.tab.view() == View::Thread).map(|t| t.key().clone());
        let Some(key) = self.store.history.iter().map(|v| &v.key).find(|k| Some(*k) != here.as_ref()).cloned() else {
            self.note("No other thread read yet");
            return;
        };
        let back = Some(self.tab.place_view());
        if here.is_some() {
            self.leave_trail();
        }
        // At the post you were on, if this tab was there.
        let post = self.tab.trail.iter().chain(&self.tab.ahead).rev().find(|t| t.key == key).map(|t| t.post);
        if self.open_thread_key(&key, Opening::at(post)) {
            self.tab.return_to = back;
        }
    }

    /// `:marks`: the marks there are here, and the uppercase ones.
    pub(super) fn list_marks(&mut self) {
        let key = self.tab.thread.as_ref().map(|t| t.key().clone());
        let mut marks: Vec<&Mark> = self.store.recall.marks.iter().filter(|m| m.name.is_ascii_uppercase() || Some(&m.key) == key.as_ref()).collect();
        marks.sort_by_key(|m| (m.name.is_ascii_uppercase(), m.name));
        let shown: Vec<String> = marks
            .iter()
            .map(|m| if Some(&m.key) == key.as_ref() { format!("{} No.{}", m.name, m.post) } else { format!("{} {}/{}/{} No.{}", m.name, m.key.site, m.key.board, m.key.no, m.post) })
            .collect();
        let msg = if shown.is_empty() { "No marks here (ma marks the post as a)".to_string() } else { shown.join("  ·  ") };
        self.info(msg);
    }

    /// `:delmarks a B`: forget those marks (lowercase ones in this thread); `:delmarks!`
    /// forgets this thread's.
    pub(super) fn delete_marks(&mut self, names: &str, all_here: bool) {
        let key = self.tab.thread.as_ref().map(|t| t.key().clone());
        let names: Vec<char> = names.chars().filter(char::is_ascii_alphabetic).collect();
        let before = self.store.recall.marks.len();
        self.store.recall.marks.retain(|m| {
            let here = Some(&m.key) == key.as_ref();
            let named = names.contains(&m.name) && (m.name.is_ascii_uppercase() || here);
            !(named || (all_here && here && m.name.is_ascii_lowercase()))
        });
        let gone = before - self.store.recall.marks.len();
        self.save();
        self.info(match gone {
            0 => "No such marks".to_string(),
            1 => "1 mark forgotten".to_string(),
            n => format!("{n} marks forgotten"),
        });
    }
}
