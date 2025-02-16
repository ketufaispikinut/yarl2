//! # YARL-2 (Yet Another Roguelike Library - 2)
//! YARL-2 is Yet Another virtual terminal library that was created for the sake of the NIH-syndrome.
//! ### Quick start:
//! ```
//! // you must create a game struct
//! pub struct Game{}
//! fn main() {
//!     // run the game
//!     yarl_2::run_game(Game{}, Config::default())
//! }
//! // you must implement this trait for your game struct
//! impl yarl_2::Yarl2Game for Game{
//!     // this function is where you would implement most of your rendering logic
//!     fn pre_draw(&mut self, window: &mut yarl_2::Window<'static>, yarl_2::keyboard: &NiceKeyboard){
//!         // display a yellow @ at 0, 0
//!         window.set_char_at(0,0,'@');
//!         window.set_fg_at(0,0,yarl_2::colors::YELLOW);
//!     }
//! }

use std::{collections::HashSet, str::FromStr};

use bytemuck::Zeroable;
use colors::{BLACK, CYAN, GREEN, RED, TRANSPARENT, WHITE, YELLOW};
use image::{DynamicImage, ImageBuffer, Rgba};
use ui::{BorderStyle, Button, FillStyle, Label, UIBox, UIData, UIDataEntry, UINode};
use wgpu::{util::DeviceExt, TextureUsages};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseButton},
    event_loop::EventLoop,
    keyboard::PhysicalKey,
    window::{Window as WinitWindow, WindowAttributes},
};
pub mod colors;
pub mod ui;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
}
unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}
/// These are the vertices that occupy the whole screen
const VERTICES: &[Vertex] = &[
    // tri 1
    Vertex {
        position: [-1., -1., 0.],
        uv: [0., 0. + 1.],
    },
    Vertex {
        position: [-1., 1., 0.],
        uv: [0., 1. - 1.],
    },
    Vertex {
        position: [1., 1., 0.],
        uv: [1., 1. - 1.],
    },
    // tri 2
    Vertex {
        position: [-1., -1., 0.],
        uv: [0., 0. + 1.],
    },
    Vertex {
        position: [1., -1., 0.],
        uv: [1., 0. + 1.],
    },
    Vertex {
        position: [1., 1., 0.],
        uv: [1., 1. - 1.],
    },
];
/// These are the vertices for individual floating letters
const VERTICES_I: &[Vertex] = &[
    // tri 1
    Vertex {
        position: [0., 0., 0.],
        uv: [0., 0. + 1.],
    },
    Vertex {
        position: [0., 1., 0.],
        uv: [0., 1. - 1.],
    },
    Vertex {
        position: [1., 1., 0.],
        uv: [1., 1. - 1.],
    },
    // tri 2
    Vertex {
        position: [0., 0., 0.],
        uv: [0., 0. + 1.],
    },
    Vertex {
        position: [1., 0., 0.],
        uv: [1., 0. + 1.],
    },
    Vertex {
        position: [1., 1., 0.],
        uv: [1., 1. - 1.],
    },
];
// This is the layout of the screen vertices, I mostly copy-pasted it from the wgpu tutorial
const VERTEX_LAYOUT: wgpu::VertexBufferLayout = /* more copy-pasting :3 */
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress, // 1.
        step_mode: wgpu::VertexStepMode::Vertex,                            // 2.
        attributes: &[
            // 3.
            wgpu::VertexAttribute {
                offset: 0,                             // 4.
                shader_location: 0,                    // 5.
                format: wgpu::VertexFormat::Float32x3, // 6.
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress, //3//2
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2, //3
            },
        ],
    };
