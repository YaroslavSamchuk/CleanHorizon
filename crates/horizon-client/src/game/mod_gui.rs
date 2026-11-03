//! Модульна система сучасного інтерфейсу (Modern Modal Glassmorphism GUI) для Horizon
use horizon_client::hud::{HudBatch, rgba};
use horizon_client::text::Text;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputBind {
    Key(KeyCode),
    Mouse(MouseButton),
}

#[derive(Clone, Debug)]
pub enum Widget {
    Toggle {
        id: &'static str,
        label: &'static str,
        value: bool,
    },
    Keybind {
        id: &'static str,
        label: &'static str,
        bind: Option<InputBind>,
    },
    Slider {
        id: &'static str,
        label: &'static str,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        suffix: &'static str,
    },
    Choice {
        id: &'static str,
        label: &'static str,
        options: Vec<&'static str>,
        selected: usize,
    },
}

pub struct Tab {
    pub name: &'static str,
    pub icon: &'static str,
    pub widgets: Vec<Widget>,
}

pub struct ModularGui {
    pub is_open: bool,
    pub active_tab: usize,
    pub tabs: Vec<Tab>,
    pub dragging_slider: Option<&'static str>,
    pub mouse_pos: [f32; 2],
    pub scroll_y: f32,
    pub dragging_scrollbar: bool,
    pub scrollbar_drag_start_y: f32,
    pub scrollbar_drag_start_scroll: f32,
    pub listening_keybind: Option<&'static str>,
}

#[derive(Clone, Copy, Debug)]
pub struct GuiLayout {
    pub x0: f32,
    pub y0: f32,
    pub menu_w: f32,
    pub menu_h: f32,
    pub sidebar_w: f32,
    pub content_x: f32,
    pub content_y: f32,
    pub content_w: f32,
    pub content_h: f32,
    pub widget_w: f32,
    pub max_scroll: f32,
    pub scroll_y: f32,
    pub has_scrollbar: bool,
    pub sb_x: f32,
    pub sb_y: f32,
    pub sb_w: f32,
    pub sb_h: f32,
    pub thumb_y: f32,
    pub thumb_h: f32,
}

pub fn format_key_name(code: KeyCode) -> String {
    if let Some(raw) = horizon_client::keybinds::key_name(code) {
        if let Some(key) = raw.strip_prefix("key.keyboard.") {
            let clean = match key {
                "keypad.1" => "NUMPAD 1",
                "keypad.2" => "NUMPAD 2",
                "keypad.3" => "NUMPAD 3",
                "keypad.4" => "NUMPAD 4",
                "keypad.5" => "NUMPAD 5",
                "keypad.6" => "NUMPAD 6",
                "keypad.7" => "NUMPAD 7",
                "keypad.8" => "NUMPAD 8",
                "keypad.9" => "NUMPAD 9",
                "keypad.0" => "NUMPAD 0",
                "left.alt" => "L-ALT",
                "right.alt" => "R-ALT",
                "left.control" => "L-CTRL",
                "right.control" => "R-CTRL",
                "left.shift" => "L-SHIFT",
                "right.shift" => "R-SHIFT",
                other => other,
            };
            return format!("[ {} ]", clean.to_uppercase());
        }
        if let Some(mouse) = raw.strip_prefix("key.mouse.") {
            return format!("[ MOUSE {} ]", mouse.to_uppercase());
        }
        format!("[ {} ]", raw.to_uppercase())
    } else {
        format!("[ {:?} ]", code)
    }
}

pub fn format_bind_name(bind: Option<InputBind>) -> String {
    match bind {
        None => "[ Немає ]".to_string(),
        Some(InputBind::Key(code)) => format_key_name(code),
        Some(InputBind::Mouse(btn)) => match btn {
            MouseButton::Left => "[ ЛКМ ]".to_string(),
            MouseButton::Right => "[ ПКМ ]".to_string(),
            MouseButton::Middle => "[ СКМ (Колесо) ]".to_string(),
            MouseButton::Back => "[ Миша 4 (Назад) ]".to_string(),
            MouseButton::Forward => "[ Миша 5 (Вперед) ]".to_string(),
            MouseButton::Other(n) => format!("[ Миша {} ]", n + 1),
        },
    }
}

pub fn serialize_bind(bind: Option<InputBind>) -> String {
    match bind {
        None => "none".to_string(),
        Some(InputBind::Key(code)) => {
            horizon_client::keybinds::key_name(code)
                .unwrap_or("key.keyboard.unknown")
                .to_string()
        }
        Some(InputBind::Mouse(btn)) => horizon_client::keybinds::mouse_name(btn),
    }
}

pub fn deserialize_bind(s: &str) -> Option<InputBind> {
    let s = s.trim();
    if s == "none" || s.is_empty() {
        None
    } else if let Some(code) = horizon_client::keybinds::key_code(s) {
        Some(InputBind::Key(code))
    } else if let Some(btn) = horizon_client::keybinds::mouse_button(s) {
        Some(InputBind::Mouse(btn))
    } else {
        None
    }
}

impl ModularGui {
    pub fn new() -> Self {
        let mut gui = Self {
            is_open: false,
            active_tab: 0,
            tabs: Vec::new(),
            dragging_slider: None,
            mouse_pos: [0.0, 0.0],
            scroll_y: 0.0,
            dragging_scrollbar: false,
            scrollbar_drag_start_y: 0.0,
            scrollbar_drag_start_scroll: 0.0,
            listening_keybind: None,
        };
        gui.init_default_tabs();
        gui.load_config();
        gui
    }

