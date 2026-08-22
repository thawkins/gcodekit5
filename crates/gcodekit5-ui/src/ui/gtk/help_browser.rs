//! GTK help browser with StackSidebar navigation
//!
//! Markdown files are loaded from GResources and displayed with inline images.

use anyhow::{anyhow, Result};
use gio::prelude::*;
use gtk4::gdk_pixbuf::PixbufLoader;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, Box, Button, HeaderBar, Label, Orientation, ScrolledWindow, Stack,
    StackSidebar, TextView, Window, WrapMode,
};
use crate::t;
use std::cell::RefCell;
use std::rc::Rc;

// ============================================================================
// CONSTANTES Y CONFIGURACIÓN
// ============================================================================

fn help_topics() -> Vec<(&'static str, String)> {
    vec![
        ("index", t!("🏠 Index")),
        ("settings", t!("🛠️ Preferences")),

        ("designer", t!("✏️ Designer")),
        ("visualizer", t!("👁️ Visualizer")),
        ("machine_control", t!("⚙️ Machine Control")),
        ("gcode_editor", t!("      •📝 G-code Editor")),
        ("device_console", t!("      •💻 Device console")),
        ("cam_tools", t!("🧰 CAM Tools")),
        ("tabbed_box_maker", t!("      •📦 Tabbed Box Maker")),
        ("jigsaw_puzzle", t!("      •🧩 Jigsaw Puzzle Generator")),
        ("laser_image_engraver", t!("      •🖼️ Laser Image Engraver")),
        ("speeds_feeds_calculator", t!("      •🖩 Speed and Feeds Calculator")),
        ("spoilboard_surfacing", t!("      •🔧 Spoilboard Surfacing")),
        ("spoilboard_grid", t!("      •📏 Create Spoilboard Grid")),
        ("gerber", t!("      •📠 Gerber To G-code")),
        ("drill_press", t!("      •🔧 Dirll Press")),
        ("device_manager", t!("🔌 Device Manager")),
        ("device_config", t!("⚙️ Device Config")),
        ("tools_manager", t!("🔨 CNC Tools")),
        ("materials_manager", t!("📦 Materials")),
    ]
}

// ============================================================================
// FUNCIONES AUXILIARES
// ============================================================================

fn running_app() -> Result<Application> {
    gio::Application::default()
    .and_then(|a| a.downcast::<Application>().ok())
    .ok_or_else(|| anyhow!("No running GtkApplication"))
}

fn get_current_language() -> String {
    let config_path = gcodekit5_settings::SettingsManager::config_file_path()
    .unwrap_or_else(|_| std::path::PathBuf::from("config.json"));

    let language = if config_path.exists() {
        gcodekit5_settings::SettingsPersistence::load_from_file(&config_path)
        .map(|p| p.config().ui.language.clone())
        .unwrap_or_else(|_| "system".to_string())
    } else {
        "system".to_string()
    };

    if language == "system" {
        let locale = std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string());
        let lang = locale.split('_').next().unwrap_or("en");
        lang.to_string()
    } else {
        language
    }
}

fn topic_resource_path(topic: &str, language: &str) -> String {
    let topic = topic.trim();
    let topic = if topic.is_empty() { "index" } else { topic };
    let lang = if language.is_empty() { "en" } else { language };
    format!("/com/gcodekit5/help/{}/{}.md", lang, topic)
}

fn load_topic_markdown(topic: &str) -> Result<String> {
    let preferred_lang = get_current_language();
    let path = topic_resource_path(topic, &preferred_lang);

    match gio::resources_lookup_data(&path, gio::ResourceLookupFlags::NONE) {
        Ok(bytes) => {
            let s = std::str::from_utf8(bytes.as_ref())
            .map_err(|e| anyhow!("Invalid UTF-8 in {}: {}", path, e))?;
            Ok(s.to_string())
        }
        Err(_) => {
            if preferred_lang != "en" {
                let fallback_path = topic_resource_path(topic, "en");
                if let Ok(bytes) = gio::resources_lookup_data(&fallback_path, gio::ResourceLookupFlags::NONE)
                {
                    let s = std::str::from_utf8(bytes.as_ref())
                    .map_err(|e| anyhow!("Invalid UTF-8 in {}: {}", fallback_path, e))?;
                    tracing::info!("Using English fallback for topic '{}'", topic);
                    return Ok(s.to_string());
                }
            }
            Err(anyhow!(
                "Missing help topic resource: {} for language {}",
                topic,
                preferred_lang
            ))
        }
    }
}