/// A font to use in the app. Note that the font must be an image with alpha, where white represents 100% character and transparent 0% character
/// It must follow the same cp437 grid such as Dwarf Fortress' fonts
pub enum Font {
    /// Represents a file, that will be loaded by the `image` crate.
    /// (Of course, you can also customize it, altough Image is much more convenient)
    Binary(&'static [u8]), //,image::ImageFormat//In this case, the file type must be provided
    /// Represents a file to load, also loaded by the `image` crate
    Path(String),
    /// Represents an image, that you may have manipulated yourself beforehand
    /// (can be used, for instance, if you want to procedurally generate fonts)
    Image(DynamicImage),
}
impl Default for Font {
    /// the default font is a variant of comic sans taken from https://dtinth.github.io/comic-mono-font/ but passed trough here http://mifki.com/df/fontgen/ to generate the grid
    fn default() -> Self {
        Font::Binary(include_bytes!("../comic_mono_cp437.png")) //)
                                                                //todo!()//Path//.to_owned()//terminal8x8
    }
}
/// The window type, with which you do rendering with
pub struct Window<'a> {
    // Winit's window, I probably could have had messed with lifetimes to make it work (it needs to be dropped after everything that depends on it) but I instead choose to just `Box::leak` it.
    window: &'static WinitWindow,
    // All these parameters are explained in the new_inner function
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    char_width: u32,
    char_height: u32,
    buffer_colors_fg: Vec<u8>,
    buffer_colors_bg: Vec<u8>,
    buffer_chars: Vec<u8>,
    set_buffer: Vec<u8>,
    set_texture: wgpu::Texture,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    char_grid_texture: wgpu::Texture,
    fg_texture: wgpu::Texture,
    bg_texture: wgpu::Texture,
    background_color: (u8, u8, u8, u8),
    config_chargrid: Config,
    dirty: bool,
    char_grid_size: wgpu::Extent3d,
    text_texture: wgpu::Texture,
    instances: Vec<InstanceData>,
    instance_buffer: wgpu::Buffer,
    instance_count: u32,
    instance_vertices: wgpu::Buffer,
    instance_pipeline: wgpu::RenderPipeline,
    surface_conf: wgpu::SurfaceConfiguration,
}
impl<'a> Window<'a> {
    async fn new_inner(
        config: Config,
        size: PhysicalSize<u32>,
        window: &'static WinitWindow,
        images: &Vec<DynamicImage>,
    ) -> Self {
        // save the padding here (define an alias)
        let padding = config.padding;
        // create the buffers's vecs that we will keep CPU-side and modify, then send to the GPU each frame
        let buffer_colors_fg = vec![0; (config.size.0 * config.size.1) as usize * 4];
        let buffer_colors_bg = vec![0; (config.size.0 * config.size.1) as usize * 4];
        let set_buffer = vec![0; (config.size.0 * config.size.1) as usize];
        let buffer_chars = vec![0; (config.size.0 * config.size.1) as usize];
        // define more aliases!
        let background_color = config.background_color;
        // Calculate the size of chars based on the assumption that the fonts are 256-character grids following cp437 encoding
        let char_width = images[0].width() / 16;
        let char_height = images[0].height() / 16;
        // define more aliases
        let cg_width = config.size.0;
        let cg_height = config.size.1;
        // calculate padding (this is magic math I could have had figured out properly but I instead fiddled with it till it worked).
        let size_x = (size.width - config.padding.0 * config.scale.0) as f32 / (size.width) as f32;
        let size_y =
            (size.height - config.padding.1 * config.scale.1) as f32 / (size.height) as f32;

        // define more aliases
        let config_chargrid = config;
        let max_instances = config_chargrid.max_instances;

        // my instance configuration is mainly for wasm32 support
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,

            ..Default::default()
        });
        // create the surface 
        let surface = instance.create_surface(window).unwrap();
        // create the adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        // create the device and the queue; we will use the device when creating ressources and the queue when issuing orders
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    // note more wasm32 things
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    // Mostly useful for debugging
                    label: Some("device"),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter() 
            .find(|f| {
                // sRGB changes how colors are scaled, which makes low RGB values brighter
                if config_chargrid.srgb {
                    f.is_srgb()
                } else {
                    !f.is_srgb()
                }
            })
            .copied()
            // in case we didn't find anything that worked, we rely on the first element
            .unwrap_or(surface_caps.formats[0]);
        // render surface's config
        let config = wgpu::SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        // we convert all our font images to rgba8
        let images_rgba8: Vec<ImageBuffer<Rgba<u8>, Vec<u8>>> =
            images.iter().map(|f| f.to_rgba8()).collect();
        // we get the dimension of the first one (all font images must be the same size since I am using an array)
        let dimensions = images_rgba8[0].dimensions();
        // texture size of the font images on the GPU
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            // note the array layers
            depth_or_array_layers: images_rgba8.len() as u32,
        };
        // creates the GPU-side texture
        let wgpu_side_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2, 
            // Format matches image format
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // Copy DST is important since we dont actually create it with content
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("text texture"),
            view_formats: &[],
        });
        // The size of the chargrid that covers the whole screen (except the padding)
        let char_grid_size = wgpu::Extent3d {
            width: cg_width,
            height: cg_height,
            depth_or_array_layers: 01,
        };
        // We create its texture
        let char_grid_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: char_grid_size, 
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2, 
            // It's a texture of single bytes, thus R8Unorm, which provides us with f32s scaled from 0. to 1. on the shader side
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("text grid texture"),
            view_formats: &[],
        });
        // creates the texture which stores foreground colors for each grid character
        let fg_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: char_grid_size, 
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // note the format
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("text fg texture"),
            view_formats: &[],
        });
        // creates the texture which stores background colors for each grid character

        let bg_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: char_grid_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("text bg texture"),
            view_formats: &[],
        });
        // creates the texture which stores which font each character uses
        // it is used in the shader to index into the font texture array
        let set_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: char_grid_size, 
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // note that this gives us a limitation of 256 font files
            format: wgpu::TextureFormat::R8Unorm, 
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("text set texture"),
            view_formats: &[],
        });
        // Generic pixel perfect sampler that clamps to the border
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        // view for the text texture, it is provided to the pipeline when it renders
        let view = wgpu_side_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        // create other views
        let view_char_grid = char_grid_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let fg_view = fg_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bg_view = bg_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let set_view = set_texture.create_view(&wgpu::TextureViewDescriptor::default()); //bg//bg
        for i in images_rgba8.iter().enumerate() {
            // ensure images are the same size
            assert_eq!(
                dimensions,
                i.1.dimensions(),
                "images must have the same size, sadly :("
            );
            // writes the font texture to the array
            queue.write_texture(
                wgpu::ImageCopyTextureBase {
                    texture: &wgpu_side_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        // note the z-offset
                        z: i.0 as u32,
                    }, 
                    aspect: wgpu::TextureAspect::All,
                },
                &i.1, 
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * dimensions.0),
                    rows_per_image: Some(dimensions.1),
                },
                wgpu::Extent3d {
                    width: texture_size.width,
                    height: texture_size.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        // writes the textures with the stored buffers
        // first, which char the grid uses
        queue.write_texture(
            wgpu::ImageCopyTextureBase {
                texture: &char_grid_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &buffer_chars,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(cg_width),
                rows_per_image: Some(cg_height),
            },
            char_grid_size,
        );
        // second, the foreground color
        queue.write_texture(
            wgpu::ImageCopyTextureBase {
                texture: &fg_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &buffer_colors_fg,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(cg_width * 4),
                rows_per_image: Some(cg_height),
            },
            char_grid_size,
        );
        // third, the background color
        queue.write_texture(
            wgpu::ImageCopyTextureBase {
                texture: &bg_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &buffer_colors_bg,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(cg_width * 4),
                rows_per_image: Some(cg_height),
            },
            char_grid_size,
        );
        // includes the shaders, first the chargrid shader
        let shader = include_str!("text_shader.wglsl");
        // then the floating characters/instances shader
        let mut shader_instance = include_str!("instance_shader.wglsl").to_owned();
        // I could have went with constants but I instead just pre-filtered the shaders
        // This is bad practice, but it Just Works
        shader_instance = shader_instance.replace("$SC_WIDTH", format!("{}", size.width).as_str());
        shader_instance =
            shader_instance.replace("$SC_HEIGHT", format!("{}", size.height).as_str());
        shader_instance = shader_instance.replace("$C_WIDTH", format!("{}", char_width).as_str());
        shader_instance = shader_instance.replace("$C_HEIGHT", format!("{}", char_height).as_str());
        shader_instance = shader_instance.replace("$PADDING_X", format!("{}", padding.0).as_str());
        shader_instance = shader_instance.replace("$PADDING_Y", format!("{}", padding.1).as_str());
        shader_instance = shader_instance.replace(
            "$SCALE_FACTOR",
            format!("{}", cg_width as f32 / (cg_height as f32)).as_str(),
        );
        shader_instance =
            shader_instance.replace("$SCALE_X", format!("{}", config_chargrid.scale.0).as_str());
        shader_instance =
            shader_instance.replace("$SCALE_Y", format!("{}", config_chargrid.scale.1).as_str()); //X//0
        let mut shader = shader.replace(
            "$SCALE_FACTOR_X",
            format!("f32({})", dimensions.0 as f32).as_str(),
        ); 
        shader = shader.replace(
            "$SCALE_FACTOR_Y",
            format!(
                "f32({})",
            )
            .as_str(),
        );
        shader = shader.replace(
            "$CHARGRID",
            format!("vec2<f32>(f32({}),f32({}))", cg_width, cg_height).as_str(),
        );
        // loads the shaders into WGPU
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text shader"),
            source: wgpu::ShaderSource::Wgsl(shader.into()),
        });
        let instance_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("instance shader"),
            source: wgpu::ShaderSource::Wgsl(shader_instance.into()),
        });
        // Here, I copied more things from the wgpu tutorial; each of these bindings echo textures & samplers from the shaders
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                // nooo i didnt copy paste anythiiing <- this is a lie
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5, //4
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2, 
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                ],
                label: Some("text rendering bind group layout"),
            });
        // creates the bind group with the layout we just provided
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    // here we actually point it to what we want it to be in the shaders
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&view_char_grid),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&fg_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&bg_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,                                             
                    resource: wgpu::BindingResource::TextureView(&set_view), 
                },
            ],
            label: Some("text rendering bind group"),
        });
        // we specify with the texture bind group layout our render pipeline layout
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("text render pipeline layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });
        // the render pipeline is a vital part of WGPU rendering
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                // specify the shader
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[VERTEX_LAYOUT],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                // also specify the shader
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    // blendstates are useful here to blend the config's background color with the color of the chargrid
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            // no depth
            depth_stencil: None,
            multiview: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            cache: None,
        });
        // this is the render pipeline that renders floating characters / instances
        let instance_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("text instances render pipeline"),
                // it uses the same layout
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    // but not the same shader
                    module: &instance_shader,
                    entry_point: Some("vs_main"),
                    // and it has an extra buffer
                    buffers: &[VERTEX_LAYOUT, INSTANCE_LAYOUT],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    // apart from the shader, the fragment uses the same configuration
                    module: &instance_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent::OVER,
                        }), 
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                // still no depth testing
                depth_stencil: None,
                multiview: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                cache: None,
        });
        
        // configure the surface
        surface.configure(&device, &config);
        // creates the vertex buffer for the triangles that cover the screen (note that we also do some math here to ensure padding works)
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(
                &VERTICES
                    .into_iter()
                    .map(|f| {
                        // apply padding
                        Vertex {
                            position: [
                                f.position[0] * size_x,
                                f.position[1] * size_y,
                                // we don't care about z-position
                                f.position[2],
                            ],
                            uv: [f.uv[0], f.uv[1]],
                        }
                    })
                    .collect::<Vec<Vertex>>(),
            ),
            // we will not write to it, so it doesn't need `COPY_DST`
            usage: wgpu::BufferUsages::VERTEX,
        });
        // this is the CPU-side buffer of instances/floating characters
        let instances = vec![InstanceData::zeroed(); max_instances as usize];
        // we create its buffer
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: bytemuck::cast_slice(&instances),
            // note that in wgpu, instances use `BufferUsages::VERTEX`
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
        });
        // we create the buffer that contains the vertices
        let instance_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance vertice buffer"),
            contents: bytemuck::cast_slice(&VERTICES_I),
            usage: wgpu::BufferUsages::VERTEX,
        });
        // we create the variable that contains our instance count (the amount of instances that currently have values)
        let instance_count = 0;
        // we return the completed window
        Self {
            window, 
            surface,
            device,
            queue,
            instances,
            instance_count,
            instance_buffer,
            text_texture: wgpu_side_texture,
            buffer_colors_bg,
            buffer_colors_fg,
            buffer_chars,
            render_pipeline,
            bind_group: texture_bind_group,
            vertex_buffer,
            char_grid_texture,
            fg_texture,
            bg_texture,
            background_color,
            config_chargrid,
            dirty: false,
            char_grid_size,
            char_width,
            char_height,
            set_buffer,
            set_texture,
            instance_vertices,
            instance_pipeline: instance_render_pipeline,
            surface_conf: config,
        }
    }
    /// THIS IS THE FUNCTION YOU MUST CALL IF YOU ARE FANCY, BUT YOU CAN ALSO JUST USE `run_game()`
    /// note: leaks memory
    pub fn new_run<T>(game: T, config: Config) -> !
    where
        T: Yarl2Game,
    {
        // extracts the images of the config
        let images: Vec<DynamicImage> = config
            .font
            .iter()
            .map(|f| {
                match f {
                    Font::Image(k) => k.clone(),
                    Font::Binary(bin) => {
                        image::ImageReader::new(std::io::Cursor::new(bin))
                            // with guessed format is important, otherwise `image` wont load those unspecified-format slices of bytes
                            .with_guessed_format()
                            .unwrap()
                            .decode()
                            .unwrap()
                    }
                    Font::Path(path) => image::ImageReader::open(path)
                        .unwrap()
                        // it is still important
                        .with_guessed_format()
                        .unwrap()
                        .decode()
                        .unwrap(),
                }
            })
            .collect();
        // does more math we will re-do later to calculate the window size
        let char_width = images[0].width() / 16;
        let char_height = images[0].height() / 16;
        let pixel_size = (
            config.size.0 * char_width,  
            config.size.1 * char_height, 
        );
        let event_loop = EventLoop::new().unwrap(); 
        // PhysicalSize is in screen pixels, so it should more or less ignore DPI (except on the web, where it does *magic*)
        let size = PhysicalSize::new(
            (pixel_size.0 + config.padding.0) * config.scale.0,
            (pixel_size.1 + config.padding.1) * config.scale.1,
        ); 
        // creates (and leaks) the window!
        let window: &'static WinitWindow = Box::leak(Box::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_inner_size(size)
                        .with_title(&config.name) 
                        // I set it up as resizable, but I haven't really spent time on this part
                        // So it just stretches the original dimensions
                        .with_resizable(!false), 
                                                 
                )
                .unwrap(),
        ));
        // Copy-pasted again
        #[cfg(target_arch = "wasm32")]
        {
            // Winit prevents sizing with CSS, so we have to set
            // the size manually when on web.
            use winit::dpi::PhysicalSize;
            let _ = window.request_inner_size(size); //PhysicalSize::new(450, 400)

            use winit::platform::web::WindowExtWebSys;
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("wasm-id-magic")?;
                    let canvas = web_sys::Element::from(window.canvas()?);
                    dst.append_child(&canvas).ok()?;
                    Some(())
                })
                .expect("Couldn't append canvas to document body.");
        }

        let return_value =
            {
                // now, since the default wasm32 target does not support `smol`, we must use a different crate to create all the variables, since they are created by our async function
                #[cfg(target_arch="wasm32")]
                {
                wasm_rs_async_executor::single_threaded::block_on(Window::new_inner(config, size, &window, &images))
                }

                #[cfg(not(target_arch="wasm32"))]
                {
                smol::block_on(Window::new_inner(config, size, &window, &images))
                }
            }
            ; 
        // wait just calls the receive_event when an event is received
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        // this creates the struct that will run the event loop
        let mut event_loop_runner = EventLoopWrapper {
            game,
            window: return_value,
            keyboard: NiceKeyboard {
                keys: HashSet::new(),
                letters: HashSet::new(),
                mouse_position: (0, 0),
                mouse_pressed: false,
            },
        };
        // this runs the `event_loop_runner`
        let _ = event_loop.run_app(&mut event_loop_runner);
        // this exits the process if we make it out of the run_app
        std::process::exit(0)
    }
    /// this function transmits all CPU-side buffers to the GPU
    /// this is quite an heavy task; it would be a good idea to instead use memory-mapped regions instead of transfering everything
    fn update(&mut self) {
        // redefine values for convenience
        let cg_width = self.config_chargrid.size.0;
        let cg_height = self.config_chargrid.size.1;
        self.queue.write_texture(
            wgpu::ImageCopyTextureBase {
                texture: &self.char_grid_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.buffer_chars,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(cg_width),
                rows_per_image: Some(cg_height),
            },
            self.char_grid_size,
        );
        self.queue.write_texture(
            wgpu::ImageCopyTextureBase {
                texture: &self.fg_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.buffer_colors_fg,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(cg_width * 4),
                rows_per_image: Some(cg_height),
            },
            self.char_grid_size, //self.
        );
        self.queue.write_texture(
            wgpu::ImageCopyTextureBase {
                texture: &self.bg_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.buffer_colors_bg,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(cg_width * 4),
                rows_per_image: Some(cg_height),
            },
            self.char_grid_size,
        );
        self.queue.write_texture(
            wgpu::ImageCopyTextureBase {
                texture: &self.set_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.set_buffer, 
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(cg_width), 
                rows_per_image: Some(cg_height),
            },
            self.char_grid_size,
        );
        self.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instances),
        );
    }
    // this function renders everything to the screen
    fn draw(&mut self) -> Result<(), wgpu::SurfaceError> {
        // if *anything* is dirty, we update *everything*
        // this is inefficient and should be improved
        if self.dirty {
            self.update();
        }
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("text rendering command encoder"),
            });
        // clear render passs
        // it fills the screen with config's background color
        {
            // setups the render pass; for a color clear pass, we dont need anything else
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            // note how we turn it to floats (it was originally an u8 tuple)
                            r: self.background_color.0 as f64 / 255.,
                            g: self.background_color.1 as f64 / 255., 
                            b: self.background_color.2 as f64 / 255.,
                            a: self.background_color.3 as f64 / 255.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        // render text pass
        // this renders the  character grid
        {
            // setup the render pass
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("text render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // since this render pass needs buffers+a pipeline, we provide them
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            // then we draw, 0..6 vertices since we have two 3-vertices triangles
            // note the 0..1, since we don't use instances
            render_pass.draw(0..6, 0..1);
        }
        // render instances pass
        // this renders the instances / floating characters
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("instance render pass"),
                // copy pasted
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // sets the data
            render_pass.set_pipeline(&self.instance_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]); 
            render_pass.set_vertex_buffer(0, self.instance_vertices.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            // draws the triangles
            // note that we now use 0..self.instance_count instead of 0..1, since we now have an instance array
            render_pass.draw(0..6, 0..self.instance_count);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
    /// sets fg at a point
    pub fn set_fg_at<P>(&mut self, x: P, y: P, fg: Col)
    where
        P: TryInto<usize>,
    {
        if let Ok(x) = x.try_into() {
            if let Ok(y) = y.try_into() {
                if x < self.config_chargrid.size.0 as usize
                    && y < self.config_chargrid.size.1 as usize
                {
                    let k = [fg.0, fg.1, fg.2, fg.3];
                    let index = (x + y * self.config_chargrid.size.0 as usize) * 4;
                    let n = &mut self.buffer_colors_fg[index..index + 4];
                    if n != &k {
                        n.copy_from_slice(&k);
                        self.dirty = true;
                    }
                }
            }
        }
    }
    /// sets "set", which represents the font to use at a position
    pub fn set_set_at<P>(&mut self, x: P, y: P, value: u8)
    where
        P: TryInto<usize>,
    {
        let k = self.text_texture.size().depth_or_array_layers; //.fg_texture//set_texture
        assert!(
            value < k as u8,
            "cannot access set higher than the amounts we have registered"
        ); //be//fg
        if let Ok(x) = x.try_into() {
            if let Ok(y) = y.try_into() {
                if x < self.config_chargrid.size.0 as usize
                    && y < self.config_chargrid.size.1 as usize
                {
                    let index = x + y * self.config_chargrid.size.0 as usize;
                    let n = self.set_buffer[index]; //&mut//buffer_colors_fg//index..index + 4
                    if n != value {
                        self.set_buffer[index] = value;
                        self.dirty = true;
                    }
                }
            }
        }
    }
    /// sets bg at a point
    pub fn set_bg_at<P>(&mut self, x: P, y: P, bg: Col)
    where
        P: TryInto<usize>,
    {
        if let Ok(x) = x.try_into() {
            if let Ok(y) = y.try_into() {
                if x < self.config_chargrid.size.0 as usize
                    && y < self.config_chargrid.size.1 as usize
                {
                    let k = [bg.0, bg.1, bg.2, bg.3];
                    let index = (x + y * self.config_chargrid.size.0 as usize) * 4;
                    let n = &mut self.buffer_colors_bg[index..index + 4];
                    if n != &k {
                        n.copy_from_slice(&k);
                        self.dirty = true;
                    }
                }
            }
        }
    }
    /// sets the char in the grid at a point
    pub fn set_char_at<P>(&mut self, x: P, y: P, character: char)
    where
        P: TryInto<usize>,
    {
        if let Ok(x) = x.try_into() {
            if let Ok(y) = y.try_into() {
                if x < self.config_chargrid.size.0 as usize
                    && y < self.config_chargrid.size.1 as usize
                {
                    if let Some(char_u8) = codepage_437::CP437_WINGDINGS.encode(character) {
                        let index=x+y*self.config_chargrid.size.0 as usize/*()*/;
                        let n = self.buffer_chars[index];
                        if n != char_u8 {
                            self.buffer_chars[index] = char_u8;
                            self.dirty = true;
                        }
                    }
                }
            }
        }
    }
    /// does the same as set_char_at, but directly does it with an u8 instead of a character (it skips the cp437 conversion)
    /// see codepage_437::CP437_WINGDINGS
    pub fn set_char_at_bin<P>(&mut self, x: P, y: P, character: u8)
    where
        P: TryInto<usize>,
    {
        if let Ok(x) = x.try_into() {
            if let Ok(y) = y.try_into() {
                if x < self.config_chargrid.size.0 as usize
                    && y < self.config_chargrid.size.1 as usize
                {
                    let char_u8 = character;
                    let index=x+y*self.config_chargrid.size.0 as usize/*()*/;
                    let n = self.buffer_chars[index];
                    if n != char_u8 {
                        self.buffer_chars[index] = char_u8;
                        self.dirty = true;
                    }
                }
            }
        }
    }
    /// prints, will not change anything color-related for the fg if it is none, same for the bg
    /// will skip non-cp437 characters, but will still count them as an empty space
    /// is an extension on print_at_set
    pub fn print_at<P, Text>(&mut self, x: P, y: P, text: Text, fg: Option<Col>, bg: Option<Col>)
    where
        P: TryInto<usize>,
        Text: ToString,
    {
        self.print_at_set(x, y, text, fg, bg, None); //set
    }
    /// prints, will not change anything color-related for the fg if it is none, same for the bg, and also same for the set
    /// will skip non-cp437 characters, but will still count them as an empty space
    pub fn print_at_set<P, Text>(
        &mut self,
        x: P,
        y: P,
        text: Text,
        fg: Option<Col>,
        bg: Option<Col>,
        set: Option<u8>,
    ) where
        P: TryInto<usize>,
        Text: ToString,
    {
        let k = text.to_string();
        if let Ok(x) = x.try_into() {
            if let Ok(y) = y.try_into() {
                for i in k.chars().enumerate() {
                    let x = x + i.0;
                    let character = i.1;
                    if let Some(char_u8) = codepage_437::CP437_WINGDINGS.encode(character) {
                        if x < self.config_chargrid.size.0 as usize
                            && y < self.config_chargrid.size.1 as usize
                        {
                            let index=x+y*self.config_chargrid.size.0 as usize/*()*/;
                            let n = self.buffer_chars[index];
                            if n != char_u8 {
                                self.buffer_chars[index] = char_u8;
                                self.dirty = true;
                            }
                            if let Some(fg) = fg {
                                self.set_fg_at(x, y, fg);
                            }
                            if let Some(bg) = bg {
                                self.set_bg_at(x, y, bg);
                            }
                            if let Some(set) = set {
                                self.set_set_at(x, y, set);
                            }
                        }
                    }
                }
            }
        }
    }
    /// fills the fg & bg buffers with transparent black and the char buffer with glyph 0x00
    /// also sets instance count to 0
    pub fn clear(&mut self) {
        self.dirty = true;
        self.buffer_chars.fill(0);
        self.buffer_colors_bg.fill(0);
        self.buffer_colors_fg.fill(0);
        self.set_buffer.fill(0);
        self.instance_count = 0;
    }
    /// will return false if couldn't add the instance due to having exceeded the limit
    /// if it returned true, that means the instance was added, and dirty will be flagged (and we will resend everything!)
    pub fn add_instance(&mut self, mut instance: InstanceData) -> bool {
        let m = self.instance_count as usize;
        if m < self.instances.len() {
            self.instances[m] = instance;
            self.dirty = true;
            self.instance_count += 1;
            true
        } else {
            false
        }
    }
    /// panics if anything is out of bounds
    pub fn take_snapshot(&self, x: u32, y: u32, width: u32, height: u32) -> Snapshot {
        let w = self.config_chargrid.size.0;
        let h = self.config_chargrid.size.1;
        if x + width >= w || y + height >= h {
            panic!("out of bound! {}> {w} or {} > {h}", x + width, y + height);
        }
        let mut s = Snapshot {
            begin: (x, y),
            size: (width, height),
            fg: Vec::with_capacity((width * height * 4) as usize),
            bg: Vec::with_capacity((width * height * 4) as usize), //f
            set: Vec::with_capacity((width * height) as usize),
            text: Vec::with_capacity((width * height) as usize), //set
        };
        for x in x..x + width {
            for y in y..y + height {
                let idx = (x + y * w) as usize;
                s.fg.push(self.buffer_colors_fg[idx * 4]);
                s.fg.push(self.buffer_colors_fg[idx * 4 + 1]);
                s.fg.push(self.buffer_colors_fg[idx * 4 + 2]);
                s.fg.push(self.buffer_colors_fg[idx * 4 + 3]);
                s.bg.push(self.buffer_colors_bg[idx * 4]);
                s.bg.push(self.buffer_colors_bg[idx * 4 + 1]);
                s.bg.push(self.buffer_colors_bg[idx * 4 + 2]);
                s.bg.push(self.buffer_colors_bg[idx * 4 + 3]);
                s.set.push(self.set_buffer[idx]);
                s.text.push(self.buffer_chars[idx]);
            }
        }
        s
    }
    /// Write a snapshot at a point
    pub fn apply_snapshot(&mut self, snapshot: &Snapshot, x: i32, y: i32) {
        let bx = x;
        let by = y;
        for x in bx..bx + snapshot.size.0 as i32 {
            for y in by..by + snapshot.size.1 as i32 {
                if x - bx < snapshot.size.0 as i32 && y - by < snapshot.size.1 as i32 {
                    let idx = ((x - bx) * snapshot.size.1 as i32 + (y - by)) as usize;
                    self.set_char_at_bin(x, y, snapshot.text[idx]);
                    self.set_set_at(x, y, snapshot.set[idx]);
                    let fg = (
                        snapshot.fg[idx * 4],
                        snapshot.fg[idx * 4 + 1],
                        snapshot.fg[idx * 4 + 2],
                        snapshot.fg[idx * 4 + 3],
                    );
                    self.set_fg_at(x, y, fg);
                    let bg = (
                        snapshot.bg[idx * 4],
                        snapshot.bg[idx * 4 + 1],
                        snapshot.bg[idx * 4 + 2],
                        snapshot.bg[idx * 4 + 3],
                    );
                    self.set_bg_at(x, y, bg);
                }
            }
        }
    }
    // copy pasted from https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface/#state-new
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        #[cfg(target_arch = "wasm32")]
        {
            // resizing on wasm32 crashes; I should investigate that
            return;
        }
        // current resizing only stretches; it would be nice to keep the proportions constant
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_conf.width = new_size.width;
            self.surface_conf.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_conf);
        }
    }
    /// draws a rectangle of values on the grid
    pub fn draw_rect(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        filled: bool,
        fg: Option<Col>,
        bg: Option<Col>,
        ch: Option<char>,
        set: Option<u8>,
    ) {
        macro_rules! set {
            ($x:ident,$y:ident) => {
                let x = $x;
                let y = $y;
                if let Some(fg) = fg {
                    self.set_fg_at(x, y, fg);
                }
                if let Some(bg) = bg {
                    self.set_bg_at(x, y, bg);
                }
                if let Some(ch) = ch {
                    self.set_char_at(x, y, ch);
                }
                if let Some(set) = set {
                    self.set_set_at(x, y, set);
                }
            };
        }
        if filled {
            for x in x..x + width {
                for y in y..y + height {
                    set!(x, y);
                }
            }
        } else {
            for x in x..x + width {
                set!(x, y);
                let y = y + height - 1;
                set!(x, y);
            }
            for y in y..y + height {
                set!(x, y);
                let x = x + width - 1;
                set!(x, y);
            }
        }
    }
    /// does the same thing as draw_rect; is a WIP function
    pub fn draw_rect_ex(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        filled: bool,
        fg: Option<Col>,
        bg: Option<Col>,
        ch: Option<char>,
        set: Option<u8>,
    ) {
        macro_rules! set {
            ($x:ident,$y:ident) => {
                let x = $x;
                let y = $y;
                if let Some(fg) = fg {
                    self.set_fg_at(x, y, fg);
                }
                if let Some(bg) = bg {
                    self.set_bg_at(x, y, bg);
                }
                if let Some(ch) = ch {
                    self.set_char_at(x, y, ch);
                }
                if let Some(set) = set {
                    self.set_set_at(x, y, set);
                }
            };
        }
        if filled {
            for x in x..x + width {
                for y in y..y + height {
                    set!(x, y);
                }
            }
        } else {
            for x in x..x + width {
                set!(x, y);
                let y = y + height - 1;
                set!(x, y);
            }
            for y in y..y + height {
                set!(x, y);
                let x = x + width - 1;
                set!(x, y);
            }
        }
    }
}
/// The color type used by this crate
pub type Col = (u8, u8, u8, u8);

