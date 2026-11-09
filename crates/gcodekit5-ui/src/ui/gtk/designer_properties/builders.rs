//! UI section builder methods for the properties panel.

use super::*;
use gcodekit5_designer::model::RasterImage;

// UI Section builders
impl PropertiesPanel {

    pub(crate) fn create_section(title: &str) -> Frame {
        let frame = Frame::new(None);

        let title_label = Label::new(None);
        title_label.set_markup(&format!("<b>{}</b>", title));
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_margin_start(8);
        title_label.set_margin_top(4);
        title_label.set_margin_bottom(4);
//        title_label.add_css_class("section-title"); // ← Opcional para CSS
        frame.set_label_widget(Some(&title_label));
        frame
    }


    pub(crate) fn build_position_section() -> (Frame, Entry, Entry, Entry, Label, Label, Label, Label) {
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

        let z_label = Label::new(Some(&t!("(-Z:)")));
        z_label.set_halign(gtk4::Align::Start);
        let pos_z_entry = Entry::new();
        pos_z_entry.set_hexpand(true);
        let z_unit_label = Label::new(Some("mm"));
        z_unit_label.set_width_chars(4);
        z_unit_label.set_halign(gtk4::Align::End);
        z_unit_label.set_xalign(1.0);

        grid.attach(&x_label, 0, 0, 1, 1);
        grid.attach(&pos_x_entry, 1, 0, 1, 1);
        grid.attach(&x_unit_label, 2, 0, 1, 1);
        grid.attach(&y_label, 0, 1, 1, 1);
        grid.attach(&pos_y_entry, 1, 1, 1, 1);
        grid.attach(&y_unit_label, 2, 1, 1, 1);
        grid.attach(&z_label, 0, 2, 1, 1);
        grid.attach(&pos_z_entry, 1, 2, 1, 1);
        grid.attach(&z_unit_label, 2, 2, 1, 1);

        frame.set_child(Some(&grid));
        (
            frame,
            pos_x_entry,
            pos_y_entry,
            pos_z_entry,
            x_unit_label,
            y_unit_label,
            z_label,
            z_unit_label,
        )
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
        let frame = Self::create_section(&t!("Rotation:"));
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
        let frame = Self::create_section(&t!("Text:"));
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

        grid.attach(&sides_label, 0, 0, 1, 1);
        grid.attach(&sides_entry, 1, 0, 1, 1);

        frame.set_child(Some(&grid));
        (frame, sides_entry)
    }