// ============================================================================
// RENDERIZADO DE MARKDOWN CON IMÁGENES INLINE
// ============================================================================

fn extract_image_src(line: &str) -> Option<(String, i32)> {
    // Buscar <img src="..." width="...">
    if let Some(src_start) = line.find("src=\"") {
        let src_start = src_start + 5;
        if let Some(src_end) = line[src_start..].find('\"') {
            let src = &line[src_start..src_start + src_end];

            // Asegurar que la ruta tiene extensión .png (si no la tiene)
            let src_with_ext = if !src.contains('.') {
                format!("{}.png", src)
            } else {
                src.to_string()
            };

            // Extraer width (opcional)
            let width = if let Some(w_start) = line.find("width=\"") {
                let w_start = w_start + 7;
                if let Some(w_end) = line[w_start..].find('\"') {
                    line[w_start..w_start + w_end].parse::<i32>().unwrap_or(300)
                } else {
                    300
                }
            } else {
                300
            };

            return Some((src_with_ext, width));
        }
    }
    None
}

fn insert_inline_image(
    buffer: &gtk4::TextBuffer,
    view: &TextView,
    image_path: &str,
    max_width: i32,
) {

    // Normalizar ruta y asegurar extensión .png
    let resource_path = if image_path.starts_with("resource://") {
        let path = image_path.strip_prefix("resource://").unwrap_or(image_path);
        if !path.contains('.') {
            format!("{}.png", path)
        } else {
            path.to_string()
        }
    } else if image_path.starts_with("../../help_images/") {
        let img_name = image_path.trim_start_matches("../../help_images/");
        if !img_name.contains('.') {
            format!("/com/gcodekit5/help/help_images/{}.png", img_name)
        } else {
            format!("/com/gcodekit5/help/help_images/{}", img_name)
        }
    } else if image_path.starts_with("help_images/") {
        let img_name = image_path.trim_start_matches("help_images/");
        if !img_name.contains('.') {
            format!("/com/gcodekit5/help/help_images/{}.png", img_name)
        } else {
            format!("/com/gcodekit5/help/help_images/{}", img_name)
        }
    } else {
        let path = image_path;
        if !path.contains('.') {
            format!("/com/gcodekit5/help/help_images/{}.png", path)
        } else {
            path.to_string()
        }
    };

    match gio::resources_lookup_data(&resource_path, gio::ResourceLookupFlags::NONE) {
        Ok(bytes) => {
            // Crear un buffer de texto para la imagen
            let loader = PixbufLoader::new();
            if loader.write(bytes.as_ref()).is_err() {
                return;
            }
            if loader.close().is_err() {
                return;
            }

            if let Some(pixbuf) = loader.pixbuf() {
                let scale = if pixbuf.width() > max_width {
                    max_width as f64 / pixbuf.width() as f64
                } else {
                    1.0
                };
                let new_height = (pixbuf.height() as f64 * scale) as i32;
                let scaled = pixbuf.scale_simple(max_width, new_height, gtk4::gdk_pixbuf::InterpType::Bilinear);

                if let Some(pixbuf) = scaled {
                    // Crear la imagen
                    let picture = gtk4::Picture::for_pixbuf(&pixbuf);
                    picture.set_margin_top(5);
                    picture.set_margin_bottom(5);
                    picture.set_margin_start(5);
                    picture.set_margin_end(5);

                    // Forzar tamaño
                    picture.set_width_request(max_width);
                    picture.set_height_request(new_height);

                    // Insertar en el texto
                    let mut iter = buffer.end_iter();
                    let anchor = buffer.create_child_anchor(&mut iter);
                    view.add_child_at_anchor(&picture, &anchor);

                    // Añadir un salto de línea después de la imagen
                    buffer.insert_at_cursor("\n");

                    // Forzar actualización del TextView
                    view.queue_draw();
                }
            }
        }
        Err(_) => {
            // Ignorar errores de imagen
        }
    }
}

