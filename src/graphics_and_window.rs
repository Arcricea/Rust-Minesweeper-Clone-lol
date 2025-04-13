use crate::camera::CameraUniform;
use crate::camera::OrthographicCamera;
use crate::game_objects::create_hashmap;
use crate::game_objects::create_minefield;
use crate::game_objects::Z_MINE;
use crate::instance;
use crate::instance::Instance;
use crate::instance::InstanceRaw;
use crate::texture::Texture;
use crate::CommonMineState;
use crate::GameState;
use crate::Mines;
use crate::Tiles;
use crate::BOARD_LENGTH;
use crate::BOARD_WIDTH;
use crate::CAMERA_MOVE_SPEED;
use glam::{Mat4, Vec2, Vec3, Vec4};
use rand::prelude::*;
use std::collections::HashMap;
use std::f32::consts::TAU;
use std::hash::Hash;
use std::ops::Not;
use std::time::Instant;
use wgpu::core::pipeline::RenderPipelineDescriptor;
use winit::dpi::PhysicalPosition;

use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: &'a Window,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    diffuse_bind_group: wgpu::BindGroup,
    diffuse_textures: Vec<Texture>,
    camera: OrthographicCamera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    // instances
    instances_raw: Vec<InstanceRaw>,
    instances_hash: HashMap<String, Vec<InstanceRaw>>,
    instance_buffer: wgpu::Buffer,
    //
    depth_texture: Texture,
    blank_instance: Vec<InstanceRaw>,
    camera_left: f32,
    camera_right: f32,
    camera_up: f32,
    camera_down: f32,
    // Keypresses
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    // Game Stuff
    time_delta: Instant,
    fps_count: u32,
    one_sec_fps: Instant,
    game_state: GameState,
    // Sprites
    sprites: HashMap<String, Vec4>,
}

impl<'a> State<'a> {
    // Creating some of the wgpu types requires async code
    async fn new(window: &'a Window) -> State<'a> {
        let size = window.inner_size();
        let smaller_dimension = size.width.min(size.height);
        let size = winit::dpi::PhysicalSize::new(smaller_dimension, smaller_dimension);
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::TEXTURE_BINDING_ARRAY,
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web, we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                    memory_hints: Default::default(),
                },
                None, // Trace
            )
            .await
            .unwrap();
        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Sprites Stuff!!!=
        let spritesheet = ["src/sprites/spritesheet1.png"];
        let sprites = create_hashmap();
        // Texture stuff!!
        let blank_instance = vec![Instance::to_raw(
            Vec2::new(0.0, 0.0),
            0.0,
            Vec2::new(0.0, 0.0),
            0.0,
            Vec4::new(0.0, 0.0, 0.0, 0.0),
            0,
        )];

        let mut diffuse_textures: Vec<Texture> = vec![];
        let mut diffuse_bytes_vec: Vec<Vec<u8>> = Vec::with_capacity(spritesheet.len());
        // -- Loading Textures --
        for sprites_path in spritesheet.iter() {
            match std::fs::read(sprites_path) {
                Ok(diffuse_bytes) => {
                    diffuse_bytes_vec.push(diffuse_bytes);
                }
                Err(e) => {
                    eprintln!("Error reading file '{}': {:?}", sprites_path, e);
                }
            }
        }
        match Texture::from_bytes(&device, &queue, &diffuse_bytes_vec) {
            Ok(texture) => {
                diffuse_textures.push(texture);
            }
            Err(e) => {
                eprintln!("Error creating texture from bytes {:?}", e)
            }
        }

