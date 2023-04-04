mod utils;

use bvh::aabb::{Bounded, AABB};
use bvh::bounding_hierarchy::BHShape;
use bvh::bvh::BVH;
use bvh::Point3;
use crevice::std430::{self, AsStd430, Std430};
use glam::Affine3A;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::mem;
use wasm_bindgen::prelude::*;

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
  ( $( $t:tt )* ) => {
    #[allow(unused_unsafe)]
    unsafe {
      web_sys::console::debug_1(&format!( $( $t )* ).into());
      // print!( $( $t )* );
    }
  }
}

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct StagingBuffer {
    pub id: u32,
}

#[wasm_bindgen]
pub struct BuiltBvh {
    pub serialized: StagingBuffer,
    pub num_nodes: u32,
}

#[wasm_bindgen]
impl StagingBuffer {
    pub fn free(&self) {
        staging_buffers_map().remove(&self.id);
    }

    fn buffer(&self) -> &mut Vec<u8> {
        staging_buffers_map().get_mut(&self.id).unwrap()
    }

    pub fn u8_view(&self) -> JsValue {
        if let Some(buf) = staging_buffers_map().get_mut(&self.id) {
            let array = unsafe { js_sys::Uint8Array::view(buf) };
            return JsValue::from(array);
        }
        return JsValue::null();
    }

    #[wasm_bindgen(constructor)]
    pub fn new(byte_length: usize) -> Self {
        //} Box<[JsValue]> {
        let map = staging_buffers_map();
        let buffer: Vec<u8> = vec![0; byte_length as usize];
        let id = unsafe { NEXT_BUFFER_ID };
        map.insert(id, buffer);
        unsafe {
            NEXT_BUFFER_ID += 1;
        }
        log!("alloacted buffer of size {}: {}", id, byte_length);
        return StagingBuffer { id };
    }

    fn from_existing_buffer(buf: Vec<u8>) -> Self {
        let map = staging_buffers_map();
        let id = unsafe { NEXT_BUFFER_ID };
        map.insert(id, buf);
        unsafe {
            NEXT_BUFFER_ID += 1;
        }
        return StagingBuffer { id };
    }
}

type StagingBufferMap = HashMap<u32, Vec<u8>>;
// lazy_static! {
//     static ref STAGING_BUFFERS: Mutex<StagingBufferMap> = Mutex::new(HashMap::new());
// }
// STAGING_BUFFERS.lock().unwrap();

static mut NEXT_BUFFER_ID: u32 = 0;
static mut STAGING_BUFFERS: Option<StagingBufferMap> = None;
fn staging_buffers_map() -> &'static mut StagingBufferMap {
    if let Some(b) = unsafe { &mut STAGING_BUFFERS } {
        return b;
    }
    let b = StagingBufferMap::new();
    unsafe {
        STAGING_BUFFERS = Some(b);
        let x = STAGING_BUFFERS.as_mut().unwrap();
        x
    }
}

#[derive(Debug, PartialEq)]
enum GeometryType {
    Triangle = 0,
    Aabb = 1,
}

impl TryFrom<i32> for GeometryType {
    type Error = ();

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(GeometryType::Triangle),
            1 => Ok(GeometryType::Aabb),
            _ => Err(()),
        }
    }
}

enum GeometryDescriptorField {
    Type = 0,
    NumPrimitives = 1,
    VbufId = 2,
    VbufByteOffset = 3,
    IbufId = 4,
    IbufByteOffset = 5,

    NumFields = 6,
}

// TODO: maybe make this an enum struct, Aabb/Triangle
#[derive(Debug)]
struct Primitive<'a> {
    blas_local_geometry_id: u32,
    within_blas_primitive_id: u32,

    // build data
    primitive_id: u32,
    // geometry_descriptor: &'a [i32; 3],
    geometry_type: GeometryType,
    vbuf: &'a [f32],
    ibuf: Option<&'a [u32]>,
}

