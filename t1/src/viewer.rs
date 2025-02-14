use eframe::egui::{self, Color32, ViewportBuilder};
use egui::Pos2;
use egui_gizmo::GizmoMode;
use glam::{Mat4, Vec3, Vec4};

// Import the debug draw implementation from previous code
use crate::debug_draw::*;
use crate::debug_draw::{du_debug_draw_poly_mesh, DebugDraw, PolyMesh};

struct EguiDebugDraw {
    lines: Vec<(Vec3, Vec3, Color32)>,
    points: Vec<(Vec3, Color32)>,
    quads: Vec<(Vec3, Vec3, Vec3, Vec3, Color32)>,
    current_mode: i32,
}

impl EguiDebugDraw {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            points: Vec::new(),
            quads: Vec::new(),
            current_mode: 0,
        }
    }

    fn clear(&mut self) {
        self.lines.clear();
        self.points.clear();
        self.quads.clear();
    }
}

impl DebugDraw for EguiDebugDraw {
    fn begin(&mut self, prim: i32, _size: f32) {
        self.current_mode = prim;
    }

    fn vertex(&mut self, pos: Vec3, color: Vec4) {
        let col = Color32::from_rgba_premultiplied(
            (color.x * 255.0) as u8,
            (color.y * 255.0) as u8,
            (color.z * 255.0) as u8,
            (color.w * 255.0) as u8,
        );

        match self.current_mode {
            DU_DRAW_LINES => {
                if self.lines.len() % 2 == 1 {
                    let last = self.lines.last_mut().unwrap();
                    last.1 = pos;
                } else {
                    self.lines.push((pos, pos, col));
                }
            }
            DU_DRAW_POINTS => {
                self.points.push((pos, col));
            }
            DU_DRAW_QUADS => {
                if self.quads.len() == 0 || self.quads.last().unwrap().4 != col {
                    self.quads.push((pos, pos, pos, pos, col));
                } else {
                    let quad = self.quads.last_mut().unwrap();
                    match self.points.len() % 4 {
                        0 => quad.0 = pos,
                        1 => quad.1 = pos,
                        2 => quad.2 = pos,
                        3 => quad.3 = pos,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn end(&mut self) {}

    fn area_to_col(&self, area: u8) -> Vec4 {
        // Simple area coloring - you can customize this
        match area {
            0 => Vec4::new(0.0, 0.0, 0.0, 0.25),
            63 => Vec4::new(0.0, 0.75, 1.0, 0.25),
            _ => Vec4::new((area as f32) / 255.0, 0.5, (255 - area) as f32 / 255.0, 0.5),
        }
    }
}

pub struct MeshViewerApp {
    mesh: PolyMesh,
    debug_draw: EguiDebugDraw,
    camera: Camera,
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

impl MeshViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Create a sample mesh
        let mesh = PolyMesh {
            verts: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(0.5, 1.0, 0.5),
            ],
            polys: vec![
                vec![0, 1, 4, RC_MESH_NULL_IDX],
                vec![1, 2, 4, RC_MESH_NULL_IDX],
                vec![2, 3, 4, RC_MESH_NULL_IDX],
                vec![3, 0, 4, RC_MESH_NULL_IDX],
            ],
            areas: vec![
                RC_WALKABLE_AREA,
                RC_WALKABLE_AREA,
                RC_NULL_AREA,
                RC_WALKABLE_AREA,
            ],
            nvp: 4,
            cs: 1.0,
            ch: 1.0,
            bmin: Vec3::new(-1.0, -1.0, -1.0),
        };

        Self {
            mesh,
            debug_draw: EguiDebugDraw::new(),
            camera: Camera::new(),
        }
    }

    fn draw_mesh(&mut self) {
        self.debug_draw.clear();
        du_debug_draw_poly_mesh(&mut self.debug_draw, &self.mesh);
    }
}
// Convert glam Mat4 to array format expected by egui_gizmo
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
            let mut gizmo = egui_gizmo::Gizmo::new("camera_gizmo")
                .view_matrix(mat4_to_array(self.camera.view_matrix()).into())
                .projection_matrix(mat4_to_array(self.camera.projection_matrix()).into())
                .mode(GizmoMode::Rotate);

            // Draw the mesh
            self.draw_mesh();

            // Draw the scene
            let (rect, _response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());

            // Update camera aspect ratio
            self.camera.aspect = rect.width() / rect.height();

            // Handle camera controls
            if ui.input(|i| i.key_pressed(egui::Key::R)) {
                self.camera = Camera::new();
            }

            // Draw gizmo
            if let Some(response) = gizmo.interact(ui) {
                // Convert mint::Quaternion to glam::Quat
                let rotation = glam::Quat::from_xyzw(
                    response.rotation.v.x,
                    response.rotation.v.y,
                    response.rotation.v.z,
                    response.rotation.s,
                );
                self.camera.position = rotation.mul_vec3(self.camera.position);
            }
            // Draw the debug geometry
            let painter = ui.painter();

            // Draw quads
            for quad in &self.debug_draw.quads {
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        pos_to_screen(quad.0, &self.camera, rect),
                        pos_to_screen(quad.1, &self.camera, rect),
                        pos_to_screen(quad.2, &self.camera, rect),
                        pos_to_screen(quad.3, &self.camera, rect),
                    ],
                    quad.4,
                    (1.0, quad.4),
                ));
            }

            // Draw lines
            for line in &self.debug_draw.lines {
                painter.line_segment(
                    [
                        pos_to_screen(line.0, &self.camera, rect),
                        pos_to_screen(line.1, &self.camera, rect),
                    ],
                    (1.0, line.2),
                );
            }

            // Draw points
            for point in &self.debug_draw.points {
                painter.circle_filled(pos_to_screen(point.0, &self.camera, rect), 3.0, point.1);
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

// Entry point
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
