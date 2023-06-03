use std::sync::Arc;

use eframe::{
    egui,
    egui_wgpu::{self, wgpu},
};

#[allow(unused_imports)]
use eframe::wgpu::util::DeviceExt;

mod camera;
use camera::{Camera, CameraUniform};

pub struct Viewer {
    camera: Camera,
}

impl Viewer {
    pub fn new<'a>(cc: &'a eframe::CreationContext<'a>) -> Option<Self> {
        // Get the WGPU render state from the eframe creation context. This can also be retrieved
        // from `eframe::Frame` when you don't have a `CreationContext` available.
        let wgpu_render_state = cc.wgpu_render_state.as_ref()?;

        let device = &wgpu_render_state.device;

        let camera = Camera::default();
        let camera_uniform = CameraUniform::new(device, camera.calc_matrix().as_ref());

        let object_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("object"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/object.wgsl").into()),
        });

        let object_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("object"),
                bind_group_layouts: &[&camera_uniform.bind_group_layout],
                push_constant_ranges: &[],
            });

        let object_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("object"),
            layout: Some(&object_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &object_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &object_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu_render_state.target_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        wgpu_render_state
            .renderer
            .write()
            .paint_callback_resources
            .insert(TriangleRenderResources {
                pipeline: object_pipeline,
                camera_uniform,
            });

        Some(Self { camera })
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        /*egui::Frame::canvas(ui.style()).show(ui, |ui| {
            self.custom_painting(ui);
        });*/
        self.custom_painting(ui);
    }

    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let available_size = ui.available_size();

        self.camera.resize(available_size.x, available_size.y);

        let (rect, response) = ui.allocate_exact_size(available_size, egui::Sense::drag());

        if response.dragged_by(egui::PointerButton::Secondary) {
            self.camera.yaw += cgmath::Rad(response.drag_delta().x * 0.005);
            self.camera.pitch += cgmath::Rad(response.drag_delta().y * 0.005);
            self.camera.has_changed = true;
        }

        // TODO: Find out how to detect scroll

        let view_proj = if self.camera.has_changed {
            self.camera.has_changed = false;
            Some(self.camera.calc_matrix())
        } else {
            None
        };

        let cb = egui_wgpu::CallbackFn::new()
            .prepare(move |_device, queue, _encoder, paint_callback_resources| {
                let resources: &TriangleRenderResources = paint_callback_resources.get().unwrap();

                // Update the camera uniform buffer if changed
                if let Some(view_proj) = view_proj {
                    resources.camera_uniform.update(queue, view_proj.as_ref());
                }

                Vec::new()
            })
            .paint(move |_info, render_pass, paint_callback_resources| {
                let resources: &TriangleRenderResources = paint_callback_resources.get().unwrap();
                resources.paint(render_pass);
            });

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };

        ui.painter().add(callback);
    }
}

struct TriangleRenderResources {
    pipeline: wgpu::RenderPipeline,
    camera_uniform: CameraUniform,
}

impl TriangleRenderResources {
    fn paint<'rp>(&'rp self, render_pass: &mut wgpu::RenderPass<'rp>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_uniform.bind_group, &[]);
        render_pass.draw(0..6, 0..2);
    }
}
