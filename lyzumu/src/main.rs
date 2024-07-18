use std::{env, path::Path};

use anyhow::Result;
use winit::{
    dpi,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use lyzumu_audio::AudioSystem;
use lyzumu_chart::parse::osu_mania::OsuManiaParser;

fn main() -> Result<()> {
    let env = env_logger::Env::default()
        .filter_or("MY_LOG_LEVEL", "trace")
        .write_style_or("MY_LOG_STYLE", "always");
    env_logger::init_from_env(env);

    let args = env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        eprintln!("Usage: {} <chart_file>", args[0]);
        std::process::exit(1);
    }

    // Parse chart file.
    let chart_file_path = &args[1];
    let chart_parent_dir = Path::new(chart_file_path).parent().unwrap_or(Path::new(""));
    let chart = OsuManiaParser::parse_file(chart_file_path)?;

    log::info!("Chart number of hit objects: {}", chart.hit_objects.len());

    // Load audio.
    let mut audio_system = AudioSystem::new()?;
    let audio_file_path = chart_parent_dir.join(&chart.info.audio_file_name);
    let sound_index =
        audio_system.load_static_sound_from_file(audio_file_path.to_str().unwrap())?;

    // Initialize window.
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("Lyzumu")
        .with_inner_size(dpi::PhysicalSize::new(1920, 1080))
        .with_position(dpi::PhysicalPosition::new(0, 0))
        .build(&event_loop)?;

    // Play audio.
    let _ = audio_system.play_static_sound(sound_index)?;

    event_loop.run(move |event, eltw| {
        eltw.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    eltw.exit();
                }
                WindowEvent::Resized(_) => {}
                WindowEvent::RedrawRequested => {}
                _ => (),
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => (),
        }
    })?;

    Ok(())
}
