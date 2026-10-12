//! What ck and ck-web, its browser helper, say to each other: one JSON object a line, ck's
//! on the helper's stdin and the helper's on its stdout. ck-web includes this file as it is,
//! so it needs nothing but serde.

use serde::{Deserialize, Serialize};

/// What ck asks of the helper. One at a time: each is answered (or fails) before the next.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum Request {
    /// Go to `url`, a page on the site posted to, so what's sent from it carries the site's
    /// cookies. Answered with `Ready` once it's there; first with `View` frames if the site
    /// wants a person to click (Cloudflare's check, DDoS-Guard's).
    Open { url: String },
    /// Send a request from the page, with its cookies: `body` as it is, or else `fields` (and
    /// a file, by its path) as a form when there are any. A `Referer` header is sent as the
    /// request's referrer. Answered with `Fetched`.
    Fetch { method: String, url: String, headers: Vec<(String, String)>, fields: Vec<(String, String)>, file: Option<Upload>, body: Option<String> },
    /// 4chan's captcha for posting on `board`: a reply in `thread`, or a new thread with 0.
    /// Its captcha comes in a frame of sys.4chan.org, and only to a page of 4chan's. Answered
    /// with `Captcha`; first with `View` frames if the site wants a person to click.
    Captcha { board: String, thread: u64 },
    /// A captcha widget of a service's (`provider`: "hcaptcha", "recaptcha", "turnstile",
    /// "yandex") with the site's `sitekey`, shown in the browser view for a person to do.
    /// Answered with `Token`.
    Widget { provider: String, sitekey: String },
    /// A click on the browser view, in the page's pixels.
    Click { x: u32, y: u32 },
    /// The wheel over the browser view, in the page's pixels: `dy` (down is positive).
    Scroll { x: u32, y: u32, dy: i32 },
    /// Stop: drop what's being done and the browser view.
    Cancel,
}

/// A file sent with a form: its path, and the form field it goes in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Upload {
    pub field: String,
    pub path: String,
}

/// A file's MIME type, as a site checks it: by what the file starts with, else by its
/// name's extension. Sent as `application/octet-stream`, jschan refuses any file.
pub fn mime(head: &[u8], name: &str) -> &'static str {
    let at = |i: usize, magic: &[u8]| head.get(i..i + magic.len()) == Some(magic);
    let brand = head.get(8..12).unwrap_or_default();
    match () {
        _ if at(0, b"\xFF\xD8\xFF") => "image/jpeg",
        _ if at(0, b"\x89PNG") => "image/png",
        _ if at(0, b"GIF8") => "image/gif",
        _ if at(0, b"RIFF") && at(8, b"WEBP") => "image/webp",
        _ if at(0, b"BM") => "image/bmp",
        _ if at(4, b"ftyp") && (brand == b"avif" || brand == b"avis") => "image/avif",
        _ if at(4, b"ftyp") && brand == b"qt  " => "video/quicktime",
        _ if at(4, b"ftyp") => "video/mp4",
        _ if at(0, b"\x1A\x45\xDF\xA3") => "video/webm",
        _ if at(0, b"OggS") => "audio/ogg",
        _ if at(0, b"fLaC") => "audio/flac",
        _ if at(0, b"ID3") || at(0, b"\xFF\xFB") || at(0, b"\xFF\xF3") => "audio/mpeg",
        _ if at(0, b"%PDF") => "application/pdf",
        _ => {
            let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
            match ext.as_str() {
                "jpg" | "jpeg" | "jfif" => "image/jpeg",
                "png" => "image/png",
                "gif" => "image/gif",
                "webp" => "image/webp",
                "avif" => "image/avif",
                "mp4" | "m4v" => "video/mp4",
                "webm" | "mkv" => "video/webm",
                "mov" => "video/quicktime",
                "mp3" => "audio/mpeg",
                "ogg" | "opus" => "audio/ogg",
                "flac" => "audio/flac",
                "pdf" => "application/pdf",
                "txt" => "text/plain",
                _ => "application/octet-stream",
            }
        }
    }
}

/// What the helper tells ck.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "is", rename_all = "snake_case")]
pub enum Reply {
    /// The page asked for is open.
    Ready,
    /// The answer to a `Fetch`: its status, its body (text, or base64 for an image), and the
    /// page's cookies after it (those its scripts can read: a site's captcha flags, LynxChan's
    /// block bypass).
    Fetched { status: u16, body: String, cookies: String },
    /// 4chan's captcha, as its captcha page gave it (the "twister").
    Captcha { twister: serde_json::Value },
    /// What a widget gave once a person did it, for the site's form.
    Token { token: String },
    /// The browser view, while the site wants a person (Cloudflare's check, hCaptcha): the
    /// part of the page with something on it, as a PNG (base64), from `left`, `top` in the
    /// page's pixels.
    View { png: String, left: u32, top: u32 },
    /// What went wrong.
    Failed { error: String },
}
