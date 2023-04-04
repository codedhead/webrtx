#ifndef _WEBRTX_GEOM_
#define _WEBRTX_GEOM_

// clang-format off
#include "layout.glsl"
#include "common.glsl"
// clang-format on

// TODO: share with bvh builder
const uint GEOM_TYPE_TRIANGLE = 0u;
const uint GEOM_TYPE_AABB = 1u;

// Note: this is included so that user_prelude knows the type
// one for each geometry
struct BvhGeometryDescriptor {
  // vBuffer stores either vertex positions, or AABBs
  uint vBufferIndex;
  // only for triangles, if indices are used (>=0), first look up this indices
  // buffer
  int iBufferIndex;
  uint vboOffset;
  uint vboStride;
  uint vioOffset;
  uint vioStride;
  // note that geometry info are duplicated for vbo and vio buffer
  uint owningGeometryType_todo_deprecate;
  uint owningGeometryFlags;  // Geometry.OPAQUE etc
};

// this can be a uniform?
const BvhGeometryDescriptor bvhReferencedGeomBuffer[] =
    _CRT_USER_BVH_GEOM_BUFFERS_INITIALIZER_LIST;

// each buffer is described by BvhGeometryDescriptor

// from application
// TODO: looks like array of buffers not supported??
// layout(set = RT_RESOURCES_BIND_SET,
//        binding = BP_GEOM_BUFFERS_START) readonly buffer BvhGeomBufferGeneral
//        {
//   float fword[];
// }
// bvhGeoBuffers[USER_NUM_TOTAL_GEOMETRY_RELATED_BUFFERS];
// #define GET_VEC3_FROM_BUFFER(bi, wordi)        \
//   vec3(bvhGeoBuffers[(bi)].fword[(wordi)],     \
//        bvhGeoBuffers[(bi)].fword[(wordi) + 1], \
//        bvhGeoBuffers[(bi)].fword[(wordi) + 2])

#define DEFINE_GEO_BUFFER(i)                                         \
  const uint BP_GEOM_BUFFERS_##i = BP_GEOM_BUFFERS_START + i;        \
  layout(set = RT_RESOURCES_BIND_SET, binding = BP_GEOM_BUFFERS_##i) \
      readonly buffer BvhGeomBuffer_##i {                            \
    float fword[];                                                   \
  }                                                                  \
  bvhGeoBuffers_##i

#define DEFINE_GEO_BUFFER_x1 DEFINE_GEO_BUFFER(0)
#define DEFINE_GEO_BUFFER_x2 \
  DEFINE_GEO_BUFFER_x1;      \
  DEFINE_GEO_BUFFER(1)
#define DEFINE_GEO_BUFFER_x3 \
  DEFINE_GEO_BUFFER_x2;      \
  DEFINE_GEO_BUFFER(2)
#define DEFINE_GEO_BUFFER_x4 \
  DEFINE_GEO_BUFFER_x3;      \
  DEFINE_GEO_BUFFER(3)
#define DEFINE_GEO_BUFFER_x5 \
  DEFINE_GEO_BUFFER_x4;      \
  DEFINE_GEO_BUFFER(4)
#define DEFINE_GEO_BUFFER_x6 \
  DEFINE_GEO_BUFFER_x5;      \
  DEFINE_GEO_BUFFER(5)
#define DEFINE_GEO_BUFFER_x7 \
  DEFINE_GEO_BUFFER_x6;      \
  DEFINE_GEO_BUFFER(6)
#define DEFINE_GEO_BUFFER_x8 \
  DEFINE_GEO_BUFFER_x7;      \
  DEFINE_GEO_BUFFER(7)

