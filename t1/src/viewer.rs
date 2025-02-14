use eframe::egui::{self, Color32, ViewportBuilder};
use egui::{Pos2, Vec2};
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
                    1 => {
                        tri.1 = pos;
                        tri.5 = uv;
                    }
                    2 => {
                        tri.2 = pos;
                        tri.6 = uv;
                    }
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

struct Camera {
    position: Vec3,
    yaw: f32,   // Horizontal rotation
    pitch: f32, // Vertical rotation
    fov: f32,
    aspect: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 2.0, 5.0),
            yaw: -90.0_f32.to_radians(),
            pitch: 0.0,
            fov: 60.0_f32.to_radians(),
            aspect: 1.0,
        }
    }

    fn view_matrix(&self) -> Mat4 {
        let forward = Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward);

        // Create view matrix that properly handles w coordinate
        let mut view = Mat4::IDENTITY;

        // Set rotation part
        view.col_mut(0)[0] = right.x;
        view.col_mut(0)[1] = right.y;
        view.col_mut(0)[2] = right.z;

        view.col_mut(1)[0] = up.x;
        view.col_mut(1)[1] = up.y;
        view.col_mut(1)[2] = up.z;

        view.col_mut(2)[0] = -forward.x;
        view.col_mut(2)[1] = -forward.y;
        view.col_mut(2)[2] = -forward.z;

        // Set translation part
        view.col_mut(3)[0] = -right.dot(self.position);
        view.col_mut(3)[1] = -up.dot(self.position);
        view.col_mut(3)[2] = forward.dot(self.position);
        view.col_mut(3)[3] = 1.0;

        view
    }

    fn projection_matrix(&self) -> Mat4 {
        let f = 1.0 / (self.fov / 2.0).tan();
        let near = 0.1;
        let far = 100.0;

        // Create perspective projection matrix with proper w coordinate handling
        let mut proj = Mat4::ZERO;
        proj.col_mut(0)[0] = f / self.aspect;
        proj.col_mut(1)[1] = f;
        proj.col_mut(2)[2] = -(far + near) / (far - near);
        proj.col_mut(2)[3] = -1.0;
        proj.col_mut(3)[2] = -(2.0 * far * near) / (far - near);

        proj
    }

    fn update(&mut self, ui: &egui::Ui) {
        let delta_time = ui.input(|i| i.unstable_dt) as f32;
        let move_speed = 5.0 * delta_time;
        let rotate_speed = 1.0 * delta_time;

        if ui.input(|i| i.pointer.secondary_down()) {
            let delta = ui.input(|i| i.pointer.delta());
            self.yaw += delta.x * 0.005;
            self.pitch =
                (self.pitch - delta.y * 0.005).clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
        }

        let forward = Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin()).normalize();

        let right = forward.cross(Vec3::Y).normalize();

        ui.input(|i| {
            if i.key_down(egui::Key::W) {
                self.position += forward * move_speed;
            }
            if i.key_down(egui::Key::S) {
                self.position -= forward * move_speed;
            }
            if i.key_down(egui::Key::A) {
                self.position -= right * move_speed;
            }
            if i.key_down(egui::Key::D) {
                self.position += right * move_speed;
            }
            if i.key_down(egui::Key::E) {
                self.position.y += move_speed;
            }
            if i.key_down(egui::Key::Q) {
                self.position.y -= move_speed;
            }
        });
    }
}

pub struct MeshViewerApp {
    mesh: InputMesh,
    debug_draw: EguiDebugDraw,
    camera: Camera,
    walkable_slope_angle: f32,
    obj_path: Option<PathBuf>,
}

fn obj_to_input_mesh(obj: &ObjData) -> InputMesh {
    let mut mesh = InputMesh::new();

    // Convert vertices
    mesh.verts = obj
        .vertices
        .iter()
        .skip(1)
        .map(|v| Vec3::new(v.x, v.y, v.z))
        .collect();

    // Triangulate faces and add indices
    let triangles = obj.triangulate();
    mesh.tris = triangles
        .iter()
        .flat_map(|tri| {
            // Adjust indices to be 0-based
            vec![tri[0] - 1, tri[1] - 1, tri[2] - 1].into_iter()
        })
        .map(|i| i as i32)
        .collect();

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
        // Create default mesh in case loading fails
        let mut default_mesh = InputMesh::new();
        default_mesh.verts = vec![
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ];
        default_mesh.tris = vec![0, 1, 2, 0, 2, 3];
        let normal1 = (default_mesh.verts[1] - default_mesh.verts[0])
            .cross(default_mesh.verts[2] - default_mesh.verts[0])
            .normalize();
        let normal2 = (default_mesh.verts[2] - default_mesh.verts[0])
            .cross(default_mesh.verts[3] - default_mesh.verts[0])
            .normalize();
        default_mesh.normals = vec![normal1, normal1, normal1, normal2, normal2, normal2];

        // Try to load dungeon.obj
        let dungeon_path = PathBuf::from("./dungeon.obj");
        let (mesh, obj_path) = if let Ok(obj_data) = obj_loader::load_obj(&dungeon_path) {
            println!("Successfully loaded dungeon.obj");
            (obj_to_input_mesh(&obj_data), Some(dungeon_path))
        } else {
            println!("Failed to load dungeon.obj, using default mesh");
            (default_mesh, None)
        };

        // Create initial camera position
        let mut camera = Camera::new();
        if let Some(path) = &obj_path {
            // If we loaded the obj, adjust camera to fit the model
            if let Ok(obj_data) = obj_loader::load_obj(path) {
                let (min, max) = obj_data.get_bounds();
                let center = Vec3::new(
                    (min.x + max.x) * 0.5,
                    (min.y + max.y) * 0.5,
                    (min.z + max.z) * 0.5,
                );
                camera.position = center + Vec3::new(0.0, 2.0, 5.0);
                camera.yaw = -90.0_f32.to_radians();
                camera.pitch = 0.0;
            }
        }

        Self {
            mesh,
            debug_draw: EguiDebugDraw::new(),
            camera,
            walkable_slope_angle: 45.0,
            obj_path,
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

            // Position camera at a good starting point relative to the model
            self.camera.position = center + Vec3::new(0.0, 2.0, 5.0);
            self.camera.yaw = -90.0_f32.to_radians();
            self.camera.pitch = 0.0;
        }
    }

    fn draw_mesh(&mut self) {
        self.debug_draw.clear();
        du_debug_draw_tri_mesh_slope(
            &mut self.debug_draw,
            &self.mesh,
            self.walkable_slope_angle,
            1.0,
        );
    }
}

