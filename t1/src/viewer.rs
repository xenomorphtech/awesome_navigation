use eframe::egui::{self, Color32, ViewportBuilder};
use egui::{Pos2, Vec2};
use egui_gizmo::GizmoMode;
use glam::{Mat4, Vec3, Vec4};
use std::path::PathBuf;

// Import the debug draw implementation and obj loader
use crate::debug_draw_b::*;
use crate::obj_loader::{self, ObjData};

struct EguiDebugDraw {
    lines: Vec<(Vec3, Vec3, Color32)>,
    points: Vec<(Vec3, Color32)>,
    tris: Vec<(Vec3, Vec3, Vec3, Color32, Vec2, Vec2, Vec2)>,
    current_mode: i32,
    texture_enabled: bool,
    vertex_count: usize,
}

impl EguiDebugDraw {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            points: Vec::new(),
            tris: Vec::new(),  
            current_mode: 0,
            texture_enabled: false,
            vertex_count: 0,
        }
    }

    fn clear(&mut self) {
        self.lines.clear();
        self.points.clear();
        self.tris.clear();
        self.vertex_count = 0;
    }
}

impl DebugDraw for EguiDebugDraw {
    fn begin(&mut self, prim: i32, _size: f32) {
        self.current_mode = prim;
        self.vertex_count = 0;
    }

    fn vertex(&mut self, pos: Vec3, color: Vec4) {
        self.vertex_uv(pos, color, Vec2::new(0.0, 0.0));
    }

    fn vertex_uv(&mut self, pos: Vec3, color: Vec4, uv: Vec2) {
        let col = Color32::from_rgba_premultiplied(
            (color.x * 255.0) as u8,
            (color.y * 255.0) as u8,
            (color.z * 255.0) as u8,
            (color.w * 255.0) as u8,
        );

        if self.current_mode == DU_DRAW_TRIS {
            if self.vertex_count % 3 == 0 {
                self.tris.push((pos, pos, pos, col, uv, uv, uv));
            } else {
                let tri = self.tris.last_mut().unwrap();
                match self.vertex_count % 3 {
                    1 => { tri.1 = pos; tri.5 = uv; },
                    2 => { tri.2 = pos; tri.6 = uv; },
                    _ => {}
                }
            }
            self.vertex_count += 1;
        }
    }

    fn texture(&mut self, state: bool) {
        self.texture_enabled = state;
    }

    fn end(&mut self) {}
}

pub struct MeshViewerApp {
    mesh: InputMesh,
    debug_draw: EguiDebugDraw,
    camera: Camera,
    walkable_slope_angle: f32,
    obj_path: Option<PathBuf>,
}

struct Camera {
    position: Vec3,
    target: Vec3,
    up: Vec3,
    fov: f32,
    aspect: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            position: Vec3::new(5.0, 5.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: 60.0_f32.to_radians(),
            aspect: 1.0,
        }
    }

    fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, 0.1, 100.0)
    }
}

fn obj_to_input_mesh(obj: &ObjData) -> InputMesh {
    let mut mesh = InputMesh::new();
    
    // Convert vertices
    mesh.verts = obj.vertices.iter().skip(1).map(|v| {
        Vec3::new(v.x, v.y, v.z)
    }).collect();

    // Triangulate faces and add indices
    let triangles = obj.triangulate();
    mesh.tris = triangles.iter().flat_map(|tri| {
        // Adjust indices to be 0-based
        vec![tri[0] - 1, tri[1] - 1, tri[2] - 1].into_iter()
    }).map(|i| i as i32).collect();

    // Calculate normals for each vertex in each triangle
    mesh.normals = Vec::new();
    for chunk in mesh.tris.chunks(3) {
        if chunk.len() == 3 {
            let v0 = mesh.verts[chunk[0] as usize];
            let v1 = mesh.verts[chunk[1] as usize];
            let v2 = mesh.verts[chunk[2] as usize];
            let normal = (v1 - v0).cross(v2 - v0).normalize();
            // Add the same normal for all three vertices of this triangle
            mesh.normals.push(normal);
            mesh.normals.push(normal);
            mesh.normals.push(normal);
        }
    }

    mesh
}

