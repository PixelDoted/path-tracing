use bevy::math::Vec3;

use crate::data::{BvhNode, Mesh, MeshData};

pub fn generate(mesh: &Mesh, mesh_data: &MeshData, mesh_nodes: &mut Vec<BvhNode>) -> u32 {
    let mut triangles: Vec<BvhTriangle> = Vec::with_capacity(mesh.tri_count as usize);
    for t in 0..mesh.tri_count {
        let i = (t * 3) as usize;

        let ai = mesh_data.indices[i] as usize;
        let bi = mesh_data.indices[i + 1] as usize;
        let ci = mesh_data.indices[i + 2] as usize;

        let av = mesh_data.vertices[ai];
        let bv = mesh_data.vertices[bi];
        let cv = mesh_data.vertices[ci];

        triangles.push(BvhTriangle {
            aabb_min: av.position.min(bv.position).min(cv.position),
            aabb_max: av.position.max(bv.position).max(cv.position),
            node_index: 0,
        });
    }

    let bvh = bvh::bvh::Bvh::build(&mut triangles);
    let nodes = bvh.flatten_custom(&|aabb, entry_index, exit_index, shape_index| BvhNode {
        aabb_min: Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z),
        aabb_max: Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z),
        entry_index,
        exit_index,
        shape_index,
    });

    let total_nodes = nodes.len();
    mesh_nodes.extend(nodes);
    total_nodes as u32
}

pub struct BvhTriangle {
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,
    node_index: usize,
}

impl bvh::aabb::Bounded<f32, 3> for BvhTriangle {
    fn aabb(&self) -> bvh::aabb::Aabb<f32, 3> {
        bvh::aabb::Aabb::with_bounds(
            self.aabb_min.to_array().into(),
            self.aabb_max.to_array().into(),
        )
    }
}

impl bvh::bounding_hierarchy::BHShape<f32, 3> for BvhTriangle {
    fn set_bh_node_index(&mut self, idx: usize) {
        self.node_index = idx;
    }

    fn bh_node_index(&self) -> usize {
        self.node_index
    }
}
