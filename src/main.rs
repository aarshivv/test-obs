use std::env::{self, current_dir};
use std::ffi::CStr;
use std::fs::File;
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use std::{cmp, thread};

use env_logger::Env;
use libobs_sources::windows::{MonitorCaptureSourceBuilder, MonitorCaptureSourceUpdater, WindowCaptureSourceBuilder, WindowCaptureSourceUpdater};
use libobs_wrapper::context::ObsContext;
use libobs_wrapper::data::video::ObsVideoInfo;
use libobs_wrapper::data::{ObsData, ObsObjectBuilder};
use libobs_wrapper::encoders::{ObsContextEncoders, ObsVideoEncoderType};
use libobs_wrapper::enums::ObsLogLevel;
use libobs_wrapper::logger::ObsLogger;
use libobs_wrapper::scenes::ObsSceneRef;
use libobs_wrapper::sources::ObsSourceRef;
use libobs_wrapper::utils::traits::ObsUpdatable;
use libobs_wrapper::utils::{AudioEncoderInfo, ObsPath, ObsString, OutputInfo, SourceInfo, StartupInfo, VideoEncoderInfo};
use libobs_window_helper::{get_all_windows, WindowInfo, WindowSearchMode};
use libobs_wrapper::sources::ObsSourceBuilder;
use libobs_wrapper::data::ObsObjectUpdater;

// use libobs::wrapper::{
//     StartupInfo, ObsContext, OutputInfo, ObsData, VideoEncoderInfo, 
//     AudioEncoderInfo, SourceInfo, ObsPath
// };

#[derive(Debug)]
struct DebugLogger {
    f: File
}
impl ObsLogger for DebugLogger {
    fn log(&mut self, level: libobs_wrapper::enums::ObsLogLevel, msg: String) {
        if level == ObsLogLevel::Debug {
            return;
        }

        self.f.write_all(format!("{}\n", msg).as_bytes()).unwrap();
    }
}

/// The string returned is the name of the obs output
pub fn initialize_obs_with_log<'a>(rec_file: ObsString, file_logger: bool) -> (ObsContext, String) {
    let _ = env_logger::Builder::from_env(Env::default().default_filter_or("debug")).is_test(true).try_init();

    // Start the OBS context
    #[allow(unused_mut)]
    let mut startup_info = StartupInfo::default();
    if file_logger {
        let _l = DebugLogger { f: File::create(current_dir().unwrap().join("obs.log")).unwrap() };
        //startup_info = startup_info.set_logger(Box::new(_l));
    }

    let mut context = ObsContext::new(startup_info).unwrap();

    // Set up output to ./recording.mp4
    let mut output_settings = ObsData::new();
    output_settings.set_string("path", rec_file);

    let output_name = "output";
    let output_info = OutputInfo::new("ffmpeg_muxer", output_name, Some(output_settings), None);

    let mut output = context.output(output_info).unwrap();

    // Register the video encoder
    let mut video_settings = ObsData::new();
    video_settings
        .set_int("bf", 0)
        .set_bool("psycho_aq", true)
        .set_bool("lookahead", true)
        .set_string("profile", "high")
        .set_string("preset", "fast")
        .set_string("rate_control", "cbr")
        .set_int("bitrate", 10000);

    let encoders = ObsContext::get_available_video_encoders();

    println!("Available encoders: {:?}", encoders);
    let encoder =  encoders.iter().find(|e| **e == ObsVideoEncoderType::H264_TEXTURE_AMF || **e == ObsVideoEncoderType::AV1_TEXTURE_AMF).unwrap();
    println!("Using encoder {:?}", encoder);
    let video_info = VideoEncoderInfo::new(
        encoder.clone(),
        "video_encoder",
        Some(video_settings),
        None,
    );

    let video_handler = ObsContext::get_video_ptr().unwrap();
    output.video_encoder(video_info, video_handler).unwrap();

    // Register the audio encoder
    let mut audio_settings = ObsData::new();
    audio_settings.set_int("bitrate", 160);

    let audio_info =
        AudioEncoderInfo::new("ffmpeg_aac", "audio_encoder", Some(audio_settings), None);

    let audio_handler = ObsContext::get_audio_ptr().unwrap();
    output.audio_encoder(audio_info, 0, audio_handler).unwrap();

    (context, output_name.to_string())
}

