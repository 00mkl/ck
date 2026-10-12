//! `u` right after a change takes it back: a filter or hidden word added, a thread or post
//! hidden by hand, an entry removed from a list. Any other key (or click) keeps the change.

use super::App;
use super::home::BoardRef;
use crate::filter::FilterConfig;
use crate::store::{ThreadKey, Visit, Watched};

/// What `u` takes back.
pub enum Undo {
    /// A filter added (at its place), and whether it was written to the config (so taking it
    /// back is written too).
    Filter(usize, FilterConfig, bool),
    /// A hidden word added, and whether it was written to the config.
    Word(String, bool),
    /// A thread or post hidden or shown by hand.
    Hidden(ThreadKey),
    /// A thread taken off Watched, from its place.
    Watched(usize, Box<Watched>),
    /// A thread taken off History, from its place.
    Visit(usize, Visit),
    /// A favorite board taken off the home screen, from its place.
    Favorite(usize, BoardRef),
    /// A recent board taken off the home screen, from its place.
    Recent(usize, String),
    /// A site hidden or shown on the home screen.
    Site(usize),
}

impl App {
    /// `u`: take back the change just made.
    pub fn undo(&mut self) {
        let Some(undo) = self.undo.take() else { return };
        match undo {
            Undo::Filter(i, f, written) => self.take_back_filter(i, &f, written),
            Undo::Word(w, written) => self.remove_hidden_word(&w, written),
            Undo::Hidden(key) => {
                let hidden = self.rehide(|a| a.store.toggle_hidden(&key.site, &key.board, key.no));
                self.save_now();
                self.info(if hidden { format!("Hid No.{} again", key.no) } else { format!("Unhid No.{}", key.no) });
            }
            Undo::Watched(i, w) => {
                let name = super::thread_name(w.key.no, &w.subject);
                self.rehide(|a| a.store.rewatch(i, *w));
                self.save_now();
                self.info(format!("Watching {name} again"));
            }
            Undo::Visit(i, v) => {
                let no = v.key.no;
                self.store.history.insert(i.min(self.store.history.len()), v);
                self.save_now();
                self.info(format!("Thread {no} is back in History"));
            }
            Undo::Favorite(i, f) => {
                let board = f.board.clone();
                self.favorites.insert(i.min(self.favorites.len()), f);
                self.save_favorites(&format!("/{board}/ back in the favorites"));
            }
            Undo::Recent(i, b) => {
                self.store.recent_boards.insert(i.min(self.store.recent_boards.len()), b);
                self.save_now();
                self.info("The board is back in the recent ones");
            }
            Undo::Site(i) => self.toggle_site_hidden(i),
        }
    }
}