impl<'a> Bounded for Primitive<'a> {
    fn aabb(&self) -> AABB {
        if self.geometry_type == GeometryType::Triangle {
            let mut indices = vec![0; 3];
            let offset = (3 * self.primitive_id) as usize;
            if let Some(ibuf) = self.ibuf {
                indices[0] = ibuf[offset] as usize;
                indices[1] = ibuf[offset + 1] as usize;
                indices[2] = ibuf[offset + 2] as usize;
            } else {
                indices[0] = offset;
                indices[1] = offset + 1;
                indices[2] = offset + 2;
            }
            let mut aabb = AABB::empty();
            for i in 0..3 {
                // NOTE: hardcoded 3xfloats vbo stride for our compact staging buffers
                let vi = indices[i] * 3;
                aabb.grow_mut(&Point3::new(
                    self.vbuf[vi],
                    self.vbuf[vi + 1],
                    self.vbuf[vi + 2],
                ));
            }
            aabb
        } else {
            // staging buffers for AABBs are all unique
            AABB::with_bounds(
                Point3::new(self.vbuf[0], self.vbuf[1], self.vbuf[2]),
                Point3::new(self.vbuf[3], self.vbuf[4], self.vbuf[5]),
            )
        }
    }
}

impl<'a> BHShape for Primitive<'a> {
    fn set_bh_node_index(&mut self, _index: usize) {
        // dont care?
        // self.node_index = index;
    }

    fn bh_node_index(&self) -> usize {
        panic!("unimplemented bh_node_index")
        //  self.node_index
    }
}

#[derive(Debug)]
struct TlasInstanceDescriptor {
    mask: u32,
    flags: u32,
    instance_id: u32,
    sbt_instance_offset: u32,
    instance_custom_index: i32,
    transform_to_world_4x3: [f32; 12],

    blas_entry_index: u32,
    blas_geometry_id_offset: u32,
    aabb: AABB, // aabb(transform_to_world * blas_aabb)
}

#[derive(Debug)]
#[repr(C)]
struct TlasInstanceDescriptorJsInput {
    mask: u32,
    flags: u32,
    instance_id: u32,
    sbt_instance_offset: u32,
    instance_custom_index: i32,
    blas_entry_index: u32,
    blas_geometry_id_offset: u32,
    blas_aabb: [f32; 6],
    transform_to_world_4x3: [f32; 12], // 4x3 column major
}

// #[wasm_bindgen(typescript_custom_section)]
// const TS_APPEND_CONTENT: &'static str = r#"
// export const TlasInstanceDescriptorField__numWords = 13;
// "#;

impl Bounded for TlasInstanceDescriptor {
    fn aabb(&self) -> AABB {
        self.aabb
    }
}

impl BHShape for TlasInstanceDescriptor {
    fn set_bh_node_index(&mut self, _index: usize) {
        // dont care?
        // self.node_index = index;
    }

    fn bh_node_index(&self) -> usize {
        panic!("unimplemented bh_node_index")
        //  self.node_index
    }
}

// TODO: buffer data layout?
// see common.glsl
#[derive(Debug, AsStd430)]
struct GPUBlasBvhNode {
    aabb: GPUAabb,
    entry_index_or_primitive_id: u32,
    exit_index: u32,
    geometry_id: i32,
}

const INTERIOR_NODE_GEOMETRY_ID: i32 = -1;

