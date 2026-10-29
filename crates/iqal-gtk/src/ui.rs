//! The main window: block list on one side, editor on the other.
//!
//! Edits go to a draft of the selected block and reach the service only
//! when the person presses Save; Discard throws the draft away. Lock
//! changes are the exception: they are saved at once (see `commit`).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use iqal_core::{BlockSet, SetId};
use iqal_i18n::{t, ta};
use iqal_ipc::{ListSummary, Request, Response, SetInfo, Status};
use gtk::glib;
use gtk::prelude::*;

pub use iqal_app::draft::{Draft, ListKey};

use crate::service::Service;
use crate::widgets::{badge, hbox, icon_button, note, vbox};
use crate::messages::{self, Sign};
use crate::{dialogs, editor};

pub struct Ui {
    pub window: gtk::ApplicationWindow,
    pub service: Service,
    sidebar: gtk::ListBox,
    content: gtk::Box,
    /// The service's state, at the bottom.
    status_line: gtk::Label,
    state: RefCell<State>,
    rebuilding: Cell<bool>,
    /// Set while the editor is replaced: its tabs report switches then.
    replacing: Cell<bool>,
    /// The editor tab shown, kept across rebuilds and blocks.
    tab: Cell<u32>,
    notebook: RefCell<Option<gtk::Notebook>>,
    save_bar: RefCell<Option<(gtk::Revealer, gtk::Label, gtk::Button)>>,
    was_dirty: Cell<bool>,
    /// (service start, change counter) of the blocks last fetched.
    loaded: Cell<Option<(i64, u64)>>,
    /// What the block list shows, to redraw it only on change.
    sidebar_rows: RefCell<Option<Vec<SidebarRow>>>,
    /// Websites of pre-made lists, by list id, with the (size, update time)
    /// they were fetched at.
    list_cache: RefCell<HashMap<String, CachedList>>,
}

/// A pre-made list's size and update time: its rules change with them.
type ListVersion = (usize, Option<i64>);
type CachedList = (ListVersion, Rc<Vec<String>>);

/// One line of the block list.
#[derive(PartialEq)]
struct SidebarRow {
    name: String,
    unsaved: bool,
    state: &'static str,
    locked: bool,
}

#[derive(Default)]
struct State {
    sets: Vec<SetInfo>,
    selected: Option<SetId>,
    /// What the editor currently shows, to rebuild it only on change.
    shown: Option<(BlockSet, bool)>,
    draft: Option<Draft>,
    status: Option<Box<Status>>,
    error: Option<String>,
}

impl Ui {
    pub fn start(app: &gtk::Application) -> Rc<Ui> {
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title(t("app-name"))
            .default_width(1000)
            .default_height(700)
            .icon_name("iqal")
            .build();

        let sidebar = gtk::ListBox::new();
        sidebar.add_css_class("navigation-sidebar");
        let content = vbox(0);
        content.set_hexpand(true);

        // Developer aid (screenshots, testing): open a given tab.
        let tab = std::env::var("IQAL_GUI_TAB").ok().and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        let ui = Rc::new(Ui {
            window,
            service: Service::new(),
            sidebar,
            content,
            status_line: gtk::Label::new(None),
            state: RefCell::new(State::default()),
            rebuilding: Cell::new(false),
            replacing: Cell::new(false),
            tab: Cell::new(tab),
            notebook: RefCell::new(None),
            save_bar: RefCell::new(None),
            was_dirty: Cell::new(false),
            loaded: Cell::new(None),
            sidebar_rows: RefCell::new(None),
            list_cache: RefCell::default(),
        });
        ui.build();
        // Every handler holds only a weak reference (no cycles between the
        // widgets and the state). The window holds the strong one, so the
        // state lives exactly as long as the window.
        let keep = ui.clone();
        ui.window.connect_destroy(move |_| {
            let _ = &keep;
        });
        ui.refresh(true);
        ui.window.present();
        ui.open_requested_dialog();

        let weak = Rc::downgrade(&ui);
        glib::timeout_add_seconds_local(3, move || match weak.upgrade() {
            Some(ui) => {
                ui.refresh(false);
                glib::ControlFlow::Continue
            }
            None => glib::ControlFlow::Break,
        });
        ui
    }

