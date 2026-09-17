use shadow_core::Result;
use shadow_vmm::{
    ChromiumSandbox, ChromiumSandboxConfig, CdpSession, VirtualDisplayConfig, VirtualDisplayServer,
};
use std::time::Instant;

#[test]
fn test_virtual_display_initialization_and_zero_host_pollution() -> Result<()> {
    let config = VirtualDisplayConfig::default();
    assert_eq!(config.display, ":99");
    assert_eq!(config.width, 1920);
    assert_eq!(config.height, 1080);
    assert_eq!(config.depth, 24);
    assert!(config.in_memory_backing);

    let mut server = VirtualDisplayServer::new(config);
    server.start()?;
    assert!(server.is_active);

    let args = server.build_xvfb_args();
    assert!(args.contains(&":99".to_string()));
    assert!(args.contains(&"1920x1080x24".to_string()));
    assert!(args.contains(&"-nolisten".to_string()));
    assert!(args.contains(&"tcp".to_string()));
    assert!(args.contains(&"-fbdir".to_string()));

    // Verify zero host screen pollution
    assert!(server.verify_zero_host_pollution());

    let envs = server.get_env_vars();
    assert!(envs.iter().any(|(k, v)| k == "DISPLAY" && v == ":99"));

    server.stop()?;
    assert!(!server.is_active);
    Ok(())
}

#[test]
fn test_chromium_sandbox_flags_and_isolation() -> Result<()> {
    let config = ChromiumSandboxConfig::default();
    assert_eq!(config.remote_debugging_port, 9222);
    assert_eq!(config.window_width, 1920);
    assert_eq!(config.window_height, 1080);

    let mut sandbox = ChromiumSandbox::new(config);
    sandbox.start()?;
    assert!(sandbox.is_running);

    let args = sandbox.build_command_args();
    assert!(args.contains(&"--no-sandbox".to_string()));
    assert!(args.contains(&"--disable-dev-shm-usage".to_string()));
    assert!(args.contains(&"--use-gl=swiftshader".to_string()));
    assert!(args.contains(&"--remote-debugging-port=9222".to_string()));
    assert!(args.contains(&"--display=:99".to_string()));
    assert!(args.contains(&"--window-size=1920,1080".to_string()));
    assert!(args.contains(&"--disable-gpu-sandbox".to_string()));

    assert_eq!(sandbox.cdp_url(), "http://127.0.0.1:9222");

    sandbox.stop()?;
    assert!(!sandbox.is_running);
    Ok(())
}

#[test]
fn test_raw_cdp_navigation_and_dom_snapshot_extraction() -> Result<()> {
    let session = CdpSession::new(2, 9222);

    // 1. Page.navigate
    let nav = session.navigate("http://localhost:3000/app")?;
    assert_eq!(nav.url, "http://localhost:3000/app");
    assert_eq!(nav.status, 200);
    assert_eq!(nav.ready_state, "complete");
    assert!(nav.title.contains("http://localhost:3000/app"));

    // 2. DOM.getDocument
    let dom = session.get_dom_document()?;
    assert_eq!(dom.node_id, 1);
    assert_eq!(dom.node_name, "#document");
    assert!(dom.children.is_some());

    let html = &dom.children.as_ref().unwrap()[0];
    assert_eq!(html.node_name, "HTML");

    // 3. Runtime.evaluate
    let eval_title = session.evaluate("document.title")?;
    assert!(eval_title["result"]["value"].as_str().unwrap().contains("ShadowOS"));

    Ok(())
}

#[test]
fn test_raw_cdp_mouse_and_keyboard_interaction() -> Result<()> {
    let session = CdpSession::new(2, 9222);

    // 1. Mouse Click
    session.click(450.0, 320.0)?;

    // 2. Keyboard Typing
    let count = session.type_text("ShadowOS Visual Agent Test")?;
    assert_eq!(count, 26);

    Ok(())
}

#[test]
fn test_viewport_framebuffer_render_latency_sub_100ms_across_50_cycles() -> Result<()> {
    let session = CdpSession::new(2, 9222);
    let mut latencies = Vec::new();

    for _ in 0..50 {
        let t0 = Instant::now();
        let shot = session.capture_screenshot("png", false)?;
        let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;

        assert_eq!(shot.format, "png");
        assert_eq!(shot.width, 1920);
        assert_eq!(shot.height, 1080);
        assert!(!shot.base64_data.is_empty());
        assert!(shot.target_met);
        assert!(shot.render_latency_ms < 100.0);
        assert!(elapsed_ms < 100.0);

        latencies.push(shot.render_latency_ms);
    }

    let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let max = latencies.iter().cloned().fold(0.0, f64::max);
    println!("50 Cycles Screenshot Latency: Avg = {:.3}ms, Max = {:.3}ms (Target: < 100.0ms)", avg, max);
    assert!(avg < 10.0, "Average latency should be well under 100ms");
    assert!(max < 100.0, "Max latency must be strictly under 100ms");

    Ok(())
}
