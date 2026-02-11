# ADR-001: GTK4 UI Framework

## Status
Accepted

## Context

GCodeKit5 needed a cross-platform UI framework that could:
- Run natively on Linux, Windows, and macOS
- Provide modern, professional-looking widgets
- Support complex custom drawing (for visualizer and designer)
- Integrate well with Rust's ownership model
- Have an active community and long-term support

The previous version (GCodeKit) used a different technology stack, and a modern rewrite required evaluating available options.

## Decision

We chose **GTK4** via the **gtk-rs** bindings for the UI framework.

### Key Factors

1. **Cross-Platform Native Performance**: GTK4 provides native performance on all major platforms without requiring a browser runtime or virtual machine.

2. **Rust Bindings Quality**: gtk-rs provides high-quality, idiomatic Rust bindings that work well with Rust's ownership and borrowing system.

3. **Custom Drawing**: GTK4's cairo-based drawing API enables the complex 2D rendering needed for the G-code visualizer and CAD designer.

4. **Modern Design**: GTK4 with libadwaita provides modern, adaptive UI components that look professional across platforms.

5. **Linux-First Development**: As the primary development and deployment platform is Linux, GTK4's excellent Linux support was beneficial.

## Consequences

### Positive
- Native performance without runtime overhead
- Excellent Linux integration and appearance
- Strong custom drawing capabilities for visualizer
- Active maintenance and modern features
- Good accessibility support built-in

### Negative
- Windows/macOS appearance less native than platform-specific toolkits
- Learning curve for developers unfamiliar with GTK
- Some advanced features require understanding GObject patterns
- Async integration requires careful handling with GTK's main loop

### Neutral
- Requires GTK4 libraries installed on target systems (or bundled)
- Uses reference counting (Rc/Arc patterns) extensively

## Alternatives Considered

| Framework | Pros | Cons | Decision |
|-----------|------|------|----------|
| **Iced** | Pure Rust, Elm-like | Less mature, limited widgets | Not mature enough |
| **egui** | Immediate mode, simple | Less native feel, redraws constantly | Not suitable for complex UI |
| **Tauri** | Web technologies | Heavy runtime, performance overhead | Unnecessary complexity |
| **Qt** | Mature, native look | Complex licensing, C++ interop | License concerns |
| **Slint** | Modern, Rust-first | Newer, smaller ecosystem | Less ecosystem support |

## References

- [GTK4 Documentation](https://docs.gtk.org/gtk4/)
- [gtk-rs Book](https://gtk-rs.org/gtk4-rs/stable/latest/book/)
- [libadwaita Documentation](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/)
