#ifndef _WEBRTX_TRACE_
#define _WEBRTX_TRACE_

// clang-format off
#include "common.glsl"
#include "geom.glsl"
#include "intersect.glsl"
#include "layout.glsl"
#include "sbt.glsl"
// clang-format on

layout(std430, set = RT_RESOURCES_BIND_SET,
       binding = BP_TLAS_BVH_TREE_NODES) readonly buffer TlasBvhTreeNodes {
  TlasBvhNode tlasBvhTreeNodes[];
};

layout(std430, set = RT_RESOURCES_BIND_SET,
       binding = BP_BLASES_BVH_TREE_NODES) readonly buffer BlasesBvhTreeNodes {
  BlasBvhNode blasesBvhTreeNodes[];
};

#define terminateRayEXT                                    \
  _CRT_INOUT_PARAM_HIT_REPORT = _CRT_HIT_REPORT_TERMINATE; \
  return

// Any shader with an incoming ray payload, incoming callable data, or hit
// attribute must only declare one variable of that type.

// TODO: add user_exception(e)
#define TRACE_RAY_ASSERT(b) \
  if (!(b)) {               \
    return;                 \
  }

#define FLOAT_EQUAL(v, expect) (abs((v) - (expect)) < 0.001)

const float RAY_TMAX = 1e38f;
const uint GEOMETRY_OPAQUE_BIT = 1;

const uint TRAVERSE_MAX_INT = 0xffffffffu;

