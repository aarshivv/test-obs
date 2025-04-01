use std::{
    env::current_exe,
    ffi::{CStr, CString},
    ptr,
    time::Duration,
};

use libobs_new::{
    calldata_t, gs_create, obs_add_data_path, obs_add_module_path, obs_audio_encoder_create,
    obs_audio_info, obs_audio_info2, obs_blending_method, obs_blending_type, obs_data_create,
    obs_data_release, obs_encoder_set_video, obs_enter_graphics, obs_get_latest_input_type_id,
    obs_get_version_string, obs_get_video, obs_initialized, obs_load_all_modules,
    obs_log_loaded_modules, obs_output_create, obs_output_get_id, obs_output_get_last_error,
    obs_output_get_proc_handler, obs_output_set_audio_encoder, obs_output_set_video_encoder,
    obs_output_start, obs_output_stop, obs_post_load_modules, obs_reset_audio, obs_reset_audio2,
    obs_reset_video, obs_scale_type_OBS_SCALE_BICUBIC, obs_scale_type_OBS_SCALE_BILINEAR,
    obs_scale_type_OBS_SCALE_LANCZOS, obs_scene_create, obs_scene_t, obs_sceneitem_crop,
    obs_sceneitem_t, obs_set_output_source, obs_shutdown, obs_source_create, obs_source_t,
    obs_startup, obs_transform_info, obs_video_encoder_create, obs_video_info, proc_handler_call,
    speaker_layout_SPEAKERS_STEREO, video_colorspace_VIDEO_CS_709,
    video_colorspace_VIDEO_CS_DEFAULT, video_format_VIDEO_FORMAT_NV12,
    video_range_type_VIDEO_RANGE_DEFAULT, video_range_type_VIDEO_RANGE_FULL, Sleep,
};
use libobs_wrapper::{
    data::{video::ObsVideoInfo, ObsData},
    utils::{ObsPath, ObsString},
};
use std::ffi::c_void;
use tokio::{task, time};
use windows::Win32::System::WinRT::{
    RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED, RO_INIT_SINGLETHREADED,
};