// Crear tag para un link y retornar el topic_id
fn create_link_tag(buffer: &gtk4::TextBuffer, topic_id: &str) -> gtk4::TextTag {
    let tag_name = format!("link-{}", topic_id);
    let tag = buffer.create_tag(Some(&tag_name), &[]).unwrap();
    
    // Aplicar color azul y subrayado
    let color = gtk4::gdk::RGBA::new(0.0, 0.4, 0.8, 1.0);
    tag.set_foreground_rgba(Some(&color));
    tag.set_underline(gtk4::pango::Underline::Single);
    tag
}

fn render_markdown(view: &TextView, md: &str, stack: &Stack) {
    let buffer = view.buffer();
    buffer.set_text("");

    // Crear tags con la sintaxis correcta
    let h1_tag = buffer.create_tag(Some("h1"), &[]).unwrap();
    h1_tag.set_weight(700); // Bold
    h1_tag.set_scale(1.6);

    let h2_tag = buffer.create_tag(Some("h2"), &[]).unwrap();
    h2_tag.set_weight(700); // Bold
    h2_tag.set_scale(1.4);

    let h3_tag = buffer.create_tag(Some("h3"), &[]).unwrap();
    h3_tag.set_weight(700); // Bold
    h3_tag.set_scale(1.2);
    
    // Conectar eventos de mouse para links
    setup_link_navigation(view, stack);

    let mut i = 0;
    let lines: Vec<&str> = md.lines().collect();

    while i < lines.len() {
        let line = lines[i].trim();

        // Headings con formato
        if let Some(text) = line.strip_prefix("### ") {
            let mut iter = buffer.end_iter();
            buffer.insert_with_tags(&mut iter, &format!("{}\n\n", text), &[&h3_tag]);
            i += 1;
            continue;
        }
        if let Some(text) = line.strip_prefix("## ") {
            let mut iter = buffer.end_iter();
            buffer.insert_with_tags(&mut iter, &format!("{}\n\n", text), &[&h2_tag]);
            i += 1;
            continue;
        }
        if let Some(text) = line.strip_prefix("# ") {
            let mut iter = buffer.end_iter();
            buffer.insert_with_tags(&mut iter, &format!("{}\n\n", text), &[&h1_tag]);
            i += 1;
            continue;
        }

        // List items - procesar links dentro
        if let Some(text) = line.strip_prefix("- ") {
            if text.contains("](help:") {
                // Procesar línea con posibles links
                buffer.insert_at_cursor("  • ");
                process_line_with_links(&buffer, view, text, stack);
                buffer.insert_at_cursor("\n");
            } else {
                buffer.insert_at_cursor(&format!("  • {}\n", text));
            }
            i += 1;
            continue;
        }

        // Separador horizontal
        if line.trim() == "---" {
            buffer.insert_at_cursor("────────────────────────────────────────────\n\n");
            i += 1;
            continue;
        }

        // Imagen HTML
        if line.contains("<img") {
            if let Some((src, width)) = extract_image_src(line) {
                insert_inline_image(&buffer, view, &src, width);
            }
            i += 1;
            continue;
        }

        // Tabla HTML
        if line.contains("<table") {
            let mut table_html = String::new();
            let mut depth = 1;

            while i < lines.len() && depth > 0 {
                let current_line = lines[i];
                table_html.push_str(current_line);
                table_html.push('\n');

                if current_line.contains("<table") && !current_line.contains("<table>") {
                    depth += 1;
                }
                if current_line.contains("</table>") {
                    depth -= 1;
                }
                i += 1;
            }

            let cleaned = clean_html_tags(&table_html);
            if !cleaned.is_empty() {
                buffer.insert_at_cursor(&cleaned);
                buffer.insert_at_cursor("\n");
            }
            continue;
        }

        // Texto normal - procesar links dentro
        if line.contains("](help:") {
            process_line_with_links(&buffer, view, line, stack);
        } else {
            let processed_line = process_line_for_display(line);
            if !processed_line.is_empty() {
                buffer.insert_at_cursor(&processed_line);
            }
        }
        buffer.insert_at_cursor("\n");
        i += 1;
    }
}

