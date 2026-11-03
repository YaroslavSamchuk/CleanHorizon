//! Кінематика реалістичного асисту, швидкий воксельний Raycast стін (ігнорує траву/ліани),
//! плавна експоненціальна інтерполяція та проекція маркера через матрицю камери Horizon.
use super::Game;
use horizon_client::hud::{HudBatch, rgba};
use horizon_client::text::Text;
use std::f32::consts::PI;

pub struct RealisticAssistState {
    /// ID поточного моба
    pub locked_mob_id: Option<u32>,
    /// Позиція захопленого моба в світі
    pub locked_mob_pos: Option<[f64; 3]>,
    /// Здоров'я моба (current, max)
    pub locked_mob_hp: Option<(f32, f32)>,
    /// Назва моба для відображення
    pub locked_mob_name: String,
    /// Прозорість мітки (0.0..1.0)
    pub lock_alpha: f32,
    /// Таймер анімації обертання маркера
    pub anim_timer: f32,
    /// Час безперервного утримання цілі
    pub target_tracked_time: f32,
    /// Фаза біомеханічного мікротремору
    pub tremor_phase: f32,
    /// Таймер збереження фокусу при короткому перекритті
    pub grace_timer: f32,
    /// Час останнього рендеру мітки на HUD
    pub last_render_time: f32,
    /// Накопичувач часу для ESP FPS обмежувача
    pub esp_fps_accum: f32,
    /// Остання цільова позиція ESP на екрані
    pub raw_screen_pos: Option<[f32; 2]>,
    /// Згладжена позиція ESP (Low-Pass Filter)
    pub smoothed_screen_pos: Option<[f32; 2]>,
    /// Повідомлення статусу на HUD (текст, залишок секунд, колір)
    pub status_toast: Option<(String, f32, u32)>,

}

impl Default for RealisticAssistState {
    fn default() -> Self {
        Self {
            locked_mob_id: None,
            locked_mob_pos: None,
            locked_mob_hp: None,
            locked_mob_name: String::new(),
            lock_alpha: 0.0,
            anim_timer: 0.0,
            target_tracked_time: 0.0,
            tremor_phase: 0.0,
            grace_timer: 0.0,
            last_render_time: 0.0,
            esp_fps_accum: 0.0,
            raw_screen_pos: None,
            smoothed_screen_pos: None,
            status_toast: Some(("Horizon Mod: Right Shift - Меню | B - Режим".into(), 6.0, rgba(203, 166, 247, 255))),

        }
    }
}

pub fn wrap_deg(mut deg: f32) -> f32 {
    deg %= 360.0;
    if deg >= 180.0 {
        deg -= 360.0;
    }
    if deg < -180.0 {
        deg += 360.0;
    }
    deg
}

