use macroquad::prelude::*;

// Dunya boyutu (blok sayisi)
const W: i32 = 24;
const H: i32 = 16;
const D: i32 = 24;

// Blok turleri
const AIR: u8 = 0;
const GRASS: u8 = 1;
const DIRT: u8 = 2;
const STONE: u8 = 3;
const WOOD: u8 = 4;
const LEAVES: u8 = 5;
const PLACEABLE: [u8; 5] = [GRASS, DIRT, STONE, WOOD, LEAVES];

fn block_name(t: u8) -> &'static str {
    match t {
        GRASS => "Cim",
        DIRT => "Toprak",
        STONE => "Tas",
        WOOD => "Odun",
        _ => "Yaprak",
    }
}

fn block_color(t: u8, p: Vec3) -> Color {
    let base = match t {
        GRASS => (0.30, 0.68, 0.25),
        DIRT => (0.55, 0.38, 0.22),
        STONE => (0.50, 0.50, 0.52),
        WOOD => (0.42, 0.28, 0.14),
        _ => (0.18, 0.50, 0.15),
    };
    // Bloklara hafif renk farki ver ki kupler birbirinden ayrilsin
    let n = ((p.x * 7.0 + p.y * 13.0 + p.z * 3.0) as i32).rem_euclid(5) as f32 * 0.04;
    let k = 0.85 + n;
    Color::new(base.0 * k, base.1 * k, base.2 * k, 1.0)
}

fn in_bounds(x: i32, y: i32, z: i32) -> bool {
    x >= 0 && x < W && y >= 0 && y < H && z >= 0 && z < D
}

struct World {
    blocks: Vec<u8>,
}

impl World {
    fn new() -> Self {
        let mut w = World {
            blocks: vec![AIR; (W * H * D) as usize],
        };

        // Arazi (sinus dalgalarindan basit yukseklik haritasi)
        for x in 0..W {
            for z in 0..D {
                let fx = x as f32;
                let fz = z as f32;
                let h = (6.0
                    + (fx * 0.35).sin() * 2.0
                    + (fz * 0.30).cos() * 2.0
                    + ((fx + fz) * 0.15).sin() * 1.5) as i32;
                let h = h.clamp(2, H - 7);
                for y in 0..h {
                    let t = if y == h - 1 {
                        GRASS
                    } else if y >= h - 4 {
                        DIRT
                    } else {
                        STONE
                    };
                    w.set(x, y, z, t);
                }
            }
        }

        // Agaclar
        for x in 2..W - 2 {
            for z in 2..D - 2 {
                let hash = (x.wrapping_mul(73856093) ^ z.wrapping_mul(19349663)) as u32 % 41;
                if hash != 0 {
                    continue;
                }
                let mut ground = 0;
                for y in (0..H).rev() {
                    if w.get(x, y, z) != AIR {
                        ground = y + 1;
                        break;
                    }
                }
                for y in ground..ground + 4 {
                    w.set(x, y, z, WOOD);
                }
                for dy in 3..=5 {
                    for dx in -1..=1 {
                        for dz in -1..=1 {
                            if dy == 5 && (dx != 0 || dz != 0) {
                                continue;
                            }
                            if w.get(x + dx, ground + dy, z + dz) == AIR {
                                w.set(x + dx, ground + dy, z + dz, LEAVES);
                            }
                        }
                    }
                }
            }
        }
        w
    }

    fn idx(x: i32, y: i32, z: i32) -> usize {
        ((y * D + z) * W + x) as usize
    }

    fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if x < 0 || x >= W || z < 0 || z >= D || y < 0 {
            return STONE; // gorunmez duvarlar ve zemin
        }
        if y >= H {
            return AIR;
        }
        self.blocks[Self::idx(x, y, z)]
    }

    fn set(&mut self, x: i32, y: i32, z: i32, t: u8) {
        if in_bounds(x, y, z) {
            self.blocks[Self::idx(x, y, z)] = t;
        }
    }

    // Sadece en az bir yuzu havaya bakan bloklari cizmek icin liste
    fn visible(&self) -> Vec<(Vec3, Color)> {
        let dirs = [
            (1, 0, 0),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
            (0, 0, 1),
            (0, 0, -1),
        ];
        let mut v = Vec::new();
        for x in 0..W {
            for y in 0..H {
                for z in 0..D {
                    let t = self.get(x, y, z);
                    if t == AIR {
                        continue;
                    }
                    let exposed = dirs
                        .iter()
                        .any(|(dx, dy, dz)| self.get(x + dx, y + dy, z + dz) == AIR);
                    if exposed {
                        let c = vec3(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                        v.push((c, block_color(t, c)));
                    }
                }
            }
        }
        v
    }
}