#define _GET_VEC3_FROM_BUFFER(bi, wordi)      \
  vec3(bvhGeoBuffers_##bi.fword[(wordi)],     \
       bvhGeoBuffers_##bi.fword[(wordi) + 1], \
       bvhGeoBuffers_##bi.fword[(wordi) + 2])

#define _GET_FROM_BUFFER_CASE(bi, wordi) \
  case bi:                               \
    return _GET_VEC3_FROM_BUFFER(bi, (wordi));

#define _GET_FROM_BUFFER_CASE_x1(wordi) _GET_FROM_BUFFER_CASE(0, wordi)

#define _GET_FROM_BUFFER_CASE_x2(wordi) \
  _GET_FROM_BUFFER_CASE_x1(wordi) _GET_FROM_BUFFER_CASE(1, wordi)

#define _GET_FROM_BUFFER_CASE_x3(wordi) \
  _GET_FROM_BUFFER_CASE_x2(wordi) _GET_FROM_BUFFER_CASE(2, wordi)

#define _GET_FROM_BUFFER_CASE_x4(wordi) \
  _GET_FROM_BUFFER_CASE_x3(wordi) _GET_FROM_BUFFER_CASE(3, wordi)

#define _GET_FROM_BUFFER_CASE_x5(wordi) \
  _GET_FROM_BUFFER_CASE_x4(wordi) _GET_FROM_BUFFER_CASE(4, wordi)

#define _GET_FROM_BUFFER_CASE_x6(wordi) \
  _GET_FROM_BUFFER_CASE_x5(wordi) _GET_FROM_BUFFER_CASE(5, wordi)

#define _GET_FROM_BUFFER_CASE_x7(wordi) \
  _GET_FROM_BUFFER_CASE_x6(wordi) _GET_FROM_BUFFER_CASE(6, wordi)

#define _GET_FROM_BUFFER_CASE_x8(wordi) \
  _GET_FROM_BUFFER_CASE_x7(wordi) _GET_FROM_BUFFER_CASE(7, wordi)

_CRT_USER_DEFINE_GEO_BUFFERS;

vec3 GET_VEC3_FROM_BUFFER(uint geoBufferIndex, uint wordIndex) {
  switch (geoBufferIndex) { _CRT_USER_GEO_BUFFERS_ACCESSOR_CASES(wordIndex) }
  return vec3(0);
}

uvec3 getTriVertIndices(uint iBufferIndex, uint offset, uint stride,
                        uint primitiveId) {
  uint vioWordOffset =
      (offset + primitiveId * stride) / 4;  // byte offset => word offset
  vec3 f3 = GET_VEC3_FROM_BUFFER(iBufferIndex, vioWordOffset);
  // return floatBitsToUint(f3);
  return uvec3(floatBitsToUint(f3.x), floatBitsToUint(f3.y),
               floatBitsToUint(f3.z));
}

vec3 getTriVertPosition(uint vBufferIndex, uint offset, uint stride,
                        uint vindex) {
  uint vboWordOffset =
      (offset + vindex * stride) / 4;  // byte offset => word offset
  return GET_VEC3_FROM_BUFFER(vBufferIndex, vboWordOffset);
}

vec3[3] getTriangleVertexPositions(BvhGeometryDescriptor g, uint primitiveId) {
  uvec3 indices;
  if (g.iBufferIndex >= 0) {
    indices = getTriVertIndices(g.iBufferIndex, g.vioOffset, g.vioStride,
                                primitiveId);
  } else {
    indices = uvec3(primitiveId * 3, primitiveId * 3 + 1, primitiveId * 3 + 2);
  }
  return vec3[](
      getTriVertPosition(g.vBufferIndex, g.vboOffset, g.vboStride, indices[0]),
      getTriVertPosition(g.vBufferIndex, g.vboOffset, g.vboStride, indices[1]),
      getTriVertPosition(g.vBufferIndex, g.vboOffset, g.vboStride, indices[2]));
}

AABB getGeometryAabb(BvhGeometryDescriptor g) {
  // aabb geometry only contains single primitive, primitiveId = 0
  const vec3 min =
      getTriVertPosition(g.vBufferIndex, g.vboOffset, g.vboStride, 0);
  const vec3 max =
      getTriVertPosition(g.vBufferIndex, g.vboOffset, g.vboStride, 1);
  AABB aabb = {min, max};
  return aabb;
}

#endif // _WEBRTX_GEOM_