/// The Config type of this crates which describes how everything functions
pub struct Config {
    /// the size (in characters) of the app
    pub size: (u32, u32),
    /// padding (in pixels) of the character grid (the padding will be background_color's color)
    /// the left-padding will be padding.0/2, etc
    pub padding: (u32, u32),
    /// the name of the window
    pub name: String,
    /// if dpi = true, then the size of the window will account for dpi factors
    /// this currently does nothing :3
    pub dpi: bool,
    /// the fonts to use
    pub font: Vec<Font>,
    /// the background color
    pub background_color: (u8, u8, u8, u8),
    /// multiplies the size of the window & all of it's pixels
    pub scale: (u32, u32),
    /// maximum amount of instances that can be drawn
    /// the higher this number, the higher the toll on the gpu (altough this should'nt become a problem until a lot of instances)
    /// Default: 128
    pub max_instances: u32, //.
    /// if we should look for srgb color space
    pub srgb: bool,
}
// this is the implementation of the game loop
impl<'a, T> ApplicationHandler for EventLoopWrapper<T/* <- that T is the game type, provided by the library's user*/>
where
    T: Yarl2Game,
{
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // I could handle this later
    }
    // this is the main function that handles the event loop

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        // we first send the event to the game itself, so that it may *do things* if it wants to (and if it thinks my implementation sucks)
        self.game.event(&event, &mut self.window);
        // we match the event
        match event {
            winit::event::WindowEvent::Resized(new_size) => {
                self.window.resize(new_size);
            }
            // this event fires when the user requests the window to close, I think
            winit::event::WindowEvent::CloseRequested => {
                // we first call the close function,
                self.game.close();
                // then exit the loop
                event_loop.exit();
            }
            winit::event::WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if button == MouseButton::Left {
                    self.keyboard.mouse_pressed = state == ElementState::Pressed;
                }
            }
            
            winit::event::WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                // note: this currently breaks on re-size and I should fix that
                // it's also broken on wasm32
                let x = position.x - self.window.config_chargrid.padding.0 as f64 / 2.;
                let y = position.y - self.window.config_chargrid.padding.1 as f64 / 2.;
                let x = x / self.window.config_chargrid.scale.0 as f64;
                let y = y / self.window.config_chargrid.scale.1 as f64;
                let x = x / self.window.char_width as f64;
                let y = y / self.window.char_height as f64;
                let pos = (x.floor() as i32, y.floor() as i32);
                self.keyboard.mouse_position = pos;
            }
            // when we must render the window
            winit::event::WindowEvent::RedrawRequested => {
                // we first call pre_draw, which should be the main update function the user utilizes
                self.game.pre_draw(&mut self.window, &mut self.keyboard);
                // we draw
                let _ = self.window.draw();
                // we call post_draw (it's mainly intended for time measuring)
                self.game.post_draw();
                // we test if we should exit
                if self.game.should_exit() {
                    // we call close
                    self.game.close();
                    // we exit the event loop
                    event_loop.exit();
                    return;
                }
                // we request redraw again, so that we have a true loop
                self.window.window.request_redraw();
            }
            // handles keyboard input (terribly, I should improve that system)
            winit::event::WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if !event.repeat {
                    if let Some(m) = event.text {
                        let data = m.chars().next().unwrap();
                        if event.state.is_pressed() {
                            self.game.text_input(data, &mut self.window); 
                            self.keyboard.letters.insert(data);
                        } else {
                            self.keyboard.letters.remove(&data);
                        }
                    }
                    if event.state == winit::event::ElementState::Pressed {
                        self.keyboard.keys.insert(event.physical_key); 
                    } else {

                        self.keyboard.keys.remove(&event.physical_key);
                    }
                } else {
                    if let Some(m) = event.text {
                        let data = m.chars().next().unwrap();
                        if event.state.is_pressed() {
                            self.game.text_input(data, &mut self.window); 
                        } else {
                         
                        }
                     
                    }
                }
            }
            _d_o_n_u_t_ => {
                // I used to debug print, but I no longer do so, since printing breaks wasm32 with my current setup
            }
        }
    }
}
/// This is the trait that your game struct should implement if you want to work with my library
/// It's used mostly as a wrapper around winit's ApplicationHandler that provides control over the yarl-2 window & input
pub trait Yarl2Game {
    /// called before drawing
    fn pre_draw(&mut self, window: &mut Window<'static>, keyboard: &NiceKeyboard); 
    // called after drawing, before calling `should_exit`
    fn post_draw(&mut self) {}
    /// called after pre_draw (after the draw), closes the window if true
    fn should_exit(&mut self) -> bool {
        false
    }
    /// is called before the window is shut down
    /// will be called if should_exit returns true
    fn close(&mut self) {}
    #[allow(unused)]
    /// is called when a character is pressed (useful if you want to read text input)
    /// will trigger from repetition
    fn text_input(&mut self, character: char, window: &mut Window) {}
    /// is called for all events
    fn event(&mut self, _event: &le_winit::event::WindowEvent, _window: &mut Window) {
    }
}
// this is a default implementation that you can run using `cargo run` that mostly showcases the library and the very broken UI system
impl Yarl2Game for () {
    fn pre_draw(&mut self, window: &mut Window<'static>, keyboard: &NiceKeyboard) 
    {
        window.clear();
        window.add_instance(InstanceData::new(
            'I',
            WHITE,
            BLACK,
            [
                keyboard.mouse_position.0 as f32,
                keyboard.mouse_position.1 as f32,
            ],
            0,
        )); 
        for x in 0..40 {
            window.print_at(
                0,
                1 + x,
                "hello world! i like trains and they dance very well å ç ▓▒░ █☺☻",
                Some((0, 0, 0, 255)),
                Some((255, 0, 0, 255)),
            ); //None
        }
        // this is UI stuff; it uses the system in `ui.rs` which is terrible and should never be used
        let data = UIData::default();
        let mut i = ui::ui_context((2, 2), (18, 18), data);
        i.add(
            UIBox {
                fill_style: FillStyle {
                    background_color: Some(RED),
                    fill_char: Some(' '),
                    ..Default::default()
                },
                placement_style: ui::BoxPlacementStyle::AlignY { height: 5 },
                ..Default::default()
            },
            |mut d| {
                d.add(
                    UIBox {
                        fill_style: FillStyle {
                            background_color: Some(YELLOW),
                            fill_char: Some('#'),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    |mut d| {
                        d.add(
                            Label {
                                foreground_color: Some(BLACK),
                                background_color: None,
                                text: "hello from b!".into(),
                            },
                            |d| d,
                        );
                        d
                    },
                );
                d.add(
                    UIBox {
                        fill_style: FillStyle {
                            background_color: Some(CYAN),
                            fill_char: Some('?'),
                            border: BorderStyle {
                                char: Some('!'),
                                ..BorderStyle::empty()
                            },
                            ..Default::default()
                        },
                        placement_style: ui::BoxPlacementStyle::Within { padding: 1 },
                        ..Default::default()
                    },
                    |mut d| {
                        d.add(
                            UIBox {
                                fill_style: FillStyle {
                                    background_color: Some(GREEN),
                                    fill_char: Some('/'),
                                    ..Default::default()
                                },
                                placement_style: ui::BoxPlacementStyle::AlignY { height: 1 },
                                ..Default::default()
                            },
                            |mut d| {
                                d.add(
                                    Label {
                                        foreground_color: Some(BLACK),
                                        background_color: None,
                                        text: "hello from a!".into(), 
                                    },
                                    |d| d,
                                );
                                d.add(
                                    Button {
                                        foreground_color: Some(BLACK),
                                        background_color: Some(YELLOW), 
                                        text: "press me!".into(),  
                                        id: "button".to_owned(),
                                        ..Default::default()
                                    },
                                    |d| d,
                                );
                                d
                            },
                        );

                        d
                    },
                );
                d.add(
                    Label {
                        foreground_color: Some(BLACK),
                        background_color: Some(WHITE),
                        text: "hello from c!".into(),
                    },
                    |d| d,
                );
                d
            },
        );
        // this makes the ui actually do things
        i.render_and_process(window, keyboard);
        let data = i.retrieve_data(); 
        if let Some(n) = data.data.get("button") {
            if let UIDataEntry::Boolean(value) = n {
                if *value {
                    println!("button down!");
                }
            }
        }
        window.set_char_at(keyboard.mouse_position.0, keyboard.mouse_position.1, ':');
        window.set_set_at(keyboard.mouse_position.0, keyboard.mouse_position.1, 1);

    }
}
// this is the wrapper which implements winit's ApplicationHandler
struct EventLoopWrapper<T: Yarl2Game> {
    game: T,
    window: Window<'static>,
    keyboard: NiceKeyboard,
}
/// Provides input access to the user without the event() function
pub struct NiceKeyboard {
    /// The key is in the hashset if the key is pressed
    pub keys: HashSet<WinitKey>,
    /// The letters that are pressed (matches the keyboard's layout)
    pub letters: HashSet<char>,
    /// The mouse's position (.0 = x .1 = y like in the rest of this lib)
    pub mouse_position: (i32, i32),
    /// TODO: implement another button than mouse left
    pub mouse_pressed: bool,
}

pub type WinitKey = PhysicalKey;
// The default config, uses the font that DEFAULT_FONT_LICENSE refers to
impl Default for Config {
    fn default() -> Self {
        Config {
            size: (64, 64),
            name: String::from_str("yarl-2 window").unwrap(),
            dpi: false,
            font: vec![
                Font::default(),
                Font::Binary(include_bytes!("../comic_sans_mono_cp437.png")),
            ],
            background_color: (255, 255, 255, 255), //Vec
            padding: (8 * 2, 8 * 2),
            scale: (2 / 2, 2 / 2),
            max_instances: 128,
            srgb: true,
        } 
    }
}
/// Runs the game
pub fn run_game<T>(game: T, config: Config) -> !
where
    T: Yarl2Game,
{
    Window::new_run(game, config)
}
/// if you need to provide this license in any way, here it is
pub const DEFAULT_FONT_LICENSE: &str = include_str!("../font_license.txt");
pub use winit::keyboard::PhysicalKey as TheKeyTypeFromWinit;
//pub use winit::keyboard::PhysicalKey;
pub use winit::keyboard::KeyCode as TheKeyCodeTypeFromWinit;
/// an instance for instanced rendering of chars, unaligned to the grid (floating characters!)
/// recommended to be used with the provided constructor
#[repr(C)]
#[derive(Clone, Copy)]
pub struct InstanceData {
    /// represents the top left position of the char
    /// one represents one character on the grid
    pub position: [f32; 2],
    /// first byte represents the set and the second the char
    pub set_char: [u8; 2],
    /// these two are obvious
    pub fg: Col,
    /// aren't they?
    pub bg: Col,
}
impl InstanceData {
    /// creates the instance, calls encode by itself instead of you having to call it manually when entering each field by yourself in InstanceData {...} format
    pub fn new(ch: char, fg: Col, bg: Col, position: [f32; 2], set: u8) -> Self {
        Self {
            set_char: [set, codepage_437::CP437_WINGDINGS.encode(ch).unwrap()],
            fg,
            bg,
            position, //aracter
        }
    }
}
/// The layout of the instances, copy-pasted again from the wgpu tutorial
const INSTANCE_LAYOUT: wgpu::VertexBufferLayout =
    /* more copy-pasting :3 */ /* moare copy-pasting :> */
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress, // 1.
        step_mode: wgpu::VertexStepMode::Instance,                          // 2.
        attributes: &[
            // 3.//Vertex
            wgpu::VertexAttribute {
                offset: 0,                             // 4.
                shader_location: 2,                    // 5.
                format: wgpu::VertexFormat::Float32x2, // 6.
            }, //0
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress, //3//2
                shader_location: 3,
                format: wgpu::VertexFormat::Uint8x2, //3
            }, //1
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress
                    + std::mem::size_of::<[u8; 2]>() as wgpu::BufferAddress, //3//2
                shader_location: 4,
                format: wgpu::VertexFormat::Unorm8x4, //3
            }, //2
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress
                    + std::mem::size_of::<[u8; 2 + 4]>() as wgpu::BufferAddress, //3//2
                shader_location: 2 + 1 + 2,           // aka 5
                format: wgpu::VertexFormat::Unorm8x4, //3
            },
        ],
    };
