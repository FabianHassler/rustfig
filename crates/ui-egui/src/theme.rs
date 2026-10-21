//! Design tokens: every colour the UI uses comes from here.

use egui::Color32;

#[derive(Debug, Clone, Copy)]
pub struct Tokens {
    pub canvas_bg: Color32,
    pub paper: Color32,
    pub page_border: Color32,
    pub grid: Color32,
    pub guide: Color32,
    pub handle_fill: Color32,
    pub handle_stroke: Color32,
    pub selection: Color32,
    pub caret: Color32,
    pub icon: Color32,
    pub icon_active: Color32,
    pub tool_active_bg: Color32,
    pub error: Color32,
    pub muted: Color32,
    /// Buttons are bevelled (the "3D" button style) rather than flat.
    pub bevel: bool,
    /// Button face: top and bottom of the soft shading, hovered, pressed.
    pub face_top: Color32,
    pub face_bottom: Color32,
    pub face_hover: Color32,
    pub face_pressed: Color32,
    /// Bevel edges: outer and inner ring on the lit (top-left) and shaded
    /// (bottom-right) sides.
    pub edge_light: Color32,
    pub edge_light_in: Color32,
    pub edge_dark: Color32,
    pub edge_dark_in: Color32,
    /// Sunken fields (message panel, file name, indicator boxes).
    pub inset_bg: Color32,
}

impl Tokens {
    /// The tokens for the current theme and button style (`bevel`: 3D).
    pub fn new(dark: bool, bevel: bool) -> Tokens {
        Tokens {
            bevel,
            ..Tokens::for_dark(dark)
        }
    }

    pub fn for_dark(dark: bool) -> Tokens {
        if dark {
            Tokens {
                canvas_bg: Color32::from_rgb(58, 60, 66),
                paper: Color32::WHITE,
                page_border: Color32::from_rgb(110, 160, 230),
                grid: Color32::from_rgba_unmultiplied(80, 110, 160, 70),
                guide: Color32::from_rgb(230, 80, 60),
                handle_fill: Color32::WHITE,
                handle_stroke: Color32::from_rgb(30, 120, 230),
                selection: Color32::from_rgb(30, 120, 230),
                caret: Color32::from_rgb(220, 40, 40),
                icon: Color32::from_gray(215),
                icon_active: Color32::WHITE,
                tool_active_bg: Color32::from_rgb(45, 95, 170),
                error: Color32::from_rgb(255, 120, 110),
                muted: Color32::from_gray(150),
                bevel: true,
                face_top: Color32::from_gray(74),
                face_bottom: Color32::from_gray(58),
                face_hover: Color32::from_gray(84),
                face_pressed: Color32::from_gray(44),
                edge_light: Color32::from_gray(122),
                edge_light_in: Color32::from_gray(94),
                edge_dark: Color32::from_gray(14),
                edge_dark_in: Color32::from_gray(34),
                inset_bg: Color32::from_gray(36),
            }
        } else {
            Tokens {
                canvas_bg: Color32::from_rgb(200, 203, 208),
                paper: Color32::WHITE,
                page_border: Color32::from_rgb(70, 130, 220),
                grid: Color32::from_rgba_unmultiplied(60, 90, 150, 60),
                guide: Color32::from_rgb(220, 60, 40),
                handle_fill: Color32::WHITE,
                handle_stroke: Color32::from_rgb(20, 100, 220),
                selection: Color32::from_rgb(20, 100, 220),
                caret: Color32::from_rgb(200, 20, 20),
                icon: Color32::from_gray(40),
                icon_active: Color32::WHITE,
                tool_active_bg: Color32::from_rgb(60, 120, 210),
                error: Color32::from_rgb(190, 30, 20),
                muted: Color32::from_gray(110),
                bevel: true,
                face_top: Color32::from_gray(246),
                face_bottom: Color32::from_gray(224),
                face_hover: Color32::from_gray(252),
                face_pressed: Color32::from_gray(204),
                edge_light: Color32::WHITE,
                edge_light_in: Color32::from_gray(238),
                edge_dark: Color32::from_gray(96),
                edge_dark_in: Color32::from_gray(170),
                inset_bg: Color32::from_gray(250),
            }
        }
    }
}
