//! The main window: block list on one side, editor on the other.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use al_dir_core::{BlockSet, SetId};
use al_dir_i18n::{t, ta};
use al_dir_ipc::{SetInfo, Status};
use gtk::glib;
use gtk::prelude::*;

use crate::service::Service;
use crate::widgets::{hbox, note, set_margins, vbox};
use crate::{dialogs, editor};

pub struct Ui {
    pub window: gtk::ApplicationWindow,
    pub service: Service,
    sidebar: gtk::ListBox,
    content: gtk::Box,
    state: RefCell<State>,
    rebuilding: Cell<bool>,
}

#[derive(Default)]
struct State {
    sets: Vec<SetInfo>,
    selected: Option<SetId>,
    /// What the editor currently shows, to rebuild it only on change.
    shown: Option<(BlockSet, bool)>,
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
            .icon_name("al-dir")
            .build();

        let sidebar = gtk::ListBox::new();
        sidebar.add_css_class("navigation-sidebar");
        let content = vbox(0);
        content.set_hexpand(true);

        let ui = Rc::new(Ui {
            window,
            service: Service::new(),
            sidebar,
            content,
            state: RefCell::new(State::default()),
            rebuilding: Cell::new(false),
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
        let header = gtk::HeaderBar::new();
        let menu = gtk::MenuButton::builder().icon_name("open-menu-symbolic").tooltip_text(t("menu")).build();
        menu.set_popover(Some(&self.menu_popover()));
        header.pack_end(&menu);
        self.window.set_titlebar(Some(&header));

        let side = vbox(6);
        side.set_size_request(260, -1);
        let scroll = gtk::ScrolledWindow::builder().vexpand(true).child(&self.sidebar).build();
        side.append(&scroll);
        let new_button = gtk::Button::with_label(&t("action-new-block"));
        new_button.add_css_class("suggested-action");
        set_margins(&new_button, 8);
        let weak = Rc::downgrade(self);
        new_button.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                dialogs::new_block(&ui);
            }
        });
        side.append(&new_button);
        // Developer aid (tests): AL_DIR_GUI_CLICK=new-block presses the button
        // as a user would, through its click handler.
        if std::env::var("AL_DIR_GUI_CLICK").as_deref() == Ok("new-block") {
            let b = new_button.clone();
            glib::timeout_add_local_once(std::time::Duration::from_secs(1), move || b.emit_clicked());
        }

        let weak = Rc::downgrade(self);
        self.sidebar.connect_row_selected(move |_, row| {
            let Some(ui) = weak.upgrade() else { return };
            if ui.rebuilding.get() {
                return;
            }
            let id = row.and_then(|r| ui.state.borrow().sets.get(r.index() as usize).map(|s| s.set.id));
            ui.select(id);
        });

        let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
        paned.set_start_child(Some(&side));
        paned.set_end_child(Some(&self.content));
        paned.set_resize_start_child(false);
        paned.set_shrink_start_child(false);
        self.window.set_child(Some(&paned));
    }

    fn menu_popover(self: &Rc<Self>) -> gtk::Popover {
        let pop = gtk::Popover::new();
        let b = vbox(2);
        type Action = fn(&Rc<Ui>);
        let items: [(&str, Action); 6] = [
            ("menu-import", dialogs::import_blocks),
            ("menu-export-all", dialogs::export_all),
            ("menu-activity", dialogs::activity),
            ("menu-stats", dialogs::statistics),
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

    /// Developer aid (screenshots, testing): AL_DIR_GUI_DIALOG=lock|map|new|settings|activity.
    fn open_requested_dialog(self: &Rc<Self>) {
        match std::env::var("AL_DIR_GUI_DIALOG").as_deref() {
            Ok("lock") => crate::lockui::lock_dialog(self),
            Ok("map") => crate::mapdialog::pick(self.window.upcast_ref(), None, |_| {}),
            Ok("new") => dialogs::new_block(self),
            Ok("settings") => dialogs::settings(self),
            Ok("activity") => dialogs::activity(self),
            Ok("stats") => dialogs::statistics(self),
            _ => {}
        }
    }

    pub fn weak(self: &Rc<Self>) -> Weak<Ui> {
        Rc::downgrade(self)
    }

    /// Reloads from the service. The editor is rebuilt only if what it
    /// shows changed (or when `force`), so typing is not interrupted.
    pub fn refresh(self: &Rc<Self>, force: bool) {
        let result = self.service.sets().and_then(|sets| Ok((sets, self.service.status()?)));
        {
            let mut st = self.state.borrow_mut();
            match result {
                Ok((sets, status)) => {
                    st.sets = sets;
                    st.status = Some(status);
                    st.error = None;
                }
                Err(e) => {
                    st.sets.clear();
                    st.status = None;
                    st.error = Some(e);
                }
            }
            if st.selected.is_some_and(|id| !st.sets.iter().any(|s| s.set.id == id)) {
                st.selected = None;
            }
            if st.selected.is_none() {
                st.selected = st.sets.first().map(|s| s.set.id);
            }
        }
        self.rebuild_sidebar();
        let current = self.current().map(|i| (i.set, i.locked));
        let changed = force || self.state.borrow().shown != current;
        if changed {
            self.rebuild_content();
        }
    }

    fn rebuild_sidebar(&self) {
        self.rebuilding.set(true);
        while let Some(child) = self.sidebar.first_child() {
            self.sidebar.remove(&child);
        }
        let st = self.state.borrow();
        let mut selected_row = None;
        for (i, info) in st.sets.iter().enumerate() {
            let row = hbox(8);
            set_margins(&row, 8);
            let name = gtk::Label::new(Some(&info.set.name));
            name.set_xalign(0.0);
            name.set_hexpand(true);
            name.set_ellipsize(gtk::pango::EllipsizeMode::End);
            row.append(&name);
            let state = if !info.set.enabled {
                t("state-off")
            } else if info.active {
                t("state-active")
            } else {
                t("state-scheduled")
            };
            let badge = gtk::Label::new(Some(&state));
            badge.add_css_class("dim-label");
            badge.add_css_class("caption");
            row.append(&badge);
            if info.locked {
                let lock = gtk::Image::from_icon_name("changes-prevent-symbolic");
                lock.set_tooltip_text(Some(&t("state-locked")));
                row.append(&lock);
            }
            self.sidebar.append(&row);
            if Some(info.set.id) == st.selected {
                selected_row = self.sidebar.row_at_index(i as i32);
            }
        }
        self.sidebar.select_row(selected_row.as_ref());
        self.rebuilding.set(false);
    }

    fn rebuild_content(self: &Rc<Self>) {
        while let Some(child) = self.content.first_child() {
            self.content.remove(&child);
        }
        let (error, current) = {
            let st = self.state.borrow();
            (st.error.clone(), self.current_from(&st))
        };
        self.state.borrow_mut().shown = current.as_ref().map(|i| (i.set.clone(), i.locked));
        let widget: gtk::Widget = match (error, current) {
            (Some(e), _) => placeholder(&t("error-title"), &e).upcast(),
            (None, Some(info)) => editor::build(self, &info),
            (None, None) => placeholder(&t("empty-title"), &t("empty-detail")).upcast(),
        };
        widget.set_vexpand(true);
        self.content.append(&widget);
    }

    fn current_from(&self, st: &State) -> Option<SetInfo> {
        st.selected.and_then(|id| st.sets.iter().find(|s| s.set.id == id).cloned())
    }

    pub fn current(&self) -> Option<SetInfo> {
        self.current_from(&self.state.borrow())
    }

    pub fn status(&self) -> Option<Box<Status>> {
        self.state.borrow().status.clone()
    }

    pub fn select(self: &Rc<Self>, id: Option<SetId>) {
        if self.state.borrow().selected == id {
            return;
        }
        self.state.borrow_mut().selected = id;
        self.rebuild_content();
    }

    /// Changes the selected block and saves it. The service refuses
    /// changes that would weaken a locked block; that is shown as an error.
    pub fn modify(self: &Rc<Self>, change: impl FnOnce(&mut BlockSet) -> Result<(), String>) {
        let Some(info) = self.current() else { return };
        let mut set = info.set;
        let result = change(&mut set).and_then(|()| self.service.save(set));
        if let Err(e) = result {
            self.error(&e);
        }
        self.refresh(true);
    }

    pub fn error(&self, message: &str) {
        let d = gtk::AlertDialog::builder().modal(true).message(t("error-title")).detail(message).build();
        d.show(Some(&self.window));
    }

    pub fn info(&self, title: &str, detail: &str) {
        let d = gtk::AlertDialog::builder().modal(true).message(title).detail(detail).build();
        d.show(Some(&self.window));
    }

    /// Asks before doing something hard to undo.
    pub fn confirm(&self, title: &str, detail: &str, ok: &str, on_ok: impl Fn() + 'static) {
        let d = gtk::AlertDialog::builder()
            .modal(true)
            .message(title)
            .detail(detail)
            .buttons([t("action-cancel"), ok.to_string()])
            .cancel_button(0)
            .default_button(1)
            .build();
        d.choose(Some(&self.window), None::<&gtk::gio::Cancellable>, move |r| {
            if r == Ok(1) {
                on_ok();
            }
        });
    }

    pub fn is_admin(&self) -> bool {
        self.state.borrow().status.as_ref().is_some_and(|s| s.admin)
    }
}

fn placeholder(title: &str, detail: &str) -> gtk::Box {
    let b = vbox(12);
    b.set_valign(gtk::Align::Center);
    b.set_halign(gtk::Align::Center);
    let icon = gtk::Image::from_icon_name("security-high-symbolic");
    icon.set_pixel_size(64);
    icon.add_css_class("dim-label");
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

/// Shown in the sidebar footer and the editor: counts for a set.
pub fn summary(info: &SetInfo) -> String {
    let s = &info.set;
    ta(
        "block-summary",
        &[
            ("web", ((s.web.block.len() + s.web.allow.len()) as i64).into()),
            ("apps", ((s.apps.block.len() + s.apps.allow.len()) as i64).into()),
        ],
    )
}
