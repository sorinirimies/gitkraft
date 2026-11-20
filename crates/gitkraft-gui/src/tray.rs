//! The tray / menu bar icon and its menu.
//!
//! * macOS and Windows: [`tray-icon`](https://docs.rs/tray-icon), created on the UI thread.
//! * Linux: the StatusNotifierItem D-Bus protocol through [`ksni`](https://docs.rs/ksni) on a
//!   thread of its own, so GTK is not needed. Desktops without a tray host (stock GNOME) just
//!   show no icon: [`start`] then answers `false` and the window behaves as without a tray.
//!
//! What the menu offers is [`entries`], plain data that every platform draws; a click comes back
//! as a [`TrayCommand`] through [`commands`].

use std::sync::{Mutex, OnceLock};

use futures::StreamExt;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures::stream::BoxStream;

/// What a menu entry asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    /// Bring the window to the front.
    Show,
    /// Bring the window to the front on the hub.
    ShowHub,
    /// Start the first enabled agent job now.
    RunAgents,
    /// Flip "start at login".
    ToggleAutostart,
    /// Flip "keep running in the tray when the window closes".
    ToggleCloseToTray,
    /// Quit GitKraft.
    Quit,
}

impl TrayCommand {
    /// Every command, in menu order.
    pub const ALL: [Self; 6] = [
        Self::Show,
        Self::ShowHub,
        Self::RunAgents,
        Self::ToggleAutostart,
        Self::ToggleCloseToTray,
        Self::Quit,
    ];

    /// The id a menu item carries.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Show => "gitkraft.show",
            Self::ShowHub => "gitkraft.hub",
            Self::RunAgents => "gitkraft.agents",
            Self::ToggleAutostart => "gitkraft.autostart",
            Self::ToggleCloseToTray => "gitkraft.close-to-tray",
            Self::Quit => "gitkraft.quit",
        }
    }

    /// The command of a menu item id.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.id() == id)
    }
}

/// What the menu shows besides its fixed entries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrayInfo {
    /// Does this instance run the agent schedule (rather than another open GitKraft)?
    pub runs_schedule: bool,
    /// Is the hub (and with it the scheduler) up at all?
    pub hub_up: bool,
    /// Enabled agent jobs.
    pub jobs: usize,
    /// Agent reports that need the user and were not looked at.
    pub unseen: usize,
    /// "Start at login" is on.
    pub autostart: bool,
    /// Closing the window keeps the app in the tray.
    pub close_to_tray: bool,
}

impl TrayInfo {
    /// The status line at the top of the menu.
    #[must_use]
    pub fn status_line(&self) -> String {
        let schedule = if !self.hub_up {
            "Agents: not started yet".to_string()
        } else if self.jobs == 0 {
            "Agents: no scheduled jobs".to_string()
        } else if self.runs_schedule {
            format!(
                "Agents: {} scheduled job{} run while GitKraft is open",
                self.jobs,
                if self.jobs == 1 { "" } else { "s" }
            )
        } else {
            "Agents: another GitKraft runs the schedule".to_string()
        };
        if self.unseen > 0 {
            format!("{schedule} · {} to look at", self.unseen)
        } else {
            schedule
        }
    }

    /// The tooltip.
    #[must_use]
    pub fn tooltip(&self) -> String {
        if self.unseen > 0 {
            format!("GitKraft — {} agent finding(s) need you", self.unseen)
        } else {
            "GitKraft".to_string()
        }
    }
}

/// One line of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// A greyed line of text.
    Label(String),
    /// A command.
    Item(TrayCommand, String),
    /// A command with a check mark.
    Check(TrayCommand, String, bool),
    /// A divider.
    Separator,
}

/// The menu for `info`.
#[must_use]
pub fn entries(info: &TrayInfo) -> Vec<Entry> {
    vec![
        Entry::Item(TrayCommand::Show, "Open GitKraft".into()),
        Entry::Item(TrayCommand::ShowHub, "Open the hub".into()),
        Entry::Separator,
        Entry::Label(info.status_line()),
        Entry::Item(TrayCommand::RunAgents, "Run the agent jobs now".into()),
        Entry::Separator,
        Entry::Check(
            TrayCommand::ToggleAutostart,
            "Start at login".into(),
            info.autostart,
        ),
        Entry::Check(
            TrayCommand::ToggleCloseToTray,
            "Keep running in the tray when the window closes".into(),
            info.close_to_tray,
        ),
        Entry::Separator,
        Entry::Item(TrayCommand::Quit, "Quit GitKraft".into()),
    ]
}