    /// Початкове налаштування розширених вкладок та параметрів
    fn init_default_tabs(&mut self) {
        // Вкладка 1: Кінематика та наведення
        self.tabs.push(Tab {
            name: "Combat",
            icon: "⚔",
            widgets: vec![
                Widget::Toggle {
                    id: "combat_enabled",
                    label: "Бойова система",
                    value: true,
                },
                Widget::Toggle {
                    id: "only_on_attack",
                    label: "Тільки при атаці (LMB)",
                    value: false,
                },
                Widget::Choice {
                    id: "combat_mode",
                    label: "Режим роботи",
                    options: vec!["Assist (Ближній бій)", "Lock-On (Дальній захват)"],
                    selected: 0,
                },
                Widget::Keybind {
                    id: "action_hotkey",
                    label: "Гаряча кнопка дії",
                    bind: Some(InputBind::Mouse(MouseButton::Middle)),
                },
                Widget::Keybind {
                    id: "switch_mode_hotkey",
                    label: "Перемикач режиму (Assist/Lock)",
                    bind: Some(InputBind::Key(KeyCode::KeyB)),
                },
                Widget::Slider {
                    id: "assist_range",
                    label: "Дистанція Assist (Ближній)",
                    value: 4.8,
                    min: 2.0,
                    max: 8.0,
                    step: 0.1,
                    suffix: " бл",
                },
                Widget::Slider {
                    id: "lock_on_range",
                    label: "Дистанція Lock-On (Дальній)",
                    value: 32.0,
                    min: 10.0,
                    max: 64.0,
                    step: 1.0,
                    suffix: " бл",
                },
                Widget::Choice {
                    id: "target_part",
                    label: "Точка наведення (Кістка)",
                    options: vec!["Груди", "Голова", "Ноги"],
                    selected: 0,
                },
                Widget::Slider {
                    id: "smoothness",
                    label: "Плавність / Швидкість",
                    value: 0.60,
                    min: 0.1,
                    max: 1.0,
                    step: 0.05,
                    suffix: "",
                },
                Widget::Slider {
                    id: "deadzone",
                    label: "Мертва зона прицілу",
                    value: 1.5,
                    min: 0.2,
                    max: 4.0,
                    step: 0.1,
                    suffix: "°",
                },
                Widget::Slider {
                    id: "fov_cone",
                    label: "Конус огляду FOV",
                    value: 65.0,
                    min: 20.0,
                    max: 120.0,
                    step: 5.0,
                    suffix: "°",
                },
                Widget::Slider {
                    id: "lead_pred",
                    label: "Час випередження",
                    value: 0.12,
                    min: 0.0,
                    max: 0.35,
                    step: 0.02,
                    suffix: " с",
                },
                Widget::Slider {
                    id: "reaction_delay",
                    label: "Затримка реакції",
                    value: 0.14,
                    min: 0.04,
                    max: 0.30,
                    step: 0.02,
                    suffix: " с",
                },
                Widget::Toggle {
                    id: "check_walls",
                    label: "Пряма видимість (Raycast)",
                    value: true,
                },
                Widget::Slider {
                    id: "human_tremor",
                    label: "Людський мікротремор",
                    value: 0.05,
                    min: 0.0,
                    max: 0.20,
                    step: 0.01,
                    suffix: "",
                },
                Widget::Choice {
                    id: "target_filter",
                    label: "Фільтр цілей",
                    options: vec!["Усі істоти", "Лише ворожі"],
                    selected: 0,
                },
            ],
        });

        // Вкладка 2: Візуальний фідбек (Lock-On)
        self.tabs.push(Tab {
            name: "Visuals",
            icon: "👁",
            widgets: vec![
                Widget::Toggle {
                    id: "show_lock_reticle",
                    label: "Маркер Lock-On на мобі",
                    value: true,
                },
                Widget::Choice {
                    id: "reticle_style",
                    label: "Стиль маркера",
                    options: vec!["Ромб ◇", "Кутики [ ]", "Коло (Статичне)", "Коло (Анімоване)"],
                    selected: 2,
                },
                Widget::Choice {
                    id: "esp_fps_limit",
                    label: "Частота оновлення ESP",
                    options: vec!["Максимальна (Hz)", "60 FPS", "45 FPS", "30 FPS", "20 FPS (Ваніль)"],
                    selected: 0,
                },
                Widget::Slider {
                    id: "esp_smooth",
                    label: "Плавність мітки ESP",
                    value: 0.80,
                    min: 0.0,
                    max: 0.95,
                    step: 0.05,
                    suffix: "",
                },
                Widget::Toggle {
                    id: "show_target_hp",
                    label: "Смужка HP над ціллю",
                    value: true,
                },
                Widget::Toggle {
                    id: "show_fov_cone",
                    label: "Індикатор зони захвату",
                    value: false,
                },
            ],
        });

        // Вкладка 3: Додаткові опції
        self.tabs.push(Tab {
            name: "Settings",
            icon: "⚙",
            widgets: vec![
                Widget::Choice {
                    id: "target_filter",
                    label: "Фільтр цілей",
                    options: vec!["Усі істоти", "Лише ворожі", "Гравці"],
                    selected: 0,
                },
                Widget::Toggle {
                    id: "crosshair_friction",
                    label: "Магнетизм прицілу",
                    value: true,
                },
                Widget::Slider {
                    id: "human_tremor",
                    label: "Фізіологічний тремор",
                    value: 0.05,
                    min: 0.0,
                    max: 0.15,
                    step: 0.01,
                    suffix: "°",
                },
            ],
        });
    }