        // -- Instance Buffer --
        let instances_raw: Vec<InstanceRaw> = blank_instance.clone();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances_raw),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let instances_hash: HashMap<String, Vec<InstanceRaw>> = HashMap::new();

        // -- Texture Bind Group Layout!! --
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: std::num::NonZeroU32::new(diffuse_textures.len() as u32),
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let diffuse_views: Vec<&wgpu::TextureView> = diffuse_textures
            .iter()
            .map(|texture| &texture.view)
            .collect();
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&diffuse_views),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_textures[0].sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        // Camera
        let initial_camera_left = -0.2;
        let initial_camera_right = 1.2;
        let initial_camera_up = 1.2;
        let initial_camera_down = -0.2;

        let camera = OrthographicCamera::new(
            initial_camera_left,
            initial_camera_right,
            initial_camera_down,
            initial_camera_up,
            -1.0,
            1.0,
        );
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        // Shaders

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Depth Buffer

        let depth_texture = Texture::create_depth_texture(&device, &config, "depth_texture");

        // Render Pipeline

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"), // 1.
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::desc(), InstanceRaw::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    // 4.
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            }), // 1.
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
            cache: None,     // 6.
        });

        // Vertex and Index Buffers

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        // Game Stuff
        let new_time_delta = Instant::now();
        let new_fps_one_sec = Instant::now();
        let new_fps_count: u32 = 0;

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            diffuse_bind_group,
            diffuse_textures,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            instances_raw,
            instances_hash,
            instance_buffer,
            depth_texture,
            blank_instance,
            camera_up: initial_camera_up,
            camera_left: initial_camera_left,
            camera_right: initial_camera_right,
            camera_down: initial_camera_down,
            //Keypresses
            is_up_pressed: false,
            is_down_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            // Game Stuff
            time_delta: new_time_delta,
            one_sec_fps: new_fps_one_sec,
            fps_count: new_fps_count,
            game_state: GameState { board: Vec::new() },
            // Sprites!!!
            sprites,
        }
    }

    pub fn window(&self) -> &Window {
        self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            let old_aspect_ratio = (self.size.width as f32) / (self.size.height as f32);
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
            let aspect_ratio = (new_size.width as f32) / (new_size.height as f32);

            let current_world_width = self.camera_right - self.camera_left;
            let current_world_height = self.camera_up - self.camera_down;
            let current_center_x = self.camera_left + current_world_width / 2.0;
            let current_center_y = self.camera_down + current_world_height / 2.0;

            if aspect_ratio > old_aspect_ratio {
                // Window became wider, maintain world height
                let new_world_width = current_world_height * aspect_ratio;
                self.camera_left = current_center_x - new_world_width / 2.0;
                self.camera_right = current_center_x + new_world_width / 2.0;
            } else if aspect_ratio < old_aspect_ratio {
                // Window became narrower, maintain world width
                let new_world_height = current_world_width / aspect_ratio;
                self.camera_down = current_center_y - new_world_height / 2.0;
                self.camera_up = current_center_y + new_world_height / 2.0;
            }
            let camera = OrthographicCamera::new(
                self.camera_left,
                self.camera_right,
                self.camera_down,
                self.camera_up,
                -1.0,
                1.0,
            );

            self.camera_uniform.update_view_proj(&camera);
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera_uniform]),
            );
        }
    }

    fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                occlusion_query_set: None,
                timestamp_writes: None,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16); // 1
            render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances_raw.len() as _);
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    //// The code I added/ Made on own :3
    fn update(&mut self) {
        self.fps_count += 1;
        if self.one_sec_fps.elapsed().as_secs() > 0 {
            // println!("{}", self.fps_count);
            self.fps_count = 0;
            self.one_sec_fps = Instant::now();
        }
        //  println!("{}", self.time_delta.elapsed().as_nanos());
        if self.is_up_pressed {
            self.camera_up += CAMERA_MOVE_SPEED
                * (self.camera_up - self.camera_down)
                * self.time_delta.elapsed().as_nanos() as f32;
            self.camera_down += CAMERA_MOVE_SPEED
                * (self.camera_up - self.camera_down)
                * self.time_delta.elapsed().as_nanos() as f32;
            self.update_camera();
        }
        if self.is_down_pressed {
            self.camera_up -= CAMERA_MOVE_SPEED
                * (self.camera_up - self.camera_down)
                * self.time_delta.elapsed().as_nanos() as f32;
            self.camera_down -= CAMERA_MOVE_SPEED
                * (self.camera_up - self.camera_down)
                * self.time_delta.elapsed().as_nanos() as f32;
            self.update_camera();
        }
        if self.is_left_pressed {
            self.camera_left -= CAMERA_MOVE_SPEED
                * (self.camera_up - self.camera_down)
                * self.time_delta.elapsed().as_nanos() as f32;
            self.camera_right -= CAMERA_MOVE_SPEED
                * (self.camera_up - self.camera_down)
                * self.time_delta.elapsed().as_nanos() as f32;
            self.update_camera();
        }
        if self.is_right_pressed {
            self.camera_left += CAMERA_MOVE_SPEED
                * (self.camera_up - self.camera_down)
                * self.time_delta.elapsed().as_nanos() as f32;
            self.camera_right += CAMERA_MOVE_SPEED
                * (self.camera_up - self.camera_down)
                * self.time_delta.elapsed().as_nanos() as f32;
            self.update_camera();
        }

        self.time_delta = Instant::now();
    }

    fn create_instance(&mut self, key: &str, new_instance: &mut Vec<InstanceRaw>) {
        if self.instances_hash.contains_key(key) {
            let current_instances = self.instances_hash.get_mut(key);
            current_instances
                .expect("key but no hashes?")
                .append(new_instance);
        } else {
            self.instances_hash
                .insert(String::from(key), new_instance.to_vec());
        }
        self.update_instance_buffer();
    }

    fn reset_instances(&mut self) {
        self.instances_hash = HashMap::new();
        self.game_state = GameState { board: Vec::new() };
        self.instances_raw = self.blank_instance.clone();
        self.update_instance_buffer();
        self.update_instance_buffer();
    }

    fn update_instance_buffer(&mut self) {
        self.instances_raw = self.blank_instance.clone();

        for (_key, value) in self.sort_hash_by_z().iter() {
            self.instances_raw.extend(value.iter().clone());
        }
        let instance_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&self.instances_raw),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        self.instance_buffer = instance_buffer;
    }

    fn sort_hash_by_z(&self) -> Vec<(String, Vec<InstanceRaw>)> {
        let mut hash_as_vec: Vec<(String, Vec<InstanceRaw>)> =
            self.instances_hash.clone().into_iter().collect();
        hash_as_vec.sort_by(|(_, vec_a), (_, vec_b)| {
            let z_a = vec_a.first().map(|instance| instance.z_index);
            let z_b = vec_b.first().map(|instance| instance.z_index);

            match (z_a, z_b) {
                (Some(za), Some(zb)) => za.partial_cmp(&zb).unwrap_or(std::cmp::Ordering::Equal),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        hash_as_vec
    }

    fn update_camera(&mut self) {
        let new_camera = OrthographicCamera::new(
            self.camera_left,
            self.camera_right,
            self.camera_down,
            self.camera_up,
            -1.0,
            1.0,
        );
        self.camera = new_camera;
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    fn camera_zoom(&mut self, delta: MouseScrollDelta) {
        if let MouseScrollDelta::LineDelta(_x, y) = delta {
            if y > 0.0 {
                let x1 = (self.camera_right + self.camera_left) / 2.0;
                let y1 = (self.camera_up + self.camera_down) / 2.0;

                self.camera_left *= 1.1;
                self.camera_right *= 1.1;
                self.camera_up *= 1.1;
                self.camera_down *= 1.1;
                let x2 = (self.camera_right + self.camera_left) / 2.0;
                let y2 = (self.camera_up + self.camera_down) / 2.0;

                self.camera_left -= x2 - x1;
                self.camera_right -= x2 - x1;
                self.camera_up -= y2 - y1;
                self.camera_down -= y2 - y1;
            }
            if y < 0.0 {
                let x1 = (self.camera_right + self.camera_left) / 2.0;
                let y1 = (self.camera_up + self.camera_down) / 2.0;

                self.camera_left /= 1.1;
                self.camera_right /= 1.1;
                self.camera_up /= 1.1;
                self.camera_down /= 1.1;
                let x2 = (self.camera_right + self.camera_left) / 2.0;
                let y2 = (self.camera_up + self.camera_down) / 2.0;

                self.camera_left -= x2 - x1;
                self.camera_right -= x2 - x1;
                self.camera_up -= y2 - y1;
                self.camera_down -= y2 - y1;
            }

            self.update_camera();
        }
    }

    /// GAMEPLAY STUFF THAT REQUIRES State
    fn click_tile(&mut self, index: usize) {
        let clicked_tile: (Vec2, f32, Vec2);
        let mut sprite_needed: Option<&str> = None;
        println!("{}", index);
        let tiles: &mut Tiles = self.game_state.board.get_mut(index).unwrap();
        let mine_index = tiles.get_mine_index();
        tiles.clicked = true;
        if tiles.has_mine() {
            clicked_tile = (
                Vec2::new(
                    tiles.position.x + (0.505 * tiles.size),
                    tiles.position.y + (0.505 * tiles.size),
                ),
                tiles.size * 0.95,
                tiles.board_position,
            );
            sprite_needed = Some("Mines");
            tiles.mine = Some(Mines::Default(CommonMineState {
                active: true,
                mine_index: mine_index.unwrap(),
            }));
        } else {
            clicked_tile = (
                Vec2::new(
                    tiles.position.x + (0.505 * tiles.size),
                    tiles.position.y + (0.505 * tiles.size),
                ),
                tiles.size * 0.95,
                tiles.board_position,
            );
        }

        if sprite_needed.is_none() {
            let num = &self.find_nearby_mines(clicked_tile.2).to_string();
            self.create_instance(
                "Numbers",
                &mut vec![Instance::to_raw(
                    clicked_tile.0,
                    0.0,
                    Vec2::new(clicked_tile.1, clicked_tile.1),
                    Z_MINE,
                    *self.sprites.get(num).expect("smth wrong with get sprite"),
                    0,
                )],
            );
        } else {
            self.create_instance(
                "Mines",
                &mut vec![Instance::to_raw(
                    clicked_tile.0,
                    0.0,
                    Vec2::new(clicked_tile.1, clicked_tile.1),
                    Z_MINE,
                    *self
                        .sprites
                        .get(sprite_needed.expect("smth wrong with num"))
                        .expect("smth wrong with get sprite"),
                    0,
                )],
            );
        }
    }

    fn propogate(&mut self, board_position: Vec2) {
        for index in nearby_possible_positions(board_position).iter() {
            if !self
                .game_state
                .board
                .get(*index)
                .expect("propgate mine index not working")
                .clicked
            {
                println!("board_index: {}, board_position: {}", index, board_position);
                self.click_tile(*index);
            }
        }
    }

    fn find_nearby_mines(&mut self, board_position: Vec2) -> u8 {
        let mut mine_amt: u8 = 0;
        for index in nearby_possible_positions(board_position).iter() {
            if let Some(potential_mine) = self.game_state.board.get(*index) {
                if potential_mine.has_mine() {
                    mine_amt += 1;
                }
            }
        }
        if mine_amt == 0 {
            self.propogate(board_position);
        }
        mine_amt
    }
}
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
    z_index: f32,
}

const INDICES: &[u16] = &[
    0, 2, 1, // First triangle
    1, 2, 3, // Second triangle
];
const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, 0.5],
        tex_coords: [0.0, 0.0],
        z_index: 0.0,
    },
    // Top-right
    Vertex {
        position: [0.5, 0.5],
        tex_coords: [1.0, 0.0],
        z_index: 0.0,
    },
    // Bottom-left
    Vertex {
        position: [-0.5, -0.5],
        tex_coords: [0.0, 1.0],
        z_index: 0.0,
    },
    // Bottom-right
    Vertex {
        position: [0.5, -0.5],
        tex_coords: [1.0, 1.0],
        z_index: 0.0,
    },
];

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