pub async fn main3() {
    let rec_file = ObsPath::from_relative("monitor_capture.mp4").build();
    let path_out = PathBuf::from(rec_file.to_string());

    let (mut context, output) = initialize_obs_with_log(rec_file, true);
    let mut scene = context.scene("test_main");
    scene.add_and_set(0);

    let monitors = MonitorCaptureSourceBuilder::get_monitors().unwrap();
    println!("MONITORS: {monitors:#?}");

    let first_m = monitors.first().unwrap();

    let source_name = "monitor_test_new";
    // let m = MonitorCaptureSourceBuilder::new(source_name)
    //     .set_monitor(&monitors[0]).build();

    // println!("M: {m:#?}");

    let mut monitor_source_settings = ObsData::new();
    monitor_source_settings
        // .set_int("monitor", first_m.id.into())
        .set_string("monitor_id", "\\\\?\\DISPLAY#BOE07F6#5&74e87ec&0&UID256#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}")
        // .set_string("id", "\\\\?\\DISPLAY#BOE07F6#5&74e87ec&0&UID256#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}")
        // .set_string("setting_id", "\\\\?\\DISPLAY#BOE07F6#5&74e87ec&0&UID256#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}")
        // .set_string("capture_method", "BitBlt")
        .set_bool("cursor", true)
        .set_bool("capture_layered_windows", true)
        .set_bool("force_sdr", false);

    // ObsSourceRef::new("monitor_capture", source_name, Some(monitor_source_settings), None).unwrap()'
    let monitor_capture_source = SourceInfo::new("monitor_capture", source_name, Some(monitor_source_settings), None);

    scene.add_source(monitor_capture_source).unwrap();

    // let old_source = scene.get_source_mut(source_name).unwrap();

    // *data.
    // println!("SSS: {old_source:?}");
    let mut output = context.get_output(&output).unwrap();
    output.start().unwrap();
    println!("Recording started");
    // std::thread::sleep(Duration::from_secs(20));
    // let mut source = context.scenes_mut().get_mut(0).unwrap().get_source_by_index(0).unwrap();
    // source.create_updater::<MonitorCaptureSourceUpdater>().set_monitor(&monitors[0]).update();
    // stdout().flush().unwrap();
    tokio::time::sleep(Duration::from_secs(15)).await;

    println!("Recording stop");

    output.stop().unwrap();
}

fn find_notepad() -> Option<WindowInfo> {
    let windows =
        WindowCaptureSourceBuilder::get_windows(WindowSearchMode::ExcludeMinimized).unwrap();
    println!("{:?}", windows);
    windows.into_iter().find(|w| {
        w.class
            .as_ref()
            .is_some_and(|e| e.to_lowercase().contains("notepad"))
    })
}

#[tokio::main]
pub async fn main() {
    // test_window_capture().await;
    main3().await;
    // let venv = env::var("LIBOBS_PATH").unwrap();
    // println!("VENV: {venv}");
}

