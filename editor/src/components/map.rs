use std::{cell::RefCell, fs::File, io::Read, path::Path, rc::Rc, time::Instant};

use image::{codecs::png::PngDecoder, ColorType, ImageDecoder, RgbaImage};
use twgpu::{
    map::{GpuMapData, GpuMapRender, GpuMapStatic},
    textures::Samplers,
    Camera, GpuCamera, TwRenderPass,
};
use twmap::{EmbeddedImage, Image, TwMap, Version};
use vek::Vec2;
use wgpu::{Color, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor, StoreOp};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{MouseScrollDelta, WindowEvent},
    window::Window,
};

use crate::{
    app::{RenderContext, WgpuContext},
    input_handler::{Cursors, Input, MultiInput},
};

use super::{utils::generation::GenerationContext, AppComponent};

pub struct MapLoader {
    wgpu_context: Rc<RefCell<WgpuContext>>,
    static_context: GpuMapStaticContext,
    dynamic_context: Option<(TwMap, GpuMapDynamicContext)>,
}

impl MapLoader {
    fn new(static_context: GpuMapStaticContext, wgpu_context: Rc<RefCell<WgpuContext>>) -> Self {
        Self {
            static_context,
            dynamic_context: None,
            wgpu_context,
        }
    }

    pub fn load(&mut self, mut tw_map: TwMap) -> &mut TwMap {
        for image in tw_map.images.iter_mut() {
            load_external_image(image, tw_map.version);
        }

        let dynamic_context =
            GpuMapDynamicContext::upload(&tw_map, &self.static_context, self.wgpu_context.clone());

        self.dynamic_context = Some((tw_map, dynamic_context));

        &mut self.dynamic_context.as_mut().unwrap().0
    }

    pub fn unload(&mut self) {
        self.dynamic_context = None;
    }

    pub fn is_loaded(&self) -> bool {
        self.dynamic_context.is_some()
    }
}

struct GpuMapStaticContext {
    camera: GpuCamera,
    samplers: Samplers,
    map: GpuMapStatic,
}

impl GpuMapStaticContext {
    pub fn new(camera: &Camera, wgpu_context: Rc<RefCell<WgpuContext>>) -> Self {
        let wgpu_context = wgpu_context.as_ref().borrow();
        Self {
            camera: GpuCamera::upload(camera, &wgpu_context.device),
            samplers: Samplers::new(&wgpu_context.device),
            map: GpuMapStatic::new(wgpu_context.config.format, &wgpu_context.device),
        }
    }
}

struct GpuMapDynamicContext {
    data: GpuMapData,
    render: GpuMapRender,
}

impl GpuMapDynamicContext {
    pub fn upload(
        tw_map: &TwMap,
        static_map_context: &GpuMapStaticContext,
        wgpu_context: Rc<RefCell<WgpuContext>>,
    ) -> Self {
        let wgpu_context = wgpu_context.as_ref().borrow();
        let data = GpuMapData::upload(tw_map, &wgpu_context.device, &wgpu_context.queue);
        let render = static_map_context.map.prepare_render(
            tw_map,
            &data,
            &static_map_context.camera,
            &static_map_context.samplers,
            &wgpu_context.device,
        );

        Self { data, render }
    }
}

pub struct TwGpuComponent {
    inputs: MultiInput,
    cursors: Cursors,

    camera: Camera,
    old_camera: Camera,

    map_loader: Rc<RefCell<MapLoader>>,
    generation: Rc<RefCell<GenerationContext>>,

    render_size: Vec2<f32>,
}

impl TwGpuComponent {
    pub fn new(
        width: u32,
        height: u32,
        wgpu_context: Rc<RefCell<WgpuContext>>,
        generation: Rc<RefCell<GenerationContext>>,
    ) -> Self {
        let render_size: Vec2<f32> = Vec2::new(width, height).az();

        let camera = Camera::new(width as f32 / height as f32);
        let old_camera = camera;

        let inputs = MultiInput::default();
        let cursors = Cursors::default();

        let static_map_context = GpuMapStaticContext::new(&camera, wgpu_context.clone());

        let map_loader = Rc::new(RefCell::new(MapLoader::new(
            static_map_context,
            wgpu_context,
        )));

        Self {
            inputs,
            cursors,
            camera,
            old_camera,
            map_loader,
            generation,
            render_size,
        }
    }

    pub fn get_map_loader_handle(&self) -> Rc<RefCell<MapLoader>> {
        self.map_loader.clone()
    }
}

