//! UI section builder methods for the properties panel.

use super::*;

// UI Section builders
impl PropertiesPanel {
    pub(crate) fn create_section(title: &str) -> Frame {
        Frame::new(Some(title))
    }

    pub(crate) fn build_position_section() -> (Frame, Entry, Entry, Label, Label) {
        let frame = Self::create_section(&t!("Position"));
        let grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let x_label = Label::new(Some(&t!("X:")));
        x_label.set_halign(gtk4::Align::Start);
        let pos_x_entry = Entry::new();
        pos_x_entry.set_hexpand(true);
        let x_unit_label = Label::new(Some("mm"));
        x_unit_label.set_width_chars(4);
        x_unit_label.set_halign(gtk4::Align::End);
        x_unit_label.set_xalign(1.0);

        let y_label = Label::new(Some(&t!("Y:")));
        y_label.set_halign(gtk4::Align::Start);
        let pos_y_entry = Entry::new();
        pos_y_entry.set_hexpand(true);
        let y_unit_label = Label::new(Some("mm"));
        y_unit_label.set_width_chars(4);
        y_unit_label.set_halign(gtk4::Align::End);
        y_unit_label.set_xalign(1.0);

        grid.attach(&x_label, 0, 0, 1, 1);
        grid.attach(&pos_x_entry, 1, 0, 1, 1);
        grid.attach(&x_unit_label, 2, 0, 1, 1);
        grid.attach(&y_label, 0, 1, 1, 1);
        grid.attach(&pos_y_entry, 1, 1, 1, 1);
        grid.attach(&y_unit_label, 2, 1, 1, 1);

        frame.set_child(Some(&grid));
        (frame, pos_x_entry, pos_y_entry, x_unit_label, y_unit_label)
    }

    pub(crate) fn build_size_section() -> (Frame, Entry, Entry, CheckButton, Label, Label) {
        let frame = Self::create_section(&t!("Size"));
        let grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let width_label = Label::new(Some(&t!("Width:")));
        width_label.set_halign(gtk4::Align::Start);
        let width_entry = Entry::new();
        width_entry.set_hexpand(true);
        let width_unit_label = Label::new(Some("mm"));
        width_unit_label.set_width_chars(4);
        width_unit_label.set_halign(gtk4::Align::End);
        width_unit_label.set_xalign(1.0);

        let height_label = Label::new(Some(&t!("Height:")));
        height_label.set_halign(gtk4::Align::Start);
        let height_entry = Entry::new();
        height_entry.set_hexpand(true);
        let height_unit_label = Label::new(Some("mm"));
        height_unit_label.set_width_chars(4);
        height_unit_label.set_halign(gtk4::Align::End);
        height_unit_label.set_xalign(1.0);

        let lock_aspect_label = Label::new(Some(&t!("Lock Aspect:")));
        lock_aspect_label.set_halign(gtk4::Align::Start);
        let lock_aspect_ratio = CheckButton::new();
        lock_aspect_ratio.set_active(true);

        grid.attach(&width_label, 0, 0, 1, 1);
        grid.attach(&width_entry, 1, 0, 1, 1);
        grid.attach(&width_unit_label, 2, 0, 1, 1);
        grid.attach(&height_label, 0, 1, 1, 1);
        grid.attach(&height_entry, 1, 1, 1, 1);
        grid.attach(&height_unit_label, 2, 1, 1, 1);
        grid.attach(&lock_aspect_label, 0, 2, 1, 1);
        grid.attach(&lock_aspect_ratio, 1, 2, 1, 1);

        frame.set_child(Some(&grid));
        (
            frame,
            width_entry,
            height_entry,
            lock_aspect_ratio,
            width_unit_label,
            height_unit_label,
        )
    }

    pub(crate) fn build_rotation_section() -> (Frame, Entry) {
        let frame = Self::create_section(&t!("Rotation"));
        let grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let rot_label = Label::new(Some(&t!("Angle:")));
        rot_label.set_halign(gtk4::Align::Start);
        let rotation_entry = Entry::new();
        rotation_entry.set_hexpand(true);
        let rot_unit = Label::new(Some("deg"));

        grid.attach(&rot_label, 0, 0, 1, 1);
        grid.attach(&rotation_entry, 1, 0, 1, 1);
        grid.attach(&rot_unit, 2, 0, 1, 1);

        frame.set_child(Some(&grid));
        (frame, rotation_entry)
    }