    /// Загальна висота контенту для вкладки
    pub fn tab_content_height(&self, tab_idx: usize) -> f32 {
        let Some(tab) = self.tabs.get(tab_idx) else { return 0.0 };
        let mut total = 0.0;
        for widget in &tab.widgets {
            total += match widget {
                Widget::Toggle { .. } => 32.0,
                Widget::Keybind { .. } => 32.0,
                Widget::Choice { .. } => 32.0,
                Widget::Slider { .. } => 40.0,
            };
        }
        total
    }

    pub const CONFIG_FILE: &'static str = "horizon-combat-mod.txt";

    pub fn config_path() -> std::path::PathBuf {
        let direct = std::path::PathBuf::from(Self::CONFIG_FILE);
        if direct.exists() {
            return direct;
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let p = std::path::Path::new(&appdata).join("horizon").join(Self::CONFIG_FILE);
            if p.exists() {
                return p;
            }
        }
        if let Some(home) = std::env::var_os("HOME") {
            let p = std::path::Path::new(&home).join(".local/share/horizon").join(Self::CONFIG_FILE);
            if p.exists() {
                return p;
            }
        }
        direct
    }

    pub fn save_config(&self) {
        let mut lines = Vec::new();
        for tab in &self.tabs {
            for widget in &tab.widgets {
                match widget {
                    Widget::Toggle { id, value, .. } => {
                        lines.push(format!("{}:{}", id, value));
                    }
                    Widget::Slider { id, value, .. } => {
                        lines.push(format!("{}:{:.2}", id, value));
                    }
                    Widget::Choice { id, selected, .. } => {
                        lines.push(format!("{}:{}", id, selected));
                    }
                    Widget::Keybind { id, bind, .. } => {
                        lines.push(format!("{}:{}", id, serialize_bind(*bind)));
                    }
                }
            }
        }
        let content = lines.join("\n");
        let path = Self::config_path();
        let _ = std::fs::write(&path, &content);
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let appdata_path = std::path::Path::new(&appdata).join("horizon").join(Self::CONFIG_FILE);
            if appdata_path != path {
                let _ = std::fs::write(appdata_path, &content);
            }
        }
    }

    pub fn load_config(&mut self) {
        let path = Self::config_path();
        let Ok(content) = std::fs::read_to_string(&path) else { return };
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, val)) = line.split_once(':') else { continue };
            let key = key.trim();
            let val = val.trim();

            for tab in &mut self.tabs {
                for widget in &mut tab.widgets {
                    match widget {
                        Widget::Toggle { id, value, .. } if *id == key => {
                            if let Ok(b) = val.parse::<bool>() {
                                *value = b;
                            }
                        }
                        Widget::Slider { id, value, min, max, .. } if *id == key => {
                            if let Ok(v) = val.parse::<f32>() {
                                *value = v.clamp(*min, *max);
                            }
                        }
                        Widget::Choice { id, selected, options, .. } if *id == key => {
                            if let Ok(idx) = val.parse::<usize>() {
                                if idx < options.len() {
                                    *selected = idx;
                                }
                            }
                        }
                        Widget::Keybind { id, bind, .. } if *id == key => {
                            *bind = deserialize_bind(val);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn get_bool(&self, id: &str) -> bool {
        for tab in &self.tabs {
            for widget in &tab.widgets {
                if let Widget::Toggle { id: w_id, value, .. } = widget {
                    if *w_id == id {
                        return *value;
                    }
                }
            }
        }
        false
    }

    pub fn set_bool(&mut self, id: &str, new_val: bool) {
        for tab in &mut self.tabs {
            for widget in &mut tab.widgets {
                if let Widget::Toggle { id: w_id, value, .. } = widget {
                    if *w_id == id {
                        *value = new_val;
                        self.save_config();
                        return;
                    }
                }
            }
        }
    }

    pub fn get_bind(&self, id: &str) -> Option<InputBind> {
        for tab in &self.tabs {
            for widget in &tab.widgets {
                if let Widget::Keybind { id: w_id, bind, .. } = widget {
                    if *w_id == id {
                        return *bind;
                    }
                }
            }
        }
        None
    }

    pub fn set_bind(&mut self, id: &str, new_bind: Option<InputBind>) {
        for tab in &mut self.tabs {
            for widget in &mut tab.widgets {
                if let Widget::Keybind { id: w_id, bind, .. } = widget {
                    if *w_id == id {
                        *bind = new_bind;
                        self.save_config();
                        return;
                    }
                }
            }
        }
    }

    pub fn set_choice(&mut self, id: &str, new_selected: usize) {
        for tab in &mut self.tabs {
            for widget in &mut tab.widgets {
                if let Widget::Choice { id: w_id, selected, options, .. } = widget {
                    if *w_id == id && new_selected < options.len() {
                        *selected = new_selected;
                        self.save_config();
                        return;
                    }
                }
            }
        }
    }

    pub fn get_float(&self, id: &str) -> f32 {
        for tab in &self.tabs {
            for widget in &tab.widgets {
                if let Widget::Slider { id: w_id, value, .. } = widget {
                    if *w_id == id {
                        return *value;
                    }
                }
            }
        }
        0.0
    }

    pub fn get_choice(&self, id: &str) -> usize {
        for tab in &self.tabs {
            for widget in &tab.widgets {
                if let Widget::Choice { id: w_id, selected, .. } = widget {
                    if *w_id == id {
                        return *selected;
                    }
                }
            }
        }
        0
    }
}

impl super::Game<'_> {
    /// Обчислення динамічної геометрії вікна з гарантованим збереженням відступів від країв екрана
    pub fn compute_mod_gui_layout(&self, win_w: f32, win_h: f32) -> GuiLayout {
        let pad_x = 12.0;
        let pad_y = 10.0;

        let avail_w = (win_w - pad_x * 2.0).max(280.0);
        let avail_h = (win_h - pad_y * 2.0).max(180.0);

        // Динамічна ширина меню (до 460px, автоматично стискається на малих екранах)
        let menu_w = 460.0_f32.min(avail_w).round();
        let sidebar_w = 110.0_f32.min(menu_w * 0.28).max(85.0).round();

        // Потрібна висота контенту для поточної вкладки
        let content_needed = self.mod_gui.tab_content_height(self.mod_gui.active_tab);
        // Заголовок і відступи займають ~28px
        let ideal_h = (content_needed + 28.0).clamp(210.0, 380.0);
        let menu_h = ideal_h.min(avail_h).round();

        // Центрування з жорсткою гарантією не виходу за межі екрана (y0 завжди >= pad_y)
        let x0 = ((win_w - menu_w) * 0.5).max(pad_x).round();
        let y0 = ((win_h - menu_h) * 0.5).max(pad_y).round();

        let content_x = x0 + sidebar_w + 12.0;
        let content_y = y0 + 12.0;
        let content_w = (menu_w - sidebar_w - 24.0).max(120.0);
        let content_h = (menu_h - 24.0).max(80.0);

        let max_scroll = (content_needed - content_h).max(0.0);
        let scroll_y = self.mod_gui.scroll_y.clamp(0.0, max_scroll);
        let has_scrollbar = max_scroll > 1.0;

        let sb_w = 4.0;
        let sb_x = content_x + content_w - sb_w;
        let sb_y = content_y;
        let sb_h = content_h;

        let thumb_h = if has_scrollbar {
            (content_h * (content_h / content_needed)).clamp(16.0, content_h)
        } else {
            content_h
        };
        let thumb_y = if max_scroll > 0.0 {
            sb_y + (scroll_y / max_scroll) * (content_h - thumb_h)
        } else {
            sb_y
        };

        let widget_w = if has_scrollbar {
            content_w - sb_w - 6.0
        } else {
            content_w
        };

        GuiLayout {
            x0,
            y0,
            menu_w,
            menu_h,
            sidebar_w,
            content_x,
            content_y,
            content_w,
            content_h,
            widget_w,
            max_scroll,
            scroll_y,
            has_scrollbar,
            sb_x,
            sb_y,
            sb_w,
            sb_h,
            thumb_y,
            thumb_h,
        }
    }

    /// Малювання розширеного модального Glassmorphism GUI з динамічним розміром та скролом
    pub fn draw_modular_gui(&mut self, batch: &mut HudBatch) {
        if !self.mod_gui.is_open {
            return;
        }

        let (win_w, win_h) = (batch.width, batch.height);
        let layout = self.compute_mod_gui_layout(win_w, win_h);
        self.mod_gui.scroll_y = layout.scroll_y;

        // 1. Затемнення фону гри
        batch.fill(&self.gui.atlas, 0.0, 0.0, win_w, win_h, rgba(0, 0, 0, 160));

        // 2. Головне вікно (Catppuccin Mocha / Dark Glass)
        batch.fill(&self.gui.atlas, layout.x0, layout.y0, layout.menu_w, layout.menu_h, rgba(24, 24, 37, 248));
        // Лавандова рамка
        self.draw_gui_border(batch, [layout.x0, layout.y0, layout.menu_w, layout.menu_h], rgba(203, 166, 247, 120));

        // 3. Сайдбар ліворуч
        batch.fill(&self.gui.atlas, layout.x0, layout.y0, layout.sidebar_w, layout.menu_h, rgba(17, 17, 27, 210));
        batch.fill(&self.gui.atlas, layout.x0 + layout.sidebar_w, layout.y0, 1.0, layout.menu_h, rgba(255, 255, 255, 30));

        // Логотип
        batch.text_styled(
            &self.gui.atlas,
            &Text::literal("✦ MOD MENU"),
            layout.x0 + 10.0,
            layout.y0 + 14.0,
            rgba(203, 166, 247, 255),
            true,
        );

        // Вкладки
        let mut tab_y = layout.y0 + 42.0;
        for (i, tab) in self.mod_gui.tabs.iter().enumerate() {
            let active = i == self.mod_gui.active_tab;
            let bg = if active { rgba(203, 166, 247, 50) } else { rgba(0, 0, 0, 0) };
            let text_color = if active { rgba(255, 255, 255, 255) } else { rgba(166, 173, 200, 255) };

            if active {
                batch.fill(&self.gui.atlas, layout.x0 + 2.0, tab_y, 3.0, 24.0, rgba(203, 166, 247, 255));
            }
            batch.fill(&self.gui.atlas, layout.x0 + 6.0, tab_y, layout.sidebar_w - 10.0, 24.0, bg);
            let label = format!("{} {}", tab.icon, tab.name);
            batch.text_styled(&self.gui.atlas, &Text::literal(&label), layout.x0 + 12.0, tab_y + 7.0, text_color, false);
            tab_y += 28.0;
        }

        // Підказка про збережену кнопку (внизу сайдбару)
        let key_label = self.settings.keys.get("key.mod_menu");
        let display_key = if key_label.is_empty() || key_label == "key.keyboard.unknown" {
            "RShift"
        } else {
            key_label
        };
        let hint = format!("[{}] Закрити", display_key);
        batch.text_styled(
            &self.gui.atlas,
            &Text::literal(&hint),
            layout.x0 + 8.0,
            layout.y0 + layout.menu_h - 14.0,
            rgba(108, 112, 134, 255),
            false,
        );

        // Кнопка закриття [✕] у верхньому правому кутку
        let close_btn_x = layout.x0 + layout.menu_w - 20.0;
        let close_btn_y = layout.y0 + 10.0;
        let [mx, my] = self.mod_gui.mouse_pos;
        let hovered_close = mx >= close_btn_x - 4.0 && mx <= close_btn_x + 16.0
            && my >= close_btn_y - 2.0 && my <= close_btn_y + 16.0;
        let close_color = if hovered_close { rgba(243, 139, 168, 255) } else { rgba(166, 173, 200, 200) };
        batch.text_styled(
            &self.gui.atlas,
            &Text::literal("✕"),
            close_btn_x,
            close_btn_y,
            close_color,
            false,
        );

        // 4. Контентна зона
        let start_vtx = batch.vertices.len();

        if let Some(tab) = self.mod_gui.tabs.get(self.mod_gui.active_tab) {
            let mut widget_y = layout.content_y - layout.scroll_y;
            for widget in &tab.widgets {
                let widget_h = match widget {
                    Widget::Toggle { .. } => 32.0,
                    Widget::Keybind { .. } => 32.0,
                    Widget::Choice { .. } => 32.0,
                    Widget::Slider { .. } => 40.0,
                };

                // Малюємо тільки якщо віджет перетинає видиму область
                if widget_y + widget_h >= layout.content_y && widget_y <= layout.content_y + layout.content_h {
                    match widget {
                        Widget::Toggle { label, value, .. } => {
                            batch.fill(&self.gui.atlas, layout.content_x, widget_y, layout.widget_w, 28.0, rgba(49, 50, 68, 120));
                            batch.text_styled(&self.gui.atlas, &Text::literal(*label), layout.content_x + 8.0, widget_y + 8.0, rgba(205, 214, 244, 255), false);

                            let sw_w = 24.0;
                            let sw_h = 13.0;
                            let sw_x = layout.content_x + layout.widget_w - sw_w - 6.0;
                            let sw_y = widget_y + 7.5;
                            let sw_bg = if *value { rgba(166, 227, 161, 255) } else { rgba(69, 71, 90, 255) };
                            batch.fill(&self.gui.atlas, sw_x, sw_y, sw_w, sw_h, sw_bg);

                            let knob_x = if *value { sw_x + sw_w - 11.0 } else { sw_x + 2.0 };
                            batch.fill(&self.gui.atlas, knob_x, sw_y + 2.0, 9.0, 9.0, rgba(255, 255, 255, 255));
                        }
                        Widget::Keybind { id, label, bind } => {
                            batch.fill(&self.gui.atlas, layout.content_x, widget_y, layout.widget_w, 28.0, rgba(49, 50, 68, 120));
                            batch.text_styled(&self.gui.atlas, &Text::literal(*label), layout.content_x + 8.0, widget_y + 8.0, rgba(205, 214, 244, 255), false);

                            let is_listening = self.mod_gui.listening_keybind == Some(id);
                            let key_str = if is_listening {
                                "[ Натисніть... ]".to_string()
                            } else {
                                format_bind_name(*bind)
                            };

                            let btn_w = (key_str.chars().count() as f32 * 6.5) + 14.0;
                            let btn_x = layout.content_x + layout.widget_w - btn_w - 6.0;
                            let btn_y = widget_y + 4.0;
                            let btn_bg = if is_listening {
                                rgba(249, 226, 175, 80)
                            } else {
                                rgba(30, 30, 46, 200)
                            };
                            batch.fill(&self.gui.atlas, btn_x, btn_y, btn_w, 20.0, btn_bg);
                            self.draw_gui_border(batch, [btn_x, btn_y, btn_w, 20.0], if is_listening { rgba(249, 226, 175, 255) } else { rgba(203, 166, 247, 100) });

                            let text_color = if is_listening { rgba(249, 226, 175, 255) } else { rgba(203, 166, 247, 255) };
                            batch.text_styled(&self.gui.atlas, &Text::literal(&key_str), btn_x + 7.0, btn_y + 5.0, text_color, false);
                        }
                        Widget::Slider { label, value, min, max, suffix, .. } => {
                            batch.fill(&self.gui.atlas, layout.content_x, widget_y, layout.widget_w, 36.0, rgba(49, 50, 68, 120));
                            let val_str = format!("{:.2}{}", value, suffix);
                            batch.text_styled(&self.gui.atlas, &Text::literal(*label), layout.content_x + 8.0, widget_y + 6.0, rgba(205, 214, 244, 255), false);
                            batch.text_styled(&self.gui.atlas, &Text::literal(&val_str), layout.content_x + layout.widget_w - 50.0, widget_y + 6.0, rgba(137, 220, 235, 255), false);

                            let bar_x = layout.content_x + 8.0;
                            let bar_y = widget_y + 24.0;
                            let bar_w = (layout.widget_w - 16.0).max(10.0);
                            batch.fill(&self.gui.atlas, bar_x, bar_y, bar_w, 4.0, rgba(69, 71, 90, 255));

                            let ratio = ((*value - *min) / (*max - *min)).clamp(0.0, 1.0);
                            batch.fill(&self.gui.atlas, bar_x, bar_y, bar_w * ratio, 4.0, rgba(137, 180, 250, 255));
                            batch.fill(&self.gui.atlas, bar_x + bar_w * ratio - 4.0, bar_y - 3.0, 8.0, 10.0, rgba(255, 255, 255, 255));
                        }
                        Widget::Choice { label, options, selected, .. } => {
                            batch.fill(&self.gui.atlas, layout.content_x, widget_y, layout.widget_w, 28.0, rgba(49, 50, 68, 120));
                            batch.text_styled(&self.gui.atlas, &Text::literal(*label), layout.content_x + 8.0, widget_y + 8.0, rgba(205, 214, 244, 255), false);

                            let cur = options.get(*selected).unwrap_or(&"None");
                            let opt_str = format!("< {} >", cur);
                            let opt_w = (opt_str.chars().count() as f32 * 6.5) + 10.0;
                            batch.text_styled(&self.gui.atlas, &Text::literal(&opt_str), layout.content_x + layout.widget_w - opt_w - 6.0, widget_y + 8.0, rgba(249, 226, 175, 255), false);
                        }
                    }
                }
                widget_y += widget_h;
            }
        }

        // Відсікаємо віджети точно по межах видимої області
        batch.clip_since(
            start_vtx,
            layout.content_x - 1.0,
            layout.content_y,
            layout.content_x + layout.content_w + 1.0,
            layout.content_y + layout.content_h,
        );

        // 5. Малювання скролбару, якщо контент перевищує висоту області
        if layout.has_scrollbar {
            batch.fill(&self.gui.atlas, layout.sb_x, layout.sb_y, layout.sb_w, layout.sb_h, rgba(49, 50, 68, 100));
            let thumb_color = if self.mod_gui.dragging_scrollbar {
                rgba(203, 166, 247, 255)
            } else {
                rgba(203, 166, 247, 160)
            };
            batch.fill(&self.gui.atlas, layout.sb_x, layout.thumb_y, layout.sb_w, layout.thumb_h, thumb_color);
        }
    }

    pub(super) fn draw_gui_border(&self, batch: &mut HudBatch, rect: [f32; 4], color: u32) {
        let [x, y, w, h] = rect;
        batch.fill(&self.gui.atlas, x, y, w, 1.0, color);
        batch.fill(&self.gui.atlas, x, y + h, w, 1.0, color);
        batch.fill(&self.gui.atlas, x, y, 1.0, h, color);
        batch.fill(&self.gui.atlas, x + w, y, 1.0, h, color);
    }

    fn gui_window_size(&self) -> (f32, f32) {
        if let Some(r) = &self.renderer {
            let b = HudBatch::new([r.config.width, r.config.height]);
            (b.width, b.height)
        } else {
            (800.0, 600.0)
        }
    }

    /// Оновлення захвату та видимості миші в залежності від поточного екрана
    pub(super) fn update_cursor_capture(&mut self) {
        let should_grab = self.screen == super::Screen::Playing
            && self.title.is_none()
            && self.player.is_some()
            && !self.mod_gui.is_open;
        self.capture(should_grab);
    }

    /// Обробка прокрутки коліщатком миші
    pub fn handle_mod_gui_scroll(&mut self, delta: f32) -> bool {
        if !self.mod_gui.is_open {
            return false;
        }
        let (win_w, win_h) = self.gui_window_size();
        let layout = self.compute_mod_gui_layout(win_w, win_h);
        if layout.max_scroll > 0.0 {
            self.mod_gui.scroll_y = (self.mod_gui.scroll_y - delta * 24.0).clamp(0.0, layout.max_scroll);
        }
        true
    }

    /// Обробка клавіші з врахуванням налаштувань, біндів та відкриття меню
    pub fn handle_mod_gui_key(&mut self, key: KeyCode, pressed: bool) -> bool {
        // 1. Режим прив'язки клавіші (Keybind listen) у відкритому меню
        if let Some(binding_id) = self.mod_gui.listening_keybind {
            if pressed {
                if key == KeyCode::Escape {
                    self.mod_gui.set_bind(binding_id, None);
                } else {
                    self.mod_gui.set_bind(binding_id, Some(InputBind::Key(key)));
                }
                self.mod_gui.listening_keybind = None;
                return true;
            }
            return true;
        }

        // 2. Гарячі клавіші швидкого перемикання під час гри (меню закрите)
        if !self.mod_gui.is_open && pressed {
            // 2.1. Перемикач режиму (Assist / Lock-On)
            if let Some(bind) = self.mod_gui.get_bind("switch_mode_hotkey") {
                if bind == InputBind::Key(key) {
                    self.switch_combat_mode();
                    return true;
                }
            }

            // 2.2. Гаряча кнопка дії
            if let Some(bind) = self.mod_gui.get_bind("action_hotkey") {
                if bind == InputBind::Key(key) {
                    self.trigger_combat_action();
                    return true;
                }
            }
        }

        // 3. Відкриття / закриття модального меню
        let bound_key = self.settings.keys.get("key.mod_menu");
        let matches_bound = horizon_client::keybinds::key_name(key) == Some(bound_key);
        let matches_fallback = key == KeyCode::ShiftRight || key == KeyCode::Numpad1 || key == KeyCode::Insert;

        if (matches_bound || matches_fallback) && pressed {
            self.mod_gui.is_open = !self.mod_gui.is_open;
            if self.mod_gui.is_open {
                self.capture(false);
            } else {
                self.mod_gui.dragging_slider = None;
                self.mod_gui.dragging_scrollbar = false;
                self.mod_gui.listening_keybind = None;
                self.update_cursor_capture();
                self.mod_gui.save_config();
            }
            return true;
        }

        if key == KeyCode::Escape && self.mod_gui.is_open && pressed {
            self.mod_gui.is_open = false;
            self.mod_gui.dragging_slider = None;
            self.mod_gui.dragging_scrollbar = false;
            self.mod_gui.listening_keybind = None;
            self.update_cursor_capture();
            self.mod_gui.save_config();
            return true;
        }

        self.mod_gui.is_open
    }

    /// Обробка натискань кнопок миші під час гри (гарячі кнопки Lock-On / Assist)
    pub fn handle_mod_gui_mouse_press(&mut self, button: MouseButton, pressed: bool) -> bool {
        if self.mod_gui.is_open || !pressed {
            return false;
        }

        // 1. Перемикач режиму (Assist / Lock-On)
        if let Some(bind) = self.mod_gui.get_bind("switch_mode_hotkey") {
            if bind == InputBind::Mouse(button) {
                self.switch_combat_mode();
                return !matches!(button, MouseButton::Left | MouseButton::Right);
            }
        }

        // 2. Гаряча кнопка дії
        if let Some(bind) = self.mod_gui.get_bind("action_hotkey") {
            if bind == InputBind::Mouse(button) {
                self.trigger_combat_action();
                return !matches!(button, MouseButton::Left | MouseButton::Right);
            }
        }

        false
    }

    /// Обробка кліків миші в меню
    pub fn handle_mod_gui_mouse_click(&mut self, button: MouseButton, pressed: bool) {
        if !self.mod_gui.is_open {
            return;
        }

        if !pressed {
            if self.mod_gui.dragging_slider.is_some() {
                self.mod_gui.save_config();
            }
            self.mod_gui.dragging_slider = None;
            self.mod_gui.dragging_scrollbar = false;
            return;
        }

        let [mx, my] = self.mod_gui.mouse_pos;
        let (win_w, win_h) = self.gui_window_size();
        let layout = self.compute_mod_gui_layout(win_w, win_h);

        // Якщо очікується призначення клавіші / кнопки миші:
        if let Some(listening_id) = self.mod_gui.listening_keybind {
            if button != MouseButton::Left {
                // Будь-яка кнопка крім ЛКМ (СКМ, ПКМ, Миша 4, Миша 5) відразу прив'язується!
                self.mod_gui.set_bind(listening_id, Some(InputBind::Mouse(button)));
                self.mod_gui.listening_keybind = None;
                return;
            } else {
                // ЛКМ: якщо клік по самій кнопці бінду - встановлюємо ЛКМ
                // якщо за її межами - скасовуємо очікування
                let mut clicked_same_widget = false;
                if mx >= layout.content_x && mx <= layout.content_x + layout.widget_w
                    && my >= layout.content_y && my <= layout.content_y + layout.content_h
                {
                    let mut widget_y = layout.content_y - layout.scroll_y;
                    if let Some(tab) = self.mod_gui.tabs.get(self.mod_gui.active_tab) {
                        for widget in &tab.widgets {
                            let widget_h = match widget {
                                Widget::Toggle { .. } => 32.0,
                                Widget::Keybind { .. } => 32.0,
                                Widget::Choice { .. } => 32.0,
                                Widget::Slider { .. } => 40.0,
                            };
                            if my >= widget_y && my <= widget_y + widget_h {
                                if let Widget::Keybind { id, .. } = widget {
                                    if *id == listening_id {
                                        clicked_same_widget = true;
                                    }
                                }
                            }
                            widget_y += widget_h;
                        }
                    }
                }

                if clicked_same_widget {
                    self.mod_gui.set_bind(listening_id, Some(InputBind::Mouse(MouseButton::Left)));
                    self.mod_gui.listening_keybind = None;
                    return;
                } else {
                    self.mod_gui.listening_keybind = None;
                }
            }
        }

        if button != MouseButton::Left {
            return;
        }

        // Клік по кнопці закриття [✕]
        let close_btn_x = layout.x0 + layout.menu_w - 20.0;
        let close_btn_y = layout.y0 + 10.0;
        if mx >= close_btn_x - 4.0 && mx <= close_btn_x + 16.0
            && my >= close_btn_y - 2.0 && my <= close_btn_y + 16.0
        {
            self.mod_gui.is_open = false;
            self.mod_gui.dragging_slider = None;
            self.mod_gui.dragging_scrollbar = false;
            self.mod_gui.listening_keybind = None;
            self.update_cursor_capture();
            self.mod_gui.save_config();
            return;
        }

        // Клік за межами вікна меню - закриваємо меню та відновлюємо курсор
        if mx < layout.x0 || mx > layout.x0 + layout.menu_w
            || my < layout.y0 || my > layout.y0 + layout.menu_h
        {
            self.mod_gui.is_open = false;
            self.mod_gui.dragging_slider = None;
            self.mod_gui.dragging_scrollbar = false;
            self.mod_gui.listening_keybind = None;
            self.update_cursor_capture();
            return;
        }

        // 1. Клік по вкладках
        if mx >= layout.x0 && mx <= layout.x0 + layout.sidebar_w {
            let mut tab_y = layout.y0 + 42.0;
            for i in 0..self.mod_gui.tabs.len() {
                if my >= tab_y && my <= tab_y + 24.0 {
                    self.mod_gui.active_tab = i;
                    self.mod_gui.scroll_y = 0.0;
                    self.mod_gui.listening_keybind = None;
                    return;
                }
                tab_y += 28.0;
            }
        }

        // 2. Клік по скролбару
        if layout.has_scrollbar && mx >= layout.sb_x - 4.0 && mx <= layout.sb_x + layout.sb_w + 4.0
            && my >= layout.sb_y && my <= layout.sb_y + layout.sb_h
        {
            self.mod_gui.dragging_scrollbar = true;
            self.mod_gui.scrollbar_drag_start_y = my;
            self.mod_gui.scrollbar_drag_start_scroll = self.mod_gui.scroll_y;

            if my < layout.thumb_y || my > layout.thumb_y + layout.thumb_h {
                let rel = ((my - layout.sb_y) / layout.sb_h).clamp(0.0, 1.0);
                self.mod_gui.scroll_y = (rel * layout.max_scroll).clamp(0.0, layout.max_scroll);
            }
            return;
        }

        // 3. Клік по віджетах (тільки якщо в межах зони контенту)
        if mx >= layout.content_x && mx <= layout.content_x + layout.widget_w
            && my >= layout.content_y && my <= layout.content_y + layout.content_h
        {
            let mut widget_y = layout.content_y - layout.scroll_y;

            if let Some(tab) = self.mod_gui.tabs.get_mut(self.mod_gui.active_tab) {
                for widget in &mut tab.widgets {
                    let widget_h = match widget {
                        Widget::Toggle { .. } => 32.0,
                        Widget::Keybind { .. } => 32.0,
                        Widget::Choice { .. } => 32.0,
                        Widget::Slider { .. } => 40.0,
                    };

                    if my >= widget_y && my <= widget_y + widget_h {
                        match widget {
                            Widget::Toggle { value, .. } => {
                                *value = !*value;
                                self.mod_gui.listening_keybind = None;
                                self.mod_gui.save_config();
                                return;
                            }
                            Widget::Keybind { id, .. } => {
                                self.mod_gui.listening_keybind = Some(id);
                                return;
                            }
                            Widget::Slider { id, value, min, max, .. } => {
                                let bar_x = layout.content_x + 8.0;
                                let bar_w = (layout.widget_w - 16.0).max(10.0);
                                let ratio = ((mx - bar_x) / bar_w).clamp(0.0, 1.0);
                                *value = *min + ratio * (*max - *min);
                                self.mod_gui.dragging_slider = Some(id);
                                self.mod_gui.listening_keybind = None;
                                return;
                            }
                            Widget::Choice { options, selected, .. } => {
                                *selected = (*selected + 1) % options.len();
                                self.mod_gui.listening_keybind = None;
                                self.mod_gui.save_config();
                                return;
                            }
                        }
                    }
                    widget_y += widget_h;
                }
            }
        }
    }

    /// Перетягування слайдера або скролбару курсором
    pub fn handle_mod_gui_cursor_move(&mut self, mx: f32, my: f32) {
        self.mod_gui.mouse_pos = [mx, my];

        let (win_w, win_h) = self.gui_window_size();
        let layout = self.compute_mod_gui_layout(win_w, win_h);

        // Перетягування скролбару
        if self.mod_gui.dragging_scrollbar && layout.max_scroll > 0.0 {
            let dy = my - self.mod_gui.scrollbar_drag_start_y;
            let scroll_per_pixel = layout.max_scroll / (layout.sb_h - layout.thumb_h).max(1.0);
            self.mod_gui.scroll_y = (self.mod_gui.scrollbar_drag_start_scroll + dy * scroll_per_pixel).clamp(0.0, layout.max_scroll);
            return;
        }

        // Перетягування слайдера
        if let Some(slider_id) = self.mod_gui.dragging_slider {
            let bar_x = layout.content_x + 8.0;
            let bar_w = (layout.widget_w - 16.0).max(10.0);
            let ratio = ((mx - bar_x) / bar_w).clamp(0.0, 1.0);

            if let Some(tab) = self.mod_gui.tabs.get_mut(self.mod_gui.active_tab) {
                for widget in &mut tab.widgets {
                    if let Widget::Slider { id, value, min, max, step, .. } = widget {
                        if *id == slider_id {
                            let raw = *min + ratio * (*max - *min);
                            *value = (raw / *step).round() * *step;
                            return;
                        }
                    }
                }
            }
        }
    }
}
