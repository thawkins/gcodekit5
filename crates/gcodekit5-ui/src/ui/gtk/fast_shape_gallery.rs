//! # Fast Shape Gallery
//!
//! Provides a gallery interface for quick insertion of parametric shapes with predefined parameters.

use crate::t;
use gcodekit5_core::{shared_none, SharedOption};
use gcodekit5_designer::model::{
    DesignGear, DesignPath, DesignSprocket, ParametricPathSource, Point, Shape,
};
use gcodekit5_designer::parametric_shapes::*;
use gtk4::prelude::*;
use gtk4::{Box, Dialog, FlowBox, Frame, Image, Label, Orientation, ResponseType, ScrolledWindow};
use std::cell::RefCell;
use std::rc::Rc;

/// Fast shape template definition
#[derive(Clone)]
pub struct FastShapeTemplate {
    pub name: String,
    pub description: String,
    pub icon_name: String,
    pub generator: Rc<dyn Fn(Point) -> Shape>,
}

impl FastShapeTemplate {
    pub fn new(
        name: String,
        description: String,
        icon_name: String,
        generator: Rc<dyn Fn(Point) -> Shape>,
    ) -> Self {
        Self {
            name,
            description,
            icon_name,
            generator,
        }
    }
}

/// Fast shape gallery dialog
#[allow(clippy::type_complexity)]
pub struct FastShapeGallery {
    dialog: Dialog,
    templates: Vec<FastShapeTemplate>,
    on_shape_selected: SharedOption<std::boxed::Box<dyn Fn(Shape) + 'static>>,
}

impl FastShapeGallery {
    pub fn new() -> Rc<Self> {
        let dialog = Dialog::builder()
            .title(t!("Fast Shapes"))
            .modal(true)
            .resizable(true)
            .default_width(600)
            .default_height(400)
            .build();

        dialog.add_button(&t!("Close"), ResponseType::Close);
        dialog.connect_response(|d, _| d.hide());

        let content = Box::new(Orientation::Vertical, 12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);

        let header = Label::new(Some(&t!("Fast Shape Gallery")));
        header.add_css_class("title-3");
        header.set_halign(gtk4::Align::Start);
        content.append(&header);

        let description = Label::new(Some(&t!(
            "Click on a shape to quickly insert it with default parameters."
        )));
        description.set_halign(gtk4::Align::Start);
        description.add_css_class("caption");
        content.append(&description);

        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .build();

        let flow_box = FlowBox::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .selection_mode(gtk4::SelectionMode::None)
            .column_spacing(12)
            .row_spacing(12)
            .homogeneous(true)
            .build();

        let mut gallery = Rc::new(Self {
            dialog: dialog.clone(),
            templates: Vec::new(),
            on_shape_selected: shared_none(),
        });

        // Create templates
        let templates = Self::create_templates();

        for template in &templates {
            let item_box = Box::new(Orientation::Vertical, 6);
            item_box.set_margin_start(8);
            item_box.set_margin_end(8);
            item_box.set_margin_top(8);
            item_box.set_margin_bottom(8);

            // Icon
            let icon_name = &template.icon_name;
            let icon = if icon_name.ends_with(".svg") {
                let resource_path = format!("/com/gcodekit5/icons/{}", icon_name);
                Image::from_resource(&resource_path)
            } else {
                Image::from_icon_name(icon_name)
            };
            icon.set_pixel_size(48);
            item_box.append(&icon);

            // Name
            let name_label = Label::new(Some(&template.name));
            name_label.add_css_class("caption-heading");
            name_label.set_halign(gtk4::Align::Center);
            item_box.append(&name_label);

            // Description
            let desc_label = Label::new(Some(&template.description));
            desc_label.set_halign(gtk4::Align::Center);
            desc_label.set_wrap(true);
            desc_label.set_max_width_chars(20);
            desc_label.add_css_class("caption");
            item_box.append(&desc_label);

            let frame = Frame::new(None);
            frame.set_child(Some(&item_box));
            frame.add_css_class("card");

            let template_clone = template.clone();
            let dialog_clone = dialog.clone();
            // GTK callback closure type inherently complex.
            #[allow(clippy::type_complexity)]
            let on_shape_selected: std::rc::Weak<
                RefCell<Option<std::boxed::Box<dyn Fn(Shape) + 'static>>>,
            > = Rc::downgrade(&gallery.on_shape_selected);

            // Make the frame clickable
            let gesture = gtk4::GestureClick::new();
            gesture.connect_pressed(move |_, _, _, _| {
                // Generate shape at origin (0,0) - designer will handle placement
                let shape = (template_clone.generator)(Point::new(0.0, 0.0));

                // Call the callback if set
                if let Some(callback_ref) = on_shape_selected.upgrade() {
                    if let Some(callback) = callback_ref.borrow().as_ref() {
                        callback(shape);
                    }
                }

                dialog_clone.hide();
            });

            frame.add_controller(gesture);
            flow_box.insert(&frame, -1);
        }

        scrolled.set_child(Some(&flow_box));
        content.append(&scrolled);

        gallery.dialog.content_area().append(&content);

        // Set templates after dialog is built
        if let Some(gallery_ref) = Rc::get_mut(&mut gallery) {
            gallery_ref.templates = templates;
        }

        gallery
    }