// Procesar una línea que contiene links con GtkTextTag
fn process_line_with_links(buffer: &gtk4::TextBuffer, _view: &TextView, line: &str, _stack: &Stack) {
    let mut pos = 0;

    while let Some(bracket_start) = line[pos..].find('[') {
        let abs_bracket = pos + bracket_start;

        // Insertar texto antes del link
        if abs_bracket > pos {
            buffer.insert_at_cursor(&line[pos..abs_bracket]);
        }

        if let Some(bracket_end) = line[abs_bracket + 1..].find(']') {
            let abs_bracket_end = abs_bracket + 1 + bracket_end;
            let link_text = &line[abs_bracket + 1..abs_bracket_end];

            if abs_bracket_end + 1 < line.len() && &line[abs_bracket_end + 1..abs_bracket_end + 2] == "(" {

                if let Some(paren_end) = line[abs_bracket_end + 2..].find(')') {
                    let abs_paren_end = abs_bracket_end + 2 + paren_end;

                    let link_target = &line[abs_bracket_end + 2..abs_paren_end];

                    if link_target.starts_with("help:") {
                        let topic_id = link_target.strip_prefix("help:").unwrap_or("");

                        // Crear tag para este link
                        let tag = create_link_tag(buffer, topic_id);

                        // Insertar texto del link con tag
                        let mut iter = buffer.end_iter();
                        buffer.insert_with_tags(&mut iter, link_text, &[&tag]);

                        pos = abs_paren_end + 1;
                        continue;
                    }
                }
            }
        }

        buffer.insert_at_cursor(&line[abs_bracket..abs_bracket + 1]);
        pos = abs_bracket + 1;
    }

    // Insertar resto del texto
    if pos < line.len() {
        buffer.insert_at_cursor(&line[pos..]);
    }
}

// Configurar navegación por links mediante eventos de mouse
fn setup_link_navigation(view: &TextView, stack: &Stack) {
    let stack_clone = stack.clone();
    let view_clone = view.clone();
    
    // Crear gesture para detectar clics
    let gesture = gtk4::GestureClick::new();
    gesture.set_button(1); // Click izquierdo
    
    gesture.connect_pressed(move |gesture, _n_press, x, y| {
        let view = &view_clone;
        
        // Obtener la posición en el buffer desde las coordenadas
        let (bx, by) = view.window_to_buffer_coords(gtk4::TextWindowType::Widget, x as i32, y as i32);
        if let Some(iter) = view.iter_at_location(bx, by) {
            // Obtener todos los tags en esta posición
            let tags = iter.tags();
            for tag in tags {
                if let Some(tag_name) = tag.name() {
                    if tag_name.starts_with("link-") {
                        let topic_id = tag_name.strip_prefix("link-").unwrap_or("");
                        if let Some(child) = stack_clone.child_by_name(topic_id) {
                            stack_clone.set_visible_child(&child);
                        }
                        gesture.set_state(gtk4::EventSequenceState::Claimed);
                        return;
                    }
                }
            }
        }
        gesture.set_state(gtk4::EventSequenceState::Denied);
    });
    
    view.add_controller(gesture);
}

fn strip_markdown_links(text: &str) -> String {
    let mut result = String::new();
    let remaining = text;
    let mut last_pos = 0;

    while let Some(bracket_start) = remaining[last_pos..].find('[') {
        let abs_start = last_pos + bracket_start;
        result.push_str(&remaining[last_pos..abs_start]);

        if let Some(bracket_end) = remaining[abs_start + 1..].find(']') {
            let abs_end = abs_start + 1 + bracket_end;
            let link_text = &remaining[abs_start + 1..abs_end];

            if remaining.len() > abs_end + 1 && &remaining[abs_end + 1..abs_end + 2] == "(" {
                if let Some(paren_end) = remaining[abs_end + 2..].find(')') {
                    result.push_str(link_text);
                    last_pos = abs_end + 2 + paren_end + 1;
                    continue;
                }
            }
        }

        result.push_str(&remaining[abs_start..]);
        break;
    }

    result.push_str(&remaining[last_pos..]);
    result
}

