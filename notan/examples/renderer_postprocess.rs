use notan::app::assets::*;
use notan::app::config::WindowConfig;
use notan::app::graphics::prelude::*;
use notan::app::{App, AppBuilder, Graphics, Plugins};
use notan::log;
use notan::prelude::*;

struct State {
    cube: Cube,
    post_process: PostProcessTarget,
}

impl AppState for State {}

#[notan::main]
fn main() -> Result<(), String> {
    notan::init_with(setup).draw(draw).build();

    Ok(())
}

fn setup(app: &mut App, gfx: &mut Graphics) -> State {
    State {
        cube: Cube::new(gfx),
        post_process: PostProcessTarget::new(gfx, 800, 600),
    }
}

fn draw(gfx: &mut Graphics, state: &mut State) {
    let cube_renderer = state.cube.create_renderer(gfx);
    gfx.render_to(&state.post_process.render_texture, &cube_renderer);

    let post_process_renderer = state.post_process.create_renderer(gfx);
    gfx.render(&post_process_renderer);
}

// - struct that represents a texture to use as a postprocess target

//language=glsl
const IMAGE_VERTEX: ShaderSource = notan::vertex_shader! {
    r#"
    #version 450

    layout(location = 0) in vec4 a_position;
    layout(location = 1) in vec2 a_texcoord;

    layout(location = 0) out vec2 v_texcoord;

    void main() {
        v_texcoord = a_texcoord;
        gl_Position = a_position;
    }
    "#
};

//language=glsl
const PIXEL_INVERT_FRAGMENT: ShaderSource = notan::fragment_shader! {
    r#"
    #version 450
    precision mediump float;

    layout(location = 0) out vec4 outColor;
    layout(location = 0) in vec2 v_texcoord;

    layout(set = 0, binding = 0) uniform sampler2D u_texture;
    layout(set = 0, binding = 0) uniform Locals {
        vec2 u_tex_size;
        float u_value;
    };

    void main() {
        vec2 size = vec2(u_value, u_value);
        vec2 coord = fract(v_texcoord) * u_tex_size;
        coord = floor(coord/size) * size;
        vec4 tex_color = texture(u_texture, coord / u_tex_size);

        float red = tex_color.r + ((1.0 - tex_color.r) * abs(sin(u_value)));
        float green = tex_color.g + ((1.0 - tex_color.g) * abs(sin(u_value)));
        float blue = tex_color.b + ((1.0 - tex_color.b) * abs(sin(u_value)));
        outColor = vec4(red, green, blue, tex_color.a);
    }
    "#
};

struct PostProcessTarget {
    render_texture: RenderTexture,
    pipeline: Pipeline,
    vertex_buffer: Buffer<f32>,
    index_buffer: Buffer<u32>,
    uniform_buffer: Buffer<f32>,
    value: f32,
}

impl PostProcessTarget {
    fn new(gfx: &mut Graphics, width: i32, height: i32) -> Self {
        let render_texture = gfx
            .create_render_texture(TextureInfo::render_texture(true, width, height))
            .unwrap();
        let pipeline = gfx
            .create_pipeline(
                &IMAGE_VERTEX,
                &PIXEL_INVERT_FRAGMENT,
                &[
                    VertexAttr::new(0, VertexFormat::Float3),
                    VertexAttr::new(1, VertexFormat::Float2),
                ],
                PipelineOptions {
                    color_blend: Some(BlendMode::NORMAL),
                    ..Default::default()
                },
            )
            .unwrap();

        #[rustfmt::skip]
        let vertices = vec![
            //pos               //coords
            1.0,  1.0, 0.0,     1.0, 1.0,
            1.0, -1.0, 0.0,     1.0, 0.0,
            -1.0, -1.0, 0.0,    0.0, 0.0,
            -1.0, 1.0, 0.0,    0.0, 1.0
        ];

        #[rustfmt::skip]
        let indices = vec![
            0, 1, 3,
            1, 2, 3,
        ];

        let uniforms = vec![800.0, 600.0, 0.0];

        let vertex_buffer = gfx.create_vertex_buffer(vertices).unwrap();
        let index_buffer = gfx.create_index_buffer(indices).unwrap();
        let uniform_buffer = gfx.create_uniform_buffer(0, uniforms).unwrap();

        Self {
            render_texture,
            pipeline,
            value: 0.0,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
        }
    }

    fn create_renderer(&mut self, gfx: &mut Graphics) -> Renderer {
        (*self.uniform_buffer.data_mut())[2] = 5.5 + self.value.sin();
        self.value += 0.005;

        let mut renderer = gfx.create_renderer();

        renderer.begin(Some(&ClearOptions::none()));
        renderer.set_pipeline(&self.pipeline);
        renderer.bind_texture(0, &self.render_texture);
        renderer.bind_vertex_buffer(&self.vertex_buffer);
        renderer.bind_index_buffer(&self.index_buffer);
        renderer.bind_uniform_buffer(&self.uniform_buffer);
        renderer.draw(0, 6);
        renderer.end();

        renderer
    }
}

// - struct that represents a Cube with the pipeline and the draw method

//language=glsl
const COLOR_VERTEX: ShaderSource = notan::vertex_shader! {
    r#"
    #version 450
    layout(location = 0) in vec4 a_position;
    layout(location = 1) in vec4 a_color;

    layout(location = 0) out vec4 v_color;

    layout(set = 0, binding = 0) uniform Locals {
        mat4 u_matrix;
    };

    void main() {
        v_color = a_color;
        gl_Position = u_matrix * a_position;
    }
    "#
};

