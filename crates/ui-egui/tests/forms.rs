//! Filling in a form in the real shell (egui_kittest): typing, Tab, check boxes, radios, choices.

use egui_kittest::Harness;
use printcraft_ui_egui::PrintCraftApp;

fn harness() -> Harness<'static, PrintCraftApp> {
    harness_bytes(include_bytes!("data/form.pdf").to_vec())
}

fn harness_bytes(bytes: Vec<u8>) -> Harness<'static, PrintCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PrintCraftApp::new();
        app.open_bytes("form.pdf", None, bytes).unwrap();
        app.set_option("left", "closed").unwrap();
        // The whole 300×400 pt page on screen.
        app.set_option("zoom", "150").unwrap();
        app
    });
    for _ in 0..60 {
        h.run_steps(2);
        if !h.state().render_pending() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    h
}

const PRINTED_SQUARE: [f64; 4] = [24.0, 18.0, 42.0, 36.0];

/// A synthetic printed square, detected from page content rather than a seeded cache.
fn printed_form() -> Harness<'static, PrintCraftApp> {
    use printcraft_cos::{Document, Object, SaveOptions, Stream, write_incremental};
    let mut doc = Document::open(std::sync::Arc::new(include_bytes!("data/form.pdf").to_vec())).unwrap();
    let page = printcraft_model::pages(&doc)[0].obj;
    let stream = doc.add(Object::Stream(Stream::flate(Default::default(), b"24 18 18 18 re S")));
    doc.update_dict(page, |d| d.set(b"Contents".to_vec(), Object::Ref(stream))).unwrap();
    harness_bytes(write_incremental(&doc, &SaveOptions::default()).unwrap())
}

fn square_center(h: &Harness<'static, PrintCraftApp>, rect: [f64; 4]) -> egui::Pos2 {
    let s = h.state();
    let doc = s.session.get(s.views[0].id).unwrap();
    printcraft_ui_egui::forms_ui::square_screen_rect(&s.views[0], &doc.info, 0, rect).unwrap().center()
}

fn click_square(h: &mut Harness<'static, PrintCraftApp>) {
    let p = square_center(h, PRINTED_SQUARE);
    h.hover_at(p);
    h.run_steps(1);
    h.drag_at(p);
    h.run_steps(1);
    h.drop_at(p);
    h.run_steps(4);
}

fn value(h: &Harness<'static, PrintCraftApp>, name: &str) -> Vec<String> {
    let s = h.state();
    s.session.get(s.views[0].id).unwrap().form.iter().find(|f| f.name == name).unwrap().value.clone()
}

/// Hover the centre of a field's widget (without clicking).
fn hover_field(h: &mut Harness<'static, PrintCraftApp>, name: &str, widget: usize) {
    let p = {
        let s = h.state();
        let doc = s.session.get(s.views[0].id).unwrap();
        let f = doc.form.iter().find(|f| f.name == name).unwrap();
        printcraft_ui_egui::forms_ui::field_screen_rect(&s.views[0], &doc.info, f, widget).expect("on screen").center()
    };
    h.hover_at(p);
    h.run_steps(4);
}

/// Click the centre of a field's widget.
fn click_field(h: &mut Harness<'static, PrintCraftApp>, name: &str, widget: usize) {
    let p = {
        let s = h.state();
        let doc = s.session.get(s.views[0].id).unwrap();
        let f = doc.form.iter().find(|f| f.name == name).unwrap();
        printcraft_ui_egui::forms_ui::field_screen_rect(&s.views[0], &doc.info, f, widget).expect("on screen").center()
    };
    h.hover_at(p);
    h.run_steps(1);
    h.drag_at(p);
    h.run_steps(1);
    h.drop_at(p);
    h.run_steps(3);
}

#[test]
fn typing_into_a_text_field_and_tabbing_on() {
    let mut h = harness();
    click_field(&mut h, "name", 0);
    assert!(h.state().views[0].forms.focus.is_some(), "the editor opened");
    h.event(egui::Event::Text("Ada Lovelace".into()));
    h.run_steps(2);
    h.key_press(egui::Key::Tab);
    h.run_steps(4);
    assert_eq!(value(&h, "name"), ["Ada Lovelace"]);
    assert_eq!(
        h.state().views[0].forms.focus.as_ref().map(|f| f.name.as_str()),
        Some("country"),
        "Tab moves to the next field that takes typing or a choice"
    );
    // The country list is open; Escape closes it. Escape also abandons a draft.
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    click_field(&mut h, "city", 0);
    assert_eq!(h.state().views[0].forms.focus.as_ref().map(|f| f.name.as_str()), Some("city"));
    h.event(egui::Event::Text("Paris".into()));
    h.run_steps(1);
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(value(&h, "city").is_empty());
    assert!(h.state().views[0].forms.focus.is_none());
}