    pub(crate) fn build_path_section() -> (Frame, CheckButton) {
        let frame = Self::create_section(&t!("Path"));
        let grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let closed_label = Label::new(Some(&t!("Closed:")));
        closed_label.set_halign(gtk4::Align::Start);
        let path_closed_check = CheckButton::new();
        path_closed_check.set_sensitive(true);

        grid.attach(&closed_label, 0, 0, 1, 1);
        grid.attach(&path_closed_check, 1, 0, 1, 1);

        frame.set_child(Some(&grid));
        (frame, path_closed_check)
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

        let teeth_label = Label::new(Some(&t!("Teeth:")));
        teeth_label.set_halign(gtk4::Align::Start);
        let gear_teeth_entry = Entry::new();

        let pa_label = Label::new(Some(&t!("Pressure Angle:")));
        pa_label.set_halign(gtk4::Align::Start);
        let gear_pressure_angle_entry = Entry::new();

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

        let teeth_label = Label::new(Some(&t!("Teeth:")));
        teeth_label.set_halign(gtk4::Align::Start);
        let sprocket_teeth_entry = Entry::new();

        let roller_label = Label::new(Some(&t!("Roller Dia:")));
        roller_label.set_halign(gtk4::Align::Start);
        let sprocket_roller_diameter_entry = Entry::new();

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

    pub(crate) fn build_timing_pulley_section() -> (Frame, Entry, Entry, Entry, Entry) {
        let frame = Self::create_section(&t!("Timing Pulley"));
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
        let timing_pulley_pitch_entry = Entry::new();

        let teeth_label = Label::new(Some(&t!("Teeth:")));
        teeth_label.set_halign(gtk4::Align::Start);
        let timing_pulley_teeth_entry = Entry::new();

        let width_label = Label::new(Some(&t!("Belt Width:")));
        width_label.set_halign(gtk4::Align::Start);
        let timing_pulley_belt_width_entry = Entry::new();

        let hole_label = Label::new(Some(&t!("Hole Radius:")));
        hole_label.set_halign(gtk4::Align::Start);
        let timing_pulley_hole_radius_entry = Entry::new();

        grid.attach(&pitch_label, 0, 0, 1, 1);
        grid.attach(&timing_pulley_pitch_entry, 1, 0, 1, 1);
        grid.attach(&teeth_label, 0, 1, 1, 1);
        grid.attach(&timing_pulley_teeth_entry, 1, 1, 1, 1);
        grid.attach(&width_label, 0, 2, 1, 1);
        grid.attach(&timing_pulley_belt_width_entry, 1, 2, 1, 1);
        grid.attach(&hole_label, 0, 3, 1, 1);
        grid.attach(&timing_pulley_hole_radius_entry, 1, 3, 1, 1);

        frame.set_child(Some(&grid));
        (
            frame,
            timing_pulley_pitch_entry,
            timing_pulley_teeth_entry,
            timing_pulley_belt_width_entry,
            timing_pulley_hole_radius_entry,
        )
    }

    pub(crate) fn build_geometry_ops_section() -> (Frame, Entry, Entry, Label, Label) {
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
        let offset_unit_label = Label::new(Some("mm"));

        let chamfer_label = Label::new(Some(&t!("Chamfer:")));
        chamfer_label.set_halign(gtk4::Align::Start);
        let chamfer_entry = Entry::new();
        chamfer_entry.set_text("2.0");
        let chamfer_unit_label = Label::new(Some("mm"));

        grid.attach(&offset_label, 0, 0, 1, 1);
        grid.attach(&offset_entry, 1, 0, 1, 1);
        grid.attach(&offset_unit_label, 2, 0, 1, 1);
        grid.attach(&chamfer_label, 0, 1, 1, 1);
        grid.attach(&chamfer_entry, 1, 1, 1, 1);
        grid.attach(&chamfer_unit_label, 2, 1, 1, 1);

        frame.set_child(Some(&grid));
        (frame, offset_entry, chamfer_entry, offset_unit_label, chamfer_unit_label)
    }

    pub(crate) fn build_cam_section() -> (
        Frame,
        CheckButton,
        DropDown,
        Entry,  // step_down_entry
        Entry,  // step_in_entry
        Entry,  // ramp_angle_entry
        DropDown, // strategy_combo
        Entry, // raster_fill_entry
        Label,  // step_down_unit_label
        Label,  // step_in_unit_label
        Label,  // step_in_label
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

        let cam_use_global_check = CheckButton::with_label(&t!("Use global values"));
        cam_use_global_check.set_active(false);

        // Step Down
        let step_down_label = Label::new(Some(&t!("Step Down:")));
        step_down_label.set_halign(gtk4::Align::Start);
        let step_down_entry = Entry::new();
        let step_down_unit_label = Label::new(Some("mm"));
        step_down_unit_label.set_width_chars(4);
        step_down_unit_label.set_halign(gtk4::Align::End);
        step_down_unit_label.set_xalign(1.0);

        // Step In (for pockets)
        let step_in_label = Label::new(Some(&t!("Step In:")));
        step_in_label.set_halign(gtk4::Align::Start);
        let step_in_entry = Entry::new();
        let step_in_unit_label = Label::new(Some("mm"));
        step_in_unit_label.set_width_chars(4);
        step_in_unit_label.set_halign(gtk4::Align::End);
        step_in_unit_label.set_xalign(1.0);

        // Desactivar Step In por defecto (Profile es el default)
        step_in_entry.set_sensitive(false);
        step_in_label.set_sensitive(false);
        step_in_unit_label.set_sensitive(false);

        // Ramp Angle
        let ramp_angle_label = Label::new(Some(&t!("Ramp Angle:")));
        ramp_angle_label.set_halign(gtk4::Align::Start);
        let ramp_angle_entry = Entry::new();
        let ramp_angle_unit_label = Label::new(Some("deg"));
        ramp_angle_unit_label.set_width_chars(4);
        ramp_angle_unit_label.set_halign(gtk4::Align::End);
        ramp_angle_unit_label.set_xalign(1.0);

        // Pocket Strategy - Solo para Pocket
        let strategy_label = Label::new(Some(&t!("Strategy:")));
        strategy_label.set_halign(gtk4::Align::Start);
        let strategy_model = StringList::new(&[]);
        strategy_model.append(&t!("Raster"));
        strategy_model.append(&t!("Offset"));
        strategy_model.append(&t!("Adaptive"));
        let strategy_combo = DropDown::new(Some(strategy_model), None::<Expression>);

        // Desactivar Strategy por defecto (Profile es el default)
        strategy_combo.set_sensitive(false);
        strategy_label.set_sensitive(false);

        // Raster Fill
        let raster_fill_label = Label::new(Some(&t!("Raster Fill (%):")));
        raster_fill_label.set_halign(gtk4::Align::Start);
        let raster_fill_entry = Entry::new();
        let raster_fill_hint = Label::new(Some("0 = no raster, 100 = full length"));
        raster_fill_hint.add_css_class("dim-label");
        raster_fill_hint.set_halign(gtk4::Align::Start);

        // Desactivar Raster Fill por defecto
        raster_fill_entry.set_sensitive(false);
        raster_fill_label.set_sensitive(false);
        raster_fill_hint.set_sensitive(false);

        // Layout
        grid.attach(&op_label, 0, 0, 1, 1);
        grid.attach(&op_type_combo, 1, 0, 1, 1);
        grid.attach(&cam_use_global_check, 0, 1, 3, 1);
        grid.attach(&step_down_label, 0, 4, 1, 1);
        grid.attach(&step_down_entry, 1, 4, 1, 1);
        grid.attach(&step_down_unit_label, 2, 4, 1, 1);
        grid.attach(&step_in_label, 0, 5, 1, 1);
        grid.attach(&step_in_entry, 1, 5, 1, 1);
        grid.attach(&step_in_unit_label, 2, 5, 1, 1);
        grid.attach(&ramp_angle_label, 0, 6, 1, 1);
        grid.attach(&ramp_angle_entry, 1, 6, 1, 1);
        grid.attach(&ramp_angle_unit_label, 2, 6, 1, 1);
        grid.attach(&strategy_label, 0, 7, 1, 1);
        grid.attach(&strategy_combo, 1, 7, 1, 1);
        grid.attach(&raster_fill_label, 0, 8, 1, 1);
        grid.attach(&raster_fill_entry, 1, 8, 1, 1);
        grid.attach(&raster_fill_hint, 0, 9, 3, 1);

        frame.set_child(Some(&grid));
        (
            frame,
            cam_use_global_check,
            op_type_combo,
            step_down_entry,
            step_in_entry,
            ramp_angle_entry,
            strategy_combo,
            raster_fill_entry,
            step_down_unit_label,
            step_in_unit_label,
            step_in_label,
        )
    }

    pub(crate) fn build_image_engraving_section() -> (
        Frame,
        Entry,        // feed_rate
        Entry,        // travel_rate
        Entry,        // min_power
        Entry,        // max_power
        Entry,        // ppi
        ComboBoxText, // scan_direction
        CheckButton,  // bidirectional
        CheckButton,  // invert
        ComboBoxText, // dithering
        Entry,        // halftone_threshold
    ) {
        let frame = Self::create_section(&t!("Laser Engraving"));
        let grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        // Feed Rate
        let feed_label = Label::new(Some(&t!("Feed Rate:")));
        feed_label.set_halign(gtk4::Align::Start);
        let feed_entry = Entry::new();
        feed_entry.set_max_width_chars(8);
        let feed_unit = Label::new(Some("mm/s"));
        feed_entry.set_text(&RasterImage::default().feed_rate.to_string());

        // Travel Rate
        let travel_label = Label::new(Some(&t!("Travel Rate:")));
        travel_label.set_halign(gtk4::Align::Start);
        let travel_entry = Entry::new();
        travel_entry.set_max_width_chars(8);
        let travel_unit = Label::new(Some("mm/s"));
        travel_entry.set_text(&RasterImage::default().travel_rate.to_string());

        // Min Power
        let min_power_label = Label::new(Some(&t!("Min Power:")));
        min_power_label.set_halign(gtk4::Align::Start);
        let min_power_entry = Entry::new();
        min_power_entry.set_max_width_chars(8);
        let power_unit = Label::new(Some("%"));
        min_power_entry.set_text(&RasterImage::default().min_power.to_string());

        // Max Power
        let max_power_label = Label::new(Some(&t!("Max Power:")));
        max_power_label.set_halign(gtk4::Align::Start);
        let max_power_entry = Entry::new();
        max_power_entry.set_max_width_chars(8);
        let max_power_unit = Label::new(Some("%"));
        max_power_entry.set_text(&RasterImage::default().max_power.to_string());

        // PPI
        let ppi_label = Label::new(Some(&t!("Resolution:")));
        ppi_label.set_halign(gtk4::Align::Start);
        let ppi_entry = Entry::new();
        ppi_entry.set_max_width_chars(8);
        let ppi_unit = Label::new(Some("PPI"));
        ppi_entry.set_text(&RasterImage::default().ppi.to_string());

        // Scan Direction
        let scan_label = Label::new(Some(&t!("Scan:")));
        scan_label.set_halign(gtk4::Align::Start);
        let scan_combo = ComboBoxText::new();
        scan_combo.append(Some("horizontal"), &t!("Horizontal"));
        scan_combo.append(Some("vertical"), &t!("Vertical"));
        scan_combo.set_active_id(Some(&RasterImage::default().scan_direction));

        // Bidirectional
        let bidir_label = Label::new(Some(&t!("Bidirectional:")));
        bidir_label.set_halign(gtk4::Align::Start);
        let bidir_check = CheckButton::new();
        bidir_check.set_active(RasterImage::default().bidirectional);

        // Dithering
        let dither_label = Label::new(Some(&t!("Dithering:")));
        dither_label.set_halign(gtk4::Align::Start);
        let dither_combo = ComboBoxText::new();
        dither_combo.append(Some("none"), &t!("None"));
        dither_combo.append(Some("threshold"), &t!("Threshold"));
        dither_combo.append(Some("floyd"), &t!("Floyd-Ste."));
        dither_combo.append(Some("atkinson"), &t!("Atkinson"));
        dither_combo.append(Some("bayer"), &t!("Bayer"));
        dither_combo.set_active_id(Some(&RasterImage::default().dithering));

        // Invert
        let invert_label = Label::new(Some(&t!("Invert:")));
        invert_label.set_halign(gtk4::Align::Start);
        let invert_check = CheckButton::new();

        // Halftone_threshold
        let halftone_label = Label::new(Some(&t!("Halftone:")));
        halftone_label.set_halign(gtk4::Align::Start);
        let halftone_entry = Entry::new();
        halftone_entry.set_max_width_chars(8);  // <-- Limitar a 8 caracteres
        halftone_entry.set_placeholder_text(Some("0-255"));
        halftone_entry.set_text("127"); // Valor por defecto
        let halftone_range = Label::new(Some("(0-255)"));
        halftone_range.add_css_class("dim-label");

        // Layout
        grid.attach(&feed_label, 0, 0, 1, 1);
        grid.attach(&feed_entry, 1, 0, 1, 1);
        grid.attach(&feed_unit, 2, 0, 1, 1);

        grid.attach(&travel_label, 0, 1, 1, 1);
        grid.attach(&travel_entry, 1, 1, 1, 1);
        grid.attach(&travel_unit, 2, 1, 1, 1);

        grid.attach(&min_power_label, 0, 2, 1, 1);
        grid.attach(&min_power_entry, 1, 2, 1, 1);
        grid.attach(&power_unit, 2, 2, 1, 1);

        grid.attach(&max_power_label, 0, 3, 1, 1);
        grid.attach(&max_power_entry, 1, 3, 1, 1);
        grid.attach(&max_power_unit, 2, 3, 1, 1);

        grid.attach(&ppi_label, 0, 4, 1, 1);
        grid.attach(&ppi_entry, 1, 4, 1, 1);
        grid.attach(&ppi_unit, 2, 4, 1, 1);

        grid.attach(&scan_label, 0, 5, 1, 1);
        grid.attach(&scan_combo, 1, 5, 1, 1);

        grid.attach(&bidir_label, 0, 6, 1, 1);
        grid.attach(&bidir_check, 1, 6, 1, 1);

        grid.attach(&invert_label, 0, 7, 1, 1);
        grid.attach(&invert_check, 1, 7, 1, 1);

        grid.attach(&dither_label, 0, 8, 1, 1);
        grid.attach(&dither_combo, 1, 8, 1, 1);

        grid.attach(&halftone_label, 0, 9, 1, 1);
        grid.attach(&halftone_entry, 1, 9, 1, 1);
        grid.attach(&halftone_range, 2, 9, 1, 1);

        frame.set_child(Some(&grid));
        (
            frame,
            feed_entry,
            travel_entry,
            min_power_entry,
            max_power_entry,
            ppi_entry,
            scan_combo,
            bidir_check,
            invert_check,
            dither_combo,
            halftone_entry,
        )
    }

    pub(crate) fn build_laser_override_section() -> (
        Frame,
        CheckButton, // use_global
        Entry,       // feed_rate
        Entry,       // power_percent
        Entry,       // passes
    ) {
        let frame = Self::create_section(&t!("Laser Config (Object)"));
        let grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        // Checkbox "Use global values"
        let use_global_check = CheckButton::new();
        let use_global_label = Label::new(Some(&t!("Use global values")));
        use_global_label.set_halign(gtk4::Align::Start);
        use_global_check.set_active(true); // Por defecto, usar globales

        let global_box = Box::new(Orientation::Horizontal, 8);
        global_box.append(&use_global_check);
        global_box.append(&use_global_label);

        // Feed Rate
        let feed_label = Label::new(Some(&t!("Feed rate:")));
        let feed_entry = Entry::new();
        feed_entry.set_max_width_chars(8);
        feed_entry.set_placeholder_text(Some(&t!("Global")));
        feed_entry.set_sensitive(false); // Por defecto deshabilitado
        let feed_unit = Label::new(Some("mm/min"));

        // Power
        let power_label = Label::new(Some(&t!("Power:")));
        let power_entry = Entry::new();
        power_entry.set_max_width_chars(8);
        power_entry.set_placeholder_text(Some(&t!("Global")));
        power_entry.set_sensitive(false); // Por defecto deshabilitado
        let power_unit = Label::new(Some("%"));

        // Passes
        let passes_label = Label::new(Some(&t!("Passes:")));
        let passes_entry = Entry::new();
        passes_entry.set_max_width_chars(8);
        passes_entry.set_placeholder_text(Some(&t!("Global")));
        passes_entry.set_sensitive(false); // Por defecto deshabilitado

        // Layout
        grid.attach(&global_box, 0, 0, 3, 1);
        grid.attach(&feed_label, 0, 1, 1, 1);
        grid.attach(&feed_entry, 1, 1, 1, 1);
        grid.attach(&feed_unit, 2, 1, 1, 1);
        grid.attach(&power_label, 0, 2, 1, 1);
        grid.attach(&power_entry, 1, 2, 1, 1);
        grid.attach(&power_unit, 2, 2, 1, 1);
        grid.attach(&passes_label, 0, 3, 1, 1);
        grid.attach(&passes_entry, 1, 3, 1, 1);

        frame.set_child(Some(&grid));

        (
            frame,
            use_global_check,
            feed_entry,
            power_entry,
            passes_entry,
        )
    }
}
