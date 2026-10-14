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

/// Separates a pre-made list's rule from the list's name in the list model.
const FROM_LIST: char = '\u{1f}';

pub fn rule_editor(ui: &Rc<Ui>, key: ListKey, add_title: &str, list_title: &str, examples: &str) -> RuleEditor {
    rule_editor_with_lists(ui, key, add_title, list_title, examples, &[])
}

/// Like [`rule_editor`], with the rules of pre-made lists shown after the
/// block's own, marked with the list's name. They cannot be removed one by
/// one: the list is switched off instead.
pub fn rule_editor_with_lists(
    ui: &Rc<Ui>,
    key: ListKey,
    add_title: &str,
    list_title: &str,
    examples: &str,
    lists: &[(String, Rc<Vec<String>>)],
) -> RuleEditor {
    // Side by side, half the width each.
    let columns = hbox(20);
    columns.set_homogeneous(true);

    // The saved list, then the pre-made lists' rules.
    let items: Vec<String> = ui.list_text(key).lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
    let model = gtk::StringList::new(&items.iter().map(String::as_str).collect::<Vec<_>>());
    // Each website once: the block's own rule, or the first list with it.
    let mut seen: std::collections::HashSet<&str> = items.iter().map(String::as_str).collect();
    let listed: Vec<String> = lists
        .iter()
        .flat_map(|(name, rules)| rules.iter().map(move |r| (name, r)))
        .filter(|(_, r)| seen.insert(r.as_str()))
        .map(|(name, r)| format!("{r}{FROM_LIST}{name}"))
        .collect();
    let from_lists = listed.len();
    let parts = gtk::gio::ListStore::new::<gtk::StringList>();
    parts.append(&model);
    parts.append(&gtk::StringList::new(&listed.iter().map(String::as_str).collect::<Vec<_>>()));
    let combined = gtk::FlattenListModel::new(Some(parts));
    let filter = gtk::StringFilter::new(Some(gtk::PropertyExpression::new(
        gtk::StringObject::static_type(),
        None::<gtk::Expression>,
        "string",
    )));
    filter.set_ignore_case(true);
    filter.set_match_mode(gtk::StringFilterMatchMode::Substring);
    let filtered = gtk::FilterListModel::new(Some(combined), Some(filter.clone()));
    let selection = gtk::MultiSelection::new(Some(filtered.clone()));
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let row = hbox(8);
        row.set_margin_start(8);
        row.set_margin_end(8);
        let l = gtk::Label::new(None);
        l.set_xalign(0.0);
        l.set_hexpand(true);
        l.add_css_class("monospace");
        l.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let source = gtk::Label::new(None);
        source.add_css_class("dim-label");
        source.add_css_class("caption");
        row.append(&l);
        row.append(&source);
        if let Some(item) = item.downcast_ref::<gtk::ListItem>() {
            item.set_child(Some(&row));
        }
    });
    factory.connect_bind(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else { return };
        let (Some(row), Some(s)) = (item.child(), item.item().and_downcast::<gtk::StringObject>()) else { return };
        let (Some(l), Some(source)) =
            (row.first_child().and_downcast::<gtk::Label>(), row.last_child().and_downcast::<gtk::Label>())
        else {
            return;
        };
        let text = s.string();
        let (rule, list) = text.split_once(FROM_LIST).unwrap_or((&text, ""));
        l.set_text(rule);
        l.set_sensitive(list.is_empty());
        source.set_text(list);
        source.set_visible(!list.is_empty());
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
            // With pre-made lists, the total; each of their rows names its list.
            let n = model.n_items() as usize + from_lists;
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
        let (remove, selection, filtered) = (remove.clone(), selection.clone(), filtered.clone());
        // The block's own rules come first: if the first one selected is from
        // a pre-made list, all are, and there is nothing to remove.
        move |_, _, _| {
            let chosen = selection.selection();
            let own = !chosen.is_empty()
                && filtered
                    .item(chosen.minimum())
                    .and_downcast::<gtk::StringObject>()
                    .is_some_and(|s| !s.string().contains(FROM_LIST));
            remove.set_sensitive(own);
        }
    });
    remove.connect_clicked({
        let (model, filtered, selection, changed) = (model.clone(), filtered.clone(), selection.clone(), changed.clone());
        move |_| {
            let chosen: std::collections::HashSet<String> = (0..filtered.n_items())
                .filter(|&i| selection.is_selected(i))
                .filter_map(|i| filtered.item(i).and_downcast::<gtk::StringObject>())
                .map(|s| s.string().to_string())
                .filter(|s| !s.contains(FROM_LIST))
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
