use super::*;
use gtk4::{Grid, WrapMode};

impl ToolsManagerView {
    pub(crate) fn create_basic_tab() -> (
        ScrolledWindow,
        Entry,
        SpinButton,
        Entry,
        ComboBoxText,
        ComboBoxText,
        ComboBoxText,
        ComboBoxText,
    ) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let grid = Grid::new();
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        let mut row = 0;

        // ID
        let id_label = Label::new(Some("ID:"));
        id_label.set_halign(Align::Start);
        let edit_id = Entry::new();
        edit_id.set_placeholder_text(Some("tool_id"));
        grid.attach(&id_label, 0, row, 1, 1);
        grid.attach(&edit_id, 1, row, 1, 1);
        row += 1;

        // Tool Number
        let num_label = Label::new(Some("Tool Number:"));
        num_label.set_halign(Align::Start);
        let edit_number = SpinButton::with_range(1.0, 999.0, 1.0);
        edit_number.set_value(1.0);
        grid.attach(&num_label, 0, row, 1, 1);
        grid.attach(&edit_number, 1, row, 1, 1);
        row += 1;

        // Name
        let name_label = Label::new(Some("Name:"));
        name_label.set_halign(Align::Start);
        let edit_name = Entry::new();
        edit_name.set_hexpand(true);
        grid.attach(&name_label, 0, row, 1, 1);
        grid.attach(&edit_name, 1, row, 1, 1);
        row += 1;

        // Tool Type
        let type_label = Label::new(Some("Tool Type:"));
        type_label.set_halign(Align::Start);
        let edit_tool_type = ComboBoxText::new();
        edit_tool_type.append_text("Flat End Mill");
        edit_tool_type.append_text("Ball End Mill");
        edit_tool_type.append_text("Corner Radius End Mill");
        edit_tool_type.append_text("V-Bit");
        edit_tool_type.append_text("Drill Bit");
        edit_tool_type.append_text("Spot Drill");
        edit_tool_type.append_text("Engraving Bit");
        edit_tool_type.append_text("Chamfer Tool");
        edit_tool_type.append_text("Specialty");
        edit_tool_type.set_active(Some(0));
        grid.attach(&type_label, 0, row, 1, 1);
        grid.attach(&edit_tool_type, 1, row, 1, 1);
        row += 1;

        // Material
        let mat_label = Label::new(Some("Tool Material:"));
        mat_label.set_halign(Align::Start);
        let edit_material = ComboBoxText::new();
        edit_material.append_text("HSS");
        edit_material.append_text("Carbide");
        edit_material.append_text("Coated Carbide");
        edit_material.append_text("Diamond");
        edit_material.set_active(Some(1));
        grid.attach(&mat_label, 0, row, 1, 1);
        grid.attach(&edit_material, 1, row, 1, 1);
        row += 1;

        // Coating
        let coating_label = Label::new(Some("Coating:"));
        coating_label.set_halign(Align::Start);
        let edit_coating = ComboBoxText::new();
        edit_coating.append_text("None");
        edit_coating.append_text("TiN");
        edit_coating.append_text("TiAlN");
        edit_coating.append_text("DLC");
        edit_coating.append_text("AlOx");
        edit_coating.set_active(Some(0));
        grid.attach(&coating_label, 0, row, 1, 1);
        grid.attach(&edit_coating, 1, row, 1, 1);
        row += 1;

        // Shank
        let shank_label = Label::new(Some("Shank:"));
        shank_label.set_halign(Align::Start);
        let edit_shank = ComboBoxText::new();
        edit_shank.append_text("Derived (Straight from shaft Ø)");
        edit_shank.append_text("Collet");
        edit_shank.append_text("Tapered");
        edit_shank.set_active(Some(0));
        grid.attach(&shank_label, 0, row, 1, 1);
        grid.attach(&edit_shank, 1, row, 1, 1);