// Oyuncu kutusu (ayak merkezli) bir blokla cakisiyor mu?
fn collides(w: &World, p: Vec3) -> bool {
    let r = 0.3;
    let xs = [(p.x - r).floor() as i32, (p.x + r).floor() as i32];
    let zs = [(p.z - r).floor() as i32, (p.z + r).floor() as i32];
    let ys = [
        p.y.floor() as i32,
        (p.y + 0.9).floor() as i32,
        (p.y + 1.69).floor() as i32,
    ];
    for &x in &xs {
        for &z in &zs {
            for &y in &ys {
                if w.get(x, y, z) != AIR {
                    return true;
                }
            }
        }
    }
    false
}

// Bakis dogrultusunda ilk blogu bul: (vurulan blok, onundeki bos hucre)
fn raycast(w: &World, origin: Vec3, dir: Vec3) -> Option<((i32, i32, i32), (i32, i32, i32))> {
    let cell = |p: Vec3| (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
    let mut prev = cell(origin);
    let mut t = 0.0;
    while t < 5.0 {
        let c = cell(origin + dir * t);
        if in_bounds(c.0, c.1, c.2) && w.get(c.0, c.1, c.2) != AIR {
            return Some((c, prev));
        }
        prev = c;
        t += 0.04;
    }
    None
}

fn draw_btn(c: Vec2, r: f32, label: &str, pressed: bool) {
    let a = if pressed { 0.55 } else { 0.25 };
    draw_circle(c.x, c.y, r, Color::new(1.0, 1.0, 1.0, a));
    draw_circle_lines(c.x, c.y, r, 3.0, Color::new(1.0, 1.0, 1.0, 0.8));
    let size = (r * 0.5) as u16;
    let d = measure_text(label, None, size, 1.0);
    draw_text(label, c.x - d.width / 2.0, c.y + d.height / 2.0, size as f32, WHITE);
}

fn conf() -> Conf {
    Conf {
        window_title: "Mini Craft".to_owned(),
        window_width: 960,
        window_height: 540,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    let mut world = World::new();
    let mut cubes = world.visible();

    let mut pos = vec3(W as f32 / 2.0, H as f32, D as f32 / 2.0);
    let mut vel_y = 0.0f32;
    let mut on_ground = false;
    let mut yaw = 0.8f32;
    let mut pitch = -0.3f32;
    let mut sel = 0usize;
    let mut cooldown = 0.0f32;

    let mut stick: Option<(u64, Vec2)> = None; // (dokunus id, baslangic noktasi)
    let mut look: Option<(u64, Vec2)> = None; // (dokunus id, son nokta)
    let mut stick_pos = Vec2::ZERO;

    loop {
        let dt = get_frame_time().min(0.05);
        let sw = screen_width();
        let sh = screen_height();
        let u = sh / 360.0;

        // Ekran dugmeleri
        let r_btn = 40.0 * u;
        let c_jump = vec2(sw - 80.0 * u, sh - 80.0 * u);
        let c_brk = vec2(sw - 190.0 * u, sh - 60.0 * u);
        let c_plc = vec2(sw - 190.0 * u, sh - 150.0 * u);
        let c_cyc = vec2(sw - 50.0 * u, 50.0 * u);

        let mut mv = Vec2::ZERO;
        let (mut jump, mut brk, mut plc, mut cycle) = (false, false, false, false);

        // ---- Dokunmatik giris ----
        for t in touches() {
            let p = t.position;
            if matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                if stick.map_or(false, |s| s.0 == t.id) {
                    stick = None;
                }
                if look.map_or(false, |l| l.0 == t.id) {
                    look = None;
                }
                continue;
            }
            if p.distance(c_jump) < r_btn {
                jump = true;
                continue;
            }
            if p.distance(c_brk) < r_btn {
                brk = true;
                continue;
            }
            if p.distance(c_plc) < r_btn {
                plc = true;
                continue;
            }
            if p.distance(c_cyc) < r_btn {
                if matches!(t.phase, TouchPhase::Started) {
                    cycle = true;
                }
                continue;
            }
            if matches!(t.phase, TouchPhase::Started) {
                if p.x < sw * 0.35 && stick.is_none() {
                    stick = Some((t.id, p));
                    stick_pos = p;
                } else if p.x >= sw * 0.35 && look.is_none() {
                    look = Some((t.id, p));
                }
            } else {
                if let Some((id, o)) = stick {
                    if id == t.id {
                        mv = ((p - o) / (60.0 * u)).clamp_length_max(1.0);
                        stick_pos = o + mv * 60.0 * u;
                    }
                }
                if let Some((id, last)) = look {
                    if id == t.id {
                        let d = p - last;
                        yaw += d.x * 0.006;
                        pitch -= d.y * 0.006;
                        look = Some((id, p));
                    }
                }
            }
        }

        // ---- Klavye (bilgisayarda test icin) ----
        if is_key_down(KeyCode::W) { mv.y -= 1.0; }
        if is_key_down(KeyCode::S) { mv.y += 1.0; }
        if is_key_down(KeyCode::A) { mv.x -= 1.0; }
        if is_key_down(KeyCode::D) { mv.x += 1.0; }
        mv = mv.clamp_length_max(1.0);
        if is_key_down(KeyCode::Left) { yaw -= 2.0 * dt; }
        if is_key_down(KeyCode::Right) { yaw += 2.0 * dt; }
        if is_key_down(KeyCode::Up) { pitch += 1.5 * dt; }
        if is_key_down(KeyCode::Down) { pitch -= 1.5 * dt; }
        if is_key_down(KeyCode::Space) { jump = true; }
        if is_key_down(KeyCode::Q) { brk = true; }
        if is_key_down(KeyCode::E) { plc = true; }
        if is_key_pressed(KeyCode::Tab) { cycle = true; }

        pitch = pitch.clamp(-1.5, 1.5);

        // ---- Hareket ve fizik ----
        let f = vec3(yaw.cos(), 0.0, yaw.sin());
        let r = vec3(-yaw.sin(), 0.0, yaw.cos());
        let step = (f * (-mv.y) + r * mv.x) * 4.5 * dt;

        let np = pos + vec3(step.x, 0.0, 0.0);
        if !collides(&world, np) {
            pos = np;
        }
        let np = pos + vec3(0.0, 0.0, step.z);
        if !collides(&world, np) {
            pos = np;
        }

        vel_y -= 25.0 * dt;
        if jump && on_ground {
            vel_y = 8.5;
        }
        on_ground = false;
        let np = pos + vec3(0.0, vel_y * dt, 0.0);
        if collides(&world, np) {
            if vel_y < 0.0 {
                on_ground = true;
            }
            vel_y = 0.0;
        } else {
            pos = np;
        }

        // ---- Kamera, hedef, kir / koy ----
        let eye = pos + vec3(0.0, 1.6, 0.0);
        let dir = vec3(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        );
        let target = raycast(&world, eye, dir);

        if cycle {
            sel = (sel + 1) % PLACEABLE.len();
        }

        cooldown -= dt;
        let mut changed = false;
        if cooldown <= 0.0 {
            if brk {
                if let Some((h, _)) = target {
                    world.set(h.0, h.1, h.2, AIR);
                    changed = true;
                    cooldown = 0.25;
                }
            } else if plc {
                if let Some((_, pv)) = target {
                    if in_bounds(pv.0, pv.1, pv.2) && world.get(pv.0, pv.1, pv.2) == AIR {
                        world.set(pv.0, pv.1, pv.2, PLACEABLE[sel]);
                        if collides(&world, pos) {
                            world.set(pv.0, pv.1, pv.2, AIR); // oyuncunun icine koyma
                        } else {
                            changed = true;
                        }
                        cooldown = 0.25;
                    }
                }
            }
        }
        if changed {
            cubes = world.visible();
        }

        // ---- Cizim: 3D ----
        clear_background(Color::new(0.53, 0.81, 0.92, 1.0));
        set_camera(&Camera3D {
            position: eye,
            target: eye + dir,
            up: vec3(0.0, 1.0, 0.0),
            fovy: 70.0f32.to_radians(),
            ..Default::default()
        });
        for (c, col) in &cubes {
            if c.distance_squared(eye) < 22.0 * 22.0 {
                draw_cube(*c, vec3(1.0, 1.0, 1.0), None, *col);
            }
        }
        if let Some((h, _)) = target {
            let c = vec3(h.0 as f32 + 0.5, h.1 as f32 + 0.5, h.2 as f32 + 0.5);
            draw_cube_wires(c, vec3(1.02, 1.02, 1.02), BLACK);
        }

        // ---- Cizim: arayuz ----
        set_default_camera();
        let (cx, cy) = (sw / 2.0, sh / 2.0);
        draw_line(cx - 10.0, cy, cx + 10.0, cy, 2.0, WHITE);
        draw_line(cx, cy - 10.0, cx, cy + 10.0, 2.0, WHITE);

        let base = stick.map(|s| s.1).unwrap_or(vec2(110.0 * u, sh - 110.0 * u));
        let knob = if stick.is_some() { stick_pos } else { base };
        draw_circle(base.x, base.y, 60.0 * u, Color::new(1.0, 1.0, 1.0, 0.15));
        draw_circle_lines(base.x, base.y, 60.0 * u, 3.0, Color::new(1.0, 1.0, 1.0, 0.6));
        draw_circle(knob.x, knob.y, 25.0 * u, Color::new(1.0, 1.0, 1.0, 0.45));

        draw_btn(c_jump, r_btn, "ZIPLA", jump);
        draw_btn(c_brk, r_btn, "KIR", brk);
        draw_btn(c_plc, r_btn, "KOY", plc);
        draw_btn(c_cyc, r_btn, "BLOK", false);

        let sel_t = PLACEABLE[sel];
        let sw_col = block_color(sel_t, vec3(0.5, 0.5, 0.5));
        draw_rectangle(12.0 * u, 12.0 * u, 28.0 * u, 28.0 * u, sw_col);
        draw_rectangle_lines(12.0 * u, 12.0 * u, 28.0 * u, 28.0 * u, 3.0, WHITE);
        draw_text(block_name(sel_t), 48.0 * u, 34.0 * u, 26.0 * u, WHITE);

        next_frame().await;
    }
}