#[test]
fn check_boxes_radios_and_choices() {
    let mut h = harness();
    click_field(&mut h, "agree", 0);
    assert_eq!(value(&h, "agree"), ["Yes"]);
    click_field(&mut h, "agree", 0);
    assert!(value(&h, "agree").is_empty());
    click_field(&mut h, "size", 1);
    assert_eq!(value(&h, "size"), ["L"]);
    click_field(&mut h, "size", 0);
    assert_eq!(value(&h, "size"), ["S"]);
    click_field(&mut h, "country", 0);
    egui_kittest::kittest::Queryable::get_by_label(&h, "France").click();
    h.run_steps(4);
    assert_eq!(value(&h, "country"), ["fr"]);
    // Clear form takes everything back; undo restores it.
    assert!(h.state_mut().execute("form.clear"));
    h.run_steps(2);
    assert!(value(&h, "country").is_empty() && value(&h, "size").is_empty());
    h.state_mut().undo();
    h.run_steps(2);
    assert_eq!(value(&h, "country"), ["fr"]);
}

#[test]
fn tabbing_into_a_filled_field_selects_it_so_typing_replaces() {
    let mut h = harness();
    click_field(&mut h, "country", 0);
    egui_kittest::kittest::Queryable::get_by_label(&h, "Canada").click();
    h.run_steps(3);
    click_field(&mut h, "city", 0);
    h.event(egui::Event::Text("Paris".into()));
    h.run_steps(1);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert_eq!(value(&h, "city"), ["Paris"]);
    // name → (Tab) country → (Tab) city, whose text is selected on entry.
    click_field(&mut h, "name", 0);
    h.key_press(egui::Key::Tab);
    h.run_steps(3);
    h.key_press(egui::Key::Tab);
    h.run_steps(4);
    assert_eq!(h.state().views[0].forms.focus.as_ref().map(|f| f.name.as_str()), Some("city"));
    h.event(egui::Event::Text("Lyon".into()));
    h.run_steps(1);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert_eq!(value(&h, "city"), ["Lyon"]);
}

#[test]
fn date_fields_offer_a_calendar() {
    use egui_kittest::kittest::Queryable;
    let mut h = harness();
    h.state_mut().apply_edit(printcraft_engine::Edit::AddField {
        page: 0,
        rect: [50.0, 40.0, 200.0, 60.0],
        kind: printcraft_engine::NewField::Date,
        name: Some("due".into()),
    });
    h.run_steps(4);
    click_field(&mut h, "due", 0);
    let (y, m, _) = h.state().session.today();
    let month =
        ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"][(m - 1) as usize];
    h.get_by_label(&format!("{month} {y}"));
    h.get_by_label("›").click();
    h.run_steps(2);
    h.get_by_label("15").click();
    h.run_steps(4);
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    assert_eq!(value(&h, "due"), vec![format!("{nm:02}/15/{ny}")], "picked in the field's format");
}

#[test]
fn toggle_popup_stays_open_across_the_gap() {
    use egui_kittest::kittest::Queryable;
    let mut h = harness();
    hover_field(&mut h, "agree", 0);
    let (field, popup) = {
        let s = h.state();
        let doc = s.session.get(s.views[0].id).unwrap();
        let f = doc.form.iter().find(|f| f.name == "agree").unwrap();
        (printcraft_ui_egui::forms_ui::field_screen_rect(&s.views[0], &doc.info, f, 0).unwrap(), s.views[0].forms.offer_rect.unwrap())
    };
    // Traverse the gap in small increments, instead of teleporting to the button.
    let gap = egui::pos2((field.right() + popup.left()) / 2.0, field.center().y);
    assert!(!field.contains(gap) && !popup.contains(gap));
    let start = field.center();
    for step in 1..=20 {
        h.hover_at(start + (gap - start) * (step as f32 / 20.0));
        h.run_steps(1);
        assert!(h.state().views[0].forms.offer.is_some());
    }
    h.get_by_label("Check").click();
    h.run_steps(4);
    assert_eq!(value(&h, "agree"), ["Yes"]);
    h.hover_at(field.left_top() - egui::vec2(20.0, 20.0));
    h.run_steps(2);
    assert!(h.state().views[0].forms.offer.is_none());
}

