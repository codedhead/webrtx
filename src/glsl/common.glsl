#ifndef _WEBRTX_COMMON_
#define _WEBRTX_COMMON_

struct AABB {
  vec3 min;
  vec3 max;
};

struct Ray {
  vec3 origin;
  vec3 direction;
  float tmin;
  float tmax;
};

// #define WEBRTX_SHADER_UNUSED (~0U)
#define WEBRTX_SHADER_UNUSED (0xffU)

struct TlasBvhNode {
  AABB aabb;  // 2*3*4
  // for interior node, this is the offset in bvhTree for the second child
  // for TLAS leaf node, this is the offset for the single BLAS referenced by
  // current instance
  //   TODO: can TLAS leaf node contain more than one BLAS when tree height is
  //   restricted?
  //   TODO: can TLAS be nested (but should not matter? since TLAS leaf has no
  //   assumption, just set the offset for next node)
  //      looks like it can
  //      https://renderdoc.org/vkspec_chunked/chap37.html#VkAccelerationStructureInstanceKHR
  // for BLAS leaf node, this is the offset for the primitive (either triangle
  // or AABB) in global indices/aabbs buffer(s)
  uint entry_index;  // _or_primitive_id
  uint exit_index;
  // uint axis;

  // >0: leaf
  uint is_leaf;

  // leaf data
  uint mask;
  uint flags;              // TODO: INSTANCE_FORCE_OPAQUE_BIT_KHR
  uint instanceId;         // used for gl_InstanceId
  uint sbtInstanceOffset;  // The start hitGroupId for all
                           // geoms within this instance
  int instanceCustomIndex;
  // mat4x3 transformToWorld;  // column major
  // mat4x3 transformToObject;
  // TODO: https://bugs.chromium.org/p/tint/issues/detail?id=1049
  float transformToWorld[12];  // column major
  float transformToObject[12];

  // For traversal
  uint blas_geometry_id_offset;
};

struct BlasBvhNode {
  AABB aabb;  // 2*3*4
  uint entry_index_or_primitive_id;
  uint exit_index;
  // uint axis;

  // geometryId >= 0: BLAS leaf
  // else: interior
  int geometryId;
  // TODO: geometry type? flags
};

#endif  // _WEBRTX_COMMON_