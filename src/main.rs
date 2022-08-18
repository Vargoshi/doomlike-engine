/// Rust rewriting of [3DSage Doom-like engine tutorial part 1](https://youtu.be/huMO4VQEwPc)
use sdl2::{event::Event, keyboard::Keycode, pixels, rect::Rect};
use std::f64::consts::PI;
use std::time::Instant;

extern crate sdl2;

const RES: u32 = 1; // 0x160x120 1=360x240 4=640x480
const SW: u32 = 160 * RES; // screen width
const SH: u32 = 120 * RES; // screen height
const SW2: u32 = SW / 2; // half of screen width
const SH2: u32 = SH / 2; // half of screen height
const PIXEL_SCALE: u32 = 4 / RES; // pixel scale for rendering
const SDL_SW: u32 = SW * PIXEL_SCALE; // SDL window width
const SDL_SH: u32 = SH * PIXEL_SCALE; // SDL window height
const NUMSECT: u32 = 4;
const NUMWALL: u32 = 16;

struct Time {
    frame1: u128, // frames to calculate fps
    frame2: u128,
}

struct Keys {
    w: bool,  // move forward
    s: bool,  // move backward
    a: bool,  // turn left
    d: bool,  // turn right
    sl: bool, // move left
    sr: bool, // move right
    m: bool,  // alternative controls
}

/// struct to hold calculated sine wave and cosine wave
struct Math {
    cos: [f64; 360],
    sin: [f64; 360],
}

struct Player {
    x: i32,
    y: i32,
    z: i32,
    a: i32,
    l: i32,
}

#[derive(Copy, Clone)]
struct Walls {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    c: i32,
}

#[derive(Copy, Clone)]
struct Sectors {
    ws: i32,
    we: i32,
    z1: i32,
    z2: i32,
    d: i32,
    c1: i32,
    c2: i32,
    surf: [i32; SW as usize],
    surface: i32,
}

/// draw pixel at x,y vales with chosen color preset
fn pixel(
    x: i32,
    y: i32,
    c: i32,
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
) -> Result<(), String> {
    let mut rgb: [u8; 3] = [0; 3];
    // yellow
    if c == 0 {
        rgb[0] = 255;
        rgb[1] = 255;
        rgb[2] = 0;
    }
    // dark yellow
    if c == 1 {
        rgb[0] = 160;
        rgb[1] = 160;
        rgb[2] = 0;
    }
    // green
    if c == 2 {
        rgb[0] = 0;
        rgb[1] = 255;
        rgb[2] = 0;
    }
    // dark green
    if c == 3 {
        rgb[0] = 0;
        rgb[1] = 160;
        rgb[2] = 0;
    }
    // cyan
    if c == 4 {
        rgb[0] = 0;
        rgb[1] = 255;
        rgb[2] = 255;
    }
    // dark cyan
    if c == 5 {
        rgb[0] = 0;
        rgb[1] = 160;
        rgb[2] = 160;
    }
    // brown
    if c == 6 {
        rgb[0] = 160;
        rgb[1] = 100;
        rgb[2] = 0;
    }
    // dark brown
    if c == 7 {
        rgb[0] = 110;
        rgb[1] = 50;
        rgb[2] = 0;
    }
    // background blue
    if c == 8 {
        rgb[0] = 0;
        rgb[1] = 60;
        rgb[2] = 130;
    }

    canvas.set_draw_color(pixels::Color::RGB(rgb[0], rgb[1], rgb[2]));
    canvas.fill_rect(Rect::new(
        x * PIXEL_SCALE as i32 + 2,
        SDL_SH as i32 - (y * PIXEL_SCALE as i32 + 2),
        PIXEL_SCALE,
        PIXEL_SCALE,
    ))?;

    Ok(())
}

fn move_player(keys: &Keys, player: &mut Player, math: &Math) {
    // turn left, right
    if keys.a == true && keys.m == false {
        player.a -= 4;
        if player.a < 0 {
            player.a += 360;
        }
    }
    if keys.d == true && keys.m == false {
        player.a += 4;
        if player.a > 359 {
            player.a -= 360;
        }
    }

    let dx = math.sin[player.a as usize] * 10.0;
    let dy = math.cos[player.a as usize] * 10.0;

    // move up, down
    if keys.w == true && keys.m == false {
        player.x += dx as i32;
        player.y += dy as i32;
    }
    if keys.s == true && keys.m == false {
        player.x -= dx as i32;
        player.y -= dy as i32;
    }

    // move left, right
    if keys.sl == true {
        player.x -= dy as i32;
        player.y += dx as i32;
    }
    if keys.sr == true {
        player.x += dy as i32;
        player.y -= dx as i32;
    }

    // look up, down
    if keys.a == true && keys.m == true {
        player.l -= 1;
    }
    if keys.d == true && keys.m == true {
        player.l += 1;
    }

    // fly up, down
    if keys.w == true && keys.m == true {
        player.z += 4;
    }
    if keys.s == true && keys.m == true {
        player.z -= 4;
    }
}

