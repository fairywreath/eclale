use std::{env, path::Path, time::Instant};

use anyhow::Result;
use nalgebra::{
    Isometry3, Matrix4, Orthographic3, Perspective3, Point3, Vector2, Vector3, Vector4,
};
use winit::{
    dpi,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    raw_window_handle::{
        HasDisplayHandle, HasRawDisplayHandle, HasRawWindowHandle, HasWindowHandle,
    },
    window::WindowBuilder,
};

use lyzumu_audio::AudioSystem;
use lyzumu_chart::parse::lzm::LzmParser;
use track_renderer::{TrackRenderer, TrackSettings};

mod track_renderer;

fn main() -> Result<()> {
    let env = env_logger::Env::default()
        .filter_or("MY_LOG_LEVEL", "debug")
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
    let lzm_parser = LzmParser::new()?;
    let chart = lzm_parser.parse_file(chart_file_path)?;

    // let chart = OsuManiaParser::parse_file(chart_file_path)?;

    log::info!("Chart number of hit objects: {}", chart.notes.len());

    // Load audio.
    let mut audio_system = AudioSystem::new()?;
    let audio_file_path = chart_parent_dir.join(&chart.header.audio_filename);
    log::info!(
        "Audio file path: {}",
        &audio_file_path.clone().to_str().unwrap()
    );
    let sound_index =
        audio_system.load_static_sound_from_file(audio_file_path.to_str().unwrap())?;

    // Initialize window.
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("Lyzumu")
        .with_inner_size(dpi::PhysicalSize::new(1920, 1080))
        .with_position(dpi::PhysicalPosition::new(0, 0))
        .build(&event_loop)?;

    let track_settings = TrackSettings { runner_speed: 25.0 };

    // Initialize renderer.
    let mut track_renderer = TrackRenderer::new(
        window.window_handle()?.raw_window_handle()?,
        window.display_handle()?.raw_display_handle()?,
        track_settings.clone(),
        &chart,
    )?;
    // let screen_dimensions = track_renderer.swapchain_extent();

    let screen_dimensions = Vector2::new(1920.0, 1080.0);

    // Play audio.
    let sound_handle = audio_system.play_static_sound(sound_index)?;

    let mut last_audio_position = 0.0;
    let mut last_render_time = Instant::now();

    let mut current_runner_position = 0.0;

    // XXX TODO: Need to find good parameters for this
    let eye = Point3::new(0.0, -3.0, -2.2);
    let target = Point3::new(0.0, 0.8, 4.4);

    let view = Isometry3::look_at_rh(&eye, &target, &Vector3::y());
    let projection = Perspective3::new(
        screen_dimensions.x as f32 / screen_dimensions.y as f32,
        3.14 / 3.0,
        0.01,
        1000.0,
    );
    let view_projection = projection.into_inner()
            * view.to_homogeneous()
            // XXX: Use view and projection matrices that fit accordingly to the vulkan coord system. (?)
            * Matrix4::new_nonuniform_scaling(&Vector3::new(-1.0, 1.0, 1.0));

    event_loop.run(move |event, eltw| {
        eltw.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    eltw.exit();
                }
                WindowEvent::Resized(_) => {}
                WindowEvent::RedrawRequested => {
                    let now = Instant::now();
                    let dt = now - last_render_time;
                    last_render_time = now;

                    let current_audio_position = sound_handle.position() as f32;
                    let _audio_dt = current_audio_position - last_audio_position;
                    last_audio_position = current_audio_position;

                    current_runner_position += dt.as_secs_f32() * track_settings.runner_speed;

                    track_renderer.update_view_projection(view_projection);
                    track_renderer.update_runner_position(current_runner_position);
                    track_renderer.render().unwrap();
                }
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