    fn build(self: &Rc<Self>) {
        // A plain title bar, as on Windows: the menu is a button above the
        // block list.
        let header = gtk::HeaderBar::new();
        self.window.set_titlebar(Some(&header));

        // Left: "New block" and "Menu", then the block list.
        let side = vbox(8);
        // As wide as on Windows, whatever its buttons would like.
        side.set_size_request(280, -1);
        side.set_hexpand(false);
        side.set_margin_start(12);
        side.set_margin_top(12);
        side.set_margin_bottom(12);
        let buttons = hbox(6);
        let new_button = icon_button(&t("action-new-block"), "list-add-symbolic", "main");
        new_button.set_hexpand(true);
        let weak = Rc::downgrade(self);
        new_button.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.after_unsaved(dialogs::new_block);
            }
        });
        // Developer aid (tests): IQAL_GUI_CLICK=new-block presses the button
        // as a user would, through its click handler.
        if std::env::var("IQAL_GUI_CLICK").as_deref() == Ok("new-block") {
            let b = new_button.clone();
            glib::timeout_add_local_once(std::time::Duration::from_secs(1), move || b.emit_clicked());
        }
        let menu = gtk::MenuButton::new();
        let menu_face = hbox(6);
        menu_face.append(&gtk::Image::from_icon_name("open-menu-symbolic"));
        menu_face.append(&gtk::Label::new(Some(&t("menu"))));
        menu.set_child(Some(&menu_face));
        menu.set_popover(Some(&self.menu_popover()));
        // Developer aid (screenshots): IQAL_GUI_DIALOG=menu opens the menu.
        if std::env::var("IQAL_GUI_DIALOG").as_deref() == Ok("menu") {
            let m = menu.clone();
            glib::timeout_add_local_once(std::time::Duration::from_secs(1), move || m.popup());
        }
        buttons.append(&new_button);
        buttons.append(&menu);
        side.append(&buttons);
        self.sidebar.add_css_class("blocks");
        let scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&self.sidebar)
            .build();
        scroll.add_css_class("blocks");
        side.append(&scroll);

        let weak = Rc::downgrade(self);
        self.sidebar.connect_row_selected(move |_, row| {
            let Some(ui) = weak.upgrade() else { return };
            if ui.rebuilding.get() {
                return;
            }
            let id = row.and_then(|r| ui.state.borrow().sets.get(r.index() as usize).map(|s| s.set.id));
            if id == ui.state.borrow().selected {
                return;
            }
            // Not from inside the signal: both lists are rebuilt. The current
            // block stays selected until unsaved changes are saved or
            // discarded.
            glib::idle_add_local_once(move || {
                if ui.is_dirty() {
                    ui.rebuild_sidebar();
                }
                ui.after_unsaved(move |ui| ui.select(id));
            });
        });

        let main = hbox(0);
        main.set_vexpand(true);
        main.append(&side);
        main.append(&self.content);
        let root = vbox(0);
        root.append(&main);
        // The service's state at the bottom, as on Windows.
        self.status_line.add_css_class("statusline");
        self.status_line.set_xalign(0.0);
        self.status_line.set_ellipsize(gtk::pango::EllipsizeMode::End);
        root.append(&self.status_line);
        self.window.set_child(Some(&root));

        let weak = Rc::downgrade(self);
        self.window.connect_close_request(move |w| {
            let Some(ui) = weak.upgrade() else { return glib::Propagation::Proceed };
            if !ui.is_dirty() {
                return glib::Propagation::Proceed;
            }
            let w = w.clone();
            ui.after_unsaved(move |_| w.destroy());
            glib::Propagation::Stop
        });

        // Ctrl+S saves.
        let keys = gtk::ShortcutController::new();
        keys.set_scope(gtk::ShortcutScope::Global);
        let weak = Rc::downgrade(self);
        keys.add_shortcut(gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string("<Control>s"),
            Some(gtk::CallbackAction::new(move |_, _| {
                if let Some(ui) = weak.upgrade() {
                    ui.save();
                }
                glib::Propagation::Stop
            })),
        ));
        self.window.add_controller(keys);
    }

    fn menu_popover(self: &Rc<Self>) -> gtk::Popover {
        let pop = gtk::Popover::new();
        pop.set_has_arrow(false);
        let b = vbox(2);
        type Action = fn(&Rc<Ui>);
        let items: [(&str, Action); 6] = [
            ("menu-import", dialogs::import_blocks),
            ("menu-export-all", dialogs::export_all),
            ("menu-activity", dialogs::activity),
            ("menu-stats", crate::stats::statistics),
            ("menu-settings", dialogs::settings),
            ("menu-about", dialogs::about),
        ];
        for (key, action) in items {
            let button = gtk::Button::with_label(&t(key));
            button.add_css_class("flat");
            if let Some(l) = button.child().and_downcast::<gtk::Label>() {
                l.set_xalign(0.0);
            }
            let weak = Rc::downgrade(self);
            let popover = pop.downgrade();
            button.connect_clicked(move |_| {
                if let Some(p) = popover.upgrade() {
                    p.popdown();
                }
                if let Some(ui) = weak.upgrade() {
                    action(&ui);
                }
            });
            b.append(&button);
        }
        pop.set_child(Some(&b));
        pop
    }

    /// Developer aid (screenshots, testing): IQAL_GUI_DIALOG=lock|map|new|settings|activity.
    fn open_requested_dialog(self: &Rc<Self>) {
        match std::env::var("IQAL_GUI_DIALOG").as_deref() {
            Ok("lock") => crate::lockui::lock_dialog(self),
            Ok("new") => dialogs::new_block(self),
            Ok("settings") => dialogs::settings(self),
            Ok("activity") => dialogs::activity(self),
            Ok("stats") => crate::stats::statistics(self),
            Ok("about") => dialogs::about(self),
            Ok("picker") => crate::appspicker::pick(self, |_| {}),
            _ => {}
        }
    }

    pub fn weak(self: &Rc<Self>) -> Weak<Ui> {
        Rc::downgrade(self)
    }

    /// Reloads from the service. The editor is rebuilt only if what it
    /// shows changed (or when `force`), and never over unsaved changes.
    pub fn refresh(self: &Rc<Self>, force: bool) {
        let result = self.load(force);
        {
            let mut st = self.state.borrow_mut();
            match result {
                Ok((sets, status)) => {
                    if let Some(sets) = sets {
                        st.sets = sets;
                    }
                    st.status = Some(status);
                    st.error = None;
                }
                Err(e) => {
                    self.loaded.set(None);
                    st.sets.clear();
                    st.status = None;
                    st.error = Some(e);
                }
            }
            if st.selected.is_some_and(|id| !st.sets.iter().any(|s| s.set.id == id)) {
                st.selected = None;
                st.draft = None;
            }
            if st.selected.is_none() {
                st.selected = st.sets.first().map(|s| s.set.id);
            }
            let latest = st.selected.and_then(|id| st.sets.iter().find(|s| s.set.id == id)).map(|i| i.set.clone());
            match (&mut st.draft, latest) {
                (Some(d), Some(base)) if d.dirty() => d.base = base,
                (draft, _) => *draft = None,
            }
        }
        self.update_status_line();
        self.rebuild_sidebar();
        if self.is_dirty() {
            self.update_dirty();
            return;
        }
        let current = self.current().map(|i| (i.set, i.locked));
        let changed = force || self.state.borrow().shown != current;
        if changed {
            self.rebuild_content();
        }
    }

    /// The service's version and time source at the bottom, as on Windows.
    fn update_status_line(&self) {
        let text = match &self.state.borrow().status {
            Some(info) => {
                let mut parts = vec![
                    ta("status-service", &[("version", info.version.clone().into())]),
                    ta("status-time", &[("source", iqal_app::describe::time_source(&info.clock).into())]),
                ];
                if !info.admin {
                    parts.push(t("status-not-admin"));
                }
                parts.join("   ·   ")
            }
            None => String::new(),
        };
        if self.status_line.text() != text {
            self.status_line.set_text(&text);
        }
    }

    /// Asks the service what changed. The blocks with all their rules are
    /// fetched only when one changed (or `full`); otherwise just their
    /// states are updated, which stays cheap with long lists.
    fn load(&self, full: bool) -> Result<(Option<Vec<SetInfo>>, Box<Status>), String> {
        let status = self.service.status()?;
        let (generation, states) = self.service.states()?;
        let key = (status.started_at, generation);
        {
            let mut st = self.state.borrow_mut();
            let same_blocks = st.sets.len() == states.len() && st.sets.iter().zip(&states).all(|(a, b)| a.set.id == b.id);
            if !full && self.loaded.get() == Some(key) && same_blocks {
                for (info, state) in st.sets.iter_mut().zip(states) {
                    info.locked = state.locked;
                    info.active = state.active;
                    info.used_today_secs = state.used_today_secs;
                    info.place_known = state.place_known;
                }
                return Ok((None, status));
            }
        }
        let sets = self.service.sets()?;
        self.loaded.set(Some(key));
        Ok((Some(sets), status))
    }

    /// Redraws the block list if what it shows changed, and selects the
    /// selected block in it.
    fn rebuild_sidebar(&self) {
        let dirty = self.is_dirty();
        let (rows, selected) = {
            let st = self.state.borrow();
            let rows: Vec<SidebarRow> = st
                .sets
                .iter()
                .map(|info| SidebarRow {
                    name: info.set.name.clone(),
                    unsaved: dirty && Some(info.set.id) == st.selected,
                    state: if !info.set.enabled {
                        "state-off"
                    } else if info.active {
                        "state-active"
                    } else {
                        "state-scheduled"
                    },
                    locked: info.locked,
                })
                .collect();
            (rows, st.sets.iter().position(|s| Some(s.set.id) == st.selected))
        };
        self.rebuilding.set(true);
        if self.sidebar_rows.borrow().as_ref() != Some(&rows) {
            while let Some(child) = self.sidebar.first_child() {
                self.sidebar.remove(&child);
            }
            for r in &rows {
                // The name, and below it gold labels: on, off or waiting,
                // and locked. The chosen one has a stripe (see theme).
                let row = vbox(4);
                row.set_margin_start(8);
                row.set_margin_end(8);
                row.set_margin_top(6);
                row.set_margin_bottom(6);
                let name = gtk::Label::new(Some(&if r.unsaved { format!("{} •", r.name) } else { r.name.clone() }));
                name.add_css_class("name");
                if r.unsaved {
                    name.set_tooltip_text(Some(&t("unsaved-changes")));
                }
                name.set_xalign(0.0);
                name.set_ellipsize(gtk::pango::EllipsizeMode::End);
                row.append(&name);
                let labels = hbox(6);
                let icon = match r.state {
                    "state-off" => "system-shutdown-symbolic",
                    "state-active" => "object-select-symbolic",
                    _ => "alarm-symbolic",
                };
                labels.append(&badge(icon, &t(r.state)));
                if r.locked {
                    labels.append(&badge("changes-prevent-symbolic", &t("state-locked")));
                }
                row.append(&labels);
                self.sidebar.append(&row);
            }
            *self.sidebar_rows.borrow_mut() = Some(rows);
        }
        let row = selected.and_then(|i| self.sidebar.row_at_index(i as i32));
        if self.sidebar.selected_row() != row {
            self.sidebar.select_row(row.as_ref());
        }
        self.rebuilding.set(false);
    }

    /// Rebuilds the editor, keeping the tab and, for the same block, how
    /// far each tab was scrolled.
    fn rebuild_content(self: &Rc<Self>) {
        let same_block = {
            let st = self.state.borrow();
            st.shown.as_ref().map(|(s, _)| s.id) == st.selected
        };
        let scrolled: Vec<f64> = match (same_block, self.notebook.borrow().as_ref()) {
            (true, Some(nb)) => (0..nb.n_pages())
                .map(|i| nb.nth_page(Some(i)).and_downcast::<gtk::ScrolledWindow>().map_or(0.0, |s| s.vadjustment().value()))
                .collect(),
            _ => Vec::new(),
        };
        self.replacing.set(true);
        // The window must never keep a removed button as its default (GTK
        // then crashes on the next change of default).
        self.window.set_default_widget(None::<&gtk::Widget>);
        while let Some(child) = self.content.first_child() {
            self.content.remove(&child);
        }
        *self.notebook.borrow_mut() = None;
        *self.save_bar.borrow_mut() = None;
        let (error, server) = {
            let st = self.state.borrow();
            (st.error.clone(), self.current_from(&st))
        };
        self.state.borrow_mut().shown = server.as_ref().map(|i| (i.set.clone(), i.locked));
        let widget: gtk::Widget = match (error, self.view()) {
            (Some(e), _) => {
                let (title, detail) = e.split_once('\n').unwrap_or((&e, ""));
                placeholder(title, detail).upcast()
            }
            (None, Some(info)) => editor::build(self, &info),
            (None, None) => placeholder(&t("empty-title"), &t("empty-detail")).upcast(),
        };
        widget.set_vexpand(true);
        self.content.append(&widget);
        // Enter in a one-line box presses Save (when it is shown and usable),
        // set only now that the button is in the window.
        if let Some((_, _, save)) = self.save_bar.borrow().as_ref() {
            self.window.set_default_widget(Some(save));
        }
        self.replacing.set(false);
        self.was_dirty.set(!self.is_dirty());
        self.update_dirty();

        if let Some(nb) = self.notebook.borrow().as_ref() {
            for (i, value) in scrolled.into_iter().enumerate() {
                if let Some(s) = nb.nth_page(Some(i as u32)).and_downcast::<gtk::ScrolledWindow>() {
                    // The page has no height until it is laid out.
                    glib::idle_add_local_once(move || s.vadjustment().set_value(value));
                }
            }
        }
    }

    /// Called by the editor for its tabs and save bar.
    pub fn editor_built(self: &Rc<Self>, notebook: &gtk::Notebook, save_bar: &gtk::Revealer, text: &gtk::Label, save: &gtk::Button) {
        notebook.set_current_page(Some(self.tab.get().min(notebook.n_pages().saturating_sub(1))));
        let weak = self.weak();
        notebook.connect_switch_page(move |nb, _, n| {
            // A notebook being taken down reports switches too; only the
            // one on screen counts.
            if let Some(ui) = weak.upgrade()
                && !ui.replacing.get()
                && ui.notebook.borrow().as_ref() == Some(nb)
            {
                ui.tab.set(n);
            }
        });
        *self.notebook.borrow_mut() = Some(notebook.clone());
        *self.save_bar.borrow_mut() = Some((save_bar.clone(), text.clone(), save.clone()));
    }

    fn current_from(&self, st: &State) -> Option<SetInfo> {
        st.selected.and_then(|id| st.sets.iter().find(|s| s.set.id == id).cloned())
    }

    /// The selected block as saved in the service.
    pub fn current(&self) -> Option<SetInfo> {
        self.current_from(&self.state.borrow())
    }

    /// The selected block as the editor shows it: with unsaved changes.
    pub fn view(&self) -> Option<SetInfo> {
        let st = self.state.borrow();
        let mut info = self.current_from(&st)?;
        if let Some(d) = &st.draft {
            let lock = info.set.lock.take();
            let pending = info.set.pending_unlock_at;
            info.set = d.set.clone();
            info.set.lock = lock;
            info.set.pending_unlock_at = pending;
        }
        Some(info)
    }

    /// The text of a rule list as the editor shows it.
    pub fn list_text(&self, key: ListKey) -> String {
        let st = self.state.borrow();
        match &st.draft {
            Some(d) => d.texts[key.index()].clone(),
            None => self.current_from(&st).map(|i| key.text_of(&i.set)).unwrap_or_default(),
        }
    }

    /// All blocks, as saved.
    pub fn blocks(&self) -> Vec<SetInfo> {
        self.state.borrow().sets.clone()
    }

    pub fn status(&self) -> Option<Box<Status>> {
        self.state.borrow().status.clone()
    }

    pub fn is_dirty(&self) -> bool {
        self.state.borrow().draft.as_ref().is_some_and(Draft::dirty)
    }

    pub fn select(self: &Rc<Self>, id: Option<SetId>) {
        if self.state.borrow().selected == id {
            return;
        }
        {
            let mut st = self.state.borrow_mut();
            st.selected = id;
            st.draft = None;
        }
        self.rebuild_sidebar();
        self.rebuild_content();
    }

    /// Runs `then` once there are no unsaved changes, asking first whether
    /// to save or discard them.
    pub fn after_unsaved(self: &Rc<Self>, then: impl Fn(&Rc<Ui>) + 'static) {
        if !self.is_dirty() {
            return then(self);
        }
        let name = self.view().map(|i| i.set.name).unwrap_or_default();
        let buttons = [t("action-cancel"), t("action-discard"), t("action-save")];
        let title = ta("confirm-unsaved", &[("name", name.into())]);
        let weak = self.weak();
        messages::show(&self.window, Some(Sign::Warning), &title, &t("confirm-unsaved-detail"), &buttons, move |r| {
            let Some(ui) = weak.upgrade() else { return };
            match r {
                Some(1) => {
                    ui.state.borrow_mut().draft = None;
                    ui.rebuild_sidebar();
                    ui.rebuild_content();
                    then(&ui);
                }
                Some(2) if ui.save() => then(&ui),
                _ => {}
            }
        });
    }

    fn with_draft<R>(&self, f: impl FnOnce(&mut Draft) -> R) -> Option<R> {
        let mut st = self.state.borrow_mut();
        if st.draft.is_none() {
            let base = self.current_from(&st)?.set;
            st.draft = Some(Draft::new(base));
        }
        st.draft.as_mut().map(f)
    }

    /// Changes the selected block's draft, without redrawing the editor:
    /// for controls that already show the new value.
    pub fn edit(self: &Rc<Self>, change: impl FnOnce(&mut BlockSet)) {
        self.with_draft(|d| change(&mut d.set));
        self.update_dirty();
    }

    /// Changes the draft and redraws the editor: for changes that show or
    /// hide parts of it.
    pub fn modify(self: &Rc<Self>, change: impl FnOnce(&mut BlockSet) -> Result<(), String>) {
        match self.with_draft(|d| change(&mut d.set)) {
            Some(Err(e)) => self.error(&e),
            Some(Ok(())) => self.rebuild_content(),
            None => {}
        }
    }

    /// Records the text of a rule list as typed.
    pub fn set_list_text(self: &Rc<Self>, key: ListKey, text: String) {
        self.with_draft(|d| d.texts[key.index()] = text);
        self.update_dirty();
    }

    /// Saves the draft. Returns whether everything is saved.
    pub fn save(self: &Rc<Self>) -> bool {
        if !self.is_dirty() {
            return true;
        }
        let finished = self.state.borrow().draft.as_ref().map(Draft::finish);
        match finished {
            Some(Ok(set)) => match self.service.save(set) {
                Ok(()) => {
                    self.state.borrow_mut().draft = None;
                    self.refresh(true);
                    true
                }
                Err(e) => {
                    self.error(&e);
                    false
                }
            },
            Some(Err(problems)) => {
                self.info(&t("save-has-errors"), &problems);
                false
            }
            None => true,
        }
    }

    pub fn discard(self: &Rc<Self>) {
        self.state.borrow_mut().draft = None;
        self.rebuild_sidebar();
        self.rebuild_content();
    }

    /// Changes the block and saves at once, with any other unsaved edits:
    /// used for locks, which take effect when confirmed. If saving fails
    /// the change is undone and the other edits are kept.
    pub fn commit(self: &Rc<Self>, change: impl FnOnce(&mut BlockSet)) -> bool {
        let Some(info) = self.current() else { return false };
        let mut set = match self.state.borrow().draft.as_ref().map(Draft::finish) {
            Some(Ok(set)) => set,
            Some(Err(problems)) => {
                self.info(&t("save-has-errors"), &problems);
                return false;
            }
            None => info.set,
        };
        change(&mut set);
        match self.service.save(set) {
            Ok(()) => {
                self.state.borrow_mut().draft = None;
                self.refresh(true);
                true
            }
            Err(e) => {
                self.error(&e);
                false
            }
        }
    }

    /// Shows or hides the save bar and the sidebar mark. On a locked block,
    /// Save is greyed while the change would weaken it, and the bar says why.
    fn update_dirty(&self) {
        let dirty = self.is_dirty();
        let weaker = if dirty {
            let st = self.state.borrow();
            let info = self.current_from(&st).filter(|i| i.locked);
            // Only for a locked block: reading the rule lists takes a moment.
            let set = info.as_ref().and(st.draft.as_ref()).and_then(|d| d.finish().ok());
            match (info, set) {
                (Some(info), Some(set)) => iqal_core::lock::weakening_reason(&info.set, &set).map(iqal_app::service::reason_text),
                _ => None,
            }
        } else {
            None
        };
        if let Some((bar, text, save)) = self.save_bar.borrow().as_ref() {
            bar.set_reveal_child(dirty);
            bar.set_visible(dirty);
            text.set_text(&match &weaker {
                Some(reason) => ta("unsaved-locked-weaker", &[("reason", reason.clone().into())]),
                None => t("unsaved-changes"),
            });
            save.set_sensitive(weaker.is_none());
        }
        if self.was_dirty.replace(dirty) != dirty {
            self.rebuild_sidebar();
        }
    }

    /// Says what went wrong, in a few words: the first line of `message`
    /// is the title, any further lines the explanation.
    pub fn error(&self, message: &str) {
        let (title, detail) = message.split_once('\n').unwrap_or((message, ""));
        messages::show(&self.window, Some(Sign::Error), title, detail, &[], |_| {});
    }

    pub fn info(&self, title: &str, detail: &str) {
        messages::show(&self.window, Some(Sign::Info), title, detail, &[], |_| {});
    }

    /// Asks before doing something hard to undo.
    pub fn confirm(&self, title: &str, detail: &str, ok: &str, on_ok: impl Fn() + 'static) {
        let buttons = [t("action-cancel"), ok.to_string()];
        messages::show(&self.window, Some(Sign::Warning), title, detail, &buttons, move |answer| {
            if answer == Some(1) {
                on_ok();
            }
        });
    }

    pub fn is_admin(&self) -> bool {
        self.state.borrow().status.as_ref().is_some_and(|s| s.admin)
    }

    /// The websites of a pre-made list, fetched once per version of it.
    pub fn list_rules(&self, list: &ListSummary) -> Rc<Vec<String>> {
        let version = (list.rules, list.updated_at);
        if let Some((v, rules)) = self.list_cache.borrow().get(&list.id)
            && *v == version
        {
            return rules.clone();
        }
        let rules = match self.service.call(Request::ListRules { id: list.id.clone() }) {
            Ok(Response::ListRules { rules }) => Rc::new(rules),
            _ => return Rc::default(),
        };
        self.list_cache.borrow_mut().insert(list.id.clone(), (version, rules.clone()));
        rules
    }
}

fn placeholder(title: &str, detail: &str) -> gtk::Box {
    let b = vbox(12);
    b.set_valign(gtk::Align::Center);
    b.set_halign(gtk::Align::Center);
    let icon = gtk::Image::from_icon_name("iqal");
    icon.set_pixel_size(80);
    b.append(&icon);
    let h = gtk::Label::new(Some(title));
    h.add_css_class("title-2");
    b.append(&h);
    let d = note(detail);
    d.set_justify(gtk::Justification::Center);
    d.set_max_width_chars(50);
    b.append(&d);
    b
}