static TX: OnceLock<UnboundedSender<TrayCommand>> = OnceLock::new();
static RX: Mutex<Option<UnboundedReceiver<TrayCommand>>> = Mutex::new(None);

fn sender() -> UnboundedSender<TrayCommand> {
    TX.get_or_init(|| {
        let (tx, rx) = unbounded();
        if let Ok(mut slot) = RX.lock() {
            *slot = Some(rx);
        }
        tx
    })
    .clone()
}

fn send(command: TrayCommand) {
    let _ = sender().unbounded_send(command);
}

/// The clicks on the menu, as a stream for an iced subscription.
pub fn commands() -> BoxStream<'static, TrayCommand> {
    let _ = sender();
    match RX.lock().ok().and_then(|mut slot| slot.take()) {
        Some(rx) => rx.boxed(),
        None => futures::stream::pending().boxed(),
    }
}

/// Puts the icon in the tray (once). Call it from the UI thread; `false` when there is no tray.
pub fn start(info: &TrayInfo) -> bool {
    // Tests never put an icon in the developer's tray.
    if cfg!(test) {
        return false;
    }
    let ok = imp::start(info);
    if !ok && cfg!(target_os = "linux") {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        let distro = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        tracing::warn!("{}", missing_host_hint(&desktop, &distro));
    }
    ok
}

/// Why there may be no tray on Linux, and what to do, for the desktop in `XDG_CURRENT_DESKTOP`
/// and the text of `/etc/os-release`.
///
/// GNOME has no tray of its own (Ubuntu ships the AppIndicator extension, Fedora, Debian, Arch,
/// `EndeavourOS` and Manjaro may not); COSMIC, KDE, XFCE, Cinnamon, MATE and `LXQt` have one.
#[must_use]
pub fn missing_host_hint(desktop: &str, os_release: &str) -> String {
    let desktop = desktop.to_ascii_lowercase();
    let id = os_release
        .lines()
        .find_map(|l| l.strip_prefix("ID="))
        .unwrap_or("")
        .trim_matches('"')
        .to_ascii_lowercase();
    let install = match id.as_str() {
        "arch" | "endeavouros" | "manjaro" | "garuda" | "cachyos" => {
            "sudo pacman -S gnome-shell-extension-appindicator"
        }
        "fedora" | "rhel" | "centos" => "sudo dnf install gnome-shell-extension-appindicator",
        "debian" | "ubuntu" | "linuxmint" | "pop" => {
            "sudo apt install gnome-shell-extension-appindicator"
        }
        "opensuse-tumbleweed" | "opensuse-leap" => {
            "sudo zypper install gnome-shell-extension-appindicator"
        }
        _ => "install your distribution's gnome-shell-extension-appindicator package",
    };
    if desktop.contains("gnome") {
        format!(
            "no tray: GNOME does not show tray icons by itself. Run `{install}`, then \
`gnome-extensions enable appindicatorsupport@rgcjonas.gmail.com` and log in again. \
GitKraft opens its window meanwhile"
        )
    } else if desktop.contains("cosmic") {
        "no tray: add the \"Status Area\" applet in COSMIC Settings → Desktop → Panel (or Dock) → \
Configure applets. GitKraft opens its window meanwhile"
            .to_string()
    } else {
        "no tray: this desktop shows no StatusNotifierItem tray (needs a panel with a system tray \
or status-notifier plugin, e.g. waybar's `tray` module). GitKraft opens its window meanwhile"
            .to_string()
    }
}

/// Shows the current state in the menu and the tooltip.
pub fn update(info: &TrayInfo) {
    imp::update(info);
}

#[cfg(any(target_os = "macos", windows))]
mod imp {
    use std::cell::RefCell;

    use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

    use super::{Entry, TrayCommand, TrayInfo, entries, send};