    fn create_templates() -> Vec<FastShapeTemplate> {
        vec![
            // Mechanical shapes
            FastShapeTemplate::new(
                t!("Spur Gear"),
                t!("Standard involute spur gear"),
                "gear.svg".to_string(),
                Rc::new(|center| {
                    Shape::Gear(DesignGear::new(
                        center, 2.0, // module
                        20,  // teeth
                    ))
                }),
            ),

            FastShapeTemplate::new(
                t!("Sprocket"),
                t!("Chain sprocket for #40 chain"),
                "sprocket1.svg".to_string(),
                Rc::new(|center| {
                    Shape::Sprocket(DesignSprocket::new(
                        center, 12.7, // pitch (ANSI 40)
                        15,   // teeth
                    ))
                }),
            ),


            FastShapeTemplate::new(
                t!("Timing Pulley"),
                t!("XL timing belt pulley"),
                "timingpulley.svg".to_string(),
                Rc::new(|center| {
                    let pitch = 5.08;
                    let teeth = 20;
                    let belt_width = 9.4;
                    let hole_radius = 5.0;
                    let mut path = DesignPath::from_lyon_path(&generate_timing_pulley(
                        center,
                        pitch,
                        teeth,
                        belt_width,
                        hole_radius,
                    ));
                    path.parametric_source = Some(ParametricPathSource::TimingPulley {
                        pitch,
                        teeth,
                        belt_width,
                        hole_radius,
                    });
                    Shape::Path(path)
                }),
            ),
            // Structural shapes
            FastShapeTemplate::new(
                t!("Slot"),
                t!("Rectangular slot/cutout"),
                "slot.svg".to_string(),
                Rc::new(|center| {
                    use gcodekit5_designer::model::DesignRectangle as Rectangle;

                    let width = 50.0;  // length
                    let height = 20.0; // width

                    // Calculamos la esquina superior izquierda a partir del centro recibido
                    let x = center.x - width / 2.0;
                    let y = center.y - height / 2.0;

                    let mut rect = Rectangle::new(x, y, width, height);
                    rect.corner_radius = 5.0; // El radio pasa a ser numérico y explícito
                    rect.is_slot = true;      // Activar el modo Slot nativo del motor

                    Shape::Rectangle(rect)
                }),
            ),
            FastShapeTemplate::new(
                t!("L-Bracket"),
                t!("L-shaped bracket with holes"),
                "Lsupport.svg".to_string(),
                Rc::new(|center| {
                    Shape::Path(DesignPath::from_lyon_path(&generate_l_bracket(
                        center, 80.0, // width
                        60.0, // height
                        5.0,  // thickness
                        2.5,  // hole diameter
                        15.0, // hole spacing
                    )))
                }),
            ),
            FastShapeTemplate::new(
                t!("U-Bracket"),
                t!("U-shaped channel bracket"),
                "Usupport.svg".to_string(),
                Rc::new(|center| {
                    Shape::Path(DesignPath::from_lyon_path(&generate_u_bracket(
                        center, 100.0, // length
                        40.0,  // width
                        5.0,   // thickness
                        2.5,   // hole diameter
                        15.0,  // hole spacing
                    )))
                }),
            ),
        ]
    }

    pub fn show(&self, parent: &impl IsA<gtk4::Window>) {
        self.dialog.set_transient_for(Some(parent));
        self.dialog.present();
    }

    pub fn connect_shape_selected<F>(&self, callback: F)
    where
        F: Fn(Shape) + 'static,
    {
        *self.on_shape_selected.borrow_mut() = Some(std::boxed::Box::new(callback));
    }
}