    pub(crate) fn build_corner_section() -> (Frame, Entry, CheckButton, Label) {
        let frame = Self::create_section(&t!("Corner"));
        let grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let radius_label = Label::new(Some(&t!("Radius:")));
        radius_label.set_halign(gtk4::Align::Start);
        let corner_radius_entry = Entry::new();
        corner_radius_entry.set_hexpand(true);
        let radius_unit_label = Label::new(Some("mm"));
        radius_unit_label.set_width_chars(4);
        radius_unit_label.set_halign(gtk4::Align::End);
        radius_unit_label.set_xalign(1.0);

        let slot_label = Label::new(Some(&t!("Slot Mode:")));
        slot_label.set_halign(gtk4::Align::Start);
        let is_slot_check = CheckButton::new();

        grid.attach(&radius_label, 0, 0, 1, 1);
        grid.attach(&corner_radius_entry, 1, 0, 1, 1);
        grid.attach(&radius_unit_label, 2, 0, 1, 1);
        grid.attach(&slot_label, 0, 1, 1, 1);
        grid.attach(&is_slot_check, 1, 1, 1, 1);

        frame.set_child(Some(&grid));
        (frame, corner_radius_entry, is_slot_check, radius_unit_label)
    }

    pub(crate) fn build_text_section() -> (
        Frame,
        Entry,
        DropDown,
        CheckButton,
        CheckButton,
        Entry,
        Label,
    ) {
        let frame = Self::create_section(&t!("Text"));
        let grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let text_content_label = Label::new(Some(&t!("Content:")));
        text_content_label.set_halign(gtk4::Align::Start);
        let text_entry = Entry::new();
        text_entry.set_hexpand(true);

        let font_label = Label::new(Some(&t!("Font:")));
        font_label.set_halign(gtk4::Align::Start);
        let font_model = StringList::new(&[]);
        font_model.append("Sans");
        for fam in font_manager::list_font_families() {
            if fam != "Sans" {
                font_model.append(&fam);
            }
        }
        let font_family_combo = DropDown::new(Some(font_model), None::<Expression>);
        font_family_combo.set_hexpand(true);

        let style_label = Label::new(Some(&t!("Style:")));
        style_label.set_halign(gtk4::Align::Start);
        let font_bold_check = CheckButton::with_label(&t!("Bold"));
        let font_italic_check = CheckButton::with_label(&t!("Italic"));
        let style_box = Box::new(Orientation::Horizontal, 8);
        style_box.append(&font_bold_check);
        style_box.append(&font_italic_check);

        let font_size_label = Label::new(Some(&t!("Size:")));
        font_size_label.set_halign(gtk4::Align::Start);
        let font_size_entry = Entry::new();
        font_size_entry.set_hexpand(true);
        let font_size_unit_label = Label::new(Some("pt"));
        font_size_unit_label.set_width_chars(4);
        font_size_unit_label.set_halign(gtk4::Align::End);
        font_size_unit_label.set_xalign(1.0);

        grid.attach(&text_content_label, 0, 0, 1, 1);
        grid.attach(&text_entry, 1, 0, 2, 1);
        grid.attach(&font_label, 0, 1, 1, 1);
        grid.attach(&font_family_combo, 1, 1, 2, 1);
        grid.attach(&style_label, 0, 2, 1, 1);
        grid.attach(&style_box, 1, 2, 2, 1);
        grid.attach(&font_size_label, 0, 3, 1, 1);
        grid.attach(&font_size_entry, 1, 3, 1, 1);
        grid.attach(&font_size_unit_label, 2, 3, 1, 1);

        frame.set_child(Some(&grid));
        (
            frame,
            text_entry,
            font_family_combo,
            font_bold_check,
            font_italic_check,
            font_size_entry,
            font_size_unit_label,
        )
    }

    pub(crate) fn build_polygon_section() -> (Frame, Entry) {
        let frame = Self::create_section(&t!("Polygon"));
        let grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let sides_label = Label::new(Some(&t!("Sides:")));
        sides_label.set_halign(gtk4::Align::Start);
        let sides_entry = Entry::new();
        sides_entry.set_hexpand(true);

        grid.attach(&sides_label, 0, 0, 1, 1);
        grid.attach(&sides_entry, 1, 0, 1, 1);

        frame.set_child(Some(&grid));
        (frame, sides_entry)
    }

    pub(crate) fn build_gear_section() -> (Frame, Entry, Entry, Entry) {
        let frame = Self::create_section(&t!("Gear"));
        let grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let module_label = Label::new(Some(&t!("Module:")));
        module_label.set_halign(gtk4::Align::Start);
        let gear_module_entry = Entry::new();
        gear_module_entry.set_hexpand(true);

        let teeth_label = Label::new(Some(&t!("Teeth:")));
        teeth_label.set_halign(gtk4::Align::Start);
        let gear_teeth_entry = Entry::new();
        gear_teeth_entry.set_hexpand(true);

        let pa_label = Label::new(Some(&t!("Pressure Angle:")));
        pa_label.set_halign(gtk4::Align::Start);
        let gear_pressure_angle_entry = Entry::new();
        gear_pressure_angle_entry.set_hexpand(true);

        grid.attach(&module_label, 0, 0, 1, 1);
        grid.attach(&gear_module_entry, 1, 0, 1, 1);
        grid.attach(&teeth_label, 0, 1, 1, 1);
        grid.attach(&gear_teeth_entry, 1, 1, 1, 1);
        grid.attach(&pa_label, 0, 2, 1, 1);
        grid.attach(&gear_pressure_angle_entry, 1, 2, 1, 1);

        frame.set_child(Some(&grid));
        (
            frame,
            gear_module_entry,
            gear_teeth_entry,
            gear_pressure_angle_entry,
        )
    }