impl AppComponent for TwGpuComponent {
    fn label(&self) -> &'static str {
        "twgpu_component"
    }
    fn on_user_input(&mut self, _window: &Window, event: &WindowEvent) -> bool {
        match *event {
            WindowEvent::Touch(touch) => {
                self.inputs.update_input(
                    &Input::from_touch(touch),
                    &mut self.camera,
                    self.render_size,
                );
            }
            WindowEvent::CursorLeft { device_id } => self.cursors.left(device_id),
            WindowEvent::CursorEntered { device_id } => self.cursors.entered(device_id),
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                if let Some(input) = self.cursors.moved(device_id, position) {
                    self.inputs
                        .update_input(&input, &mut self.camera, self.render_size);
                }
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {
                if let Some(input) = self.cursors.input(device_id, state, button) {
                    self.inputs
                        .update_input(&input, &mut self.camera, self.render_size);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let zoom_out = match delta {
                    MouseScrollDelta::LineDelta(_, dy) => dy.is_sign_positive(),
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => {
                        y.is_sign_positive()
                    }
                };
                if zoom_out {
                    self.camera.zoom /= 1.1;
                } else {
                    self.camera.zoom *= 1.1;
                }
            }
            _ => {}
        }

        // pass through other input handlers
        false
    }

    fn on_render(
        &mut self,
        _window: &Window,
        render_context: Option<&mut RenderContext>,
        wgpu_context: &Rc<RefCell<WgpuContext>>,
    ) {
        let wgpu_context = wgpu_context.borrow();

        self.inputs.update_camera(
            &mut self.camera,
            &self.old_camera,
            self.render_size,
            self.cursors.any_position(),
        );

        let time = Instant::now().elapsed().as_secs() as i64;

        self.map_loader
            .borrow()
            .static_context
            .camera
            .update(&self.camera, &wgpu_context.queue);

        if let Some(context) = render_context {
            let frame_view = &context.surface_view;

            let render_pass = context
                .command_encoders
                .get_mut(self.label())
                .unwrap()
                .begin_render_pass(&RenderPassDescriptor {
                    label: Some(self.label()),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &frame_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            let mut tw_render_pass =
                TwRenderPass::new(render_pass, self.render_size.az(), &self.camera);

            if let Some((tw_map, context)) = &self.map_loader.borrow().dynamic_context {
                context.data.update(
                    tw_map,
                    &self.camera,
                    self.render_size.az(),
                    time,
                    time,
                    &wgpu_context.queue,
                );

                context.render.render_background(&mut tw_render_pass);
                context.render.render_foreground(&mut tw_render_pass);
            }
        }

        self.old_camera = self.camera;

        // hack: weird way to poll
        if let Some(tw_map) = self.generation.borrow_mut().take_map() {
            self.map_loader.borrow_mut().unload();
            self.map_loader.borrow_mut().load(tw_map);
            println!("loaded");
        }
    }

    fn on_resize(&mut self, size: PhysicalSize<u32>) {
        self.render_size = Vec2::new(size.width, size.height).az();
        self.camera
            .switch_aspect_ratio(self.render_size.x / self.render_size.y);
        self.inputs.update_map_positions(&self.camera);
    }
}

pub fn load_image<P: AsRef<Path>>(path: P) -> Image {
    let mut buf = Vec::new();
    let mut file = File::open(&path).unwrap();

    file.read_to_end(&mut buf).unwrap();

    let image_decoder = PngDecoder::new(buf.as_slice()).unwrap();
    assert_eq!(image_decoder.color_type(), ColorType::Rgba8); // TODO: better error handling

    let mut image_buffer = vec![0_u8; image_decoder.total_bytes() as usize];
    let (width, height) = image_decoder.dimensions();
    image_decoder.read_image(&mut image_buffer).unwrap();

    let rgba_image = RgbaImage::from_vec(width, height, image_buffer).unwrap();

    Image::Embedded(EmbeddedImage {
        name: path.as_ref().file_name().unwrap().to_str().unwrap().to_string(),
        image: rgba_image.into(),
    })
}

fn load_external_image(external_image: &mut Image, version: Version) {
    if let Image::External(ex) = external_image {
        let _version = match version {
            Version::DDNet06 => "06",
            Version::Teeworlds07 => "07",
        };

        let path = format!("data/mapres/{}.png", ex.name);
        
        let embedded_image = load_image(path);

        *external_image = embedded_image;
    }
}
