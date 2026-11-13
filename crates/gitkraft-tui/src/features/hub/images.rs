//! Image preview for PR and issue pictures, through the terminal's best graphics
//! protocol (Kitty, iTerm2, Sixel, or half blocks).

use std::collections::HashMap;

use ratatui_image::protocol::StatefulProtocol;

/// Largest image fetched, in bytes.
pub const MAX_BYTES: usize = 10 * 1024 * 1024;

/// A decoded image ready to draw.
pub struct Protocol(pub StatefulProtocol);

impl std::fmt::Debug for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Protocol(..)")
    }
}

/// Where one image is.
#[derive(Debug)]
pub enum Entry {
    /// Being fetched.
    Loading,
    /// Decoded.
    Ready(Box<Protocol>),
    /// Could not be shown.
    Failed(String),
}

/// The image overlay.
#[derive(Debug, Default)]
pub struct Images {
    /// The overlay is showing.
    pub open: bool,
    /// Images of the current item.
    pub urls: Vec<String>,
    /// Which one is shown.
    pub index: usize,
    cache: HashMap<String, Entry>,
}

impl Images {
    /// Opens the overlay on `urls`.
    pub fn open_on(&mut self, urls: Vec<String>) {
        self.index = 0;
        self.open = !urls.is_empty();
        self.urls = urls;
    }

    /// Closes the overlay (decoded images stay cached).
    pub fn close(&mut self) {
        self.open = false;
    }

    /// The URL shown.
    pub fn current(&self) -> Option<&str> {
        self.urls.get(self.index).map(String::as_str)
    }

    /// Moves to the next image, wrapping.
    pub fn step(&mut self, delta: isize) {
        if !self.urls.is_empty() {
            self.index =
                (self.index as isize + delta).rem_euclid(self.urls.len() as isize) as usize;
        }
    }

    /// What is known about `url`.
    pub fn entry(&self, url: &str) -> Option<&Entry> {
        self.cache.get(url)
    }

    /// Mutable access for drawing.
    pub fn entry_mut(&mut self, url: &str) -> Option<&mut Entry> {
        self.cache.get_mut(url)
    }

    /// Whether a fetch for `url` is needed (not loading, not cached).
    pub fn needs_fetch(&self, url: &str) -> bool {
        !self.cache.contains_key(url)
    }

    /// Marks `url` as being fetched.
    pub fn mark_loading(&mut self, url: &str) {
        self.cache.insert(url.to_string(), Entry::Loading);
    }

    /// Stores a decoded image.
    pub fn set_ready(&mut self, url: &str, protocol: StatefulProtocol) {
        self.cache
            .insert(url.to_string(), Entry::Ready(Box::new(Protocol(protocol))));
    }

    /// Stores a failure, so it is not retried every frame.
    pub fn set_failed(&mut self, url: &str, why: String) {
        self.cache.insert(url.to_string(), Entry::Failed(why));
    }
}

/// Decodes `bytes` into an image, refusing oversized or broken data.
pub fn decode(bytes: &[u8]) -> Result<image::DynamicImage, String> {
    if bytes.len() > MAX_BYTES {
        return Err(format!(
            "image is {} MB, the limit is {} MB",
            bytes.len() / (1024 * 1024),
            MAX_BYTES / (1024 * 1024)
        ));
    }
    image::load_from_memory(bytes).map_err(|e| format!("not a readable image: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_image::picker::Picker;

    fn png() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([200, 30, 30, 255]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut out, image::ImageFormat::Png)
            .expect("encode");
        out.into_inner()
    }

    #[test]
    fn decodes_png_and_rejects_garbage_and_huge_data() {
        let img = decode(&png()).expect("png");
        assert_eq!((img.width(), img.height()), (8, 8));
        assert!(decode(b"definitely not an image").is_err());
        assert!(
            decode(&vec![0u8; MAX_BYTES + 1])
                .expect_err("too big")
                .contains("limit")
        );
        assert!(decode(&[]).is_err());
    }

    #[test]
    fn navigation_wraps_and_empty_lists_do_not_open() {
        let mut i = Images::default();
        i.open_on(vec![]);
        assert!(!i.open);
        i.step(1);
        assert_eq!(i.current(), None);
        i.open_on(vec!["a".into(), "b".into(), "c".into()]);
        assert!(i.open);
        assert_eq!(i.current(), Some("a"));
        i.step(-1);
        assert_eq!(i.current(), Some("c"));
        i.step(1);
        i.step(1);
        assert_eq!(i.current(), Some("b"));
        i.close();
        assert!(!i.open);
    }

    #[test]
    fn the_cache_remembers_loading_ready_and_failed() {
        let mut i = Images::default();
        assert!(i.needs_fetch("u"));
        i.mark_loading("u");
        assert!(!i.needs_fetch("u"));
        assert!(matches!(i.entry("u"), Some(Entry::Loading)));
        let proto = Picker::halfblocks().new_resize_protocol(decode(&png()).expect("png"));
        i.set_ready("u", proto);
        assert!(matches!(i.entry("u"), Some(Entry::Ready(_))));
        i.set_failed("v", "nope".into());
        assert!(matches!(i.entry("v"), Some(Entry::Failed(w)) if w == "nope"));
        assert!(i.entry_mut("zzz").is_none());
    }
}
