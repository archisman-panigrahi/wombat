use adw::prelude::*;

const APP_CSS: &str = r#"
@define-color calc_case #2f3440;
@define-color calc_case_edge #151922;
@define-color calc_panel #111827;
@define-color calc_display #0b1220;
@define-color calc_display_glow #1dd6bd;
@define-color calc_key #e5e7eb;
@define-color calc_key_text #172033;
@define-color calc_operator #fb923c;
@define-color calc_function #aeb7c4;
@define-color calc_accent #14b8a6;
@define-color calc_magenta #f472b6;

window.wombat-window.wombat-calculator-look {
    background: radial-gradient(circle at 20% 0%, rgba(20, 184, 166, 0.22), transparent 32%),
                radial-gradient(circle at 88% 12%, rgba(244, 114, 182, 0.16), transparent 30%),
                radial-gradient(circle at 55% 100%, rgba(56, 189, 248, 0.14), transparent 36%),
                linear-gradient(135deg, #1b2130, #0f172a 52%, #252132);
    color: #f8fafc;
}

.wombat-calculator-look headerbar.wombat-chrome {
    background: linear-gradient(180deg, rgba(73, 82, 99, 0.86), rgba(31, 41, 55, 0.82));
    color: #f8fafc;
    box-shadow: inset 0 1px rgba(255, 255, 255, 0.12),
                inset 0 -1px rgba(255, 255, 255, 0.07),
                0 12px 28px rgba(0, 0, 0, 0.22);
}

.wombat-calculator-look .wombat-title {
    color: #cffafe;
    font-weight: 800;
    text-shadow: 0 0 18px rgba(103, 232, 249, 0.70);
}

.wombat-calculator-look headerbar.wombat-chrome button,
.wombat-calculator-look headerbar.wombat-chrome button image {
    color: #f8fafc;
}

.wombat-calculator-look headerbar.wombat-chrome button:hover {
    background: rgba(103, 232, 249, 0.14);
    color: #ecfeff;
}

.wombat-calculator-look headerbar.wombat-chrome button:focus,
.wombat-calculator-look headerbar.wombat-chrome button:focus-visible {
    background: rgba(20, 184, 166, 0.18);
    color: #ecfeff;
    box-shadow: inset 0 0 0 1px rgba(125, 211, 252, 0.28);
}

.wombat-calculator-look .wombat-workspace {
    background: radial-gradient(circle at 12% 0%, rgba(103, 232, 249, 0.16), transparent 36%),
                linear-gradient(180deg, rgba(75, 85, 99, 0.74), rgba(31, 41, 55, 0.84));
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 26px;
    padding: 16px;
    box-shadow: inset 0 1px rgba(255, 255, 255, 0.18),
                inset 0 -24px 48px rgba(15, 23, 42, 0.28),
                0 28px 72px rgba(0, 0, 0, 0.42),
                0 0 64px rgba(20, 184, 166, 0.12);
}

.wombat-calculator-look .wombat-history-frame,
.wombat-calculator-look .wombat-completions-frame {
    background: radial-gradient(circle at 18% 0%, rgba(29, 214, 189, 0.11), transparent 34%),
                linear-gradient(180deg, rgba(15, 23, 42, 0.96), rgba(2, 6, 23, 0.94));
    border: 1px solid rgba(125, 211, 252, 0.26);
    border-radius: 18px;
    box-shadow: inset 0 2px 18px rgba(0, 0, 0, 0.56),
                inset 0 0 44px rgba(20, 184, 166, 0.08),
                0 2px rgba(255, 255, 255, 0.08),
                0 18px 38px rgba(0, 0, 0, 0.26);
}

.wombat-calculator-look .wombat-history {
    color: #f8fafc;
}

.wombat-calculator-look .wombat-history text {
    background: rgba(2, 6, 23, 0.62);
    color: #f8fafc;
}

.wombat-calculator-look .wombat-status {
    color: #dbeafe;
    font-weight: 600;
    text-shadow: 0 1px 8px rgba(15, 23, 42, 0.62);
}

.wombat-calculator-look .wombat-input {
    min-height: 44px;
    border-radius: 14px;
    border: 1px solid rgba(125, 211, 252, 0.34);
    background: linear-gradient(180deg, rgba(15, 23, 42, 0.94), rgba(2, 6, 23, 0.88));
    color: #f8fafc;
    box-shadow: inset 0 2px 10px rgba(0, 0, 0, 0.50),
                0 0 24px rgba(56, 189, 248, 0.10);
}

.wombat-calculator-look .wombat-input:focus {
    border-color: @calc_display_glow;
    box-shadow: 0 0 0 3px rgba(20, 184, 166, 0.18),
                0 0 32px rgba(29, 214, 189, 0.20),
                inset 0 2px 10px rgba(0, 0, 0, 0.50);
}

.wombat-calculator-look button {
    border-radius: 13px;
    text-shadow: 0 1px rgba(255, 255, 255, 0.18);
}

.wombat-calculator-look button.wombat-run {
    background: linear-gradient(180deg, #5eead4, #14b8a6 45%, #0f766e);
    color: #ecfeff;
    border: none;
    font-weight: 800;
    text-shadow: none;
    box-shadow: none;
}

.wombat-calculator-look button.wombat-secondary {
    background: linear-gradient(180deg, #f8fafc, #cbd5e1 48%, @calc_function);
    color: #1f2937;
    border: none;
    font-weight: 700;
    box-shadow: none;
}

.wombat-calculator-look button.wombat-operator {
    background: linear-gradient(180deg, #fed7aa, #fb923c 52%, #f97316);
    color: #431407;
    border: none;
    font-weight: 800;
    box-shadow: none;
}

.wombat-calculator-look button.wombat-constant {
    background: linear-gradient(180deg, #ffffff, #f1f5f9 52%, @calc_key);
    color: @calc_key_text;
    border: none;
    font-weight: 700;
    box-shadow: none;
}

.wombat-calculator-look button.wombat-run:hover,
.wombat-calculator-look button.wombat-secondary:hover,
.wombat-calculator-look button.wombat-operator:hover,
.wombat-calculator-look button.wombat-constant:hover {
    box-shadow: none;
    filter: brightness(1.05);
}

.wombat-calculator-look button.wombat-run:active,
.wombat-calculator-look button.wombat-secondary:active,
.wombat-calculator-look button.wombat-operator:active,
.wombat-calculator-look button.wombat-constant:active {
    box-shadow: none;
    filter: brightness(0.92);
}

.wombat-calculator-look .wombat-sidebar {
    background: radial-gradient(circle at 100% 0%, rgba(103, 232, 249, 0.12), transparent 36%),
                linear-gradient(180deg, rgba(47, 52, 64, 0.92), rgba(17, 24, 39, 0.94));
    color: #f8fafc;
    border-left: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow: inset 1px 0 rgba(255, 255, 255, 0.06);
}

.wombat-calculator-look .wombat-sidebar button.flat {
    background: rgba(15, 23, 42, 0.28);
    color: #f8fafc;
    border-radius: 12px;
    box-shadow: inset 0 0 0 1px rgba(226, 232, 240, 0.08);
}

.wombat-calculator-look .wombat-sidebar button.flat image,
.wombat-calculator-look .wombat-sidebar button.flat label {
    color: #f8fafc;
}

.wombat-calculator-look .wombat-sidebar button.flat:hover {
    background: rgba(103, 232, 249, 0.18);
    color: #ecfeff;
    box-shadow: inset 0 0 0 1px rgba(125, 211, 252, 0.26);
}

.wombat-calculator-look .wombat-sidebar button.flat:focus,
.wombat-calculator-look .wombat-sidebar button.flat:focus-visible {
    background: rgba(20, 184, 166, 0.22);
    color: #ecfeff;
    box-shadow: inset 0 0 0 1px rgba(125, 211, 252, 0.36);
}

.wombat-calculator-look .wombat-sidebar row {
    border-radius: 12px;
}

.wombat-calculator-look .wombat-completion-button {
    color: #a5f3fc;
    font-weight: 650;
}

.wombat-calculator-look .wombat-completion-button:focus,
.wombat-calculator-look .wombat-completion-button:focus-visible,
.wombat-calculator-look .wombat-completion-button:hover,
.wombat-calculator-look .wombat-completion-button:checked {
    background: rgba(20, 184, 166, 0.20);
    color: #ecfeff;
    box-shadow: inset 0 0 0 1px rgba(125, 211, 252, 0.24);
}
"#;

pub fn load_app_css() {
    let Some(display) = gtk::gdk::Display::default() else {
        eprintln!("Failed to load application CSS: no default display");
        return;
    };

    let provider = gtk::CssProvider::new();
    provider.load_from_data(APP_CSS);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn apply_calculator_look(window: &adw::ApplicationWindow, enabled: bool) {
    if enabled {
        window.add_css_class("wombat-calculator-look");
    } else {
        window.remove_css_class("wombat-calculator-look");
    }
}