unsafe impl bytemuck::Pod for InstanceData {}
unsafe impl bytemuck::Zeroable for InstanceData {}
#[derive(Clone)]
/// Represents a snapshot taken from screen memory, which can then be drawn
pub struct Snapshot {
    pub begin: (u32, u32),
    pub size: (u32, u32),
    pub fg: Vec<u8>,
    pub bg: Vec<u8>,
    pub set: Vec<u8>,
    pub text: Vec<u8>, //ch
}
/// Can be used to pretty-print text fragments with different colors, and also implements semi-working text wrapping
pub struct TextBuilder {
    pub segments: Vec<TextSegment>,
    fg: Col,
    bg: Col,
    set: u8,
}
impl TextBuilder {
    /// returns the len
    pub fn len(&self) -> usize {
        let mut accumulator = 0;
        for i in &self.segments {
            accumulator += i.text.len();
        }
        accumulator
    }
    /// creates it
    pub fn create() -> Self {
        Self {
            segments: Vec::new(),
            fg: WHITE,
            bg: BLACK,
            set: 0,
        }
    }
    /// pushes text onto it
    pub fn text<T>(mut self, text: T) -> Self
    where
        T: ToString,
    {
        self.segments.push(TextSegment {
            text: text.to_string(),
            fg: self.fg,
            bg: self.bg,
            set: self.set,
        });
        self
    }
    /// sets the fg that will be used in the next call to text()
    pub fn fg(mut self, fg: Col) -> Self {
        self.fg = fg;
        self
    }
    /// sets the bg that will be used in the next call to text()