pub async fn monitor_record() {
    unsafe {
        if obs_initialized() {
            println!("Already initialized, can't do it again");
            return;
        }

        println!(
            "OBS VERSION: {}",
            CStr::from_ptr(obs_get_version_string()).to_string_lossy()
        );
        let locale_str = ObsString::new("en-US");

        if !obs_startup(locale_str.as_ptr(), ptr::null(), ptr::null_mut()) {
            println!("Startup failed");
            return;
        }

        obs_add_data_path(ObsPath::from_relative("data/libobs").build().as_ptr());
        obs_add_module_path(
            ObsPath::from_relative("obs-plugins/64bit").build().as_ptr(),
            ObsPath::from_relative("data/obs-plugins/%module%")
                .build()
                .as_ptr(),
        );

        // let scene = obs_scene_create(ObsString::new("MAIN").as_ptr());

        // let a = gs_create(ptr::null_mut(), ObsString::new("libobs-d3d11").as_ptr(), 0);
        // obs_enter_graphics();

        let main_width = 1920;
        let main_height = 1080;

        let mut test_obs = ObsVideoInfo::default();

        let mut obs_video_info = obs_video_info {
            graphics_module: ObsString::new("libobs-d3d11").as_ptr(),
            fps_num: 60,
            fps_den: 1,
            base_width: main_width,
            base_height: main_height,
            output_width: main_width,
            output_height: main_height,
            output_format: video_format_VIDEO_FORMAT_NV12,
            adapter: 0,
            gpu_conversion: true,
            colorspace: video_colorspace_VIDEO_CS_DEFAULT,
            range: video_range_type_VIDEO_RANGE_DEFAULT,
            scale_type: obs_scale_type_OBS_SCALE_LANCZOS,
        };

        let reset_video_code = obs_reset_video(test_obs.as_ptr());

        if reset_video_code != 0 {
            println!("error reseting video in obs");
            return;
        }

        let obs_audio_info = obs_audio_info {
            samples_per_sec: 44100,
            speakers: speaker_layout_SPEAKERS_STEREO,
        };
        let reset_audio_code = obs_reset_audio(&obs_audio_info);
        if !reset_audio_code {
            println!("error reseting audio in obs");
            return;
        }

        // let scene = obs_scene_create(ObsString::new("MAIN").as_ptr());

        obs_load_all_modules();
        obs_log_loaded_modules();
        obs_post_load_modules();

        let video_source = obs_source_create(
            ObsString::new("monitor_capture").as_ptr(),
            ObsString::new("Screen capture source").as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        obs_set_output_source(0, video_source);

        let mut video_encoder_settings = ObsData::new();
        video_encoder_settings.set_bool("use_bufsize", true);
        video_encoder_settings.set_string("profile", "high");
        video_encoder_settings.set_string("preset", "veryfast");
        video_encoder_settings.set_string("rate_control", "CRF");
        video_encoder_settings.set_int("crf", 20);

        let video_encoder = obs_video_encoder_create(
            ObsString::new("obs_x264").as_ptr(),
            ObsString::new("simple_h264_recording").as_ptr(),
            video_encoder_settings.as_ptr(),
            ptr::null_mut(),
        );
        obs_encoder_set_video(video_encoder, obs_get_video());
        obs_data_release(video_encoder_settings.as_ptr());

        let audio_source = obs_source_create(
            ObsString::new("wasapi_output_capture").as_ptr(),
            ObsString::new("Audio Capture Source").as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        obs_set_output_source(1, audio_source);
        // let audio_encoder = obs_audio_encoder_create(ObsString::new("ffmpeg_aac").as_ptr(), ObsString::new("simple_aac_recording").as_ptr(), ptr::null_mut(), 0, ptr::null_mut());

        let mut record_output_settings = ObsData::new();
        record_output_settings
            .set_string("path", ObsPath::from_relative("./test_record.mp4").build());
        let record_output = obs_output_create(
            ObsString::new("ffmpeg_muxer").as_ptr(),
            ObsString::new("simple_ffmpeg_output").as_ptr(),
            record_output_settings.as_ptr(),
            ptr::null_mut(),
        );
        obs_data_release(record_output_settings.as_ptr());

        obs_output_set_video_encoder(record_output, video_encoder);
        // obs_output_set_audio_encoder(record_output, audio_encoder, 0);

        let mut buffer_output_settings = ObsData::new();
        buffer_output_settings.set_string("directory", ObsPath::from_relative("./").build());
        buffer_output_settings.set_string("format", "%CCYY-%MM-%DD %hh-%mm-%ss");
        buffer_output_settings.set_string("extension", "mp4");
        buffer_output_settings.set_int("max_time_sec", 15);
        buffer_output_settings.set_int("max_size_mb", 500);
        let buffer_output = obs_output_create(
            ObsString::new("replay_buffer").as_ptr(),
            ObsString::new("replay_buffer_output").as_ptr(),
            buffer_output_settings.as_ptr(),
            ptr::null_mut(),
        );
        obs_data_release(buffer_output_settings.as_ptr());

        obs_output_set_video_encoder(buffer_output, video_encoder);
        // obs_output_set_audio_encoder(buffer_output, audio_encoder, 0);

        let record_output_start_success = obs_output_start(record_output);
        println!("RECORD OUTPUT START: {record_output_start_success}");
        if !record_output_start_success {
            let error_message =
                CStr::from_ptr(obs_output_get_last_error(record_output)).to_string_lossy();
            println!("ERROR OCCURRED: ERROR: {error_message}");
        }

        // let record_output_start_success = obs_output_start(buffer_output);
        // println!("BUFFER OUTPUT START: {record_output_start_success}");
        // if !record_output_start_success {
        //     let error_message = CStr::from_ptr(obs_output_get_last_error(buffer_output)).to_string_lossy();
        //     println!("ERROR OCCURRED: ERROR: {error_message}");
        // }

        // time::sleep(Duration::from_secs(5)).await; // Wait for 5 seconds

        // // Equivalent to `calldata_t cd = new();`
        // let mut cd = calldata_t {
        //     stack: std::ptr::null_mut(), // NULL pointer for safety
        //     size: 0,
        //     capacity: 0,
        //     fixed: false,
        // };

        // let ph = unsafe { obs_output_get_proc_handler(buffer_output) };
        // let result = unsafe { proc_handler_call(ph, ObsString::new("save").as_ptr(), &mut cd) };

        // println!("Buffer output successful save: {}", result);

        std::thread::sleep(Duration::from_secs(10));

        let record_output_id = CStr::from_ptr(obs_output_get_id(record_output)).to_string_lossy();
        println!("ID: {record_output_id}");

        obs_output_stop(record_output);

        obs_shutdown();
    }
}

pub fn test_main() {
    // STARTUP
    unsafe {
        println!("Starting OBS initialization process");

        if libobs_new::obs_initialized() {
            panic!("error: obs already initialized");
        }
        println!("OBS not yet initialized, continuing");

        #[cfg(not(target_os = "windows"))]
        {
            println!("Setting NIX platform to X11_EGL");
            libobs_new::obs_set_nix_platform(
                libobs_new::obs_nix_platform_type_OBS_NIX_PLATFORM_X11_EGL,
            );
            println!("Opening X display");
            let display = x_open_display(ptr::null_mut());
            println!("X display pointer: {:?}", display);
            libobs_new::obs_set_nix_platform_display(display);
            println!("NIX platform display set");
        }

        #[cfg(target_os = "windows")]
        {
            println!("Setting log handler on Windows");
            libobs_new::base_set_log_handler(Some(log_handler), ptr::null_mut());
            println!("Log handler set successfully");
        }

        println!("Retrieving libobs version string...");
        let version_ptr = libobs_new::obs_get_version_string();
        println!("Version string pointer: {:?}", version_ptr);

        println!(
            "libobs version: {}",
            CStr::from_ptr(version_ptr)
                .to_str()
                .unwrap_or("Failed to read version string")
        );

        println!("Starting OBS with locale 'en-US'");
        let locale = CString::new("en-US").unwrap();
        println!("Locale pointer: {:?}", locale.as_ptr());

        let startup_result = libobs_new::obs_startup(locale.as_ptr(), ptr::null(), ptr::null_mut());

        if !startup_result {
            panic!("error on libobs startup");
        }
        println!("OBS startup successful");

        let scene = obs_scene_create(ObsString::new("MAIN").as_ptr());

        // let a = gs_create(ptr::null_mut(), ObsString::new("libobs-d3d11").as_ptr(), 0);
        // obs_enter_graphics();

        let curr_exe = current_exe().unwrap();
        let curr_exe = curr_exe.parent().unwrap();

        let data_path = curr_exe.join("./data/libobs/");
        let data_path = data_path.to_str().unwrap();

        println!("Adding data path {}", data_path);
        let data_path = CString::new(data_path).unwrap();
        println!("Data path pointer: {:?}", data_path.as_ptr());
        libobs_new::obs_add_data_path(data_path.as_ptr());
        println!("Data path added successfully");

        println!("Adding module path");
        let module_bin_path =
            CString::new(curr_exe.join("./obs-plugins/64bit/").to_str().unwrap()).unwrap();
        let module_path = curr_exe.join("./data/obs-plugins/%module%/");
        let module_data_path = module_path.to_str().unwrap();
        let module_data_path = CString::new(module_data_path).unwrap();
        println!(
            "Module bin path pointer: {:?}",
            module_bin_path.to_str().unwrap()
        );
        println!(
            "Module data path pointer: {:?}",
            module_data_path.to_str().unwrap()
        );

        libobs_new::obs_add_module_path(module_bin_path.as_ptr(), module_data_path.as_ptr());
        println!("Module paths added successfully");

        // Audio settings
        println!("Configuring audio settings");
        let mut avi = libobs_new::obs_audio_info2 {
            samples_per_sec: 48000,
            speakers: libobs_new::speaker_layout_SPEAKERS_STEREO,
            max_buffering_ms: 960,
            fixed_buffering: false,
        };
        println!("Resetting audio system");
        let reset_audio_result = libobs_new::obs_reset_audio2(&mut avi);
        println!("Audio reset result: {}", reset_audio_result);

        // Video settings - scene rendering resolution
        println!("Configuring video settings");
        let main_width = 1920;
        let main_height = 1080;

        #[cfg(target_os = "windows")]
        let graphics_module = CString::new("libobs-d3d11").unwrap();
        #[cfg(not(target_os = "windows"))]
        let graphics_module = CString::new("libobs-opengl").unwrap();

        // println!("Graphics module: {:?}", graphics_module.as_ptr());

        let mut ovi = libobs_new::obs_video_info {
            adapter: 0,
            #[cfg(target_os = "windows")]
            graphics_module: graphics_module.as_ptr(),
            #[cfg(not(target_os = "windows"))]
            graphics_module: graphics_module.as_ptr(),
            fps_num: 60,
            fps_den: 1,
            base_width: main_width,
            base_height: main_height,
            output_width: main_width,
            output_height: main_height,
            output_format: libobs_new::video_format_VIDEO_FORMAT_NV12,
            gpu_conversion: true,
            colorspace: libobs_new::video_colorspace_VIDEO_CS_DEFAULT,
            range: libobs_new::video_range_type_VIDEO_RANGE_DEFAULT,
            scale_type: libobs_new::obs_scale_type_OBS_SCALE_BILINEAR,
        };

        // let mut g = libobs_new::gs_get_context();
        // let test = libobs_new::gs_create(&mut g, graphics_module.as_ptr(), 0);

        println!("Resetting video system");
        let reset_video_code = libobs_new::obs_reset_video(&mut ovi);
        if reset_video_code != 0 {
            panic!("error on libobs reset video: {}", reset_video_code);
        }
        println!("Video reset successful");

        let b = libobs_new::obs_set_audio_monitoring_device(
            CString::new("Default").unwrap().as_ptr(),
            CString::new("default").unwrap().as_ptr(),
        );
        // let a = gs_create(ptr::null_mut(), ObsString::new("libobs-d3d11").as_ptr(), 0);
        RoInitialize(RO_INIT_SINGLETHREADED).unwrap();

        // Load modules
        println!("Loading all modules");
        libobs_new::obs_load_all_modules();
        println!("Logging loaded modules");
        libobs_new::obs_log_loaded_modules();
        println!("Post-loading modules");
        libobs_new::obs_post_load_modules();
        println!("Module loading complete");

        // add encoder
        let video_encoder_settings = libobs_new::obs_data_create();
        libobs_new::obs_data_set_bool(
            video_encoder_settings,
            CString::new("use_bufsize").unwrap().as_ptr(),
            true,
        );
        libobs_new::obs_data_set_string(
            video_encoder_settings,
            CString::new("profile").unwrap().as_ptr(),
            CString::new("high").unwrap().as_ptr(),
        );
        libobs_new::obs_data_set_string(
            video_encoder_settings,
            CString::new("preset").unwrap().as_ptr(),
            CString::new("veryfast").unwrap().as_ptr(),
        );
        libobs_new::obs_data_set_string(
            video_encoder_settings,
            CString::new("rate_control").unwrap().as_ptr(),
            CString::new("CRF").unwrap().as_ptr(),
        );
        libobs_new::obs_data_set_int(
            video_encoder_settings,
            CString::new("crf").unwrap().as_ptr(),
            20,
        );

        let video_encoder = libobs_new::obs_video_encoder_create(
            CString::new("obs_x264").unwrap().as_ptr(),
            CString::new("simple_h264_recording").unwrap().as_ptr(),
            video_encoder_settings,
            ptr::null_mut(),
        );

        // audio encoder
        // SETUP NEW AUDIO ENCODER
        let audio_encoder = libobs_new::obs_audio_encoder_create(
            CString::new("ffmpeg_aac").unwrap().as_ptr(),
            CString::new("simple_aac_recording").unwrap().as_ptr(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
        );

        // let ou = libobs_new::obs_set_output_source(0, video_sour)

        let video_source_settings = ptr::null_mut();
        // let video_source_settings = libobs_new::obs_data_create();
        // libobs_new::obs_data_set_string(
        //     video_source_settings,
        //     CString::new("monitor_id").unwrap().as_ptr(),
        //     CString::new("\\\\?\\DISPLAY#BOE07F6#5&74e87ec&0&UID256#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}").unwrap().as_ptr(),
        // );

        // SETUP NEW VIDEO SOURCE
        let monitor_capture = CString::new("monitor_capture").unwrap();
        // let monitor_id = obs_get_latest_input_type_id(monitor_capture);
        let video_source = libobs_new::obs_source_create(
            #[cfg(target_os = "windows")]
            obs_get_latest_input_type_id(monitor_capture.as_ptr()),
            #[cfg(not(target_os = "windows"))]
            CString::new("xshm_input").unwrap().as_ptr(),
            CString::new("Screen Capture Source").unwrap().as_ptr(),
            video_source_settings,
            ptr::null_mut(),
        );

        let mut data = AddSourceData {
            source: video_source,
            transform: ptr::null(),
            crop: ptr::null(),
            blend_method: ptr::null(),
            blend_mode: ptr::null(),
            visible: true,
            scene_item: ptr::null_mut(),
        };

        obs_enter_graphics();
        libobs_new::obs_scene_atomic_update(
            scene,
            Some(add_source),
            &mut data as *mut _ as *mut c_void,
        );
        libobs_new::obs_leave_graphics();

        let settings = libobs_new::obs_source_get_settings(video_source);
        if settings.is_null() {
            return;
        }

        let json_ptr = libobs_new::obs_data_get_json(settings);
        if !json_ptr.is_null() {
            let json_cstr = CStr::from_ptr(json_ptr);
            if let Ok(json_str) = json_cstr.to_str() {
                println!("Source Settings: {}", json_str);
            }
        } else {
            println!("Failed to convert settings to JSON.");
        }

        // Convert "monitor_id" key to C string
        let key = CString::new("monitor_id").expect("CString::new failed");
        libobs_new::obs_data_set_string(
            settings,
            CString::new("monitor_id").unwrap().as_ptr(),
            CString::new(
                "\\\\?\\DISPLAY#BOE07F6#5&74e87ec&0&UID256#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}",
            )
            .unwrap()
            .as_ptr(),
        );
        libobs_new::obs_data_set_int(
            settings,
            CString::new("method").unwrap().as_ptr(),
            libobs_new::window_capture_method_METHOD_WGC.into(),
        );
        // libobs_new::obs_data_set_string(
        //     settings,
        //     CString::new("monitor_id").unwrap().as_ptr(),
        //     CString::new("\\\\?\\DISPLAY#BOE07F6#5&74e87ec&0&UID256#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}").unwrap().as_ptr(),
        // );
        // Set new monitor ID
        // obs_data_set_int(settings, key.as_ptr(), monitor_id);

        // Apply the updated settings
        libobs_new::obs_source_update(video_source, settings);
        obs_data_release(settings);

        libobs_new::obs_scene_add(scene, video_source);
        libobs_new::obs_data_release(video_source_settings);
        libobs_new::obs_set_output_source(0, video_source); // 0 = VIDEO CHANNEL

        // SETUP NEW VIDEO ENCODER

        libobs_new::obs_encoder_set_video(video_encoder, libobs_new::obs_get_video());
        libobs_new::obs_data_release(video_encoder_settings);

        // SETUP NEW AUDIO SOURCE
        #[cfg(target_os = "windows")]
        let audio_source = libobs_new::obs_source_create(
            CString::new("wasapi_output_capture").unwrap().as_ptr(),
            CString::new("Audio Capture Source").unwrap().as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
        );

        // SETUP NEW RECORD OUTPUT
        let record_output_settings = libobs_new::obs_data_create();
        libobs_new::obs_data_set_string(
            record_output_settings,
            CString::new("path").unwrap().as_ptr(),
            CString::new("./record.mp4").unwrap().as_ptr(),
        );
        let record_output = libobs_new::obs_output_create(
            CString::new("ffmpeg_muxer").unwrap().as_ptr(),
            CString::new("simple_ffmpeg_output").unwrap().as_ptr(),
            record_output_settings,
            ptr::null_mut(),
        );
        libobs_new::obs_data_release(record_output_settings);

        #[cfg(not(target_os = "windows"))]
        let audio_source = {
            let audio_encoder_settings = libobs_new::obs_data_create();
            libobs_new::obs_data_set_string(
                audio_encoder_settings,
                CString::new("device_id").unwrap().as_ptr(),
                CString::new("default").unwrap().as_ptr(),
            );
            let source = libobs_new::obs_source_create(
                CString::new("pulse_output_capture").unwrap().as_ptr(),
                CString::new("Audio Capture Source").unwrap().as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            libobs_new::obs_data_release(audio_encoder_settings);
            source
        };

        libobs_new::obs_set_output_source(1, audio_source); // 1 = AUDIO CHANNEL

        libobs_new::obs_encoder_set_audio(audio_encoder, libobs_new::obs_get_audio());

        libobs_new::obs_output_set_video_encoder(record_output, video_encoder);
        libobs_new::obs_output_set_audio_encoder(record_output, audio_encoder, 0);

        // SETUP NEW BUFFER OUTPUT (OPTIONAL, just demonstrating it here in example that multiple outputs can be run)
        let buffer_output_settings = libobs_new::obs_data_create();
        libobs_new::obs_data_set_string(
            buffer_output_settings,
            CString::new("directory").unwrap().as_ptr(),
            CString::new("./").unwrap().as_ptr(),
        );
        libobs_new::obs_data_set_string(
            buffer_output_settings,
            CString::new("format").unwrap().as_ptr(),
            CString::new("%CCYY-%MM-%DD %hh-%mm-%ss").unwrap().as_ptr(),
        );
        libobs_new::obs_data_set_string(
            buffer_output_settings,
            CString::new("extension").unwrap().as_ptr(),
            CString::new("mp4").unwrap().as_ptr(),
        );
        libobs_new::obs_data_set_int(
            buffer_output_settings,
            CString::new("max_time_sec").unwrap().as_ptr(),
            15,
        );
        libobs_new::obs_data_set_int(
            buffer_output_settings,
            CString::new("max_size_mb").unwrap().as_ptr(),
            500,
        );
        libobs_new::obs_data_release(buffer_output_settings);

        // START RECORD OUTPUT
        let record_output_start_success = libobs_new::obs_output_start(record_output);
        println!(
            "record output successful start: {}",
            record_output_start_success
        );
        if !record_output_start_success {
            println!(
                "record output error: '{}'",
                CStr::from_ptr(libobs_new::obs_output_get_last_error(record_output))
                    .to_str()
                    .unwrap()
            );
        }

        // Print output IDs
        println!(
            "Record Output id is {}",
            CStr::from_ptr(libobs_new::obs_output_get_id(record_output))
                .to_str()
                .unwrap()
        );

        // Wait for user input before shutting down
        println!("Press enter to stop recording and clean up...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        libobs_new::obs_output_stop(record_output);
        libobs_new::obs_output_release(record_output);

        libobs_new::obs_encoder_release(audio_encoder);
        libobs_new::obs_encoder_release(video_encoder);

        libobs_new::obs_source_release(audio_source);
        libobs_new::obs_source_release(video_source);

        RoUninitialize();

        libobs_new::obs_shutdown();

        println!("OBS shutdown completed");
    }
}

#[cfg(not(target_os = "windows"))]
fn x_open_display(display: *mut c_void) -> *mut c_void {
    extern "C" {
        fn XOpenDisplay(display: *mut c_void) -> *mut c_void;
    }

    unsafe { XOpenDisplay(display) }
}

#[cfg(target_os = "windows")]
pub(crate) unsafe extern "C" fn log_handler(
    log_level: i32,
    msg: *const i8,
    args: *mut i8,
    _params: *mut c_void,
) {
    // Simple logger that prints directly to console
    // In a real-world application, you would use vsnprintf to format the message properly

    let level_str = match log_level {
        libobs_new::LOG_ERROR => "ERROR",
        libobs_new::LOG_WARNING => "WARNING",
        libobs_new::LOG_INFO => "INFO",
        libobs_new::LOG_DEBUG => "DEBUG",
        _ => "UNKNOWN",
    };

    let formatted = vsprintf::vsprintf(msg, args);
    if formatted.is_err() {
        eprintln!("Failed to format log message");
        return;
    }
    println!("[{}] {}", level_str, formatted.unwrap());
}

// Define Rust bindings for C types
#[repr(C)]
struct AddSourceData {
    source: *mut obs_source_t,
    transform: *const obs_transform_info,
    crop: *const obs_sceneitem_crop,
    blend_method: *const obs_blending_method,
    blend_mode: *const obs_blending_type,
    visible: bool,
    scene_item: *mut obs_sceneitem_t,
}

// extern "C" {
//     fn obs_scene_add(scene: *mut obs_scene_t, source: *mut obs_source_t) -> *mut obs_sceneitem_t;
//     fn obs_sceneitem_set_info2(item: *mut obs_sceneitem_t, transform: *const obs_transform_info);
//     fn obs_sceneitem_set_crop(item: *mut obs_sceneitem_t, crop: *const obs_crop);
//     fn obs_sceneitem_set_blending_method(item: *mut obs_sceneitem_t, method: u32);
//     fn obs_sceneitem_set_blending_mode(item: *mut obs_sceneitem_t, mode: u32);
//     fn obs_sceneitem_set_visible(item: *mut obs_sceneitem_t, visible: bool);
// }

// Safe wrapper for `AddSource`
unsafe extern "C" fn add_source(data: *mut c_void, scene: *mut obs_scene_t) {
    let data = data as *mut AddSourceData;
    if data.is_null() || scene.is_null() {
        return;
    }
    let data = &mut *data;

    let sceneitem = libobs_new::obs_scene_add(scene, data.source);
    if sceneitem.is_null() {
        return;
    }

    if !data.transform.is_null() {
        libobs_new::obs_sceneitem_set_info2(sceneitem, data.transform);
    }
    if !data.crop.is_null() {
        libobs_new::obs_sceneitem_set_crop(sceneitem, data.crop);
    }
    if !data.blend_method.is_null() {
        libobs_new::obs_sceneitem_set_blending_method(sceneitem, *data.blend_method);
    }
    if !data.blend_mode.is_null() {
        libobs_new::obs_sceneitem_set_blending_mode(sceneitem, *data.blend_mode);
    }

    libobs_new::obs_sceneitem_set_visible(sceneitem, data.visible);
    data.scene_item = sceneitem;
}