fn pos_to_screen(pos: Vec3, camera: &Camera, rect: egui::Rect) -> Option<Pos2> {
    let view_proj = camera.projection_matrix() * camera.view_matrix();
    // Convert Vec3 to Vec4 for clip space
    let clip_pos = view_proj.project_point3(pos);
    let clip_pos = Vec4::new(clip_pos.x, clip_pos.y, clip_pos.z, 1.0);

    // Handle near plane clipping - if point is behind or very close to camera
    if clip_pos.z <= 0.001 {
        return None;
    }

    // Perspective divide
    let w = clip_pos.w;
    let ndc = Vec3::new(clip_pos.x / w, clip_pos.y / w, clip_pos.z / w);

    // More lenient frustum culling - allow points slightly outside the frustum
    // This helps prevent lines from disappearing near screen edges
    const MARGIN: f32 = 0.2; // 20% margin
    if ndc.x < -1.0 - MARGIN
        || ndc.x > 1.0 + MARGIN
        || ndc.y < -1.0 - MARGIN
        || ndc.y > 1.0 + MARGIN
        || ndc.z > 1.0 + MARGIN
    {
        return None;
    }

    // Clamp coordinates to screen bounds
    let x = (ndc.x * 0.5 + 0.5).clamp(0.0, 1.0) * rect.width() + rect.min.x;
    let y = (1.0 - (ndc.y * 0.5 + 0.5)).clamp(0.0, 1.0) * rect.height() + rect.min.y;

    Some(Pos2::new(x, y))
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

            // Update camera before drawing
            self.camera.update(ui);

            self.draw_mesh();

            let (rect, _response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
            self.camera.aspect = rect.width() / rect.height();

            // Reset camera position when R is pressed
            if ui.input(|i| i.key_pressed(egui::Key::R)) {
                self.camera = Camera::new();
            }

            let painter = ui.painter();

            // Draw all triangle
            for tri in &self.debug_draw.tris {
                let points = vec![
                    pos_to_screen(tri.0, &self.camera, rect),
                    pos_to_screen(tri.1, &self.camera, rect),
                    pos_to_screen(tri.2, &self.camera, rect),
                ];
            
                // Convert Vec<Option<Pos2>> to Vec<Pos2> by filtering out None values
                let valid_points: Vec<Pos2> = points.into_iter()
                    .filter_map(|p| p)  // Removes None values and unwraps Some values
                    .collect();
            
                // Only draw if we have all three points (no points were culled)
                if valid_points.len() == 3 {
                    painter.add(egui::Shape::convex_polygon(
                        valid_points,
                        tri.3,  // Fill color
                        (1.0, tri.3),  // Stroke width and color
                    ));
                }
            }

            // Add control instructions
            ui.painter().text(
                rect.min + egui::vec2(10.0, 10.0),
                egui::Align2::LEFT_TOP,
                "Controls:\nWASD - Move\nQ/E - Up/Down\nRight Click + Drag - Look\nR - Reset Camera",
                egui::FontId::proportional(14.0),
                Color32::WHITE,
            );
        });
    }
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

// Create a minimal test mesh that should trigger the rendering artifacts
fn create_test_mesh() -> InputMesh {
    let mut mesh = InputMesh::new();

    // Create just two triangles that share an edge
    mesh.verts = vec![
        Vec3::new(0.0, 0.0, 0.0),  // Bottom vertex
        Vec3::new(-1.0, 2.0, 0.0), // Top left
        Vec3::new(1.0, 2.0, 0.0),  // Top right
        Vec3::new(0.0, 1.0, 1.0),  // Back middle
    ];

    // Create two triangles
    mesh.tris = vec![
        0, 1, 2, // Front triangle
        0, 2, 3, // Back triangle with steep slope
    ];

    // Calculate normals
    let v0 = mesh.verts[0];
    let v1 = mesh.verts[1];
    let v2 = mesh.verts[2];
    let v3 = mesh.verts[3];

    // Calculate normals for both triangles
    let normal1 = (v1 - v0).cross(v2 - v0).normalize();
    let normal2 = (v2 - v0).cross(v3 - v0).normalize();

    mesh.normals = vec![
        normal1, normal1, normal1, // First triangle
        normal2, normal2, normal2, // Second triangle
    ];

    mesh
}

// Test function to load and render the minimal test case
fn test_rendering(app: &mut MeshViewerApp) {
    app.mesh = create_test_mesh();
    app.camera.position = Vec3::new(0.0, 1.0, 3.0);
    app.camera.yaw = -90.0_f32.to_radians();
    app.camera.pitch = 0.0;
    app.walkable_slope_angle = 45.0;
}
