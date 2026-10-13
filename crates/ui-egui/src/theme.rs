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
}

impl Tokens {
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
            }
        }
    }
}
