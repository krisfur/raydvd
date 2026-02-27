use clap::Parser;
use raylib::{core::window, ffi, prelude::*};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[cfg(target_os = "linux")]
use ksni::{blocking::TrayMethods, menu::StandardItem};

const LOGO_DRAW_WIDTH: f32 = 240.0;
const SPEED_X: f32 = 240.0;
const SPEED_Y: f32 = 180.0;
const CORNER_FLASH_FRAMES: u8 = 12;
const DEFAULT_CORNER_MARGIN: i32 = 5;
const MAX_STEP_PIXELS: f32 = 16.0;
const BOUNCE_JITTER_DEGREES: f32 = 0.45;

#[derive(Parser, Debug)]
#[command(name = "raydvd", version, about = "Transparent bouncing DVD overlay")]
struct Args {
    #[arg(
        short = 's',
        long,
        default_value_t = 1.0,
        value_parser = parse_speed_multiplier,
        help = "Multiply logo speed by this value (> 0)"
    )]
    speed: f32,

    #[arg(
        short = 'c',
        long,
        default_value_t = DEFAULT_CORNER_MARGIN,
        value_parser = parse_corner_margin,
        help = "Corner hit margin in pixels (>= 0)"
    )]
    corner: i32,

    #[arg(short = 't', long, help = "Draw center-point trace path")]
    trace: bool,
}

fn parse_speed_multiplier(input: &str) -> Result<f32, String> {
    let value: f32 = input
        .parse()
        .map_err(|_| format!("'{input}' is not a valid float"))?;
    if value > 0.0 && value.is_finite() {
        Ok(value)
    } else {
        Err("speed must be a finite value greater than 0".to_string())
    }
}