pub fn tex_from_coords(coords: [u16; 4]) -> Vec4 {
    Vec4::new(
        coords[0] as f32 / 32.0,
        coords[1] as f32 / 32.0,
        coords[2] as f32 / 32.0,
        coords[3] as f32 / 32.0,
    )
}

fn nearby_possible_positions(board_position: Vec2) -> Vec<usize> {
    const VAL: f32 = BOARD_WIDTH as f32 - 1.0;
    const VALY: f32 = BOARD_LENGTH as f32 - 1.0;
    match (board_position.x, board_position.y) {
        (0.0, VALY) => {
            vec![
                //  y-1
                (board_position.x + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                (board_position.x + 1.0 + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                // y
                (board_position.x + 1.0 + (board_position.y) * BOARD_WIDTH as f32) as usize,
            ]
        }
        (VAL, VALY) => {
            vec![
                //  y-1
                (board_position.x - 1.0 + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                (board_position.x + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                // y
                (board_position.x - 1.0 + (board_position.y) * (BOARD_WIDTH as f32)) as usize,
            ]
        }
        (_, VALY) => {
            vec![
                //  y-1
                (board_position.x - 1.0 + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                (board_position.x + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                (board_position.x + 1.0 + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                // y
                (board_position.x - 1.0 + (board_position.y) * (BOARD_WIDTH as f32)) as usize,
                (board_position.x + 1.0 + (board_position.y) * BOARD_WIDTH as f32) as usize,
                // y+1
            ]
        }
        (0.0, 0.0) => {
            vec![
                // y
                (board_position.x + 1.0 + (board_position.y) * BOARD_WIDTH as f32) as usize,
                // y+1
                (board_position.x + (board_position.y + 1.0) * (BOARD_WIDTH as f32)) as usize,
                (board_position.x + 1.0 + (board_position.y + 1.0) * BOARD_WIDTH as f32) as usize,
            ]
        }
        (VAL, 0.0) => {
            vec![
                // y
                (board_position.x - 1.0 + (board_position.y) * (BOARD_WIDTH as f32)) as usize,
                // y+1
                (board_position.x - 1.0 + (board_position.y + 1.0) * (BOARD_WIDTH as f32)) as usize,
                (board_position.x + (board_position.y + 1.0) * (BOARD_WIDTH as f32)) as usize,
            ]
        }
        (_, 0.0) => {
            vec![
                // y
                (board_position.x - 1.0 + (board_position.y) * (BOARD_WIDTH as f32)) as usize,
                (board_position.x + 1.0 + (board_position.y) * BOARD_WIDTH as f32) as usize,
                // y+1
                (board_position.x - 1.0 + (board_position.y + 1.0) * (BOARD_WIDTH as f32)) as usize,
                (board_position.x + (board_position.y + 1.0) * (BOARD_WIDTH as f32)) as usize,
                (board_position.x + 1.0 + (board_position.y + 1.0) * BOARD_WIDTH as f32) as usize,
            ]
        }
        (0.0, _) => {
            vec![
                //  y-1
                (board_position.x + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                (board_position.x + 1.0 + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                // y
                (board_position.x + 1.0 + (board_position.y) * BOARD_WIDTH as f32) as usize,
                // y+1
                (board_position.x + (board_position.y + 1.0) * (BOARD_WIDTH as f32)) as usize,
                (board_position.x + 1.0 + (board_position.y + 1.0) * BOARD_WIDTH as f32) as usize,
            ]
        }
        (VAL, _) => {
            vec![
                //  y-1
                (board_position.x - 1.0 + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                (board_position.x + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                // y
                (board_position.x - 1.0 + (board_position.y) * (BOARD_WIDTH as f32)) as usize,
                // y+1
                (board_position.x - 1.0 + (board_position.y + 1.0) * (BOARD_WIDTH as f32)) as usize,
                (board_position.x + (board_position.y + 1.0) * (BOARD_WIDTH as f32)) as usize,
            ]
        }
        (_, _) => {
            vec![
                //  y-1
                (board_position.x - 1.0 + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                (board_position.x + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                (board_position.x + 1.0 + (board_position.y - 1.0) * BOARD_WIDTH as f32) as usize,
                // y
                (board_position.x - 1.0 + (board_position.y) * (BOARD_WIDTH as f32)) as usize,
                (board_position.x + 1.0 + (board_position.y) * BOARD_WIDTH as f32) as usize,
                // y+1
                (board_position.x - 1.0 + (board_position.y + 1.0) * (BOARD_WIDTH as f32)) as usize,
                (board_position.x + (board_position.y + 1.0) * (BOARD_WIDTH as f32)) as usize,
                (board_position.x + 1.0 + (board_position.y + 1.0) * BOARD_WIDTH as f32) as usize,
            ]
        }
    }
}

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Minesweeper :3")
        .with_inner_size(winit::dpi::PhysicalSize::new(800, 800))
        .with_min_inner_size(winit::dpi::PhysicalSize::new(200, 200))
        .build(&event_loop)
        .unwrap();

    let mut last_cursor_position: Option<Vec2> = None;
    let mut last_cursor_position_test: Option<Vec2> = None;
    let mut render_state = State::new(&window).await;
    let mut new_board: Vec<InstanceRaw>;
    (new_board, render_state.game_state) = create_minefield(render_state.sprites.clone());
    render_state.create_instance("Tiles", &mut new_board);

    let _ = event_loop.run(move |event, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == render_state.window().id() => {
            if !render_state.input(event) {
                match event {
                    WindowEvent::CursorMoved {
                        device_id: _,
                        position,
                    } => {
                        //last_cursor_position = Some(*position);

                        // make 0,0 camera left, down , render_state.width and height being left
                        // and right and interpolate otherwise

                        let new_position: Vec2 = Vec2::new(
                            (render_state.camera_left
                                + position.x as f32 / render_state.size.width as f32
                                    * (render_state.camera_right - render_state.camera_left)),
                            (render_state.camera_up - render_state.camera_down)
                                + (render_state.camera_down
                                    - position.y as f32 / render_state.size.height as f32
                                        * (render_state.camera_up - render_state.camera_down)),
                        );
                        last_cursor_position = Some(new_position);
                        last_cursor_position_test = Some(Vec2::new(
                            position.x as f32,
                            render_state.size.height as f32 - position.y as f32,
                        ));

                        //println!("{}{}", position.x, position.y);
                        /* println!(
                            "x: {} y: {}",
                            last_cursor_position.unwrap().x as f32 / render_state.size.width as f32,
                            1.0 - last_cursor_position.unwrap().y as f32
                                / render_state.size.height as f32
                        ); */
                    }
                    WindowEvent::KeyboardInput {
                        device_id: _,
                        event,
                        is_synthetic: _,
                    } => {
                        let is_pressed = event.state == ElementState::Pressed;
                        match event.physical_key {
                            winit::keyboard::PhysicalKey::Code(KeyCode::Tab) => {
                                render_state.reset_instances();
                            }
                            winit::keyboard::PhysicalKey::Code(KeyCode::Space) => {
                                if render_state.game_state.board.is_empty() {
                                    let mut new_board: Vec<InstanceRaw>;
                                    (new_board, render_state.game_state) =
                                        create_minefield(render_state.sprites.clone());
                                    render_state.create_instance("Tiles", &mut new_board);
                                }
                            }
                            winit::keyboard::PhysicalKey::Code(KeyCode::ArrowUp) => {
                                render_state.is_up_pressed = is_pressed;
                            }
                            winit::keyboard::PhysicalKey::Code(KeyCode::ArrowDown) => {
                                render_state.is_down_pressed = is_pressed;
                            }
                            winit::keyboard::PhysicalKey::Code(KeyCode::ArrowLeft) => {
                                render_state.is_left_pressed = is_pressed;
                            }
                            winit::keyboard::PhysicalKey::Code(KeyCode::ArrowRight) => {
                                render_state.is_right_pressed = is_pressed;
                            }
                            winit::keyboard::PhysicalKey::Code(KeyCode::ShiftRight) => {
                                render_state.camera_left = 0.0;
                                render_state.camera_up = 1.0;
                                render_state.camera_right = 1.0;
                                render_state.camera_down = 0.0;
                                render_state.update_camera();
                            }
                            _ => {}
                        }
                    }

                    WindowEvent::MouseInput {
                        device_id: _,
                        state,
                        button,
                    } => {
                        if *button == MouseButton::Left && *state == ElementState::Pressed {
                            println!("Left Mouse");
                            println!(
                                "original: {},{} |  new: {},{}",
                                last_cursor_position_test.unwrap().x,
                                last_cursor_position_test.unwrap().y,
                                last_cursor_position.unwrap().x,
                                last_cursor_position.unwrap().y
                            );
                            let mut board_position: Option<Vec2> = None;
                            let mut unclicked_count: u32 = 0;
                            let mut flagged_list: Option<Vec<usize>> = None;

                            for tiles in render_state.game_state.board.iter() {
                                if tiles.is_clicked(last_cursor_position.unwrap()) {
                                    if !tiles.clicked {
                                        if !tiles.flagged {
                                            board_position = Some(tiles.board_position);
                                        }
                                        unclicked_count += 1;
                                    } else {
                                        let mut non_flag_list: Vec<usize> = Vec::new();

                                        for index in
                                            nearby_possible_positions(tiles.board_position).iter()
                                        {
                                            if !render_state
                                                .game_state
                                                .board
                                                .get(*index)
                                                .unwrap()
                                                .flagged
                                                && !render_state
                                                    .game_state
                                                    .board
                                                    .get(*index)
                                                    .unwrap()
                                                    .clicked
                                            {
                                                non_flag_list.push(*index);
                                                println!("{}", index);
                                            }
                                        }

                                        flagged_list = Some(non_flag_list);
                                    }
                                }
                            }
                            if flagged_list.is_some() {
                                for index in flagged_list.unwrap().iter() {
                                    render_state.click_tile(*index);
                                }
                            }
                            if board_position.is_some() {
                                let board_position = board_position.unwrap();
                                render_state.click_tile(
                                    (board_position.x + board_position.y * BOARD_WIDTH as f32)
                                        as usize,
                                );
                                let new_clicked_tile = render_state
                                    .game_state
                                    .board
                                    .get(
                                        (board_position.x + board_position.y * BOARD_WIDTH as f32)
                                            as usize,
                                    )
                                    .expect("idefk");
                                // println!("{}", unclicked_count);
                                if unclicked_count == BOARD_WIDTH * BOARD_LENGTH
                                    && new_clicked_tile.has_mine()
                                {
                                    let mut board_not_ready = true;
                                    while board_not_ready {
                                        let mut new_board: Vec<InstanceRaw>;
                                        (new_board, render_state.game_state) =
                                            create_minefield(render_state.sprites.clone());
                                        render_state.create_instance("Tiles", &mut new_board);
                                        println!("hi");
                                        if let Some(tile) = render_state.game_state.board.get(
                                            (board_position.x
                                                + board_position.y * BOARD_WIDTH as f32)
                                                as usize,
                                        ) {
                                            if !tile.has_mine() {
                                                board_not_ready = false;
                                                println!(
                                                    "{}",
                                                    render_state.find_nearby_mines(board_position)
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if *button == MouseButton::Right && *state == ElementState::Pressed {
                            println!("Right Mouse");
                            let mut clicked_tile: Option<(Vec2, f32, Vec2)> = None;
                            let mut new_flag = true;

                            for tiles in render_state.game_state.board.iter_mut() {
                                if !tiles.clicked && tiles.is_clicked(last_cursor_position.unwrap())
                                {
                                    if tiles.flagged {
                                        tiles.flagged = false;
                                        new_flag = false;
                                    } else {
                                        tiles.flagged = true;
                                    }
                                    clicked_tile = Some((
                                        Vec2::new(
                                            tiles.position.x + 0.55 * tiles.size,
                                            tiles.position.y + 0.55 * tiles.size,
                                        ),
                                        tiles.size,
                                        tiles.board_position,
                                    ));
                                }
                            }
                            if clicked_tile.is_some() && new_flag {
                                render_state.create_instance(
                                    "Flags",
                                    &mut vec![Instance::to_raw(
                                        clicked_tile.unwrap().0,
                                        0.0,
                                        Vec2::new(clicked_tile.unwrap().1, clicked_tile.unwrap().1),
                                        Z_MINE,
                                        *render_state.sprites.get("Flags").expect("no flag :c"),
                                        0,
                                    )],
                                );
                            } else if clicked_tile.is_some() && !new_flag {
                                let mut flags_vec = render_state
                                    .instances_hash
                                    .get("Flags")
                                    .expect(
                                        "checking for removing flag when flags doesnt exist wtf?",
                                    )
                                    .clone();
                                let mut flag_index: Option<usize> = None;
                                for (index, flags_instances) in flags_vec.iter().enumerate() {
                                    if flags_instances
                                        == &Instance::to_raw(
                                            clicked_tile.unwrap().0,
                                            0.0,
                                            Vec2::new(
                                                clicked_tile.unwrap().1,
                                                clicked_tile.unwrap().1,
                                            ),
                                            Z_MINE,
                                            *render_state.sprites.get("Flags").expect("no flag :c"),
                                            0,
                                        )
                                    {
                                        flag_index = Some(index);
                                    }
                                }
                                if let Some(i) = flag_index {
                                    flags_vec.remove(i);
                                    render_state.instances_hash.remove_entry("Flags");
                                    render_state
                                        .instances_hash
                                        .insert(String::from("Flags"), flags_vec);
                                    render_state.update_instance_buffer();
                                }
                            }
                        }
                    }
                    WindowEvent::MouseWheel {
                        device_id: _,
                        delta,
                        phase: _,
                    } => render_state.camera_zoom(*delta),
                    WindowEvent::RedrawRequested => {
                        render_state.update();
                        render_state.window().request_redraw();

                        match render_state.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                render_state.resize(render_state.size)
                            }
                            Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => {
                                log::error!("OutOfMemory");
                                control_flow.exit();
                            }

                            Err(wgpu::SurfaceError::Timeout) => {
                                log::warn!("Surface timeout")
                            }
                        }
                    }
                    WindowEvent::CloseRequested => control_flow.exit(),
                    WindowEvent::Resized(physical_size) => {
                        render_state.resize(*physical_size);
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    });
}
