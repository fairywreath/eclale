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

use eclale_audio::AudioSystem;
use eclale_chart::parse::ogkr::create_chart_from_ogkr_file;

use renderer::{
    track_description::{TrackDescription, TrackSettings},
    track_renderer::TrackRenderer,
};

mod renderer;

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
    let chart = create_chart_from_ogkr_file(chart_file_path)?;

    log::info!("Successfully parsed chart file {}", chart_file_path);
    log::info!(
        "Number of hit notes {}, bells {}, platforms {}, lane types {}",
        chart.data.notes.hits.len(),
        chart.data.notes.contacts.len(),
        chart.data.track.platforms.len(),
        chart.data.track.lanes.len()
    );

    let chart_speed = 1.0;
    let runner_speed = 15.0;

    let render_track_settings = TrackSettings { runner_speed };
    let render_track_description = TrackDescription::from_chart(&chart, render_track_settings);

    // Load audio.
    // let mut audio_system = AudioSystem::new()?;
    // let audio_file_path = chart_parent_dir.join(&chart.header.audio_filename);
    // log::info!(
    //     "Audio file path: {}",
    //     &audio_file_path.clone().to_str().unwrap()
    // );
    // let sound_index =
    //     audio_system.load_static_sound_from_file(audio_file_path.to_str().unwrap())?;

    // Initialize window.
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("eclale")
        .with_inner_size(dpi::PhysicalSize::new(1920, 1080))
        .with_position(dpi::PhysicalPosition::new(0, 0))
        .build(&event_loop)?;

    // let track_settings = TrackSettings { runner_speed: 9.0 };

    let mut track_renderer = TrackRenderer::new(
        window.window_handle()?.raw_window_handle()?,
        window.display_handle()?.raw_display_handle()?,
        render_track_description,
    )?;
    let screen_dimensions = track_renderer.swapchain_extent();

    // let screen_dimensions = Vector2::new(1920.0, 1080.0);

    // Play audio.
    // let sound_handle = audio_system.play_static_sound(sound_index)?;

    let mut last_audio_position = 0.0;
    let mut last_render_time = Instant::now();

    let mut current_runner_position = 0.0;

    let eye = Point3::new(0.0, -1.3, -2.5);
    let target = Point3::new(0.0, 2.0, 2.5);

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

    let mut elapsed_time = 0.0;

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

                    elapsed_time += dt.as_secs_f32() * chart_speed;

                    // // let current_audio_position = sound_handle.position() as f32;
                    // // let _audio_dt = current_audio_position - last_audio_position;
                    // // last_audio_position = current_audio_position;

                    current_runner_position += dt.as_secs_f32() * runner_speed * chart_speed;

                    track_renderer.update_view_projection(view_projection);
                    track_renderer.update_runner_position(current_runner_position, elapsed_time);
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