impl Game<'_> {
    /// Прямий та надійний поворот камери
    pub fn apply_legit_mouse_turn(&mut self, desired_dyaw: f32, desired_dpitch: f32) {
        if desired_dyaw.abs() < 1e-4 && desired_dpitch.abs() < 1e-4 {
            return;
        }

        self.yaw += desired_dyaw;
        self.pitch = (self.pitch + desired_dpitch).clamp(-90.0, 90.0);
    }

    /// Спільний кінематичний розрахунок повороту прицілу для Assist та Lock-On.
    /// Враховує Плавність (Smoothness), Мертву зону (Deadzone), Випередження (Lead),
    /// Затримку реакції (Reaction Delay) та Людський тремор (Tremor).
    pub fn calculate_aim_step(
        &mut self,
        target_pos: [f64; 3],
        target_vel: [f64; 3],
        dt: f32,
    ) -> (f32, f32) {
        let Some(player) = &self.player else { return (0.0, 0.0); };
        let eye_pos = [
            player.position.x,
            player.position.y + 1.62,
            player.position.z,
        ];

        let deadzone = self.mod_gui.get_float("deadzone").clamp(0.1, 4.0);
        let smoothness = self.mod_gui.get_float("smoothness").clamp(0.05, 1.0);
        let tremor_amp = self.mod_gui.get_float("human_tremor");
        let lead_pred = self.mod_gui.get_float("lead_pred").clamp(0.0, 0.5) as f64;
        let reaction_delay = self.mod_gui.get_float("reaction_delay").clamp(0.02, 0.35);

        // 1. Враховуємо випередження руху цілі (Lead Prediction)
        let aim_pos = [
            target_pos[0] + target_vel[0] * lead_pred * 20.0,
            target_pos[1] + target_vel[1] * lead_pred * 20.0,
            target_pos[2] + target_vel[2] * lead_pred * 20.0,
        ];

        let dx = aim_pos[0] - eye_pos[0];
        let dy = aim_pos[1] - eye_pos[1];
        let dz = aim_pos[2] - eye_pos[2];
        let xz_dist = (dx * dx + dz * dz).sqrt();

        let target_yaw = ((-dx).atan2(dz) * 180.0 / (PI as f64)) as f32;
        let target_pitch = ((-dy).atan2(xz_dist) * 180.0 / (PI as f64)) as f32;

        let delta_yaw = wrap_deg(target_yaw - self.yaw);
        let delta_pitch = target_pitch - self.pitch;
        let error_len = (delta_yaw * delta_yaw + delta_pitch * delta_pitch).sqrt();

        if error_len <= deadzone {
            return (0.0, 0.0);
        }

        // 2. М'яка зона переходу від мертвої зони (Ramp)
        let ramp_t = ((error_len - deadzone) / 3.0).clamp(0.0, 1.0);
        let ramp = ramp_t.max(0.25);

        // 3. Затримка реакції (Reaction Delay)
        let reaction_ratio = (self.assist_state.target_tracked_time / reaction_delay).clamp(0.0, 1.0);
        let reaction_ramp = (0.35 + 0.65 * reaction_ratio).clamp(0.35, 1.0);

        // 4. Плавність (Smoothness):
        let responsiveness = (1.10 - smoothness).clamp(0.1, 1.0);
        let base_speed = responsiveness.powf(1.8) * 36.0 + 2.0;

        let tracking_speed = base_speed * ramp * reaction_ramp;
        let step_factor = (1.0 - (-tracking_speed * dt).exp()).clamp(0.0, 1.0);

        // 5. Людський біомеханічний мікро-тремор (Tremor)
        let tremor = if tremor_amp > 0.001 {
            let p = self.assist_state.tremor_phase;
            let ty = (p.sin() * 0.6 + (p * 1.7).sin() * 0.4) * (tremor_amp * 0.08);
            let tp = ((p * 1.2).cos() * 0.6 + (p * 1.9).cos() * 0.4) * (tremor_amp * 0.08);
            [ty, tp]
        } else {
            [0.0, 0.0]
        };

        let step_yaw = delta_yaw * step_factor + tremor[0];
        let step_pitch = delta_pitch * step_factor + tremor[1];

        (step_yaw, step_pitch)
    }

    /// Воксельний DDA-промінь прямої видимості.
    /// Перевіряє лише повнотілі тверді блоки (камінь, дерево, земля тощо).
    /// Трава, квіти, ліани, папороть, бамбук та вода НЕ блокують огляд.
    pub fn is_line_of_sight_blocked(&self, start: [f64; 3], end: [f64; 3]) -> bool {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let dz = end[2] - start[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        if dist < 0.25 {
            return false;
        }

        let dir = [dx / dist, dy / dist, dz / dist];
        let mut pos = start.map(|v| v.floor() as i32);
        let end_pos = end.map(|v| v.floor() as i32);
        let step = dir.map(|d| if d > 0.0 { 1 } else { -1 });
        let delta = dir.map(|d| {
            if d.abs() < 1e-9 {
                f64::INFINITY
            } else {
                (1.0 / d).abs()
            }
        });

        let mut t_max = [0.0; 3];
        for i in 0..3 {
            let boundary = if dir[i] > 0.0 {
                f64::from(pos[i]) + 1.0
            } else {
                f64::from(pos[i])
            };
            t_max[i] = if dir[i].abs() < 1e-9 {
                f64::INFINITY
            } else {
                (boundary - start[i]) / dir[i]
            };
        }

        let mut t = 0.0;
        let max_t = dist - 0.2; // не перевіряємо сам блок, у якому стоїть моб

        while t < max_t {
            if pos == end_pos {
                break;
            }

            let state = self.world.block(pos[0], pos[1], pos[2]);
            if !self.res.facts().is_air(state) {
                let f = self.res.facts().get(state);
                // Тільки реальні тверді перешкоди блокують видимість:
                if f.full_collision || (f.solid_render && f.can_occlude) {
                    return true;
                }
            }

            if t_max[0] < t_max[1] {
                if t_max[0] < t_max[2] {
                    t = t_max[0];
                    t_max[0] += delta[0];
                    pos[0] += step[0];
                } else {
                    t = t_max[2];
                    t_max[2] += delta[2];
                    pos[2] += step[2];
                }
            } else if t_max[1] < t_max[2] {
                t = t_max[1];
                t_max[1] += delta[1];
                pos[1] += step[1];
            } else {
                t = t_max[2];
                t_max[2] += delta[2];
                pos[2] += step[2];
            }
        }

        false
    }

    /// Головна гаряча кнопка дії: у Lock-On циклічно лочить/розлочує, в Assist вмикає/вимикає
    pub fn trigger_combat_action(&mut self) {
        let mode = self.mod_gui.get_choice("combat_mode");
        if mode == 1 {
            // Режим Lock-On: перемикання локу (Захопити ціль <-> Скинути)
            if self.assist_state.locked_mob_id.is_some() {
                self.unlock_target();
                self.assist_state.status_toast = Some((
                    "Lock-On: Скинуто (Вільний огляд)".into(),
                    1.2,
                    rgba(243, 139, 168, 255),
                ));
            } else {
                if !self.mod_gui.get_bool("combat_enabled") {
                    self.mod_gui.set_bool("combat_enabled", true);
                }
                self.lock_target_in_range();
            }
        } else {
            // Режим Assist
            self.toggle_assist();
        }
    }

    /// Швидке перемикання режиму: Assist (Ближній бій) <-> Lock-On (Дальній захват)
    pub fn switch_combat_mode(&mut self) {
        let cur = self.mod_gui.get_choice("combat_mode");
        let next = if cur == 0 { 1 } else { 0 };
        self.mod_gui.set_choice("combat_mode", next);

        if next == 1 {
            let r = self.mod_gui.get_float("lock_on_range");
            self.assist_state.status_toast = Some((
                format!("Режим: LOCK-ON (Захват [{:.0} бл])", r),
                1.5,
                rgba(166, 227, 161, 255),
            ));
        } else {
            self.unlock_target();
            let r = self.mod_gui.get_float("assist_range");
            self.assist_state.status_toast = Some((
                format!("Режим: ASSIST (Ближній бій [{:.1} бл])", r),
                1.5,
                rgba(137, 220, 235, 255),
            ));
        }
    }


    /// Скидання захопленої цілі
    pub fn unlock_target(&mut self) {
        self.assist_state.locked_mob_id = None;
        self.assist_state.locked_mob_pos = None;
        self.assist_state.locked_mob_hp = None;
        self.assist_state.locked_mob_name.clear();
        self.assist_state.target_tracked_time = 0.0;
        self.assist_state.raw_screen_pos = None;
        self.assist_state.smoothed_screen_pos = None;
        self.assist_state.lock_alpha = 0.0;
        self.assist_state.status_toast = Some(("Lock-On: ЗНЯТО".into(), 1.0, rgba(243, 139, 168, 255)));
    }

    /// Перемикання звичайного асисту
    pub fn toggle_assist(&mut self) {
        let currently_on = self.mod_gui.get_bool("combat_enabled");
        let new_state = !currently_on;
        self.mod_gui.set_bool("combat_enabled", new_state);

        if new_state {
            self.assist_state.status_toast = Some(("Assist: ON".into(), 1.0, rgba(166, 227, 161, 255)));
        } else {
            self.unlock_target();
            self.assist_state.status_toast = Some(("Assist: OFF".into(), 1.0, rgba(243, 139, 168, 255)));
        }
    }

    /// Пошук та захоплення цілі на великій відстані (10-64 блоки) за спільними налаштуваннями
    pub fn lock_target_in_range(&mut self) {
        let Some(player) = &self.player else { return };

        let max_reach = self.mod_gui.get_float("lock_on_range").clamp(10.0, 64.0) as f64;
        let max_fov = self.mod_gui.get_float("fov_cone").clamp(10.0, 120.0);
        let check_walls = self.mod_gui.get_bool("check_walls");
        let bone_choice = self.mod_gui.get_choice("target_part");
        let target_filter = self.mod_gui.get_choice("target_filter");

        let eye_pos = [
            player.position.x,
            player.position.y + 1.62,
            player.position.z,
        ];

        let mut closest_target = None;
        let mut min_score = f32::MAX;

        for mob in &self.mobs {
            if mob.health <= 0.0 {
                continue;
            }

            if target_filter == 1 && mob.kind.category != "monster" {
                continue;
            }

            let h = mob.height() as f64;
            let target_y = if h < 1.0 {
                mob.position.y + h * 0.65
            } else {
                match bone_choice {
                    1 => mob.position.y + h * 0.88,
                    2 => mob.position.y + h * 0.20,
                    _ => mob.position.y + h * 0.65,
                }
            };

            let target_pos = [mob.position.x, target_y, mob.position.z];
            let dx = target_pos[0] - eye_pos[0];
            let dy = target_y - eye_pos[1];
            let dz = target_pos[2] - eye_pos[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();

            if dist > max_reach || dist < 0.5 {
                continue;
            }

            let xz_dist = (dx * dx + dz * dz).sqrt();
            let target_yaw = ((-dx).atan2(dz) * 180.0 / (PI as f64)) as f32;
            let target_pitch = ((-dy).atan2(xz_dist) * 180.0 / (PI as f64)) as f32;

            let delta_yaw = wrap_deg(target_yaw - self.yaw).abs();
            let delta_pitch = (target_pitch - self.pitch).abs();
            let ang_dist = (delta_yaw * delta_yaw + delta_pitch * delta_pitch).sqrt();

            if ang_dist > max_fov * 0.5 {
                continue;
            }

            if check_walls && self.is_line_of_sight_blocked(eye_pos, target_pos) {
                continue;
            }

            // Пріоритет центру прицілу гравця + невелика вага відстані
            let score = ang_dist * 1.5 + (dist as f32 * 0.6);
            if score < min_score {
                min_score = score;
                let raw_name = mob.kind.id.strip_prefix("minecraft:").unwrap_or(&mob.kind.id);
                let formatted_name = raw_name.replace('_', " ");
                let max_hp = mob.kind.max_health.max(1.0);
                closest_target = Some((mob.id, target_pos, (mob.health, max_hp), formatted_name, dist));
            }
        }

        if let Some((mob_id, target, hp, name, dist)) = closest_target {
            self.assist_state.locked_mob_id = Some(mob_id);
            self.assist_state.locked_mob_pos = Some(target);
            self.assist_state.locked_mob_hp = Some(hp);
            self.assist_state.locked_mob_name = name.clone();
            self.assist_state.lock_alpha = 1.0;
            self.assist_state.target_tracked_time = 0.2;
            self.assist_state.grace_timer = 0.5;
            self.assist_state.status_toast = Some((
                format!("Lock-On: ЗАХОПЛЕНО [{}] ({:.1} бл)", name, dist),
                1.5,
                rgba(166, 227, 161, 255),
            ));
        } else {
            self.assist_state.status_toast = Some((
                "Lock-On: Ціль не знайдено".into(),
                1.2,
                rgba(249, 226, 175, 255),
            ));
        }
    }

    /// Оновлення асисту та Lock-On кожного кадру
    pub fn update_realistic_assist(&mut self, dt: f32) {
        let dt = dt.clamp(0.001, 0.05);
        self.assist_state.anim_timer += dt * 1.5;
        self.assist_state.tremor_phase += dt * 45.0;

        // Оновлення таймера повідомлення статусу на HUD
        if let Some((_, timer, _)) = &mut self.assist_state.status_toast {
            *timer -= dt;
            if *timer <= 0.0 {
                self.assist_state.status_toast = None;
            }
        }

        // Головний перемикач: якщо бойова система вимкнена, меню відкрите або ми не в активному геймплеї
        if !self.mod_gui.get_bool("combat_enabled")
            || self.mod_gui.is_open
            || self.screen != super::Screen::Playing
            || self.title.is_some()
        {
            if self.assist_state.locked_mob_id.is_some() {
                self.unlock_target();
            } else {
                self.fade_out_lock(dt);
            }
            return;
        }

        let Some(player) = &self.player else {
            self.fade_out_lock(dt);
            return;
        };

        let eye_pos = [
            player.position.x,
            player.position.y + 1.62,
            player.position.z,
        ];

        let mode = self.mod_gui.get_choice("combat_mode");
        let bone_choice = self.mod_gui.get_choice("target_part");
        let target_filter = self.mod_gui.get_choice("target_filter");
        let check_walls = self.mod_gui.get_bool("check_walls");

        // 1. Якщо утримується ціль у Lock-On (hard lock)
        if mode == 1 && self.assist_state.locked_mob_id.is_some() {
            let locked_id = self.assist_state.locked_mob_id.unwrap();
            let mut found_mob = None;
            for mob in &self.mobs {
                if mob.id == locked_id {
                    found_mob = Some((
                        mob.health,
                        mob.kind.max_health.max(1.0),
                        mob.height() as f64,
                        mob.position,
                        [mob.velocity.x, mob.velocity.y, mob.velocity.z],
                    ));
                    break;
                }
            }

            if let Some((health, max_hp, h, pos, vel)) = found_mob {
                if health <= 0.0 {
                    self.unlock_target();
                    self.assist_state.status_toast = Some(("Lock-On: Ціль знищено".into(), 1.2, rgba(249, 226, 175, 255)));
                    return;
                }

                let target_y = if h < 1.0 {
                    pos.y + h * 0.65
                } else {
                    match bone_choice {
                        1 => pos.y + h * 0.88,
                        2 => pos.y + h * 0.20,
                        _ => pos.y + h * 0.65,
                    }
                };
                let target_pos = [pos.x, target_y, pos.z];

                let dx = target_pos[0] - eye_pos[0];
                let dy = target_pos[1] - eye_pos[1];
                let dz = target_pos[2] - eye_pos[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();

                let lock_on_range = self.mod_gui.get_float("lock_on_range").clamp(10.0, 64.0) as f64;
                if dist > lock_on_range + 5.0 {
                    self.unlock_target();
                    self.assist_state.status_toast = Some(("Lock-On: Втрачено (Далеко)".into(), 1.2, rgba(243, 139, 168, 255)));
                    return;
                }

                if check_walls && self.is_line_of_sight_blocked(eye_pos, target_pos) {
                    if self.assist_state.grace_timer > 0.0 {
                        self.assist_state.grace_timer -= dt;
                    } else {
                        self.unlock_target();
                        self.assist_state.status_toast = Some(("Lock-On: Втрачено (Стіна)".into(), 1.2, rgba(243, 139, 168, 255)));
                        return;
                    }
                } else {
                    self.assist_state.grace_timer = 0.5;
                }

                self.assist_state.locked_mob_pos = Some(target_pos);
                self.assist_state.locked_mob_hp = Some((health, max_hp));
                self.assist_state.lock_alpha = 1.0;
                self.assist_state.target_tracked_time += dt;

                let (step_yaw, step_pitch) = self.calculate_aim_step(target_pos, vel, dt);
                self.apply_legit_mouse_turn(step_yaw, step_pitch);
                return;
            } else {
                self.unlock_target();
                self.assist_state.status_toast = Some(("Lock-On: Ціль зникла".into(), 1.2, rgba(249, 226, 175, 255)));
                return;
            }
        }

        // 2. Якщо обрано режим Lock-On, але зараз ціль НЕ захоплена — МИ НІКОГО НЕ ШУКАЄМО АВТОМАТИЧНО!
        // Захоплення в режимі Lock-On відбувається ТІЛЬКИ по натисканню гарячої кнопки дії (action_hotkey)!
        if mode == 1 {
            self.fade_out_lock(dt);
            return;
        }

        // 3. Режим Assist (mode == 0): динамічний асист у ближньому бою
        let only_on_attack = self.mod_gui.get_bool("only_on_attack");
        if only_on_attack && !self.left {
            self.fade_out_lock(dt);
            return;
        }

        let max_reach = self.mod_gui.get_float("assist_range").clamp(2.0, 8.0) as f64;
        let max_fov = self.mod_gui.get_float("fov_cone").clamp(10.0, 120.0);

        let mut best_target: Option<(u32, [f64; 3], [f64; 3], (f32, f32), String)> = None;
        let mut min_score = f32::MAX;

        for mob in &self.mobs {
            if mob.health <= 0.0 {
                continue;
            }

            if target_filter == 1 && mob.kind.category != "monster" {
                continue;
            }

            let h = mob.height() as f64;
            let target_y = if h < 1.0 {
                mob.position.y + h * 0.65
            } else {
                match bone_choice {
                    1 => mob.position.y + h * 0.88,
                    2 => mob.position.y + h * 0.20,
                    _ => mob.position.y + h * 0.65,
                }
            };

            let target_pos = [mob.position.x, target_y, mob.position.z];
            let dx = target_pos[0] - eye_pos[0];
            let dy = target_pos[1] - eye_pos[1];
            let dz = target_pos[2] - eye_pos[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();

            if dist > max_reach || dist < 0.3 {
                continue;
            }

            let xz_dist = (dx * dx + dz * dz).sqrt();
            let target_yaw = ((-dx).atan2(dz) * 180.0 / (PI as f64)) as f32;
            let target_pitch = ((-dy).atan2(xz_dist) * 180.0 / (PI as f64)) as f32;

            let delta_yaw = wrap_deg(target_yaw - self.yaw).abs();
            let delta_pitch = (target_pitch - self.pitch).abs();
            let ang_dist = (delta_yaw * delta_yaw + delta_pitch * delta_pitch).sqrt();

            if ang_dist > max_fov * 0.5 {
                continue;
            }

            if check_walls && self.is_line_of_sight_blocked(eye_pos, target_pos) {
                continue;
            }

            let score = ang_dist + (dist as f32 * 0.8);
            if score < min_score {
                min_score = score;
                let raw_name = mob.kind.id.strip_prefix("minecraft:").unwrap_or(&mob.kind.id);
                let formatted_name = raw_name.replace('_', " ");
                let max_hp = mob.kind.max_health.max(1.0);
                best_target = Some((
                    mob.id,
                    target_pos,
                    [mob.velocity.x, mob.velocity.y, mob.velocity.z],
                    (mob.health, max_hp),
                    formatted_name,
                ));
            }
        }

        if let Some((mob_id, target, vel, hp, name)) = best_target {
            self.assist_state.grace_timer = 0.2;

            let is_same = self.assist_state.locked_mob_id == Some(mob_id);
            if is_same {
                self.assist_state.target_tracked_time += dt;
            } else {
                self.assist_state.target_tracked_time = 0.0;
                self.assist_state.locked_mob_id = Some(mob_id);
            }

            self.assist_state.locked_mob_pos = Some(target);
            self.assist_state.locked_mob_hp = Some(hp);
            self.assist_state.locked_mob_name = name;
            self.assist_state.lock_alpha = (self.assist_state.lock_alpha + dt * 6.0).min(1.0);

            let (step_yaw, step_pitch) = self.calculate_aim_step(target, vel, dt);
            self.apply_legit_mouse_turn(step_yaw, step_pitch);
        } else {
            if self.assist_state.grace_timer > 0.0 {
                self.assist_state.grace_timer -= dt;
            } else {
                self.fade_out_lock(dt);
            }
        }
    }

    pub(super) fn fade_out_lock(&mut self, dt: f32) {
        self.assist_state.lock_alpha = (self.assist_state.lock_alpha - dt * 3.5).max(0.0);
        if self.assist_state.lock_alpha <= 0.0 {
            self.assist_state.locked_mob_id = None;
            self.assist_state.locked_mob_pos = None;
            self.assist_state.locked_mob_hp = None;
            self.assist_state.locked_mob_name.clear();
            self.assist_state.target_tracked_time = 0.0;
            self.assist_state.raw_screen_pos = None;
            self.assist_state.smoothed_screen_pos = None;
        }
    }

    /// Відмальовка прицілу Lock-On на HUD точно за координатами матриці камери Horizon
    /// з підтримкою інтерполяції рухів, налаштуванням FPS та фільтром плавного згладжування (Low-Pass Filter)
    pub fn draw_lock_on_reticle(&mut self, batch: &mut HudBatch) {
        let (win_w, win_h) = (batch.width, batch.height);

        // 0. Відмальовка статусного банера під час перемикання хоткеєм
        if let Some((text, _, color)) = &self.assist_state.status_toast {
            let text_w = (text.chars().count() as f32 * 6.0) + 16.0;
            let toast_x = ((win_w - text_w) * 0.5).round();
            let toast_y = (win_h * 0.5 + 24.0).round();
            batch.fill(&self.gui.atlas, toast_x, toast_y, text_w, 18.0, rgba(17, 17, 27, 220));
            self.draw_gui_border(batch, [toast_x, toast_y, text_w, 18.0], *color);
            batch.text_styled(&self.gui.atlas, &Text::literal(text), toast_x + 8.0, toast_y + 5.0, *color, false);
        }

        if self.assist_state.lock_alpha <= 0.01 || !self.mod_gui.get_bool("show_lock_reticle") {
            self.assist_state.smoothed_screen_pos = None;
            self.assist_state.raw_screen_pos = None;
            return;
        }

        if self.player.is_none() {
            return;
        }

        let now = self.started.elapsed().as_secs_f32();
        let dt = if self.assist_state.last_render_time > 0.0 {
            (now - self.assist_state.last_render_time).clamp(0.001, 0.1)
        } else {
            0.016
        };
        self.assist_state.last_render_time = now;

        // Налаштування ліміту FPS для оновлення мітки ESP
        let esp_fps_choice = self.mod_gui.get_choice("esp_fps_limit");
        let target_interval = match esp_fps_choice {
            1 => 1.0 / 60.0,
            2 => 1.0 / 45.0,
            3 => 1.0 / 30.0,
            4 => 1.0 / 20.0,
            _ => 0.0, // Максимальна (кожен рендер-кадр гри)
        };

        self.assist_state.esp_fps_accum += dt;
        let should_update_pos = target_interval <= 0.0
            || self.assist_state.esp_fps_accum >= target_interval
            || self.assist_state.raw_screen_pos.is_none();

        if should_update_pos {
            self.assist_state.esp_fps_accum = 0.0;

            // 1. Інтерполяція позиції моба між попереднім і поточним тиком (20 TPS -> 120+ FPS)
            let alpha = f64::from(self.clock.fraction());
            let bone_choice = self.mod_gui.get_choice("target_part");

            let mob_interp_pos = if let Some(mob_id) = self.assist_state.locked_mob_id {
                if let Some(mob) = self.mobs.iter().find(|m| m.id == mob_id) {
                    let h = mob.height() as f64;
                    let target_y_offset = if h < 1.0 {
                        h * 0.65
                    } else {
                        match bone_choice {
                            1 => h * 0.88,
                            2 => h * 0.20,
                            _ => h * 0.65,
                        }
                    };
                    let mx = mob.previous.x + (mob.position.x - mob.previous.x) * alpha;
                    let my = mob.previous.y + (mob.position.y - mob.previous.y) * alpha + target_y_offset;
                    let mz = mob.previous.z + (mob.position.z - mob.previous.z) * alpha;
                    Some([mx, my, mz])
                } else {
                    self.assist_state.locked_mob_pos
                }
            } else {
                self.assist_state.locked_mob_pos
            };

            let Some(mob_pos) = mob_interp_pos else { return };

            // 2. Інтерполяція камери гравця (eye position та rotation)
            let camera_alpha = self.clock.camera_fraction();
            let (eye_pos, view_yaw, view_pitch) = match self.camera_view(camera_alpha as f32) {
                Some((c, y, p)) => {
                    let (sy, sp) = self.shaken_view(y, p);
                    ([c.x, c.y, c.z], sy, sp)
                }
                None => {
                    let p = self.player.as_ref().unwrap();
                    ([p.position.x, p.position.y + 1.62, p.position.z], self.yaw, self.pitch)
                }
            };

            let view = horizon_client::camera::view_matrix(view_yaw, view_pitch);
            let rel = glam::Vec3::new(
                (mob_pos[0] - eye_pos[0]) as f32,
                (mob_pos[1] - eye_pos[1]) as f32,
                (mob_pos[2] - eye_pos[2]) as f32,
            );
            let cam = view.transform_point3(rel);

            let depth = -cam.z;
            if depth <= 0.25 {
                self.assist_state.raw_screen_pos = None;
                return;
            }

            let fov_deg = (self.settings.fov as f32).clamp(30.0, 130.0);
            let focal = (win_h * 0.5) / (fov_deg.to_radians() * 0.5).tan();

            let raw_x = (win_w * 0.5) + (cam.x / depth) * focal;
            let raw_y = (win_h * 0.5) - (cam.y / depth) * focal;
            self.assist_state.raw_screen_pos = Some([raw_x, raw_y]);
        }

        let Some(raw_pos) = self.assist_state.raw_screen_pos else { return };

        // 3. Фільтр плавного згладжування тремтіння (EMA Low-Pass Filter)
        let smooth_val = self.mod_gui.get_float("esp_smooth").clamp(0.0, 0.95);
        let (screen_x, screen_y) = if smooth_val > 0.01 {
            let response = 45.0 * (1.0 - smooth_val);
            let factor = (1.0 - (-response * dt).exp()).clamp(0.05, 1.0);
            let smoothed = match self.assist_state.smoothed_screen_pos {
                Some(last) => [
                    last[0] + (raw_pos[0] - last[0]) * factor,
                    last[1] + (raw_pos[1] - last[1]) * factor,
                ],
                None => raw_pos,
            };
            self.assist_state.smoothed_screen_pos = Some(smoothed);
            (smoothed[0].round(), smoothed[1].round())
        } else {
            self.assist_state.smoothed_screen_pos = Some(raw_pos);
            (raw_pos[0].round(), raw_pos[1].round())
        };

        if screen_x < -50.0 || screen_x > win_w + 50.0 || screen_y < -50.0 || screen_y > win_h + 50.0 {
            return;
        }

        let alpha_byte = (self.assist_state.lock_alpha * 240.0) as u8;
        let marker_color = rgba(203, 166, 247, alpha_byte); // Лаванда
        let bg_color = rgba(17, 17, 27, (alpha_byte as f32 * 0.75) as u8);

        let style = self.mod_gui.get_choice("reticle_style");
        let r = 12.0;

        match style {
            1 => {
                // Стиль "Кутики [ ]"
                let k = 4.0;
                batch.fill(&self.gui.atlas, screen_x - r, screen_y - r, k, 1.5, marker_color);
                batch.fill(&self.gui.atlas, screen_x - r, screen_y - r, 1.5, k, marker_color);
                batch.fill(&self.gui.atlas, screen_x + r - k, screen_y - r, k, 1.5, marker_color);
                batch.fill(&self.gui.atlas, screen_x + r - 1.5, screen_y - r, 1.5, k, marker_color);
                batch.fill(&self.gui.atlas, screen_x - r, screen_y + r - 1.5, k, 1.5, marker_color);
                batch.fill(&self.gui.atlas, screen_x - r, screen_y + r - k, 1.5, k, marker_color);
                batch.fill(&self.gui.atlas, screen_x + r - k, screen_y + r - 1.5, k, 1.5, marker_color);
                batch.fill(&self.gui.atlas, screen_x + r - 1.5, screen_y + r - k, 1.5, k, marker_color);
            }
            2 => {
                // Стиль "Коло (Статичне)" — чітке кільце без обертання, стабільне без тремтіння
                for i in 0..12 {
                    let a = i as f32 * (PI / 6.0);
                    let (sa, ca) = a.sin_cos();
                    batch.fill(&self.gui.atlas, screen_x + ca * r - 1.0, screen_y + sa * r - 1.0, 2.0, 2.0, marker_color);
                }
            }
            3 => {
                // Стиль "Коло (Анімоване)" — повільне обертання
                for i in 0..8 {
                    let a = (i as f32 * PI / 4.0) + self.assist_state.anim_timer;
                    let (sa, ca) = a.sin_cos();
                    batch.fill(&self.gui.atlas, screen_x + ca * r - 1.0, screen_y + sa * r - 1.0, 2.0, 2.0, marker_color);
                }
            }
            _ => {
                // Стиль "Ромб ◇" — статичний стильний приціл
                let d = r * 0.9;
                batch.fill(&self.gui.atlas, screen_x - 1.5, screen_y - d - 1.0, 3.0, 2.0, marker_color);
                batch.fill(&self.gui.atlas, screen_x + d, screen_y - 1.0, 2.0, 3.0, marker_color);
                batch.fill(&self.gui.atlas, screen_x - 1.5, screen_y + d, 3.0, 2.0, marker_color);
                batch.fill(&self.gui.atlas, screen_x - d - 1.0, screen_y - 1.0, 2.0, 3.0, marker_color);
            }
        }

        // Центральна цятка
        batch.fill(&self.gui.atlas, screen_x - 1.0, screen_y - 1.0, 2.0, 2.0, marker_color);

        // Смужка HP моба
        if self.mod_gui.get_bool("show_target_hp") {
            if let Some((cur_hp, max_hp)) = self.assist_state.locked_mob_hp {
                let max_hp = max_hp.max(1.0);
                let bar_w = 40.0;
                let bar_h = 3.5;
                let bar_x = screen_x - bar_w * 0.5;
                let bar_y = screen_y - r - 9.0;

                // Темний фон смужки
                batch.fill(&self.gui.atlas, bar_x - 1.0, bar_y - 1.0, bar_w + 2.0, bar_h + 2.0, bg_color);
                let fill_ratio = (cur_hp / max_hp).clamp(0.0, 1.0);

                let hp_color = if fill_ratio > 0.5 {
                    rgba(166, 227, 161, alpha_byte) // Зелений
                } else if fill_ratio > 0.25 {
                    rgba(249, 226, 175, alpha_byte) // Жовтий
                } else {
                    rgba(243, 139, 168, alpha_byte) // Червоний
                };
                batch.fill(&self.gui.atlas, bar_x, bar_y, bar_w * fill_ratio, bar_h, hp_color);

                // Назва моба над смужкою HP
                if !self.assist_state.locked_mob_name.is_empty() {
                    let info = format!("{} ({:.0})", self.assist_state.locked_mob_name, cur_hp);
                    let text_x = screen_x - (info.chars().count() as f32 * 2.8);
                    batch.text_styled(
                        &self.gui.atlas,
                        &Text::literal(&info),
                        text_x,
                        bar_y - 9.0,
                        rgba(205, 214, 244, alpha_byte),
                        false,
                    );
                }
            }
        }
    }
}