// ============================================================================
// CREACIÓN DE PÁGINAS DE AYUDA
// ============================================================================
fn create_help_page(topic_id: &str, _title: &str, stack: &Stack) -> (String, ScrolledWindow) {
    let text_view = TextView::new();
    text_view.set_wrap_mode(WrapMode::Word);
    text_view.set_editable(false);
    text_view.set_cursor_visible(false);
    text_view.set_left_margin(15);
    text_view.set_right_margin(15);
    text_view.set_top_margin(15);
    text_view.set_bottom_margin(15);
    text_view.add_css_class("help-text");

    // === NUEVO: CONTROLADOR PARA CAMBIAR EL CURSOR A MANO ("POINTER") ===
    let motion_controller = gtk4::EventControllerMotion::new();
    let view_clone = text_view.clone();

    motion_controller.connect_motion(move |_, x, y| {
        let mut is_over_link = false;

        // Convertir píxeles de la ventana a coordenadas internas del texto
        let (buffer_x, buffer_y) = view_clone.window_to_buffer_coords(
            gtk4::TextWindowType::Widget,
            x as i32,
            y as i32,
        );

        // Obtener la posición del texto bajo el puntero
        if let Some(iter) = view_clone.iter_at_location(buffer_x, buffer_y) {
            // Recorrer las etiquetas de este carácter
            for tag in iter.tags() {
                if let Some(name) = tag.name() {
                    // Si el tag empieza por "link-", el ratón está sobre un enlace
                    if name.starts_with("link-") {
                        is_over_link = true;
                        break;
                    }
                }
            }
        }

        // Cambiar el diseño del cursor
        if is_over_link {
            view_clone.set_cursor(gtk4::gdk::Cursor::from_name("pointer", None).as_ref());
        } else {
            view_clone.set_cursor(None); // Restaura el cursor por defecto
        }
    });

    // Añadir el detector de movimiento al TextView
    text_view.add_controller(motion_controller);
    // ===================================================================

    // Cargar y renderizar contenido
    match load_topic_markdown(topic_id) {
        Ok(md) => {
            render_markdown(&text_view, &md, stack);
        }
        Err(e) => {
            text_view
            .buffer()
            .set_text(&format!("Error loading help topic '{}':\n{}", topic_id, e));
        }
    }

    let scroller = ScrolledWindow::builder()
    .child(&text_view)
    .hscrollbar_policy(gtk4::PolicyType::Never)
    .build();

    (topic_id.to_string(), scroller)
}

// ============================================================================
// FUNCIONES PÚBLICAS
// ============================================================================

/// Show the help browser with StackSidebar navigation
pub fn present(topic: &str) {
    let Ok(app) = running_app() else {
        return;
    };
    let parent = app.active_window();
    present_for_parent(topic, parent.as_ref());
}