fn parse_corner_margin(input: &str) -> Result<i32, String> {
    let value: i32 = input
        .parse()
        .map_err(|_| format!("'{input}' is not a valid integer"))?;
    if value >= 0 {
        Ok(value)
    } else {
        Err("corner margin must be an integer >= 0".to_string())
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum LogoColor {
    Red,
    Orange,
    Yellow,
    Lime,
    Cyan,
    Blue,
    Violet,
    Magenta,
    Pink,
    White,
    Gold,
}

impl LogoColor {
    const ALL: [LogoColor; 10] = [
        LogoColor::Red,
        LogoColor::Orange,
        LogoColor::Yellow,
        LogoColor::Lime,
        LogoColor::Cyan,
        LogoColor::Blue,
        LogoColor::Violet,
        LogoColor::Magenta,
        LogoColor::Pink,
        LogoColor::White,
    ];

    const CORNER_FLASH: [LogoColor; 7] = [
        LogoColor::Gold,
        LogoColor::Red,
        LogoColor::Yellow,
        LogoColor::Lime,
        LogoColor::Cyan,
        LogoColor::Blue,
        LogoColor::Magenta,
    ];

    fn color(self) -> Color {
        match self {
            LogoColor::Red => Color::new(255, 62, 62, 255),
            LogoColor::Orange => Color::new(255, 146, 44, 255),
            LogoColor::Yellow => Color::new(255, 218, 56, 255),
            LogoColor::Lime => Color::new(128, 255, 74, 255),
            LogoColor::Cyan => Color::new(66, 233, 255, 255),
            LogoColor::Blue => Color::new(70, 132, 255, 255),
            LogoColor::Violet => Color::new(140, 98, 255, 255),
            LogoColor::Magenta => Color::new(255, 74, 234, 255),
            LogoColor::Pink => Color::new(255, 104, 164, 255),
            LogoColor::White => Color::new(255, 255, 255, 255),
            LogoColor::Gold => Color::new(255, 210, 60, 255),
        }
    }
}

fn random_logo_color(excluding: LogoColor) -> LogoColor {
    loop {
        let idx = unsafe { ffi::GetRandomValue(0, (LogoColor::ALL.len() - 1) as i32) } as usize;
        let choice = LogoColor::ALL[idx];
        if choice != excluding {
            return choice;
        }
    }
}

fn apply_bounce_jitter(vel: &mut Vector2) {
    let speed = (vel.x * vel.x + vel.y * vel.y).sqrt();
    if speed <= f32::EPSILON {
        return;
    }

    let jitter = unsafe { ffi::GetRandomValue(-1000, 1000) } as f32 / 1000.0;
    let angle = jitter * BOUNCE_JITTER_DEGREES.to_radians();
    let (sin_a, cos_a) = angle.sin_cos();

    let rotated_x = vel.x * cos_a - vel.y * sin_a;
    let rotated_y = vel.x * sin_a + vel.y * cos_a;
    let rotated_len = (rotated_x * rotated_x + rotated_y * rotated_y).sqrt();
    if rotated_len > f32::EPSILON {
        let scale = speed / rotated_len;
        vel.x = rotated_x * scale;
        vel.y = rotated_y * scale;
    }
}

#[cfg(target_os = "linux")]
struct TrayApp {
    running: Arc<AtomicBool>,
}

#[cfg(target_os = "linux")]
impl ksni::Tray for TrayApp {
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "raydvd".to_string()
    }

    fn title(&self) -> String {
        "💿 raydvd".to_string()
    }

    fn icon_name(&self) -> String {
        "media-optical".to_string()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "💿 raydvd".to_string(),
            description: "Transparent DVD overlay".to_string(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![ksni::MenuItem::Standard(StandardItem {
            label: "Quit".into(),
            activate: Box::new(|this: &mut Self| {
                this.running.store(false, Ordering::Relaxed);
            }),
            ..Default::default()
        })]
    }
}

fn main() {
    let args = Args::parse();

    let running = Arc::new(AtomicBool::new(true));
    #[cfg(target_os = "linux")]
    let _tray_handle = {
        let tray = TrayApp {
            running: Arc::clone(&running),
        };
        match tray.spawn() {
            Ok(handle) => Some(handle),
            Err(err) => {
                eprintln!("warning: tray unavailable: {err:?}");
                None
            }
        }
    };

    let (mut rl, thread) = raylib::init()
        .size(1280, 720)
        .title("raydvd")
        .undecorated()
        .transparent()
        .build();

    let logo_path = format!("{}/src/dvd.png", env!("CARGO_MANIFEST_DIR"));
    let mut logo_image = Image::load_image(&logo_path)
        .expect("failed to load src/dvd.png; ensure the file exists and is valid PNG");
    logo_image.color_replace(Color::BLACK, Color::WHITE);
    let logo_texture = rl
        .load_texture_from_image(&thread, &logo_image)
        .expect("failed to create texture from src/dvd.png");
    let logo_scale = LOGO_DRAW_WIDTH / logo_texture.width() as f32;
    let logo_width = logo_texture.width() as f32 * logo_scale;
    let logo_height = logo_texture.height() as f32 * logo_scale;
    let mut logo_color = LogoColor::Cyan;

    rl.set_target_fps(60);
    unsafe {
        ffi::SetWindowState(
            (ffi::ConfigFlags::FLAG_WINDOW_MOUSE_PASSTHROUGH as u32)
                | (ffi::ConfigFlags::FLAG_WINDOW_TOPMOST as u32)
                | (ffi::ConfigFlags::FLAG_WINDOW_ALWAYS_RUN as u32),
        );
    }

    let mut monitor = window::get_current_monitor();
    let mut screen_width = window::get_monitor_width(monitor);
    let mut screen_height = window::get_monitor_height(monitor);
    if screen_width <= 0 || screen_height <= 0 {
        screen_width = rl.get_screen_width();
        screen_height = rl.get_screen_height();
    }
    rl.set_window_position(0, 0);
    rl.set_window_size(screen_width, screen_height);

    let mut pos = Vector2::new(
        (screen_width as f32 - logo_width) * 0.5,
        (screen_height as f32 - logo_height) * 0.5,
    );
    let mut vel = Vector2::new(SPEED_X * args.speed, SPEED_Y * args.speed);
    let mut corner_flash_frames = 0u8;
    let mut corner_flash_step = 0usize;
    let mut trace_points: Vec<Vector2> = Vec::new();
    if args.trace {
        trace_points.push(Vector2::new(
            pos.x + logo_width * 0.5,
            pos.y + logo_height * 0.5,
        ));
    }

    while running.load(Ordering::Relaxed) && !rl.window_should_close() {
        let ctrl_pressed = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
            || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
        if rl.is_window_focused() && ctrl_pressed && rl.is_key_pressed(KeyboardKey::KEY_C) {
            running.store(false, Ordering::Relaxed);
            continue;
        }

        let current_monitor = window::get_current_monitor();
        if current_monitor != monitor {
            monitor = current_monitor;
            screen_width = window::get_monitor_width(monitor).max(1);
            screen_height = window::get_monitor_height(monitor).max(1);
            rl.set_window_position(0, 0);
            rl.set_window_size(screen_width, screen_height);
        } else {
            screen_width = rl.get_screen_width().max(1);
            screen_height = rl.get_screen_height().max(1);
        }

        let dt = rl.get_frame_time();
        let distance = vel.x.abs().max(vel.y.abs()) * dt;
        let steps = (distance / MAX_STEP_PIXELS).ceil().max(1.0) as i32;
        let sub_dt = dt / steps as f32;
        let mut bounced_x_any = false;
        let mut bounced_y_any = false;
        let mut corner_hit = false;

        for _ in 0..steps {
            let mut bounced_x = false;
            let mut bounced_y = false;

            pos.x += vel.x * sub_dt;
            pos.y += vel.y * sub_dt;

            if pos.x <= 0.0 {
                pos.x = 0.0;
                vel.x = vel.x.abs();
                bounced_x = true;
            } else if pos.x + logo_width >= screen_width as f32 {
                pos.x = screen_width as f32 - logo_width;
                vel.x = -vel.x.abs();
                bounced_x = true;
            }

            if pos.y <= 0.0 {
                pos.y = 0.0;
                vel.y = vel.y.abs();
                bounced_y = true;
            } else if pos.y + logo_height >= screen_height as f32 {
                pos.y = screen_height as f32 - logo_height;
                vel.y = -vel.y.abs();
                bounced_y = true;
            }

            if bounced_x {
                bounced_x_any = true;
            }
            if bounced_y {
                bounced_y_any = true;
            }

            let corner_margin = args.corner as f32;
            let near_top = pos.y <= corner_margin;
            let near_bottom = pos.y + logo_height >= screen_height as f32 - corner_margin;
            let near_left = pos.x <= corner_margin;
            let near_right = pos.x + logo_width >= screen_width as f32 - corner_margin;

            let near_corner = (near_left || near_right) && (near_top || near_bottom);

            if (bounced_x && bounced_y) || (near_corner && (bounced_x || bounced_y)) {
                corner_hit = true;
            }
        }

        if corner_hit {
            println!(
                "corner hit at ({:.1}, {:.1}) with speed {:.2}x",
                pos.x + logo_width * 0.5,
                pos.y + logo_height * 0.5,
                args.speed
            );
            logo_color = LogoColor::Gold;
            corner_flash_frames = CORNER_FLASH_FRAMES;
            corner_flash_step = 0;
        } else if bounced_x_any || bounced_y_any {
            logo_color = random_logo_color(logo_color);
        }

        if bounced_x_any || bounced_y_any {
            apply_bounce_jitter(&mut vel);
        }

        let draw_color = if corner_flash_frames > 0 {
            let flash_color =
                LogoColor::CORNER_FLASH[corner_flash_step % LogoColor::CORNER_FLASH.len()];
            corner_flash_frames -= 1;
            corner_flash_step += 1;
            flash_color.color()
        } else {
            logo_color.color()
        };

        if args.trace {
            trace_points.push(Vector2::new(
                pos.x + logo_width * 0.5,
                pos.y + logo_height * 0.5,
            ));
        }

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(0, 0, 0, 0));
        if args.trace {
            for segment in trace_points.windows(2) {
                d.draw_line_v(segment[0], segment[1], Color::new(255, 255, 255, 70));
            }
        }
        d.draw_texture_ex(&logo_texture, pos, 0.0, logo_scale, draw_color);
    }
}