//language=glsl
const COLOR_FRAGMENT: ShaderSource = notan::fragment_shader! {
    r#"
    #version 450
    precision mediump float;

    layout(location = 0) in vec4 v_color;
    layout(location = 0) out vec4 color;

    void main() {
        color = v_color;
    }
    "#
};

struct Cube {
    pipeline: Pipeline,
    vertex_buffer: Buffer<f32>,
    index_buffer: Buffer<u32>,
    uniform_buffer: Buffer<f32>,
    mvp: glam::Mat4,
    angle: f32,
}

impl Cube {
    fn new(gfx: &mut Graphics) -> Self {
        let pipeline = gfx
            .create_pipeline(
                &COLOR_VERTEX,
                &COLOR_FRAGMENT,
                &[
                    VertexAttr::new(0, VertexFormat::Float3),
                    VertexAttr::new(1, VertexFormat::Float4),
                ],
                PipelineOptions {
                    depth_stencil: DepthStencil {
                        write: true,
                        compare: CompareMode::Less,
                    },
                    ..Default::default()
                },
            )
            .unwrap();

        #[rustfmt::skip]
        let vertices = vec![
            -1.0, -1.0, -1.0,   1.0, 0.0, 0.0, 1.0,
            1.0, -1.0, -1.0,   1.0, 0.0, 0.0, 1.0,
            1.0,  1.0, -1.0,   1.0, 0.0, 0.0, 1.0,
            -1.0,  1.0, -1.0,   1.0, 0.0, 0.0, 1.0,

            -1.0, -1.0,  1.0,   0.0, 1.0, 0.0, 1.0,
            1.0, -1.0,  1.0,   0.0, 1.0, 0.0, 1.0,
            1.0,  1.0,  1.0,   0.0, 1.0, 0.0, 1.0,
            -1.0,  1.0,  1.0,   0.0, 1.0, 0.0, 1.0,

            -1.0, -1.0, -1.0,   0.0, 0.0, 1.0, 1.0,
            -1.0,  1.0, -1.0,   0.0, 0.0, 1.0, 1.0,
            -1.0,  1.0,  1.0,   0.0, 0.0, 1.0, 1.0,
            -1.0, -1.0,  1.0,   0.0, 0.0, 1.0, 1.0,

            1.0, -1.0, -1.0,    1.0, 0.5, 0.0, 1.0,
            1.0,  1.0, -1.0,    1.0, 0.5, 0.0, 1.0,
            1.0,  1.0,  1.0,    1.0, 0.5, 0.0, 1.0,
            1.0, -1.0,  1.0,    1.0, 0.5, 0.0, 1.0,

            -1.0, -1.0, -1.0,   0.0, 0.5, 1.0, 1.0,
            -1.0, -1.0,  1.0,   0.0, 0.5, 1.0, 1.0,
            1.0, -1.0,  1.0,   0.0, 0.5, 1.0, 1.0,
            1.0, -1.0, -1.0,   0.0, 0.5, 1.0, 1.0,

            -1.0,  1.0, -1.0,   1.0, 0.0, 0.5, 1.0,
            -1.0,  1.0,  1.0,   1.0, 0.0, 0.5, 1.0,
            1.0,  1.0,  1.0,   1.0, 0.0, 0.5, 1.0,
            1.0,  1.0, -1.0,   1.0, 0.0, 0.5, 1.0,
        ];

        #[rustfmt::skip]
        let indices = vec![
            0, 1, 2,  0, 2, 3,
            6, 5, 4,  7, 6, 4,
            8, 9, 10,  8, 10, 11,
            14, 13, 12,  15, 14, 12,
            16, 17, 18,  16, 18, 19,
            22, 21, 20,  23, 22, 20
        ];

        let projection = glam::Mat4::perspective_rh_gl(45.0, 4.0 / 3.0, 0.1, 100.0);
        let view = glam::Mat4::look_at_rh(
            glam::Vec3::new(4.0, 3.0, 3.0),
            glam::Vec3::new(0.0, 0.0, 0.0),
            glam::Vec3::new(0.0, 1.0, 0.0),
        );
        let mvp = glam::Mat4::identity() * projection * view;

        let vertex_buffer = gfx.create_vertex_buffer(vertices).unwrap();
        let index_buffer = gfx.create_index_buffer(indices).unwrap();
        let uniform_buffer = gfx
            .create_uniform_buffer(0, mvp.to_cols_array().to_vec())
            .unwrap();

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            mvp,
            angle: 0.0,
        }
    }

    fn create_renderer(&mut self, gfx: &mut Graphics) -> Renderer {
        self.uniform_buffer
            .data_mut()
            .copy_from_slice(&rotated_matrix(self.mvp, self.angle));

        let mut renderer = gfx.create_renderer();
        renderer.begin(Some(&ClearOptions {
            color: Some(Color::new(0.1, 0.2, 0.3, 1.0)),
            depth: Some(1.0),
            ..Default::default()
        }));

        renderer.set_pipeline(&self.pipeline);
        renderer.bind_uniform_buffer(&self.uniform_buffer);
        renderer.bind_vertex_buffer(&self.vertex_buffer);
        renderer.bind_index_buffer(&self.index_buffer);
        renderer.draw(0, 36);
        renderer.end();

        self.angle += 0.01;

        renderer
    }
}
fn rotated_matrix(base: glam::Mat4, angle: f32) -> [f32; 16] {
    let rot_x = glam::Mat4::from_rotation_x(angle);
    let rot_y = glam::Mat4::from_rotation_y(angle);
    (base * rot_x * rot_y).to_cols_array()
}