#[wasm_bindgen]
pub fn build_blas(blas_descriptor_buffer_id: u32) -> BuiltBvh {
    utils::set_panic_hook();
    let map = staging_buffers_map();
    // TODO: error handling
    let buf = map.get(&blas_descriptor_buffer_id).unwrap();
    let buf_i32_le: &[i32] = unsafe { buf.align_to().1 };
    let num_geoms = buf_i32_le[0];
    let num_total_primitives = buf_i32_le[1];
    log!("blas geom desc: {}, {}", num_geoms, num_total_primitives);
    assert!(num_geoms > 0 && num_total_primitives > 0,);
    assert!(buf_i32_le.len() as i32 == 2 + num_geoms * (GeometryDescriptorField::NumFields as i32));
    let mut primitives = Vec::<Primitive>::with_capacity(num_total_primitives as usize);
    for gi in 0..num_geoms as u32 {
        let offset = 2 + (GeometryDescriptorField::NumFields as usize) * gi as usize;
        // [geom_type, num_primitives, vbuf_id, ibuf_id]
        let geom: &[i32] =
            &buf_i32_le[offset..offset + (GeometryDescriptorField::NumFields as usize)];
        let np = geom[GeometryDescriptorField::NumPrimitives as usize];
        let vbuf = map
            .get(&(geom[GeometryDescriptorField::VbufId as usize] as u32))
            .unwrap();
        let vbuf_byte_offset = geom[GeometryDescriptorField::VbufByteOffset as usize] as u32;
        let vbuf_f32_le: &[f32] = unsafe { vbuf[(vbuf_byte_offset as usize)..].align_to().1 };
        let mut ibuf_u32_le: Option<&[u32]> = None;
        if geom[GeometryDescriptorField::Type as usize] == GeometryType::Triangle as i32
            && geom[GeometryDescriptorField::IbufId as usize] >= 0
        {
            let ibuf = map
                .get(&(geom[GeometryDescriptorField::IbufId as usize] as u32))
                .unwrap();
            let ibuf_byte_offset = geom[GeometryDescriptorField::IbufByteOffset as usize] as u32;
            ibuf_u32_le = Some(unsafe { ibuf[(ibuf_byte_offset as usize)..].align_to().1 });
        }
        for pi in 0..np as u32 {
            primitives.push(Primitive {
                blas_local_geometry_id: gi,
                primitive_id: pi,
                within_blas_primitive_id: primitives.len() as u32,
                geometry_type: GeometryType::try_from(geom[GeometryDescriptorField::Type as usize])
                    .unwrap(),
                vbuf: vbuf_f32_le,
                ibuf: ibuf_u32_le,
            });
        }
    }

    // log!("building from primitives: {:?}", primitives);
    let bvh = BVH::build(&mut primitives);
    let num_bvh_nodes = bvh.nodes.len() as u32;
    // log!("bvh tree: {:?}", bvh.nodes);

    let mut sizer = std430::Sizer::new();
    sizer.add::<GPUBlasBvhNode>();
    let node_array_stride = sizer.add::<GPUBlasBvhNode>();
    let staging_buffer = StagingBuffer::new(num_bvh_nodes as usize * node_array_stride);
    let staging_buffer_u8 = staging_buffer.buffer();
    staging_buffer_u8.truncate(0); // !!
    let mut writer = std430::Writer::new(staging_buffer_u8);

    let mut blas_node_ctor = |aabb: &AABB, entry, mut exit, within_blas_primitive_id| {
        if exit >= num_bvh_nodes {
            exit = u32::max_value();
        }
        let node = if entry == u32::max_value() {
            // leaf
            let p = &primitives[within_blas_primitive_id as usize];
            // currently leaf only contains single shape/primitive
            GPUBlasBvhNode {
                aabb: (&p.aabb()).into(),                    // not inf->-inf
                entry_index_or_primitive_id: p.primitive_id, // local
                exit_index: exit,
                geometry_id: p.blas_local_geometry_id as i32,
            }
        } else {
            GPUBlasBvhNode {
                aabb: aabb.into(),
                entry_index_or_primitive_id: entry,
                exit_index: exit,
                geometry_id: INTERIOR_NODE_GEOMETRY_ID, // interior
            }
        };
        writer.write(&node).unwrap();
    };

    bvh.flatten_custom(&mut blas_node_ctor);
    let aligned_size = align_to(staging_buffer_u8.len(), Std430GPUBlasBvhNode::ALIGNMENT);
    if staging_buffer_u8.len() < aligned_size {
        staging_buffer_u8.resize(aligned_size, 0);
    }
    BuiltBvh {
        serialized: staging_buffer,
        num_nodes: num_bvh_nodes,
    }
}

// TODO: default as identity matrix
#[derive(Debug, AsStd430, Default)]
struct Float12 {
    f0: f32,
    f1: f32,
    f2: f32,
    f3: f32,
    f4: f32,
    f5: f32,
    f6: f32,
    f7: f32,
    f8: f32,
    f9: f32,
    f10: f32,
    f11: f32,
}