impl MeshViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut mesh = InputMesh::new();
        
        // Add default simple mesh if no OBJ is loaded
        mesh.verts = vec![
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new( 1.0, 0.0, -1.0),
            Vec3::new( 1.0, 1.0,  1.0),
            Vec3::new(-1.0, 1.0,  1.0),
        ];

        mesh.tris = vec![0, 1, 2, 0, 2, 3];

        let normal1 = (mesh.verts[1] - mesh.verts[0])
            .cross(mesh.verts[2] - mesh.verts[0])
            .normalize();
        let normal2 = (mesh.verts[2] - mesh.verts[0])
            .cross(mesh.verts[3] - mesh.verts[0])
            .normalize();

        mesh.normals = vec![normal1, normal1, normal1, normal2, normal2, normal2];

        Self {
            mesh,
            debug_draw: EguiDebugDraw::new(),
            camera: Camera::new(),
            walkable_slope_angle: 45.0,
            obj_path: None,
        }
    }

    fn load_obj(&mut self, path: PathBuf) {
        if let Ok(obj_data) = obj_loader::load_obj(&path) {
            self.mesh = obj_to_input_mesh(&obj_data);
            self.obj_path = Some(path);

            // Adjust camera to fit the model
            let (min, max) = obj_data.get_bounds();
            let center = Vec3::new(
                (min.x + max.x) * 0.5,
                (min.y + max.y) * 0.5,
                (min.z + max.z) * 0.5,
            );
            let size = Vec3::new(
                max.x - min.x,
                max.y - min.y,
                max.z - min.z,
            );
            let max_size = size.x.max(size.y).max(size.z);
            
            self.camera.target = center;
            self.camera.position = center + Vec3::new(max_size * 2.0, max_size * 2.0, max_size * 2.0);
        }
    }

    fn draw_mesh(&mut self) {
        self.debug_draw.clear();
        du_debug_draw_tri_mesh_slope(
            &mut self.debug_draw,
            &self.mesh,
            self.walkable_slope_angle,
            1.0
        );
    }
}

fn mat4_to_array(mat: Mat4) -> [[f32; 4]; 4] {
    let cols = mat.to_cols_array_2d();
    [
        [cols[0][0], cols[0][1], cols[0][2], cols[0][3]],
        [cols[1][0], cols[1][1], cols[1][2], cols[1][3]],
        [cols[2][0], cols[2][1], cols[2][2], cols[2][3]],
        [cols[3][0], cols[3][1], cols[3][2], cols[3][3]],
    ]
}

impl eframe::App for MeshViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Add file picker
            ui.horizontal(|ui| {
                if ui.button("Load OBJ").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("OBJ files", &["obj"])
                        .pick_file() 
                    {
                        self.load_obj(path);
                    }
                }

                if let Some(path) = &self.obj_path {
                    ui.label(format!("Loaded: {}", path.display()));
                }

                ui.separator();

                ui.label("Walkable Slope Angle:");
                ui.add(egui::Slider::new(&mut self.walkable_slope_angle, 0.0..=90.0));
            });

            let mut gizmo = egui_gizmo::Gizmo::new("camera_gizmo")
                .view_matrix(mat4_to_array(self.camera.view_matrix()).into())
                .projection_matrix(mat4_to_array(self.camera.projection_matrix()).into())
                .mode(GizmoMode::Rotate);

            self.draw_mesh();

            let (rect, _response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());

            self.camera.aspect = rect.width() / rect.height();

            if ui.input(|i| i.key_pressed(egui::Key::R)) {
                self.camera = Camera::new();
            }

            if let Some(response) = gizmo.interact(ui) {
                let rotation = glam::Quat::from_xyzw(
                    response.rotation.v.x,
                    response.rotation.v.y,
                    response.rotation.v.z,
                    response.rotation.s,
                );
                self.camera.position = rotation.mul_vec3(self.camera.position);
            }

            let painter = ui.painter();

            for tri in &self.debug_draw.tris {
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        pos_to_screen(tri.0, &self.camera, rect),
                        pos_to_screen(tri.1, &self.camera, rect),
                        pos_to_screen(tri.2, &self.camera, rect),
                    ],
                    tri.3,
                    (1.0, tri.3),
                ));
            }
        });
    }
}

fn pos_to_screen(pos: Vec3, camera: &Camera, rect: egui::Rect) -> Pos2 {
    let view_proj = camera.projection_matrix() * camera.view_matrix();
    let clip_pos = view_proj.project_point3(pos);
    Pos2::new(
        (clip_pos.x * 0.5 + 0.5) * rect.width() + rect.min.x,
        (1.0 - (clip_pos.y * 0.5 + 0.5)) * rect.height() + rect.min.y,
    )
}

pub fn run() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Mesh Viewer",
        options,
        Box::new(|cc| Box::new(MeshViewerApp::new(cc))),
    )
}
