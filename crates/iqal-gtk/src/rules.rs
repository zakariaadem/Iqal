//! Editing a rule list: a box to type or paste new rules with an Add
//! button, next to the saved list, which can be searched and from which
//! selected rules can be removed. Long lists stay quick: only the rows on
//! screen are drawn.

use std::rc::Rc;

use iqal_i18n::{t, ta};
use gtk::prelude::*;

use crate::ui::{ListKey, Ui};
use crate::widgets::{heading, hbox, label, note, vbox};

/// Height of the add box and of the list beside it.
const BOX_HEIGHT: i32 = 220;

/// The editor for one list, with a place for more "add" buttons and a
/// function that adds rules from elsewhere (a file, the app picker).
pub struct RuleEditor {
    pub widget: gtk::Widget,
    /// Under the Add button: for "Import from a file…" and similar.
    pub actions: gtk::Box,
    /// Adds rules to the list; returns how many were new. Rules that cannot
    /// be used are put in the add box, marked, for fixing.
    pub add: Rc<dyn Fn(Vec<String>) -> usize>,
}

pub fn rule_editor(ui: &Rc<Ui>, key: ListKey, add_title: &str, list_title: &str, examples: &str) -> RuleEditor {
    // Side by side, half the width each.
    let columns = hbox(20);
    columns.set_homogeneous(true);

    // The saved list.
    let items: Vec<String> = ui.list_text(key).lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
    let model = gtk::StringList::new(&items.iter().map(String::as_str).collect::<Vec<_>>());
    let filter = gtk::StringFilter::new(Some(gtk::PropertyExpression::new(
        gtk::StringObject::static_type(),
        None::<gtk::Expression>,
        "string",
    )));
    filter.set_ignore_case(true);
    filter.set_match_mode(gtk::StringFilterMatchMode::Substring);
    let filtered = gtk::FilterListModel::new(Some(model.clone()), Some(filter.clone()));
    let selection = gtk::MultiSelection::new(Some(filtered.clone()));
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let l = gtk::Label::new(None);
        l.set_xalign(0.0);
        l.add_css_class("monospace");
        l.set_ellipsize(gtk::pango::EllipsizeMode::End);
        l.set_margin_start(8);
        l.set_margin_end(8);
        if let Some(item) = item.downcast_ref::<gtk::ListItem>() {
            item.set_child(Some(&l));
        }
    });
    factory.connect_bind(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else { return };
        if let (Some(l), Some(s)) = (item.child().and_downcast::<gtk::Label>(), item.item().and_downcast::<gtk::StringObject>()) {
            l.set_text(&s.string());
        }
    });
    let list = gtk::ListView::new(Some(selection.clone()), Some(factory));
    // Addresses and program names read left to right in every language.
    list.set_direction(gtk::TextDirection::Ltr);
    let empty = note(&t("list-empty"));
    empty.set_halign(gtk::Align::Center);
    empty.set_wrap(false);
    let stack = gtk::Stack::new();
    stack.add_named(&gtk::ScrolledWindow::builder().child(&list).vexpand(true).build(), Some("list"));
    stack.add_named(&empty, Some("empty"));
    let search = gtk::SearchEntry::builder().placeholder_text(t("search")).build();
    let frame = vbox(0);
    frame.add_css_class("frame");
    frame.append(&search);
    frame.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    frame.append(&stack);
    frame.set_size_request(-1, BOX_HEIGHT);
    // Exactly as tall as the add box, not stretched to fill the column.
    frame.set_vexpand(false);

    let list_heading = heading(list_title);
    let remove = gtk::Button::with_label(&t("action-remove-selected"));
    remove.set_halign(gtk::Align::Start);
    remove.set_sensitive(false);
    let right = vbox(6);
    right.append(&list_heading);
    right.append(&frame);
    right.append(&remove);

    // Counts, the empty state, and the draft follow the list.
    let refresh = {
        let (model, stack, list_heading, title) = (model.clone(), stack.clone(), list_heading.clone(), list_title.to_string());
        move || {
            let n = model.n_items();
            list_heading.set_text(&format!("{title} · {n}"));
            stack.set_visible_child_name(if n == 0 { "empty" } else { "list" });
        }
    };
    refresh();
    let all = {
        let model = model.clone();
        move || -> Vec<String> { (0..model.n_items()).filter_map(|i| model.string(i)).map(|s| s.to_string()).collect() }
    };
    let changed = {
        let (weak, all, refresh) = (ui.weak(), all.clone(), refresh.clone());
        Rc::new(move || {
            refresh();
            if let Some(ui) = weak.upgrade() {
                ui.set_list_text(key, all().join("\n"));
            }
        })
    };
    search.connect_search_changed({
        let filter = filter.clone();
        move |s| filter.set_search(Some(&s.text()))
    });
    selection.connect_selection_changed({
        let (remove, selection) = (remove.clone(), selection.clone());
        move |_, _, _| remove.set_sensitive(!selection.selection().is_empty())
    });
    remove.connect_clicked({
        let (model, filtered, selection, changed) = (model.clone(), filtered.clone(), selection.clone(), changed.clone());
        move |_| {
            let chosen: std::collections::HashSet<String> = (0..filtered.n_items())
                .filter(|&i| selection.is_selected(i))
                .filter_map(|i| filtered.item(i).and_downcast::<gtk::StringObject>())
                .map(|s| s.string().to_string())
                .collect();
            if chosen.is_empty() {
                return;
            }
            let kept: Vec<String> =
                (0..model.n_items()).filter_map(|i| model.string(i)).map(|s| s.to_string()).filter(|s| !chosen.contains(s)).collect();
            model.splice(0, model.n_items(), &kept.iter().map(String::as_str).collect::<Vec<_>>());
            changed();
        }
    });

    // The add box.
    let buffer = gtk::TextBuffer::new(None);
    let invalid = gtk::TextTag::builder().name("invalid").underline(gtk::pango::Underline::Error).build();
    buffer.tag_table().add(&invalid);
    let input = gtk::TextView::builder().buffer(&buffer).monospace(true).accepts_tab(false).top_margin(8).bottom_margin(8).left_margin(10).right_margin(10).build();
    input.set_direction(gtk::TextDirection::Ltr);
    let input_scroll = gtk::ScrolledWindow::builder().child(&input).build();
    input_scroll.add_css_class("frame");
    input_scroll.set_size_request(-1, BOX_HEIGHT);
    // The examples show inside the empty box, so both boxes line up.
    let hint = note(&ta("list-empty-hint", &[("examples", examples.to_string().into())]));
    hint.set_valign(gtk::Align::Start);
    hint.set_margin_top(8);
    hint.set_margin_start(12);
    hint.set_margin_end(12);
    hint.set_can_target(false);
    let input_box = gtk::Overlay::new();
    input_box.set_child(Some(&input_scroll));
    input_box.add_overlay(&hint);
    let problem = label("");
    problem.add_css_class("error");
    problem.set_visible(false);
    problem.set_max_width_chars(30);
    let add_button = gtk::Button::with_label(&t("action-add"));
    add_button.add_css_class("suggested-action");
    let actions = hbox(6);
    actions.append(&add_button);
    let left = vbox(6);
    left.append(&heading(add_title));
    left.append(&input_box);
    left.append(&actions);
    left.append(&problem);

    let add: Rc<dyn Fn(Vec<String>) -> usize> = {
        let (model, all, changed, buffer, problem, invalid) =
            (model.clone(), all.clone(), changed.clone(), buffer.clone(), problem.clone(), invalid.clone());
        Rc::new(move |lines: Vec<String>| {
            let mut present: std::collections::HashSet<String> = all().into_iter().collect();
            let (mut fresh, mut bad) = (Vec::new(), Vec::new());
            for line in lines.iter().map(|l| l.trim()).filter(|l| !l.is_empty()) {
                match key.check(line) {
                    Ok(()) if present.insert(line.to_string()) => fresh.push(line.to_string()),
                    Ok(()) => {}
                    Err(e) => bad.push((line.to_string(), e)),
                }
            }
            if !fresh.is_empty() {
                model.splice(model.n_items(), 0, &fresh.iter().map(String::as_str).collect::<Vec<_>>());
                changed();
            }
            // What could not be used stays in the add box, marked.
            if !bad.is_empty() {
                let text: Vec<&str> = bad.iter().map(|(l, _)| l.as_str()).collect();
                buffer.set_text(&text.join("\n"));
                buffer.apply_tag(&invalid, &buffer.start_iter(), &buffer.end_iter());
                let (line, why) = &bad[0];
                let mut message = ta("rule-invalid", &[("rule", line.clone().into()), ("problem", why.clone().into())]);
                if bad.len() > 1 {
                    message.push(' ');
                    message.push_str(&ta("lines-invalid-more", &[("count", (bad.len() as i64 - 1).into())]));
                }
                problem.set_text(&message);
                problem.set_visible(true);
            }
            fresh.len()
        })
    };
    add_button.connect_clicked({
        let (buffer, add, problem) = (buffer.clone(), add.clone(), problem.clone());
        move |_| {
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
            buffer.set_text("");
            problem.set_visible(false);
            add(text.lines().map(String::from).collect());
        }
    });
    // Typing again clears the marks; the examples show while it is empty.
    buffer.connect_changed({
        let problem = problem.clone();
        move |b| {
            hint.set_visible(b.char_count() == 0);
            if problem.is_visible() && b.char_count() == 0 {
                problem.set_visible(false);
            }
        }
    });

    columns.append(&left);
    columns.append(&right);
    RuleEditor { widget: columns.upcast(), actions, add }
}