#[test]
fn printed_square_preserves_other_stamps() {
    use printcraft_engine::{Edit, FillMark, NewAnnotation, Shape, StampKind, Style};
    let mut h = printed_form();
    for shape in
        [Shape::Mark { rect: PRINTED_SQUARE, mark: FillMark::Cross }, Shape::Stamp { rect: PRINTED_SQUARE, stamp: StampKind::Approved, by: None }]
    {
        let style = Style::default_for(&shape);
        assert!(h.state_mut().apply_edit(Edit::AddAnnotation(NewAnnotation {
            page: 0,
            shape,
            style,
            contents: String::new(),
            author: "Reviewer".into()
        })));
    }
    h.run_steps(3);
    let stamps = |h: &Harness<'static, PrintCraftApp>| {
        let s = h.state();
        let mut names: Vec<String> = s.session.get(s.views[0].id).unwrap().info.annotations.iter().filter_map(|a| a.stamp.clone()).collect();
        names.sort();
        names
    };
    assert_eq!(stamps(&h), ["Approved", "PCCross"]);
    click_square(&mut h);
    assert_eq!(stamps(&h), ["Approved", "PCCheck", "PCCross"]);
    // Detection and the scoped comment refresh both retain the check's identity.
    click_square(&mut h);
    assert_eq!(stamps(&h), ["Approved", "PCCross"]);
}

#[test]
fn printed_square_commits_the_active_text_draft() {
    let mut h = printed_form();
    click_field(&mut h, "name", 0);
    h.event(egui::Event::Text("Ada Lovelace".into()));
    h.run_steps(2);
    assert_eq!(h.state().views[0].forms.focus.as_ref().unwrap().text, "Ada Lovelace");
    click_square(&mut h);
    assert_eq!(value(&h, "name"), ["Ada Lovelace"]);
    let s = h.state();
    assert!(s.session.get(s.views[0].id).unwrap().info.annotations.iter().any(|a| a.stamp.as_deref() == Some("PCCheck")));
    h.state_mut().undo();
    h.run_steps(2);
    assert_eq!(value(&h, "name"), ["Ada Lovelace"], "undo removes only the check");
    h.state_mut().undo();
    h.run_steps(2);
    assert!(value(&h, "name").is_empty(), "the preceding step committed the text");
}

#[test]
fn selecting_added_content_over_a_printed_square_does_not_toggle_checks() {
    use printcraft_engine::{AddedText, Edit};
    let mut h = printed_form();
    assert!(
        h.state_mut()
            .apply_edit(Edit::AddText { page: 0, text: AddedText { rect: [24.0, 18.0, 65.0, 36.0], text: "Text".into(), ..Default::default() } })
    );
    h.state_mut().set_option("tool", "edit").unwrap();
    h.run_steps(4);
    click_square(&mut h);
    let s = h.state();
    assert_eq!(s.views[0].content.selected, Some((0, 0)), "the added text is selected");
    assert!(s.session.get(s.views[0].id).unwrap().info.annotations.is_empty(), "selecting text did not stamp a check");
}

#[test]
fn dropping_a_comment_on_a_printed_square_finishes_the_move() {
    use printcraft_engine::{Edit, NewAnnotation, Shape, Style};
    let mut h = printed_form();
    let start_rect = [80.0, 18.0, 98.0, 36.0];
    let shape = Shape::Rectangle { rect: start_rect };
    let style = Style::default_for(&shape);
    let annotation = NewAnnotation { page: 0, shape, style, contents: String::new(), author: String::new() };
    assert!(h.state_mut().apply_edit(Edit::AddAnnotation(annotation)));
    h.run_steps(3);
    let (start, end) = (square_center(&h, start_rect), square_center(&h, PRINTED_SQUARE));
    h.hover_at(start);
    h.run_steps(2);
    h.drag_at(start);
    h.run_steps(1);
    h.hover_at(start + egui::vec2(-12.0, 0.0));
    h.run_steps(2);
    assert!(h.state().views[0].comments.gesture.is_some(), "the move started");
    h.drop_at(end);
    h.run_steps(4);
    let s = h.state();
    assert!(s.views[0].comments.gesture.is_none(), "the drop finished the move");
    let annotations = &s.session.get(s.views[0].id).unwrap().info.annotations;
    assert_eq!(annotations.len(), 1, "the drag did not add a check");
    for (actual, expected) in annotations[0].rect.into_iter().zip(PRINTED_SQUARE) {
        assert!((f64::from(actual) - expected).abs() < 0.001, "the comment moved onto the square");
    }
}