void traceRayEXT(AccelerationStructureEXT topLevel, uint rayFlags,
                 uint cullMask,
                 uint rayType,      // a.k.a sbtRecordOffset,
                 uint numRayTypes,  // a.k.a sbtRecordStride,
                 uint missIndex, const vec3 _crt_WorldRayOriginEXT,
                 const float _crt_RayTminEXT,
                 const vec3 _crt_WorldRayDirectionEXT, float _crt_RayTmaxEXT,
                 int payload) {
  // float wgrt_RayTmaxEXT = RAY_TMAX;
  // uint wgrt_HitKindEXT = 0;
  vec3 invWorldRayDir = 1.0 / _crt_WorldRayDirectionEXT;

  float buf_closestHitAttributes[_CRT_HIT_ATTRIBUTES_MAX_WORDS];

  uint closestSbtIndex = 0;
  int localGeometryId = -1;
  uint localPrimitiveId = 0;
  uint _cur = 0;  // topLevel.handle
  while (_cur < TRAVERSE_MAX_INT) {
    TlasBvhNode node = tlasBvhTreeNodes[_cur];
    if ((node.is_leaf > 0 && (node.mask & cullMask) == 0) ||
        !intersect_aabb(_crt_WorldRayOriginEXT, invWorldRayDir, _crt_RayTminEXT,
                        _crt_RayTmaxEXT, node.aabb)) {
      _cur = node.exit_index;
      continue;
    }
    if (node.is_leaf == 0) {  // interior node
      // +offset if in blas tree
      _cur = node.entry_index;
      continue;
    }

    // TLAS leaf
    uint sbtInstanceOffset = node.sbtInstanceOffset;
    uint blas_geometry_id_offset = node.blas_geometry_id_offset;
    // mat4x3 _crt_ObjectToWorldEXT = node.transformToWorld;
    // mat4x3 _crt_WorldToObjectEXT = node.transformToObject;
    // TODO: https://bugs.chromium.org/p/tint/issues/detail?id=1049
    mat4x3 _crt_ObjectToWorldEXT =
        mat4x3(vec3(node.transformToWorld[0], node.transformToWorld[1],
                    node.transformToWorld[2]),
               vec3(node.transformToWorld[3], node.transformToWorld[4],
                    node.transformToWorld[5]),
               vec3(node.transformToWorld[6], node.transformToWorld[7],
                    node.transformToWorld[8]),
               vec3(node.transformToWorld[9], node.transformToWorld[10],
                    node.transformToWorld[11]));
    mat4x3 _crt_WorldToObjectEXT =
        mat4x3(vec3(node.transformToObject[0], node.transformToObject[1],
                    node.transformToObject[2]),
               vec3(node.transformToObject[3], node.transformToObject[4],
                    node.transformToObject[5]),
               vec3(node.transformToObject[6], node.transformToObject[7],
                    node.transformToObject[8]),
               vec3(node.transformToObject[9], node.transformToObject[10],
                    node.transformToObject[11]));
    // // transpose
    // mat3x4 _crt_ObjectToWorld3x4EXT;
    // mat3x4 _crt_WorldToObject3x4EXT;
    vec3 _crt_ObjectRayOriginEXT =
        _crt_WorldToObjectEXT * vec4(_crt_WorldRayOriginEXT, 1.0);
    vec3 _crt_ObjectRayDirectionEXT =
        _crt_WorldToObjectEXT * vec4(_crt_WorldRayDirectionEXT, 0.0);
    vec3 invObjectRayDir = 1.0 / _crt_ObjectRayDirectionEXT;
    // reset previous instanceId?

    // entering blas tree
    _cur = node.entry_index;
    uint blas_index_offset = _cur;
    uint instance_exit_index = node.exit_index;
    while (_cur < TRAVERSE_MAX_INT) {
      BlasBvhNode node = blasesBvhTreeNodes[_cur];
      // TODO: skip blas root if instance transform matrix is absent or identity
      if (!intersect_aabb(_crt_ObjectRayOriginEXT, invObjectRayDir,
                          _crt_RayTminEXT, _crt_RayTmaxEXT, node.aabb)) {
        if (node.exit_index ==
            TRAVERSE_MAX_INT) {  // leaving blas into tlas tree
          _cur = instance_exit_index;
          break;
        } else {
          _cur = node.exit_index + blas_index_offset;
        }
        continue;
      } else if (node.geometryId < 0) {  // interior node
        _cur = node.entry_index_or_primitive_id + blas_index_offset;
        continue;
      }

      float buf_hitAttributes[_CRT_HIT_ATTRIBUTES_MAX_WORDS];
      float t = _crt_RayTminEXT - 1.0;
      uint hitKind = 0;
      // vBufferIndex always point to the geometry
      BvhGeometryDescriptor g =
          bvhReferencedGeomBuffer[node.geometryId + blas_geometry_id_offset];
      uint sbtIndex = sbtHitGroupIndex(sbtInstanceOffset, node.geometryId,
                                       numRayTypes, rayType);
      uint terminate_or_ignore = _CRT_HIT_REPORT_IGNORE;
      // TODO: per spec, all instances in the leaf node should contain same
      // geom type? if (node.geometryType == GEOM_TYPE_TRIANGLE) {
      bool hit = false;
      if (g.owningGeometryType_todo_deprecate == GEOM_TYPE_TRIANGLE) {
        vec3 positions[3] =
            getTriangleVertexPositions(g, node.entry_index_or_primitive_id);
        vec3 n;
        // TODO: use object ray instead?
        hit = intersect_triangle_branchless(
            _crt_ObjectRayOriginEXT, _crt_RayTminEXT,
            _crt_ObjectRayDirectionEXT, _crt_RayTmaxEXT, positions[0],
            positions[1], positions[2], n, t, buf_hitAttributes[0],
            buf_hitAttributes[1]);
        if (hit) {
          n = normalize((n * _crt_WorldToObjectEXT).xyz);
          buf_hitAttributes[2] = n.x;
          buf_hitAttributes[3] = n.y;
          buf_hitAttributes[4] = n.z;
          hitKind = dot(n, _crt_WorldRayDirectionEXT) > 0
                        ? gl_HitKindFrontFacingTriangleEXT
                        : gl_HitKindBackFacingTriangleEXT;
        }
      } else {
        // skip duplicated AABB test if containing only one primitive
        // TODO: or always skip this aabb test?
        // TODO: aabb test tmax
        // if (node.numPrimitives == 1 ||
        //     intersect_aabb(_crt_WorldRayOriginEXT, invRayDir,
        //     getGeometryAabb(g))) {
        hit = invokeShaderIndirect_intersect(
            sbtIndex, _crt_WorldRayOriginEXT, _crt_RayTminEXT,
            _crt_WorldRayDirectionEXT, _crt_RayTmaxEXT, _crt_ObjectRayOriginEXT,
            t, _crt_ObjectRayDirectionEXT, _crt_WorldToObjectEXT,
            _crt_ObjectToWorldEXT, node.geometryId,
            node.entry_index_or_primitive_id, buf_hitAttributes);
        // }
      }
      // TODO: invoke more directly with identifier
      // TODO: make sure hitT/rayTmax is correct here
      if (hit) {
        terminate_or_ignore = invokeShaderIndirect_anyHit(
            sbtIndex, _crt_WorldRayOriginEXT, _crt_RayTminEXT,
            _crt_WorldRayDirectionEXT, t /* _crt_RayTmaxEXT */, node.geometryId,
            node.entry_index_or_primitive_id, buf_hitAttributes);
      }

      if (terminate_or_ignore == _CRT_HIT_REPORT_TERMINATE) {
        // TODO: need to invoke rchit?
        // _cur = TRAVERSE_MAX_INT;
        // break;
        return;
      }
      // TODO(!!): this is wrong, skipping all hits
      // opaque (or no anyhit shader)
      // if ((g.owningGeometryFlags & GEOMETRY_OPAQUE_BIT) != 0)
      if (terminate_or_ignore == _CRT_HIT_REPORT_CONFIRMED) {
        // TODO(): make them global? if not supporting recursive call
        // gl_HitKindEXT = hitKind;
        closestSbtIndex = sbtIndex;
        buf_closestHitAttributes = buf_hitAttributes;
        _crt_RayTmaxEXT = t;
        localGeometryId = node.geometryId;
        localPrimitiveId = node.entry_index_or_primitive_id;
      }

      if (node.exit_index == TRAVERSE_MAX_INT) {
        // leaving blas into tlas tree
        _cur = instance_exit_index;
        break;
      } else {
        _cur = node.exit_index + blas_index_offset;
      }
    }
  }

  // TODO: rchit should select ray payload based on the index
  if (localGeometryId >= 0) {
    invokeShaderIndirect_closestHit(closestSbtIndex, _crt_WorldRayOriginEXT,
                                    _crt_RayTminEXT, _crt_WorldRayDirectionEXT,
                                    _crt_RayTmaxEXT, localGeometryId,
                                    localPrimitiveId, buf_closestHitAttributes);
  } else {
    invokeShaderIndirect_rayMiss(sbtRayMissIndex(missIndex),
                                 _crt_WorldRayDirectionEXT);
  }
}

#endif // _WEBRTX_TRACE_