/// Show the help browser with an explicit parent window
pub fn present_for_parent(initial_topic: &str, parent: Option<&Window>) {
    let Ok(app) = running_app() else {
        return;
    };

    let window = Window::builder()
    .application(&app)
    .title(&t!("GCodeKit5 Help"))
    .default_width(1000)
    .default_height(700)
    .build();

    if let Some(parent) = parent {
        window.set_transient_for(Some(parent));
        window.set_modal(false);
    }

    // =========================================================================
    // 1. ESTRUCTURA DEL HISTORIAL DE NAVEGACIÓN
    // =========================================================================
    struct HelpHistory {
        back_stack: Vec<String>,
        forward_stack: Vec<String>,
        current: String,
    }

    let history = Rc::new(RefCell::new(HelpHistory {
        back_stack: Vec::new(),
        forward_stack: Vec::new(),
        current: initial_topic.to_string(),
    }));

    // =========================================================================
    // 2. HEADER BAR Y BOTONES ATRÁS/ADELANTE
    // =========================================================================
    let header = HeaderBar::new();
    window.set_titlebar(Some(&header));

    // Botón Atrás
    let btn_back = Button::from_icon_name("go-previous-symbolic");
    btn_back.set_tooltip_text(Some(&t!("Back")));
    btn_back.set_sensitive(false);
    header.pack_start(&btn_back);

    // Botón Adelante
    let btn_forward = Button::from_icon_name("go-next-symbolic");
    btn_forward.set_tooltip_text(Some(&t!("Forward")));
    btn_forward.set_sensitive(false);
    header.pack_start(&btn_forward);

    // Título
    let title_label = Label::new(Some(&t!("Help")));
    title_label.set_halign(Align::Center);
    header.set_title_widget(Some(&title_label));

    // CSS para estilo consistente (sin pango)
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(
        "
        .help-sidebar {
            background-color: @theme_bg_color;
            border-right: 1px solid @borders;
        }

        /* Forzar fuente sans-serif en TextView */
        textview {
            font-family: sans-serif;
            font-size: 11pt;
        }

        textview text {
            font-family: sans-serif;
            font-size: 11pt;
        }
        ",
    );

    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // Contenedor principal
    let main_box = Box::new(Orientation::Horizontal, 0);

    // Sidebar para navegación
    let sidebar = StackSidebar::new();
    sidebar.add_css_class("help-sidebar");
    sidebar.set_width_request(220);

    // Stack para las páginas de contenido
    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
    stack.set_transition_duration(200);

    // Cargar todas las páginas de ayuda y guardar referencia a la primera
    let first_page: Option<gtk4::Widget> = None;

    for (topic_id, title) in help_topics() {
        let (_, scroller) = create_help_page(topic_id, &title, &stack);
        stack.add_titled(&scroller, Some(topic_id), &title);
    }

    // Conectar sidebar con stack
    sidebar.set_stack(&stack);

    // =========================================================================
    // 3. LÓGICA DE CONEXIÓN DE LOS BOTONES
    // =========================================================================
    let stack_clone = stack.clone();
    let history_clone = history.clone();
    let btn_back_clone = btn_back.clone();
    let btn_forward_clone = btn_forward.clone();

    btn_back.connect_clicked(move |_| {
        let prev = {
            // Abrimos y cerramos el préstamo mutuo inmediatamente en este bloque corto
            let mut hist = history_clone.borrow_mut();
            if let Some(prev_page) = hist.back_stack.pop() {
                let current_clone = hist.current.clone();
                hist.forward_stack.push(current_clone);
                hist.current = prev_page.clone();
                Some(prev_page)
            } else {
                None
            }
        }; // Aquí 'hist' queda completamente liberado

        if let Some(page) = prev {
            stack_clone.set_visible_child_name(&page);
            // Actualizamos los botones de forma segura
            let hist = history_clone.borrow();
            btn_back_clone.set_sensitive(!hist.back_stack.is_empty());
            btn_forward_clone.set_sensitive(!hist.forward_stack.is_empty());
        }
    });

    let stack_clone = stack.clone();
    let history_clone = history.clone();
    let btn_back_clone = btn_back.clone();
    let btn_forward_clone = btn_forward.clone();

    btn_forward.connect_clicked(move |_| {
        let next = {
            // Abrimos y cerramos el préstamo mutuo inmediatamente en este bloque corto
            let mut hist = history_clone.borrow_mut();
            if let Some(next_page) = hist.forward_stack.pop() {
                let current_clone = hist.current.clone();
                hist.back_stack.push(current_clone);
                hist.current = next_page.clone();
                Some(next_page)
            } else {
                None
            }
        }; // Aquí 'hist' queda completamente liberado

        if let Some(page) = next {
            stack_clone.set_visible_child_name(&page);
            // Actualizamos los botones de forma segura
            let hist = history_clone.borrow();
            btn_back_clone.set_sensitive(!hist.back_stack.is_empty());
            btn_forward_clone.set_sensitive(!hist.forward_stack.is_empty());
        }
    });

    // =========================================================================
    // 4. DETECTAR NAVEGACIÓN MANUAL (SI SE PINCHA EN EL SIDEBAR O EN LOS LINKS)
    // =========================================================================
    let history_clone = history.clone();
    let btn_back_clone = btn_back.clone();
    let btn_forward_clone = btn_forward.clone();

    stack.connect_visible_child_name_notify(move |s| {
        if let Some(new_page) = s.visible_child_name() {
            let new_page_str = new_page.to_string();

            // Primero leemos para comprobar si realmente cambió la página
            let un_cambio_real = history_clone.borrow().current != new_page_str;

            if un_cambio_real {
                let mut hist = history_clone.borrow_mut();

                // Comprobamos si la página nueva coincide con la que tocaría por historial
                // (Si coincide, significa que el cambio viene de los botones, no del usuario pulsando un link)
                let es_boton_atras = hist.back_stack.last().map_or(false, |p| p == &new_page_str);
                let es_boton_adelante = hist.forward_stack.last().map_or(false, |p| p == &new_page_str);

                if !es_boton_atras && !es_boton_adelante {
                    // El usuario ha pinchado un link o el sidebar: guardamos historial manual
                    let current_clone = hist.current.clone();
                    hist.back_stack.push(current_clone);
                    hist.current = new_page_str;
                    hist.forward_stack.clear(); // Limpiamos adelantes al divergir
                }

                // Actualizar estado visual de los botones
                btn_back_clone.set_sensitive(!hist.back_stack.is_empty());
                btn_forward_clone.set_sensitive(!hist.forward_stack.is_empty());
            }
        }
    });



    // Seleccionar página inicial
    if let Some(child) = stack.child_by_name(initial_topic) {
        stack.set_visible_child(&child);
    } else if let Some(first) = first_page {
        stack.set_visible_child(&first);
    }

    // Organizar en la ventana
    main_box.append(&sidebar);
    main_box.append(&stack);

    window.set_child(Some(&main_box));
    window.present();
}