impl From<&[f32; 12]> for Float12 {
    fn from(m: &[f32; 12]) -> Self {
        Float12 {
            f0: m[0],
            f1: m[1],
            f2: m[2],
            f3: m[3],
            f4: m[4],
            f5: m[5],
            f6: m[6],
            f7: m[7],
            f8: m[8],
            f9: m[9],
            f10: m[10],
            f11: m[11],
        }
    }
}

// TODO: deprecate this
// https://bugs.chromium.org/p/tint/issues/detail?id=1049
// column major
type Mat4x3Workaround = Float12;

#[derive(Debug, AsStd430)]
struct MintMat4Wrap {
    mat4: mint::ColumnMatrix4<f32>,
}

impl Default for MintMat4Wrap {
    fn default() -> Self {
        MintMat4Wrap {
            mat4: mint::ColumnMatrix4::<f32>::from([
                [1f32, 0.0, 0.0, 0.0],
                [0.0, 1f32, 0.0, 0.0],
                [0.0, 0.0, 1f32, 0.0],
                [0.0, 0.0, 0.0, 1f32],
            ]),
        }
    }
}

impl From<&[f32; 12]> for MintMat4Wrap {
    fn from(m: &[f32; 12]) -> Self {
        MintMat4Wrap {
            mat4: mint::ColumnMatrix4::<f32>::from([
                [m[0], m[1], m[2], 0.0],
                [m[3], m[4], m[5], 0.0],
                [m[6], m[7], m[8], 0.0],
                [m[9], m[10], m[11], 1.0],
            ]),
        }
    }
}

// should have been Point3 but AsStd430 is not implemented
#[derive(Debug, AsStd430)]
struct GPUAabb {
    min: mint::Vector3<f32>,
    max: mint::Vector3<f32>,
}

impl Default for GPUAabb {
    fn default() -> Self {
        GPUAabb {
            min: mint::Vector3::<f32>::from([f32::MAX, f32::MAX, f32::MAX]),
            max: mint::Vector3::<f32>::from([f32::MIN, f32::MIN, f32::MIN]),
        }
    }
}

impl From<&AABB> for GPUAabb {
    fn from(aabb: &AABB) -> Self {
        GPUAabb {
            min: mint::Vector3::<f32>::from([aabb.min.x, aabb.min.y, aabb.min.z]),
            max: mint::Vector3::<f32>::from([aabb.max.x, aabb.max.y, aabb.max.z]),
        }
    }
}

// see common.glsl
#[derive(Default, Debug, AsStd430)]
struct GPUTlasBvhNode {
    aabb: GPUAabb,
    entry_index: u32,
    exit_index: u32,
    is_leaf: u32,

    // TLAS leaf data
    mask: u32,
    flags: u32,
    instance_id: u32,
    sbt_instance_offset: u32,
    // geoms within this instance
    instance_custom_index: i32,
    transform_to_world: Mat4x3Workaround, // MintMat4Wrap, // this is exactly the same layout as mat4x3
    transform_to_object: Mat4x3Workaround, // MintMat4Wrap, // this is exactly the same layout as mat4x3

    // For traversal
    blas_geometry_id_offset: u32,
}

fn flat_bvh_nodes_to_u8_view<T>(nodes: Vec<T>) -> Vec<u8> {
    let u8_view_flat_bvh: Vec<u8> = unsafe {
        let ratio = mem::size_of::<T>() / mem::size_of::<u8>();
        // Ensure the original vector is not dropped.
        let mut v_clone = std::mem::ManuallyDrop::new(nodes);
        Vec::from_raw_parts(
            v_clone.as_mut_ptr() as *mut u8,
            v_clone.len() * ratio,
            v_clone.capacity() * ratio,
        )
    };
    u8_view_flat_bvh
}