    thread_local! {
        static TRAY: RefCell<Option<(TrayIcon, TrayInfo)>> = const { RefCell::new(None) };
    }

    fn menu(info: &TrayInfo) -> Menu {
        let menu = Menu::new();
        for entry in entries(info) {
            let _ = match entry {
                Entry::Label(text) => menu.append(&MenuItem::new(text, false, None)),
                Entry::Item(cmd, text) => {
                    menu.append(&MenuItem::with_id(cmd.id(), text, true, None))
                }
                Entry::Check(cmd, text, on) => {
                    menu.append(&CheckMenuItem::with_id(cmd.id(), text, true, on, None))
                }
                Entry::Separator => menu.append(&PredefinedMenuItem::separator()),
            };
        }
        menu
    }

    pub(super) fn start(info: &TrayInfo) -> bool {
        if TRAY.with(|t| t.borrow().is_some()) {
            return true;
        }
        let (rgba, size) = crate::window::icon_rgba();
        let Ok(icon) = Icon::from_rgba(rgba.to_vec(), size, size) else {
            return false;
        };
        MenuEvent::set_event_handler(Some(|event: MenuEvent| {
            if let Some(cmd) = TrayCommand::from_id(&event.id.0) {
                send(cmd);
            }
        }));
        #[cfg(windows)]
        tray_icon::TrayIconEvent::set_event_handler(Some(|event: tray_icon::TrayIconEvent| {
            if let tray_icon::TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                button_state: tray_icon::MouseButtonState::Up,
                ..
            } = event
            {
                send(TrayCommand::Show);
            }
        }));
        let built = TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip(info.tooltip())
            .with_menu(Box::new(menu(info)))
            // macOS opens the menu on a click; Windows shows the window and keeps the menu on the
            // right button.
            .with_menu_on_left_click(cfg!(target_os = "macos"))
            .build();
        match built {
            Ok(tray) => {
                TRAY.with(|t| *t.borrow_mut() = Some((tray, info.clone())));
                true
            }
            Err(e) => {
                tracing::warn!("no tray icon: {e}");
                false
            }
        }
    }

    pub(super) fn update(info: &TrayInfo) {
        TRAY.with(|t| {
            if let Some((tray, shown)) = t.borrow_mut().as_mut()
                && shown != info
            {
                tray.set_menu(Some(Box::new(menu(info))));
                let _ = tray.set_tooltip(Some(info.tooltip()));
                *shown = info.clone();
            }
        });
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::sync::OnceLock;
    use std::time::Duration;

    use ksni::TrayMethods;
    use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

    use super::{Entry, TrayCommand, TrayInfo, entries, send};

    static UPDATES: OnceLock<UnboundedSender<TrayInfo>> = OnceLock::new();

    #[derive(Debug)]
    struct Tray {
        info: TrayInfo,
    }

    /// RGBA to the ARGB32 (network order) the protocol wants.
    fn argb(rgba: &[u8]) -> Vec<u8> {
        rgba.chunks_exact(4)
            .flat_map(|p| [p[3], p[0], p[1], p[2]])
            .collect()
    }

    impl ksni::Tray for Tray {
        fn id(&self) -> String {
            "gitkraft".into()
        }

        fn title(&self) -> String {
            self.info.tooltip()
        }

        fn icon_name(&self) -> String {
            crate::window::APP_ID.into()
        }

        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            let (rgba, size) = crate::window::icon_rgba();
            vec![ksni::Icon {
                width: size as i32,
                height: size as i32,
                data: argb(rgba),
            }]
        }

        fn activate(&mut self, _x: i32, _y: i32) {
            send(TrayCommand::Show);
        }

        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            use ksni::menu::{CheckmarkItem, StandardItem};
            entries(&self.info)
                .into_iter()
                .map(|entry| match entry {
                    Entry::Label(label) => StandardItem {
                        label,
                        enabled: false,
                        ..Default::default()
                    }
                    .into(),
                    Entry::Item(cmd, label) => StandardItem {
                        label,
                        activate: Box::new(move |_: &mut Self| send(cmd)),
                        ..Default::default()
                    }
                    .into(),
                    Entry::Check(cmd, label, checked) => CheckmarkItem {
                        label,
                        checked,
                        activate: Box::new(move |_: &mut Self| send(cmd)),
                        ..Default::default()
                    }
                    .into(),
                    Entry::Separator => ksni::MenuItem::Separator,
                })
                .collect()
        }
    }

    pub(super) fn start(info: &TrayInfo) -> bool {
        if UPDATES.get().is_some() {
            return true;
        }
        let (tx, mut rx) = unbounded_channel::<TrayInfo>();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<bool>();
        let first = info.clone();
        let spawned = std::thread::Builder::new()
            .name("gitkraft-tray".into())
            .spawn(move || {
                let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    let _ = ready_tx.send(false);
                    return;
                };
                rt.block_on(async move {
                    let handle = match (Tray { info: first }).spawn().await {
                        Ok(handle) => handle,
                        Err(e) => {
                            tracing::warn!("no tray icon: {e}");
                            let _ = ready_tx.send(false);
                            return;
                        }
                    };
                    let _ = ready_tx.send(true);
                    while let Some(info) = rx.recv().await {
                        handle.update(move |t: &mut Tray| t.info = info).await;
                    }
                });
            });
        if spawned.is_err()
            || !ready_rx
                .recv_timeout(Duration::from_secs(5))
                .unwrap_or(false)
        {
            return false;
        }
        let _ = UPDATES.set(tx);
        true
    }

    pub(super) fn update(info: &TrayInfo) {
        if let Some(tx) = UPDATES.get() {
            let _ = tx.send(info.clone());
        }
    }
}

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
mod imp {
    use super::TrayInfo;