fn clear_background(canvas: &mut sdl2::render::Canvas<sdl2::video::Window>) -> Result<(), String> {
    for y in 0..SH as i32 {
        for x in 0..SW as i32 {
            pixel(x, y, 8, canvas)?;
        }
    }
    Ok(())
}

fn clip_behind_player(x1: &mut i32, y1: &mut i32, z1: &mut i32, x2: i32, y2: i32, z2: i32) {
    let da = *y1;
    let db = y2;
    let mut d = da - db;
    if d == 0 {
        d = 1;
    }
    let s = da / (da - db);
    *x1 = *x1 + s * (x2 - *x1);
    *y1 = *y1 + s * (y2 - *y1);
    if *y1 == 0 {
        *y1 = 1;
    }
    *z1 = *z1 + s * (z2 - *z1);
}

fn draw_wall(
    mut x1: i32,
    mut x2: i32,
    b1: i32,
    b2: i32,
    t1: i32,
    t2: i32,
    c: i32,
    s: usize,
    sectors: &mut Vec<Sectors>,
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
) -> Result<(), String> {
    let dyb = b2 - b1;
    let dyt = t2 - t1;
    let mut dx = x2 - x1;
    if dx == 0 {
        dx = 1;
    }
    let xs = x1;

    if x1 < 1 {
        x1 = 1;
    }
    if x2 < 1 {
        x2 = 1;
    }
    if x1 > SW as i32 - 1 {
        x1 = SW as i32 - 1;
    }
    if x2 > SW as i32 - 1 {
        x2 = SW as i32 - 1;
    }

    for x in x1..x2 {
        let mut y1 = dyb as f32 * (x as f32 - xs as f32 + 0.5) / dx as f32 + b1 as f32;
        let mut y2 = dyt as f32 * (x as f32 - xs as f32 + 0.5) / dx as f32 + t1 as f32;

        if y1 < 1.0 {
            y1 = 1.0;
        }
        if y2 < 1.0 {
            y2 = 1.0;
        }
        if y1 > SH as f32 - 1.0 {
            y1 = SH as f32 - 1.0;
        }
        if y2 > SH as f32 - 1.0 {
            y2 = SH as f32 - 1.0;
        }

        if sectors[s].surface == 1 {
            sectors[s].surf[x as usize] = y1 as i32;
            continue;
        }

        if sectors[s].surface == 2 {
            sectors[s].surf[x as usize] = y2 as i32;
            continue;
        }

        if sectors[s].surface == -1 {
            for y in sectors[s].surf[x as usize]..y1 as i32 {
                pixel(x, y as i32, sectors[s].c1, canvas)?;
            }
        }

        if sectors[s].surface == -2 {
            for y in y2 as i32..sectors[s].surf[x as usize] {
                pixel(x, y as i32, sectors[s].c2, canvas)?;
            }
        }

        for y in y1 as i32..y2 as i32 {
            pixel(x, y as i32, c, canvas)?;
        }
    }

    Ok(())
}

fn dist(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    let distance = (x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1);
    return distance;
}

