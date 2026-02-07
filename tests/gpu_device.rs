// This test is ignored by default because it requires GPU & platform support.
// Run locally: `cargo test --test gpu_device -- --ignored --nocapture`
#[test]
#[ignore]
fn create_device_and_adapter() {
    pollster::block_on(async {
        let backends = wgpu::Backends::all();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        // Request adapter w/out surface (headless check)
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("no adapter found");

        let info = adapter.get_info();
        println!(
            "Test adapter: {} ({:?}) backend: {:?}",
            info.name, info.device_type, info.backend
        );

        // Ensure we can create a device + queue
        let (_device, _queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("libforge_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .unwrap();
}
