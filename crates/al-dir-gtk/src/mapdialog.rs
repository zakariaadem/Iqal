//! Choosing an area on a map (OpenStreetMap through libshumate): click to
//! place the centre, set the radius.

use std::cell::Cell;
use std::rc::Rc;

use al_dir_core::GeoArea;
use al_dir_i18n::t;
use gtk::prelude::*;
use libshumate::prelude::*;

use crate::widgets::{hbox, label, note, set_margins, vbox};

const EARTH_RADIUS_M: f64 = 6_371_008.8;

pub fn pick(parent: &gtk::Window, current: Option<GeoArea>, on_done: impl Fn(GeoArea) + 'static) {
    let win = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(t("map-title"))
        .default_width(820)
        .default_height(640)
        .build();
    let body = vbox(10);
    set_margins(&body, 12);
    body.append(&note(&t("map-help")));

    let simple = libshumate::SimpleMap::new();
    simple.set_vexpand(true);
    let registry = libshumate::MapSourceRegistry::with_defaults();
    if let Some(source) = registry.by_id(libshumate::MAP_SOURCE_OSM_MAPNIK) {
        simple.set_map_source(Some(&source));
    }
    let viewport = simple.viewport().expect("map has a viewport");
    let map = simple.map().expect("simple map has a map");

    let path = libshumate::PathLayer::new(&viewport);
    path.set_fill(true);
    path.set_fill_color(Some(&gtk::gdk::RGBA::new(0.12, 0.48, 0.36, 0.25)));
    path.set_stroke_color(Some(&gtk::gdk::RGBA::new(0.12, 0.48, 0.36, 0.9)));
    map.add_layer(&path);
    let markers = libshumate::MarkerLayer::new(&viewport);
    map.add_layer(&markers);
    let marker = libshumate::Marker::new();
    let pin = gtk::Image::from_icon_name("mark-location-symbolic");
    pin.set_pixel_size(32);
    marker.set_child(Some(&pin));

    let center: Rc<Cell<Option<(f64, f64)>>> = Rc::new(Cell::new(None));
    let radius = gtk::SpinButton::with_range(50.0, 20_000.0, 50.0);
    radius.set_value(300.0);

    let redraw = {
        let (center, path, markers, marker, radius) =
            (center.clone(), path.clone(), markers.clone(), marker.clone(), radius.clone());
        Rc::new(move || {
            path.remove_all();
            let Some((lat, lon)) = center.get() else { return };
            marker.set_location(lat, lon);
            if marker.parent().is_none() {
                markers.add_marker(&marker);
            }
            let r = radius.value();
            for i in 0..=72 {
                let a = (i as f64) * std::f64::consts::TAU / 72.0;
                let dlat = (r / EARTH_RADIUS_M) * a.cos();
                let dlon = (r / EARTH_RADIUS_M) * a.sin() / lat.to_radians().cos().max(0.01);
                path.add_node(&libshumate::Coordinate::new_full(lat + dlat.to_degrees(), lon + dlon.to_degrees()));
            }
        })
    };

    if let Some(GeoArea::Circle { lat, lon, radius_m }) = current {
        center.set(Some((lat, lon)));
        radius.set_value(radius_m);
        viewport.set_zoom_level(15.0);
        map.center_on(lat, lon);
    } else {
        viewport.set_zoom_level(3.0);
        map.center_on(25.0, 20.0);
    }
    redraw();

    let click = gtk::GestureClick::new();
    {
        let (center, redraw, viewport, map) = (center.clone(), redraw.clone(), viewport.clone(), map.clone());
        click.connect_released(move |_, _, x, y| {
            let (lat, lon) = viewport.widget_coords_to_location(&map, x, y);
            center.set(Some((lat, lon)));
            redraw();
        });
    }
    map.add_controller(click);
    {
        let redraw = redraw.clone();
        radius.connect_value_changed(move |_| redraw());
    }
    body.append(&simple);

    let row = hbox(8);
    row.append(&label(&t("map-radius")));
    row.append(&radius);
    row.append(&label(&t("unit-metres")));
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    row.append(&spacer);
    let cancel = gtk::Button::with_label(&t("action-cancel"));
    let ok = gtk::Button::with_label(&t("action-use-area"));
    ok.add_css_class("suggested-action");
    row.append(&cancel);
    row.append(&ok);
    body.append(&row);
    win.set_child(Some(&body));

    {
        let win = win.clone();
        cancel.connect_clicked(move |_| win.close());
    }
    {
        let win = win.clone();
        ok.connect_clicked(move |_| {
            if let Some((lat, lon)) = center.get() {
                on_done(GeoArea::Circle { lat, lon, radius_m: radius.value() });
                win.close();
            }
        });
    }
    win.present();
}