fn draw3_d(
    math: &Math,
    player: &Player,
    sectors: &mut Vec<Sectors>,
    walls: &mut Vec<Walls>,
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
) -> Result<(), String> {
    let mut wx: [i32; 4] = [0; 4];
    let mut wy: [i32; 4] = [0; 4];
    let mut wz: [i32; 4] = [0; 4];

    let cs = math.cos[player.a as usize];
    let sn = math.sin[player.a as usize];

    for s in 0..(NUMSECT - 1) as usize {
        for w in 0..(NUMSECT as usize - s - 1) {
            if sectors[w].d < sectors[w + 1].d {
                let st = sectors[w];
                sectors[w] = sectors[w + 1];
                sectors[w + 1] = st;
            }
        }
    }

    for s in 0..NUMSECT as usize {
        sectors[s].d = 0;

        if player.z < sectors[s].z1 {
            sectors[s].surface = 1;
        } else if player.z > sectors[s].z2 {
            sectors[s].surface = 2;
        } else {
            sectors[s].surface = 0;
        }

        for render_loop in 0..2 {
            for wall in &mut walls[sectors[s].ws as usize..sectors[s].we as usize] {
                let mut x1 = wall.x1 - player.x;
                let mut y1 = wall.y1 - player.y;
                let mut x2 = wall.x2 - player.x;
                let mut y2 = wall.y2 - player.y;

                if render_loop == 0 {
                    let mut swp = x1;
                    x1 = x2;
                    x2 = swp;
                    swp = y1;
                    y1 = y2;
                    y2 = swp;
                }

                wx[0] = (x1 as f64 * cs - y1 as f64 * sn) as i32;
                wx[1] = (x2 as f64 * cs - y2 as f64 * sn) as i32;

                wx[2] = wx[0];
                wx[3] = wx[1];

                wy[0] = (y1 as f64 * cs + x1 as f64 * sn) as i32;
                wy[1] = (y2 as f64 * cs + x2 as f64 * sn) as i32;

                wy[2] = wy[0];
                wy[3] = wy[1];
                sectors[s].d += dist(0, 0, (wx[0] + wx[1]) / 2, wy[0] + wy[1] / 2);

                wz[0] = 0 - sectors[s].z1 - player.z + ((player.l * wy[0]) / 32);
                wz[1] = 0 - sectors[s].z1 - player.z + ((player.l * wy[1]) / 32);

                wz[2] = wz[0] + sectors[s].z2;
                wz[3] = wz[1] + sectors[s].z2;

                if wy[0] < 1 && wy[1] < 1 {
                    continue;
                }

                let wx1 = wx[1];
                let wy1 = wy[1];
                let wz1 = wz[1];

                let wx3 = wx[3];
                let wy3 = wy[3];
                let wz3 = wz[3];

                if wy[0] < 1 {
                    clip_behind_player(&mut wx[0], &mut wy[0], &mut wz[0], wx1, wy1, wz1);
                    clip_behind_player(&mut wx[2], &mut wy[2], &mut wz[2], wx3, wy3, wz3);
                }

                let wx0 = wx[0];
                let wy0 = wy[0];
                let wz0 = wz[0];

                let wx2 = wx[2];
                let wy2 = wy[2];
                let wz2 = wz[2];

                if wy[1] < 1 {
                    clip_behind_player(&mut wx[1], &mut wy[1], &mut wz[1], wx0, wy0, wz0);
                    clip_behind_player(&mut wx[3], &mut wy[3], &mut wz[3], wx2, wy2, wz2);
                }

                wx[0] = wx[0] * 200 / wy[0] + SW2 as i32;
                wy[0] = wz[0] * 200 / wy[0] + SH2 as i32;
                wx[1] = wx[1] * 200 / wy[1] + SW2 as i32;
                wy[1] = wz[1] * 200 / wy[1] + SH2 as i32;

                wx[2] = wx[2] * 200 / wy[2] + SW2 as i32;
                wy[2] = wz[2] * 200 / wy[2] + SH2 as i32;
                wx[3] = wx[3] * 200 / wy[3] + SW2 as i32;
                wy[3] = wz[3] * 200 / wy[3] + SH2 as i32;

                draw_wall(
                    wx[0], wx[1], wy[0], wy[1], wy[2], wy[3], wall.c, s, sectors, canvas,
                )?;
            }
            sectors[s].d /= sectors[s].we - sectors[s].ws;
            sectors[s].surface *= -1;
        }
    }

    Ok(())
}

fn display(
    time: &mut Time,
    keys: &Keys,
    player: &mut Player,
    time_instant: Instant,
    math: &Math,
    sectors: &mut Vec<Sectors>,
    walls: &mut Vec<Walls>,
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
) -> Result<(), String> {
    // calculate framerate
    if (time.frame1 - time.frame2) >= 50 {
        clear_background(canvas)?;
        move_player(keys, player, math);
        draw3_d(math, player, sectors, walls, canvas)?;

        time.frame2 = time.frame1;

        canvas.present();
    }

    time.frame1 = time_instant.elapsed().as_millis();

    Ok(())
}