pub async fn test_window_capture() {
    let rec_file = ObsPath::from_relative("window_capture.mp4").build();
    let path_out = PathBuf::from(rec_file.to_string());

    let mut window = find_notepad();
    if window.is_none() {
        Command::new("notepad.exe").spawn().unwrap();
        std::thread::sleep(Duration::from_millis(350));

        window = find_notepad();
    }

    let window = window.expect("Couldn't find notepad window");

    println!("Recording {:#?}", window);

    let (mut context, output_name) = initialize_obs_with_log(rec_file, true);
    let mut scene = context.scene("main");
    scene.add_and_set(0);

    println!("IDIDDDIDI: {}", window.obs_id.as_str());

    let source_name = "test_capture";
    WindowCaptureSourceBuilder::new(source_name)
        .set_window(&window)
        .add_to_scene(&mut scene)
        .unwrap();

    let output = context.get_output(&output_name).unwrap();
    output.start().unwrap();
    println!("Recording started");

    let windows =
        WindowCaptureSourceBuilder::get_windows(WindowSearchMode::ExcludeMinimized).unwrap()
        .into_iter()
        .filter(|e| e.obs_id.to_lowercase().contains("code"))
        .collect::<Vec<_>>();
    for i in 0..cmp::min(5, windows.len()) {
        let mut source = context.scenes_mut().get_mut(0).unwrap().get_source_by_index(0).unwrap();
        let w = windows.get(i).unwrap();
        println!("Setting to {:?}", w.obs_id);
        source.create_updater::<WindowCaptureSourceUpdater>()
            .set_window(w)
            .update();

        println!("Recording for {} seconds", i);
        stdout().flush().unwrap();
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    println!("Recording stop");

    let mut output = context.get_output(&output_name).unwrap();
    output.stop().unwrap();

    // test_video(&path_out).await.unwrap();
}


pub fn main2() {
    // Start the OBS context

    // let res = get_all_windows(WindowSearchMode::ExcludeMinimized, false).unwrap();
    //     for i in res {
    //         /// This struct contains all crucial information about the window like title, class name, obs_id etc.
    //         println!("{:?}", i);
    //     }

    let startup_info = StartupInfo::default();
    let mut context = ObsContext::new(startup_info).unwrap();

    let obs_vid_info = ObsVideoInfo::default();
    context.reset_video(obs_vid_info).unwrap();

    // Set up output to ./recording.mp4
    let mut output_settings = ObsData::new();
    output_settings
        .set_string("path", ObsPath::from_relative("recording.mp4").build());

    let output_info = OutputInfo::new(
        "ffmpeg_muxer", "output", Some(output_settings), None
    );

    let mut output = context.output(output_info).unwrap();

    // Register the video encoder
    let mut video_settings = ObsData::new();
    video_settings
        .set_int("bf", 2)
        .set_bool("psycho_aq", true)
        .set_bool("lookahead", true)
        .set_string("profile", "high")
        // .set_string("preset", "hq")
        .set_string("rate_control", "cbr")
        .set_int("bitrate", 1000);

    let video_info = VideoEncoderInfo::new(
        "obs_x264",
        "video_encoder",
        Some(video_settings),
        None,
    );

    let video_handler = ObsContext::get_video_ptr().unwrap();
    output.video_encoder(video_info, video_handler).unwrap();
    
    // Register the audio encoder
    let mut audio_settings = ObsData::new();
    audio_settings.set_int("bitrate", 100);

    let audio_info = AudioEncoderInfo::new(
        "ffmpeg_aac", 
        "audio_encoder", 
        Some(audio_settings), 
        None
    );

    let audio_handler = ObsContext::get_audio_ptr().unwrap();
    output.audio_encoder(audio_info, 0, audio_handler).unwrap();

    // let video_source_info = SourceInfo::new(
    //     "monitor_capture", 
    //     "video_source", 
    //     Some(video_source_data), 
    //     None
    // );

    // let scene = ObsSceneRef::new(name, None)
    // Register the source and record
    // output.source(video_source_info, 0).unwrap();

        // Create the video source using game capture
    // let mut video_source_data = ObsData::new();
    // video_source_data
    //     .set_string("capture_mode", "window")
    //     .set_string("window", "")
    //     .set_bool("capture_cursor", true);
    // let _ = ObsSourceRef::new("monitor_capture", "video_source", Some(video_source_data), None).unwrap();

    let mut scene = context.scene("main");

    MonitorCaptureSourceBuilder::new("monitor_test")
        .set_monitor(&MonitorCaptureSourceBuilder::get_monitors().unwrap()[0])
        .add_to_scene(&mut scene)
        .unwrap();

    scene.add_and_set(0);

    output.start().unwrap();

    println!("recording for 10 seconds...");
    thread::sleep(Duration::new(10, 0));

    // Open any fullscreen application and
    // Success!
    output.stop().unwrap();
}