/// Create a help button that opens the help browser
pub fn make_help_button(topic: &'static str) -> Button {
    let btn = Button::from_icon_name("dialog-question-symbolic");
    btn.set_tooltip_text(Some(&t!("Help (F1)")));

    let topic = topic.to_string();
    btn.connect_clicked(move |_| {
        present(&topic);
    });

    btn
}

// Limpiar etiquetas HTML y convertirlas a formato texto
#[allow(unused_assignments)]
#[allow(unused_variables)]
fn clean_html_tags(text: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_ul = false;  // Quita el underscore
    let mut skip_content = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            //            in_tag = true;
            skip_content = false;

            // Detectar etiquetas especiales
            let mut tag = String::new();
            let mut peek = chars.clone();
            while let Some(&next) = peek.peek() {
                if next == '>' {
                    break;
                }
                tag.push(next);
                peek.next();
            }

            if tag.starts_with("ul") || tag == "/ul" {
                in_ul = tag != "/ul";
                if !in_ul {
                    result.push('\n');
                }
            } else if tag.starts_with("li") {
                result.push_str("  • ");
            } else if tag.starts_with("tr") || tag.starts_with("/tr") {
                result.push('\n');
            } else if tag.starts_with("td") {
                result.push_str("  ");
            } else if tag.starts_with("/td") {
                result.push_str("  ");
            } else if tag.starts_with("table") || tag.starts_with("/table") {
                result.push_str("\n");
            } else if tag.starts_with("p") {
                result.push_str("\n");
            } else if tag == "br" || tag == "br/" {
                result.push_str("\n");
            } else if tag.starts_with("span") {
                // Ignorar spans
            } else if tag.starts_with("div") {
                result.push_str("\n");
            } else {
                // Otras etiquetas: ignorar
            }

            // Avanzar hasta el cierre de la etiqueta
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == '>' {
                    break;
                }
            }
            in_tag = false;
            continue;
        }

        if !in_tag && !skip_content {
            result.push(c);
        }
    }

    // Limpiar espacios múltiples y líneas vacías excesivas
    let cleaned = result
    .lines()
    .filter(|line| !line.trim().is_empty())
    .collect::<Vec<_>>()
    .join("\n");

    cleaned
}

// Procesar una línea de markdown/HTML
fn process_line_for_display(line: &str) -> String {
    let line = line.trim();

    // Saltar líneas que son parte de una tabla
    if line.contains("<table") || line.contains("</table>") ||
        line.contains("<tr") || line.contains("</tr>") ||
        line.contains("<td") || line.contains("</td>") {
            return String::new();
        }
        // Si no contiene HTML, devolver el texto limpio de links
        if !line.contains('<') {
            return strip_markdown_links(line);
        }

        // Limpiar HTML y convertirlo a texto
        clean_html_tags(line)
}