fn main() -> Result<(), String> {
    let mut keys = Keys {
        w: false,
        s: false,
        a: false,
        d: false,
        sl: false,
        sr: false,
        m: false,
    };

    let time_instant = Instant::now();
    let mut time = Time {
        frame1: 0,
        frame2: 0,
    };

    let mut math = Math {
        cos: [0.0; 360],
        sin: [0.0; 360],
    };

    let mut player = Player {
        x: 70,
        y: -110,
        z: 20,
        a: 0,
        l: 0,
    };

    let mut walls: Vec<Walls> = vec![
        Walls {
            x1: 0,
            y1: 0,
            x2: 0,
            y2: 0,
            c: 0
        };
        30
    ];

    let mut sectors: Vec<Sectors> = vec![
        Sectors {
            ws: 0,
            we: 0,
            z1: 0,
            z2: 0,
            d: 0,
            c1: 0,
            c2: 0,
            surf: [0; SW as usize],
            surface: 0
        };
        30
    ];

    let load_sectors: Vec<i32> = vec![
        0, 4, 0, 40, 2, 3, 4, 8, 0, 40, 4, 5, 8, 12, 0, 40, 6, 7, 12, 16, 0, 40, 0, 1,
    ];

    let load_walls: Vec<i32> = vec![
        0, 0, 32, 0, 0, 32, 0, 32, 32, 1, 32, 32, 0, 32, 0, 0, 32, 0, 0, 1, 64, 0, 96, 0, 2, 96, 0,
        96, 32, 3, 96, 32, 64, 32, 2, 64, 32, 64, 0, 3, 64, 64, 96, 64, 4, 96, 64, 96, 96, 5, 96,
        96, 64, 96, 4, 64, 96, 64, 64, 5, 0, 64, 32, 64, 6, 32, 64, 32, 96, 7, 32, 96, 0, 96, 6, 0,
        96, 0, 64, 7,
    ];

    for x in 0..360 {
        math.cos[x] = (x as f64 / 180.0 * PI).cos();
        math.sin[x] = (x as f64 / 180.0 * PI).sin();
    }

    let mut v1 = 0;
    let mut v2 = 0;

    for sector in &mut sectors[0..NUMSECT as usize] {
        sector.ws = load_sectors[v1 + 0];
        sector.we = load_sectors[v1 + 1];
        sector.z1 = load_sectors[v1 + 2];
        sector.z2 = load_sectors[v1 + 3] - load_sectors[v1 + 2];
        sector.c1 = load_sectors[v1 + 4];
        sector.c2 = load_sectors[v1 + 5];
        v1 += 6;
        for wall in &mut walls[sector.ws as usize..sector.we as usize] {
            wall.x1 = load_walls[v2 + 0];
            wall.y1 = load_walls[v2 + 1];
            wall.x2 = load_walls[v2 + 2];
            wall.y2 = load_walls[v2 + 3];
            wall.c = load_walls[v2 + 4];
            v2 += 5;
        }
    }

    let sdl_context = sdl2::init()?;
    let video_subsys = sdl_context.video()?;
    let window = video_subsys
        .window("Doomlike", SDL_SW, SDL_SH)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    let mut events = sdl_context.event_pump()?;

    'main: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => break 'main,

                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => {
                    if keycode == Keycode::Escape {
                        break 'main;
                    }
                    if keycode == Keycode::A {
                        keys.a = true;
                    }
                    if keycode == Keycode::D {
                        keys.d = true;
                    }
                    if keycode == Keycode::W {
                        keys.w = true;
                    }
                    if keycode == Keycode::S {
                        keys.s = true;
                    }
                    if keycode == Keycode::M {
                        keys.m = true;
                    }
                    if keycode == Keycode::Q {
                        keys.sl = true;
                    }
                    if keycode == Keycode::E {
                        keys.sr = true;
                    }
                }
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => {
                    if keycode == Keycode::A {
                        keys.a = false;
                    }
                    if keycode == Keycode::D {
                        keys.d = false;
                    }
                    if keycode == Keycode::W {
                        keys.w = false
                    }
                    if keycode == Keycode::S {
                        keys.s = false;
                    }
                    if keycode == Keycode::M {
                        keys.m = false;
                    }
                    if keycode == Keycode::Q {
                        keys.sl = false;
                    }
                    if keycode == Keycode::E {
                        keys.sr = false;
                    }
                }
                _ => {}
            }
        }

        display(
            &mut time,
            &keys,
            &mut player,
            time_instant,
            &math,
            &mut sectors,
            &mut walls,
            &mut canvas,
        )?;
    }

    Ok(())
}