fn transform_aabb(affine_m: &[f32; 12], aabb: &[f32; 6]) -> AABB {
    // Begin at T.
    let mut new_min: [f32; 3] = [affine_m[9], affine_m[10], affine_m[11]];
    let mut new_max: [f32; 3] = [affine_m[9], affine_m[10], affine_m[11]];

    for r in 0..3 {
        for c in 0..3 {
            let a = affine_m[c * 3 + r] * aabb[c]; // aabb.min[c]
            let b = affine_m[c * 3 + r] * aabb[c + 3]; // aabb.max[c]
            new_min[r] += a.min(b);
            new_max[r] += a.max(b);
        }
    }
    AABB::with_bounds(
        Point3::new(new_min[0], new_min[1], new_min[2]),
        Point3::new(new_max[0], new_max[1], new_max[2]),
    )
}

fn inv(m: &[f32; 12]) -> [f32; 12] {
    let m = Affine3A::from_cols_array(m);
    let iv = m.inverse();
    iv.to_cols_array()
}

fn align_to(x: usize, to: usize) -> usize {
    (x + to - 1) / to * to
}

#[wasm_bindgen]
pub fn build_tlas(tlas_descriptor_buffer_id: u32) -> BuiltBvh {
    utils::set_panic_hook();
    let map = staging_buffers_map();
    // TODO: error handling
    let buf = map.get(&tlas_descriptor_buffer_id).unwrap();
    let buf_i32_le: &[i32] = unsafe { buf.align_to().1 };
    let num_blases = buf_i32_le[0];
    assert!(num_blases > 0);
    let num_blases = num_blases as usize;
    assert!(buf.len() == 1 * 4 + num_blases * mem::size_of::<TlasInstanceDescriptorJsInput>());

    let buf_descriptors: &[TlasInstanceDescriptorJsInput] = unsafe { buf[1..].align_to().1 };
    assert!(num_blases == buf_descriptors.len());

    let mut instances = Vec::<TlasInstanceDescriptor>::with_capacity(num_blases);
    for inst in buf_descriptors {
        instances.push(TlasInstanceDescriptor {
            mask: inst.mask,
            flags: inst.flags,
            instance_id: inst.instance_id,
            sbt_instance_offset: inst.sbt_instance_offset,
            instance_custom_index: inst.instance_custom_index,
            blas_entry_index: inst.blas_entry_index,
            blas_geometry_id_offset: inst.blas_geometry_id_offset,
            aabb: transform_aabb(&inst.transform_to_world_4x3, &inst.blas_aabb),
            transform_to_world_4x3: inst.transform_to_world_4x3,
        })
    }

    log!("building from tlas instances: {:?}", instances);
    let bvh = BVH::build(&mut instances);
    let num_bvh_nodes = bvh.nodes.len() as u32;
    log!("tlas bvh tree: {:?}", bvh.nodes);

    let mut sizer = std430::Sizer::new();
    sizer.add::<GPUTlasBvhNode>();
    let node_array_stride = sizer.add::<GPUTlasBvhNode>();
    let staging_buffer = StagingBuffer::new(num_bvh_nodes as usize * node_array_stride);
    let staging_buffer_u8 = staging_buffer.buffer();
    staging_buffer_u8.truncate(0); // !!
    let mut writer = std430::Writer::new(staging_buffer_u8);
    let mut tlas_node_ctor = |aabb: &AABB, entry, mut exit, instance_id| {
        if exit >= num_bvh_nodes {
            exit = u32::max_value();
        }
        let node = if entry == u32::max_value() {
            // leaf
            let inst = &instances[instance_id as usize];
            // currently leaf only contains single shape/primitive
            GPUTlasBvhNode {
                aabb: (&inst.aabb).into(), // this is the transformed aabb of the blas root aabb
                entry_index: inst.blas_entry_index,
                exit_index: exit,
                is_leaf: 1,
                // TODO: store leaf data in input instance
                mask: inst.mask,
                flags: inst.flags,
                instance_id: inst.instance_id,
                sbt_instance_offset: inst.sbt_instance_offset,
                instance_custom_index: inst.instance_custom_index,
                blas_geometry_id_offset: inst.blas_geometry_id_offset,
                transform_to_world: (&inst.transform_to_world_4x3).into(),
                transform_to_object: (&inv(&inst.transform_to_world_4x3)).into(),
            }
        } else {
            GPUTlasBvhNode {
                aabb: aabb.into(),
                entry_index: entry,
                exit_index: exit,
                is_leaf: 0,
                mask: 0,
                flags: 0,
                instance_id: 0,
                sbt_instance_offset: 0,
                instance_custom_index: 0,
                blas_geometry_id_offset: 0,
                transform_to_world: Mat4x3Workaround::default(),
                transform_to_object: Mat4x3Workaround::default(),
            }
        };
        writer.write(&node).unwrap();
    };
    bvh.flatten_custom(&mut tlas_node_ctor);
    let aligned_size = align_to(staging_buffer_u8.len(), Std430GPUTlasBvhNode::ALIGNMENT);
    if staging_buffer_u8.len() < aligned_size {
        staging_buffer_u8.resize(aligned_size, 0);
    }
    BuiltBvh {
        serialized: staging_buffer,
        num_nodes: num_bvh_nodes,
    }
}

