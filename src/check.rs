
#[derive(Clone)]
struct MainContext {
    pub obs_context: Arc<Mutex<ObsContext>>,
    pub output_name: String,
}

impl MainContext {
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut startup_info = StartupInfo::default();
        #[allow(unused_mut)]
        let mut context = ObsContext::new(startup_info).unwrap();

        let mut output_settings = ObsData::new();
        let rec_file = ObsPath::from_relative("monitor_capture.mp4").build();
        let path_out = PathBuf::from(rec_file.to_string());
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
        // let encoder =  encoders.iter().find(|e| **e == ObsVideoEncoderType::H264_TEXTURE_AMF || **e == ObsVideoEncoderType::AV1_TEXTURE_AMF).unwrap();
        // println!("Using encoder {:?}", encoder);
        let video_info =
            VideoEncoderInfo::new("obs_x264", "video_encoder", Some(video_settings), None);

        let video_handler = ObsContext::get_video_ptr().unwrap();
        output.video_encoder(video_info, video_handler).unwrap();

        // Register the audio encoder
        let mut audio_settings = ObsData::new();
        audio_settings.set_int("bitrate", 160);

        let audio_info =
            AudioEncoderInfo::new("ffmpeg_aac", "audio_encoder", Some(audio_settings), None);

        let audio_handler = ObsContext::get_audio_ptr().unwrap();
        output.audio_encoder(audio_info, 0, audio_handler).unwrap();

        let mut scene = context.scene("test_main");
        scene.add_and_set(0);

        let monitors = MonitorCaptureSourceBuilder::get_monitors().unwrap();
        println!("MONITORS: {monitors:#?}");

        let first_m = monitors.first().unwrap();
        let source_name = "monitor_test_new";
        let other = MonitorCaptureSourceBuilder::new(source_name)
            .set_monitor(&monitors[0])
            .build();

        let m = MonitorCaptureSourceBuilder::new(source_name)
            .set_monitor(&monitors[0])
            .set_capture_method(ObsDisplayCaptureMethod::MethodWgc)
            .add_to_scene(&mut scene)
            .unwrap();

        unsafe {
            let mut audio_data = ObsData::new();
            audio_data.set_string(
                "device_id",
                "default",
            );
            // audio_data.set_int("sample_rate", 48000);
            // audio_data.set_int("channel", 2);
            // audio_data.set_bool("use_device_timing", true);
            // audio_data.set_bool("enable_push_to_talk", false);
            let audio_source = libobs::obs_source_create(
                CString::new("wasapi_output_capture").unwrap().as_ptr(),
                CString::new("Audio Capture Source").unwrap().as_ptr(),
                audio_data.as_ptr(),
                ptr::null_mut(),
            );

            libobs::obs_set_output_source(1, audio_source); // 1 = AUDIO CHANNEL
        }

        list_video_devices();

        // let context = use_context::<MainContext>();
        //             let obs_context = context.clone().obs_context.clone();

        //             let output = obs_context.lock().unwrap().get_output("output").unwrap();

        //             output.start().unwrap();

        return Self {
            obs_context: Arc::new(Mutex::new(context)),
            output_name: output_name.to_string(),
        };
    }
}

impl Default for MainContext {
    fn default() -> Self {
        Self::new()
    }
}

