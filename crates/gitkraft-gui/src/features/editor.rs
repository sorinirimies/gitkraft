//! Editor selector widget for the header toolbar.
//!
//! Provides a [`pick_list()`] drop-down that lets the user switch between
//! all supported editors defined in `gitkraft_core` at runtime.

use iced::widget::pick_list;
use iced::{Element, Length};

use crate::message::Message;

/// All editor variants (including `None`) for the pick list.
fn all_editors() -> Vec<gitkraft_core::Editor> {
    use gitkraft_core::Editor;
    vec![
        Editor::None,
        Editor::Helix,
        Editor::Neovim,
        Editor::Vim,
        Editor::Nano,
        Editor::Micro,
        Editor::Emacs,
        Editor::VSCode,
        Editor::Zed,
        Editor::Sublime,
        Editor::RustRover,
        Editor::IntelliJIdea,
        Editor::WebStorm,
        Editor::PyCharm,
        Editor::GoLand,
        Editor::CLion,
        Editor::Fleet,
        Editor::AndroidStudio,
    ]
}

/// Create an editor selector [`pick_list()`] widget.
///
/// The widget displays the name of the current editor and, when opened, lists
/// every editor returned by [`all_editors`]. Selecting a new entry emits
/// [`Message::EditorChanged`] with the chosen [`gitkraft_core::Editor`].
pub fn editor_selector(current: &gitkraft_core::Editor) -> Element<'static, Message> {
    let choices = all_editors();
    let selected = choices.iter().find(|e| *e == current).cloned();

    pick_list(choices, selected, Message::EditorChanged)
        .placeholder("Select editor")
        .text_size(13.0)
        .width(Length::Fixed(160.0))
        .into()
}
