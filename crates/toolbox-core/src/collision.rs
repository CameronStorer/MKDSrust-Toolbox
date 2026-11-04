//! Material/slope classification is configurable; semantic research can replace
//! the defaults without changing the KCL encoder.
use crate::Result;
use nitro_formats::{
    kcl::{Kcl, Triangle},
    math::{cross, normalize},
    model::Model,
};
#[derive(Clone, Debug)]
pub struct AttributeTable {
    pub floor: u16,
    pub wall: u16,
    pub floor_angle_degrees: f32,
    pub materials: Vec<(String, u16)>,
}
impl Default for AttributeTable {
    fn default() -> Self {
        Self {
            floor: mkds_data::collision_attribute(0, false, true),
            wall: mkds_data::collision_attribute(8, true, false),
            floor_angle_degrees: 50.,
            materials: vec![
                (
                    "boost".into(),
                    mkds_data::collision_attribute(18, false, true),
                ),
                (
                    "wading".into(),
                    mkds_data::collision_attribute(3, false, true),
                ),
                (
                    "road".into(),
                    mkds_data::collision_attribute(0, false, true),
                ),
                (
                    "grass".into(),
                    mkds_data::collision_attribute(2, false, true),
                ),
                (
                    "offroad".into(),
                    mkds_data::collision_attribute(3, false, true),
                ),
                (
                    "water".into(),
                    mkds_data::collision_attribute(3, false, true),
                ),
                (
                    "lava".into(),
                    mkds_data::collision_attribute(15, false, false),
                ),
                (
                    "fall".into(),
                    mkds_data::collision_attribute(15, false, false),
                ),
                (
                    "wall".into(),
                    mkds_data::collision_attribute(8, true, false),
                ),
            ],
        }
    }
}
pub fn generate(model: &Model, table: &AttributeTable) -> Result<Kcl> {
    let mut triangles = vec![];
    for mesh in &model.meshes {
        nitro_formats::model::validate_mesh(mesh)?;
        let name = model
            .materials
            .get(mesh.material)
            .map_or("", |m| m.name.as_str())
            .to_ascii_lowercase();
        for indices in mesh.indices.chunks_exact(3) {
            let positions = [
                mesh.positions[indices[0] as usize],
                mesh.positions[indices[1] as usize],
                mesh.positions[indices[2] as usize],
            ];
            let normal = normalize(cross(
                std::array::from_fn(|k| positions[1][k] - positions[0][k]),
                std::array::from_fn(|k| positions[2][k] - positions[0][k]),
            ));
            let attribute = table
                .materials
                .iter()
                .find(|(keyword, _)| name.contains(&keyword.to_ascii_lowercase()))
                .map_or_else(
                    || {
                        if normal[1].abs() >= table.floor_angle_degrees.to_radians().cos() {
                            table.floor
                        } else {
                            table.wall
                        }
                    },
                    |(_, a)| *a,
                );
            triangles.push(Triangle {
                positions,
                normal,
                attribute,
            });
        }
    }
    Ok(Kcl::from_triangles(&triangles)?)
}
