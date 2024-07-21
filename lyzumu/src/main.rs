use std::{env, path::Path, time::Instant};

use anyhow::Result;
use lyzumu_graphics::{
    mesh,
    renderer::track::{TrackRenderer, TrackUniformBufferData},
    scene::{SceneHitObject, TrackScene},
};
use nalgebra::{Isometry3, Matrix4, Perspective3, Point3, Vector3, Vector4};
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
use lyzumu_chart::parse::osu_mania::OsuManiaParser;

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

    // Initialize renderer.
    let mut track_renderer = TrackRenderer::new(
        window.window_handle()?.raw_window_handle()?,
        window.display_handle()?.raw_display_handle()?,
    )?;
    let screen_dimensions = track_renderer.swapchain_extent();

    let runner_speed = 10.0 as f32;

    let mut renderer_hit_objects = Vec::new();
    let num_lanes = 4.0;
    let lane_width = 2.0 / num_lanes;
    let lane_x_offset = (lane_width / 2.0) - 1.0;
    chart.hit_objects.iter().for_each(|hit_object| {
        let lane = (hit_object.position.0 * num_lanes / 512.0).floor();
        let x_translation = lane_x_offset + (lane * lane_width);
        let z_translation = hit_object.time / 1000.0 * runner_speed;
        let renderer_hit_object = SceneHitObject {
            transform: Matrix4::new_translation(&Vector3::new(x_translation, 0.0, z_translation)),
            base_color: Vector4::new(1.0, 0.0, 0.0, 1.0),
        };
        renderer_hit_objects.push(renderer_hit_object);
    });
    let renderer_scene_data = TrackScene {
        hit_object_mesh: mesh::cuboid::Cuboid::new(0.4, 0.1, 0.2).into(),
        // hit_object_mesh: mesh::cuboid::Cuboid::new(1.0, 1.0, 1.0).into(),
        hit_objects: renderer_hit_objects,
    };
    track_renderer.load_scene(renderer_scene_data)?;

    // Play audio.
    let sound_handle = audio_system.play_static_sound(sound_index)?;

    let mut last_audio_position = 0.0;
    let mut last_render_time = Instant::now();

    let mut current_runner_position = 0.0;

    // XXX TODO: Need to find good parameters for this
    let eye = Point3::new(0.0, -1.54, 0.2);
    let target = Point3::new(0.0, 0.7, 3.0);

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

    let mut scene_constants = TrackUniformBufferData {
        view_projection,
        runner_transform: Matrix4::identity(),
    };

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

                    // println!("DT: {}", dt.as_secs_f32());
                    let current_audio_position = sound_handle.position() as f32;
                    if last_audio_position != current_audio_position {
                        println!(
                            "last vs current sound position  {} {} {}",
                            last_audio_position,
                            current_audio_position,
                            current_audio_position - last_audio_position
                        );
                        last_audio_position = current_audio_position;
                    }

                    let audio_dt = current_audio_position - last_audio_position;
                    last_audio_position = current_audio_position;

                    // current_runner_position += audio_dt * runner_speed;
                    current_runner_position += dt.as_secs_f32() * runner_speed;
                    scene_constants.runner_transform = Matrix4::new_translation(&Vector3::new(
                        0.0,
                        0.0,
                        -current_runner_position as _,
                    ));

                    track_renderer.update_scene_constants(scene_constants.clone());

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