    pub(crate) fn build_sprocket_section() -> (Frame, Entry, Entry, Entry) {
        let frame = Self::create_section(&t!("Sprocket"));
        let grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let pitch_label = Label::new(Some(&t!("Pitch:")));
        pitch_label.set_halign(gtk4::Align::Start);
        let sprocket_pitch_entry = Entry::new();
        sprocket_pitch_entry.set_hexpand(true);

        let teeth_label = Label::new(Some(&t!("Teeth:")));
        teeth_label.set_halign(gtk4::Align::Start);
        let sprocket_teeth_entry = Entry::new();
        sprocket_teeth_entry.set_hexpand(true);

        let roller_label = Label::new(Some(&t!("Roller Dia:")));
        roller_label.set_halign(gtk4::Align::Start);
        let sprocket_roller_diameter_entry = Entry::new();
        sprocket_roller_diameter_entry.set_hexpand(true);

        grid.attach(&pitch_label, 0, 0, 1, 1);
        grid.attach(&sprocket_pitch_entry, 1, 0, 1, 1);
        grid.attach(&teeth_label, 0, 1, 1, 1);
        grid.attach(&sprocket_teeth_entry, 1, 1, 1, 1);
        grid.attach(&roller_label, 0, 2, 1, 1);
        grid.attach(&sprocket_roller_diameter_entry, 1, 2, 1, 1);

        frame.set_child(Some(&grid));
        (
            frame,
            sprocket_pitch_entry,
            sprocket_teeth_entry,
            sprocket_roller_diameter_entry,
        )
    }