    pub fn bg(mut self, bg: Col) -> Self {
        self.bg = bg;
        self
    }
    /// sets the set/font that will be used in the next call to text()

    pub fn set(mut self, set: u8) -> Self {
        self.set = set;
        self
    }
    /// prints with a darkening factor (col_sub)
    /// is a wrapper on print_sub_cutoff
    pub fn print_sub(
        &self,
        window: &mut Window,
        pos: (i32, i32),
        width_end: i32,
        col_sub: Col,
        return_x: i32,
    ) -> (i32, i32) {
        self.print_sub_cutoff(window, pos, width_end, col_sub, return_x, None)
    }
    /// prints with a darkening faction and a cutoff point on the y axis
    pub fn print_sub_cutoff(
        &self,
        window: &mut Window,
        pos: (i32, i32),
        width_end: i32,
        col_sub: Col,
        return_x: i32,
        cutoff_y: Option<i32>,
    ) -> (i32, i32) {
        fn sub(a: Col, b: Col) -> Col {
            (
                a.0.saturating_sub(b.0),
                a.1.saturating_sub(b.1),
                a.2.saturating_sub(b.2),
                255,
            )
        }
        let mut x = pos.0;
        let mut y = pos.1;
        'b: for seg in &self.segments {
            for ch in seg.text.chars() {
                if let Some(limit) = cutoff_y {
                    if y >= limit {
                        break 'b;
                    }
                }
                let fg = sub(seg.fg, col_sub);
                let bg = sub(seg.bg, col_sub);
                let set = seg.set;
                window.set_bg_at(x, y, bg);
                window.set_char_at(x, y, ch); //aracter
                window.set_fg_at(x, y, fg);
                window.set_set_at(x, y, set);

                if x > width_end {
                    x = return_x; //pos.0;
                    y += 1;
                } else {
                    x += 1;
                }
                // else{

                // }
            }
        }
        (x, y)
    }
    /// prints with wrapping
    /// is a wrapper on print_sub
    /// the text begins at pos and will return to return_x when it bypasses width_end on the x axis
    pub fn print(
        &self,
        window: &mut Window,
        pos: (i32, i32),
        width_end: i32,
        return_x: i32,
    ) -> (i32, i32) {
        self.print_sub(window, pos, width_end, TRANSPARENT, return_x) 
    }
}
/// A text segment used by TextBuilder
pub struct TextSegment {
    pub text: String,
    pub fg: Col,
    pub bg: Col,
    pub set: u8,
}
// Public use so that one may properly receive events
pub use winit as le_winit;
/// Converts a char to its cp437 u8 representation
/// Will panic if the char is not cp437
pub fn ch_to_u8(ch: char) -> u8 {
    codepage_437::CP437_WINGDINGS.encode(ch).unwrap()
}
/// Convers an u8 to a char via cp437
pub fn u8_to_ch(u: u8) -> char {
    codepage_437::CP437_WINGDINGS.decode(u)
}