        scroll.set_child(Some(&grid));
        (
            scroll,
            edit_id,
            edit_number,
            edit_name,
            edit_tool_type,
            edit_material,
            edit_coating,
            edit_shank,
        )
    }

    pub(crate) fn create_geometry_tab() -> (
        ScrolledWindow,
        Entry,
        Entry,
        Entry,
        Entry,
        SpinButton,
        Entry,
        Entry,
    ) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_hexpand(true);

        let grid = Grid::new();
        grid.set_hexpand(true);
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        let mut row = 0;

        let dia_label = Label::new(Some("Diameter (mm):"));
        dia_label.set_halign(Align::Start);
        let edit_diameter = Entry::new();
        edit_diameter.set_hexpand(true);
        edit_diameter.set_text("6.350");
        grid.attach(&dia_label, 0, row, 1, 1);
        grid.attach(&edit_diameter, 1, row, 1, 1);
        row += 1;

        let len_label = Label::new(Some("Length (mm):"));
        len_label.set_halign(Align::Start);
        let edit_length = Entry::new();
        edit_length.set_hexpand(true);
        edit_length.set_text("50.000");
        grid.attach(&len_label, 0, row, 1, 1);
        grid.attach(&edit_length, 1, row, 1, 1);
        row += 1;

        let flute_label = Label::new(Some("Flute Length (mm):"));
        flute_label.set_halign(Align::Start);
        let edit_flute_length = Entry::new();
        edit_flute_length.set_hexpand(true);
        edit_flute_length.set_text("20.000");
        grid.attach(&flute_label, 0, row, 1, 1);
        grid.attach(&edit_flute_length, 1, row, 1, 1);
        row += 1;

        let shaft_label = Label::new(Some("Shaft Diameter (mm):"));
        shaft_label.set_halign(Align::Start);
        let edit_shaft_diameter = Entry::new();
        edit_shaft_diameter.set_hexpand(true);
        edit_shaft_diameter.set_text("6.350");
        grid.attach(&shaft_label, 0, row, 1, 1);
        grid.attach(&edit_shaft_diameter, 1, row, 1, 1);
        row += 1;

        let flutes_label = Label::new(Some("Number of Flutes:"));
        flutes_label.set_halign(Align::Start);
        let edit_flutes = SpinButton::with_range(1.0, 8.0, 1.0);
        edit_flutes.set_hexpand(true);
        edit_flutes.set_value(2.0);
        grid.attach(&flutes_label, 0, row, 1, 1);
        grid.attach(&edit_flutes, 1, row, 1, 1);
        row += 1;

        let cr_label = Label::new(Some("Corner Radius (mm):"));
        cr_label.set_halign(Align::Start);
        let edit_corner_radius = Entry::new();
        edit_corner_radius.set_hexpand(true);
        edit_corner_radius.set_placeholder_text(Some("Only for corner radius end mills"));
        grid.attach(&cr_label, 0, row, 1, 1);
        grid.attach(&edit_corner_radius, 1, row, 1, 1);
        row += 1;

        let tip_label = Label::new(Some("Tip Angle (deg):"));
        tip_label.set_halign(Align::Start);
        let edit_tip_angle = Entry::new();
        edit_tip_angle.set_hexpand(true);
        edit_tip_angle.set_placeholder_text(Some("Used for drills / V-bits"));
        grid.attach(&tip_label, 0, row, 1, 1);
        grid.attach(&edit_tip_angle, 1, row, 1, 1);

        scroll.set_child(Some(&grid));
        (
            scroll,
            edit_diameter,
            edit_length,
            edit_flute_length,
            edit_shaft_diameter,
            edit_flutes,
            edit_corner_radius,
            edit_tip_angle,
        )
    }

    pub(crate) fn create_manufacturer_tab() -> (ScrolledWindow, Entry, Entry, TextView) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let vbox = Box::new(Orientation::Vertical, 10);
        vbox.set_margin_top(10);
        vbox.set_margin_bottom(10);
        vbox.set_margin_start(10);
        vbox.set_margin_end(10);

        // Manufacturer
        let mfg_grid = Grid::new();
        mfg_grid.set_column_spacing(10);
        let mfg_label = Label::new(Some("Manufacturer:"));
        mfg_label.set_halign(Align::Start);
        let edit_manufacturer = Entry::new();
        edit_manufacturer.set_hexpand(true);
        mfg_grid.attach(&mfg_label, 0, 0, 1, 1);
        mfg_grid.attach(&edit_manufacturer, 1, 0, 1, 1);
        vbox.append(&mfg_grid);

        // Part Number
        let pn_grid = Grid::new();
        pn_grid.set_column_spacing(10);
        let pn_label = Label::new(Some("Part Number:"));
        pn_label.set_halign(Align::Start);
        let edit_part_number = Entry::new();
        edit_part_number.set_hexpand(true);
        pn_grid.attach(&pn_label, 0, 0, 1, 1);
        pn_grid.attach(&edit_part_number, 1, 0, 1, 1);
        vbox.append(&pn_grid);

        // Description
        let desc_frame = Frame::new(Some("Description"));
        let edit_description = TextView::new();
        edit_description.set_wrap_mode(WrapMode::Word);
        edit_description.set_height_request(80);
        let desc_scroll = ScrolledWindow::new();
        desc_scroll.set_child(Some(&edit_description));
        desc_frame.set_child(Some(&desc_scroll));
        vbox.append(&desc_frame);

        scroll.set_child(Some(&vbox));
        (
            scroll,
            edit_manufacturer,
            edit_part_number,
            edit_description,
        )
    }

    pub(crate) fn create_notes_tab() -> (ScrolledWindow, TextView) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);

        let vbox = Box::new(Orientation::Vertical, 10);
        vbox.set_margin_top(10);
        vbox.set_margin_bottom(10);
        vbox.set_margin_start(10);
        vbox.set_margin_end(10);

        let label = Label::new(Some("Additional Notes:"));
        label.set_halign(Align::Start);
        vbox.append(&label);

        let edit_notes = TextView::new();
        edit_notes.set_wrap_mode(WrapMode::Word);
        edit_notes.set_vexpand(true);
        vbox.append(&edit_notes);

        scroll.set_child(Some(&vbox));
        (scroll, edit_notes)
    }
}