    pub(crate) fn build_geometry_ops_section() -> (Frame, Entry, Entry, Entry, Label, Label, Label)
    {
        let frame = Self::create_section(&t!("Geometry Operations"));
        let grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let offset_label = Label::new(Some(&t!("Offset:")));
        offset_label.set_halign(gtk4::Align::Start);
        let offset_entry = Entry::new();
        offset_entry.set_text("1.0");
        offset_entry.set_hexpand(true);
        let offset_unit_label = Label::new(Some("mm"));

        let fillet_label = Label::new(Some(&t!("Fillet:")));
        fillet_label.set_halign(gtk4::Align::Start);
        let fillet_entry = Entry::new();
        fillet_entry.set_text("2.0");
        fillet_entry.set_hexpand(true);
        let fillet_unit_label = Label::new(Some("mm"));

        let chamfer_label = Label::new(Some(&t!("Chamfer:")));
        chamfer_label.set_halign(gtk4::Align::Start);
        let chamfer_entry = Entry::new();
        chamfer_entry.set_text("2.0");
        chamfer_entry.set_hexpand(true);
        let chamfer_unit_label = Label::new(Some("mm"));

        grid.attach(&offset_label, 0, 0, 1, 1);
        grid.attach(&offset_entry, 1, 0, 1, 1);
        grid.attach(&offset_unit_label, 2, 0, 1, 1);
        grid.attach(&fillet_label, 0, 1, 1, 1);
        grid.attach(&fillet_entry, 1, 1, 1, 1);
        grid.attach(&fillet_unit_label, 2, 1, 1, 1);
        grid.attach(&chamfer_label, 0, 2, 1, 1);
        grid.attach(&chamfer_entry, 1, 2, 1, 1);
        grid.attach(&chamfer_unit_label, 2, 2, 1, 1);

        frame.set_child(Some(&grid));
        (
            frame,
            offset_entry,
            fillet_entry,
            chamfer_entry,
            offset_unit_label,
            fillet_unit_label,
            chamfer_unit_label,
        )
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn build_cam_section() -> (
        Frame,
        DropDown,
        Entry,
        Entry,
        Entry,
        Entry,
        DropDown,
        Entry,
        Label,
        Label,
        Label,
    ) {
        let frame = Self::create_section(&t!("CAM Properties"));
        let grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        // Operation Type
        let op_label = Label::new(Some(&t!("Operation:")));
        op_label.set_halign(gtk4::Align::Start);
        let op_model = StringList::new(&[]);
        op_model.append(&t!("Profile"));
        op_model.append(&t!("Pocket"));
        let op_type_combo = DropDown::new(Some(op_model), None::<Expression>);
        op_type_combo.set_hexpand(true);

        // Pocket Depth
        let depth_label = Label::new(Some(&t!("Depth:")));
        depth_label.set_halign(gtk4::Align::Start);
        let depth_entry = Entry::new();
        depth_entry.set_hexpand(true);
        let depth_unit_label = Label::new(Some("mm"));
        depth_unit_label.set_width_chars(4);
        depth_unit_label.set_halign(gtk4::Align::End);
        depth_unit_label.set_xalign(1.0);

        // Step Down
        let step_down_label = Label::new(Some(&t!("Step Down:")));
        step_down_label.set_halign(gtk4::Align::Start);
        let step_down_entry = Entry::new();
        step_down_entry.set_hexpand(true);
        let step_down_unit_label = Label::new(Some("mm"));
        step_down_unit_label.set_width_chars(4);
        step_down_unit_label.set_halign(gtk4::Align::End);
        step_down_unit_label.set_xalign(1.0);

        // Step In (for pockets)
        let step_in_label = Label::new(Some(&t!("Step In:")));
        step_in_label.set_halign(gtk4::Align::Start);
        let step_in_entry = Entry::new();
        step_in_entry.set_hexpand(true);
        let step_in_unit_label = Label::new(Some("mm"));
        step_in_unit_label.set_width_chars(4);
        step_in_unit_label.set_halign(gtk4::Align::End);
        step_in_unit_label.set_xalign(1.0);

        // Ramp Angle
        let ramp_angle_label = Label::new(Some(&t!("Ramp Angle:")));
        ramp_angle_label.set_halign(gtk4::Align::Start);
        let ramp_angle_entry = Entry::new();
        ramp_angle_entry.set_hexpand(true);
        let ramp_angle_unit_label = Label::new(Some("deg"));
        ramp_angle_unit_label.set_width_chars(4);
        ramp_angle_unit_label.set_halign(gtk4::Align::End);
        ramp_angle_unit_label.set_xalign(1.0);

        // Pocket Strategy
        let strategy_label = Label::new(Some(&t!("Strategy:")));
        strategy_label.set_halign(gtk4::Align::Start);
        let strategy_model = StringList::new(&[]);
        strategy_model.append(&t!("Raster"));
        strategy_model.append(&t!("Offset"));
        strategy_model.append(&t!("Adaptive"));
        let strategy_combo = DropDown::new(Some(strategy_model), None::<Expression>);
        strategy_combo.set_hexpand(true);

        // Raster Fill (inverse inset)
        let raster_fill_label = Label::new(Some(&t!("Raster Fill (%):")));
        raster_fill_label.set_halign(gtk4::Align::Start);
        let raster_fill_entry = Entry::new();
        raster_fill_entry.set_hexpand(true);
        let raster_fill_hint = Label::new(Some("0 = no raster, 100 = full length"));
        raster_fill_hint.add_css_class("dim-label");
        raster_fill_hint.set_halign(gtk4::Align::Start);

        grid.attach(&op_label, 0, 0, 1, 1);
        grid.attach(&op_type_combo, 1, 0, 1, 1);
        grid.attach(&depth_label, 0, 1, 1, 1);
        grid.attach(&depth_entry, 1, 1, 1, 1);
        grid.attach(&depth_unit_label, 2, 1, 1, 1);
        grid.attach(&step_down_label, 0, 2, 1, 1);
        grid.attach(&step_down_entry, 1, 2, 1, 1);
        grid.attach(&step_down_unit_label, 2, 2, 1, 1);
        grid.attach(&step_in_label, 0, 3, 1, 1);
        grid.attach(&step_in_entry, 1, 3, 1, 1);
        grid.attach(&step_in_unit_label, 2, 3, 1, 1);
        grid.attach(&ramp_angle_label, 0, 4, 1, 1);
        grid.attach(&ramp_angle_entry, 1, 4, 1, 1);
        grid.attach(&ramp_angle_unit_label, 2, 4, 1, 1);
        grid.attach(&strategy_label, 0, 5, 1, 1);
        grid.attach(&strategy_combo, 1, 5, 1, 1);
        grid.attach(&raster_fill_label, 0, 6, 1, 1);
        grid.attach(&raster_fill_entry, 1, 6, 1, 1);
        grid.attach(&raster_fill_hint, 0, 7, 3, 1);

        frame.set_child(Some(&grid));
        (
            frame,
            op_type_combo,
            depth_entry,
            step_down_entry,
            step_in_entry,
            ramp_angle_entry,
            strategy_combo,
            raster_fill_entry,
            depth_unit_label,
            step_down_unit_label,
            step_in_unit_label,
        )
    }
}