    pub(super) fn start(_info: &TrayInfo) -> bool {
        false
    }

    pub(super) fn update(_info: &TrayInfo) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_has_its_own_id_and_the_menu_offers_each_once() {
        for c in TrayCommand::ALL {
            assert_eq!(TrayCommand::from_id(c.id()), Some(c));
        }
        assert_eq!(TrayCommand::from_id("nope"), None);
        let menu = entries(&TrayInfo::default());
        for c in TrayCommand::ALL {
            let n = menu
                .iter()
                .filter(|e| matches!(e, Entry::Item(x, _) | Entry::Check(x, _, _) if *x == c))
                .count();
            assert_eq!(n, 1, "{c:?}");
        }
    }

    #[test]
    fn the_check_marks_follow_the_settings() {
        let info = TrayInfo {
            autostart: true,
            ..TrayInfo::default()
        };
        let checks: Vec<_> = entries(&info)
            .into_iter()
            .filter_map(|e| match e {
                Entry::Check(c, _, on) => Some((c, on)),
                _ => None,
            })
            .collect();
        assert_eq!(
            checks,
            [
                (TrayCommand::ToggleAutostart, true),
                (TrayCommand::ToggleCloseToTray, false)
            ]
        );
    }

    #[test]
    fn the_status_line_says_who_runs_the_schedule() {
        let mut i = TrayInfo::default();
        assert_eq!(i.status_line(), "Agents: not started yet");
        i.hub_up = true;
        assert_eq!(i.status_line(), "Agents: no scheduled jobs");
        i.jobs = 2;
        assert_eq!(
            i.status_line(),
            "Agents: another GitKraft runs the schedule"
        );
        i.runs_schedule = true;
        i.unseen = 1;
        assert_eq!(
            i.status_line(),
            "Agents: 2 scheduled jobs run while GitKraft is open · 1 to look at"
        );
        assert!(i.tooltip().contains("1 agent finding"));
    }

    #[test]
    fn the_hint_names_the_package_of_the_distribution_and_covers_cosmic() {
        let arch = missing_host_hint("GNOME", "NAME=\"EndeavourOS\"\nID=endeavouros\n");
        assert!(arch.contains("pacman -S gnome-shell-extension-appindicator"));
        assert!(missing_host_hint("ubuntu:GNOME", "ID=ubuntu").contains("apt install"));
        assert!(missing_host_hint("GNOME", "ID=fedora").contains("dnf install"));
        assert!(missing_host_hint("COSMIC", "ID=pop").contains("Status Area"));
        assert!(missing_host_hint("sway", "ID=arch").contains("waybar"));
    }
}