mod tests {
    use crate::{build_blas, staging_buffers_map, StagingBufferMap};

    #[test]
    /// Verify contents of the bounding hierarchy for a fixed scene structure
    fn test_debug_bug() {
        let map = staging_buffers_map();
        {
            map.insert(
                11,
                vec![
                    51, 51, 10, 68, 0, 0, 0, 0, 0, 0, 0, 0, 102, 102, 9, 68, 0, 0, 0, 0, 205, 204,
                    11, 68, 0, 0, 11, 68, 51, 51, 9, 68, 205, 204, 11, 68, 0, 0, 11, 68, 51, 51, 9,
                    68, 0, 0, 0, 0,
                ],
            );
            map.insert(
                9,
                vec![
                    0, 0, 0, 0, 0, 0, 0, 0, 205, 204, 11, 68, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 51, 51, 9, 68, 0, 0, 0, 0, 0, 0, 0, 0, 51, 51, 9, 68, 205, 204, 11,
                    68,
                ],
            );
            map.insert(
                7,
                vec![
                    51, 51, 10, 68, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 205, 204, 11, 68, 102, 102, 9, 68, 0, 0, 0, 0, 205, 204,
                    11, 68,
                ],
            );
            map.insert(
                6,
                vec![
                    0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0,
                ],
            );
            map.insert(
                1,
                vec![
                    0, 0, 11, 68, 51, 51, 9, 68, 0, 0, 0, 0, 0, 0, 11, 68, 51, 51, 9, 68, 205, 204,
                    11, 68, 0, 0, 0, 0, 51, 51, 9, 68, 205, 204, 11, 68, 0, 0, 0, 0, 51, 51, 9, 68,
                    0, 0, 0, 0, 0, 0, 85, 67, 51, 51, 9, 68, 0, 0, 99, 67, 0, 0, 85, 67, 51, 51, 9,
                    68, 0, 0, 166, 67, 0, 128, 171, 67, 51, 51, 9, 68, 0, 0, 166, 67, 0, 128, 171,
                    67, 51, 51, 9, 68, 0, 0, 99, 67,
                ],
            );
            map.insert(
                14,
                vec![
                    0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0,
                    0, 0, 5, 0, 0, 0, 6, 0, 0, 0, 4, 0, 0, 0, 6, 0, 0, 0, 7, 0, 0, 0, 8, 0, 0, 0,
                    9, 0, 0, 0, 10, 0, 0, 0, 8, 0, 0, 0, 10, 0, 0, 0, 11, 0, 0, 0, 12, 0, 0, 0, 13,
                    0, 0, 0, 14, 0, 0, 0, 12, 0, 0, 0, 14, 0, 0, 0, 15, 0, 0, 0, 16, 0, 0, 0, 17,
                    0, 0, 0, 18, 0, 0, 0, 16, 0, 0, 0, 18, 0, 0, 0, 19, 0, 0, 0, 20, 0, 0, 0, 21,
                    0, 0, 0, 22, 0, 0, 0, 20, 0, 0, 0, 22, 0, 0, 0, 23, 0, 0, 0,
                ],
            );
            map.insert(
                5,
                vec![
                    102, 102, 9, 68, 0, 0, 0, 0, 205, 204, 11, 68, 0, 0, 0, 0, 0, 0, 0, 0, 205,
                    204, 11, 68, 0, 0, 0, 0, 51, 51, 9, 68, 205, 204, 11, 68, 0, 0, 11, 68, 51, 51,
                    9, 68, 205, 204, 11, 68,
                ],
            );
            map.insert(
                10,
                vec![
                    0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0,
                ],
            );
            map.insert(
                12,
                vec![
                    0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0,
                ],
            );
            map.insert(
                16,
                vec![
                    2, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0,
                    0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0,
                ],
            );
            map.insert(
                2,
                vec![
                    0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4, 0,
                    0, 0, 5, 0, 0, 0, 6, 0, 0, 0, 6, 0, 0, 0, 7, 0, 0, 0, 4, 0, 0, 0,
                ],
            );
            map.insert(
                3,
                vec![
                    0, 128, 171, 67, 51, 19, 9, 68, 0, 0, 99, 67, 0, 128, 171, 67, 51, 19, 9, 68,
                    0, 0, 166, 67, 0, 0, 85, 67, 51, 19, 9, 68, 0, 0, 166, 67, 0, 0, 85, 67, 51,
                    19, 9, 68, 0, 0, 99, 67,
                ],
            );
            map.insert(
                4,
                vec![
                    0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0,
                ],
            );
            map.insert(
                8,
                vec![
                    0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0,
                ],
            );
            map.insert(
                0,
                vec![
                    0, 0, 22, 67, 0, 0, 0, 0, 0, 0, 72, 66, 0, 0, 175, 67, 0, 0, 72, 67, 0, 0, 122,
                    67,
                ],
            );
            map.insert(
                13,
                vec![
                    0, 128, 211, 67, 0, 0, 165, 67, 0, 0, 119, 67, 0, 128, 132, 67, 0, 0, 165, 67,
                    0, 0, 148, 67, 0, 0, 157, 67, 0, 0, 165, 67, 0, 0, 228, 67, 0, 0, 236, 67, 0,
                    0, 165, 67, 0, 0, 203, 67, 0, 128, 211, 67, 0, 0, 0, 0, 0, 0, 119, 67, 0, 128,
                    211, 67, 0, 0, 165, 67, 0, 0, 119, 67, 0, 0, 236, 67, 0, 0, 165, 67, 0, 0, 203,
                    67, 0, 0, 236, 67, 0, 0, 0, 0, 0, 0, 203, 67, 0, 0, 236, 67, 0, 0, 0, 0, 0, 0,
                    203, 67, 0, 0, 236, 67, 0, 0, 165, 67, 0, 0, 203, 67, 0, 0, 157, 67, 0, 0, 165,
                    67, 0, 0, 228, 67, 0, 0, 157, 67, 0, 0, 0, 0, 0, 0, 228, 67, 0, 0, 157, 67, 0,
                    0, 0, 0, 0, 0, 228, 67, 0, 0, 157, 67, 0, 0, 165, 67, 0, 0, 228, 67, 0, 128,
                    132, 67, 0, 0, 165, 67, 0, 0, 148, 67, 0, 128, 132, 67, 0, 0, 0, 0, 0, 0, 148,
                    67, 0, 128, 132, 67, 0, 0, 0, 0, 0, 0, 148, 67, 0, 128, 132, 67, 0, 0, 165, 67,
                    0, 0, 148, 67, 0, 128, 211, 67, 0, 0, 165, 67, 0, 0, 119, 67, 0, 128, 211, 67,
                    0, 0, 0, 0, 0, 0, 119, 67, 0, 0, 236, 67, 0, 0, 0, 0, 0, 0, 203, 67, 0, 0, 157,
                    67, 0, 0, 0, 0, 0, 0, 228, 67, 0, 128, 132, 67, 0, 0, 0, 0, 0, 0, 148, 67, 0,
                    128, 211, 67, 0, 0, 0, 0, 0, 0, 119, 67,
                ],
            );
        }

        print!("{:?}", map);
        build_blas(16);
    }
}
