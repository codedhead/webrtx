#ifndef _WEBRTX_SBT_
#define _WEBRTX_SBT_

// clang-format off
#include "layout.glsl"
#include "common.glsl"
#include "sbt_buffer.glsl"
// clang-format on

const uint SBT_HANDLE_SIZE_BYTES = 4u;
const uint SBT_HANDLE_SIZE_WORDS = 1u;

#define RINT_ID_MASK 0xffu
#define RCHIT_ID_MASK 0xff00u
#define RAHIT_ID_MASK 0xff0000u

#define RInt(identifier) ((identifier)&0xffu)
#define RCHit(identifier) (((identifier)&0xff00u) >> 8)
#define RAHit(identifier) (((identifier)&0xff0000u) >> 16)

// https://renderdoc.org/vkspec_chunked/chap39.html#shader-binding-table-hit-shader-indexing
uint sbtHitGroupIndex(uint instanceId, int geometryId, uint num_ray_types,
                      uint ray_type) {
  return rtUniformParams.sbtStartHit +
         rtUniformParams.sbtStrideHit *
             (instanceId + geometryId * num_ray_types + ray_type);
}

uint sbtRayMissIndex(uint ray_type) {
  return rtUniformParams.sbtStartRayMiss +
         ray_type * rtUniformParams.sbtStrideRayMiss;
}

void invokeShaderIndirect_rayGen(uint sbtByteIndex) {
  uint _CRT_PARAM_SHADER_RECORD_WORD_OFFSET = sbtByteIndex / 4;
  uint rgen = _CRT_SBT_BUFFER_NAME[_CRT_PARAM_SHADER_RECORD_WORD_OFFSET];
  // if (rgen == WEBRTX_SHADER_UNUSED) {
  //   return;
  // }
  _CRT_PARAM_SHADER_RECORD_WORD_OFFSET += SBT_HANDLE_SIZE_WORDS;
  _CRT_USER_RGEN_TABLE
}

void invokeShaderIndirect_rayMiss(uint sbtByteIndex,
                                  const vec3 _crt_WorldRayDirectionEXT) {
  uint _CRT_PARAM_SHADER_RECORD_WORD_OFFSET = sbtByteIndex / 4;
  uint rmiss = _CRT_SBT_BUFFER_NAME[_CRT_PARAM_SHADER_RECORD_WORD_OFFSET];
  if (rmiss == WEBRTX_SHADER_UNUSED) {
    return;
  }

  _CRT_PARAM_SHADER_RECORD_WORD_OFFSET += SBT_HANDLE_SIZE_WORDS;
  _CRT_USER_RMISS_TABLE
}

void invokeShaderIndirect_closestHit(
    const uint sbtByteIndex, const vec3 _crt_WorldRayOriginEXT,
    const float _crt_RayTminEXT, const vec3 _crt_WorldRayDirectionEXT,
    const float _crt_RayTmaxEXT,  // float _gl_RayTmaxEXT, uint _gl_HitKindEXT,
    const int _crt_GeometryIndexEXT, const uint _crt_PrimitiveID,
    const float _CRT_PARAM_HIT_ATTRIBUTES[_CRT_HIT_ATTRIBUTES_MAX_WORDS]
    // TODO: make these global for non-recursive ray tracing?
    // ,int gl_PrimitiveID, int gl_InstanceID, int gl_InstanceCustomIndexEXT,
    // int gl_GeometryIndexEXT
) {
  uint _CRT_PARAM_SHADER_RECORD_WORD_OFFSET = sbtByteIndex / 4;
  uint hitShaderGroupIdentifier =
      _CRT_SBT_BUFFER_NAME[_CRT_PARAM_SHADER_RECORD_WORD_OFFSET];
  uint rchit = RCHit(hitShaderGroupIdentifier);
  if (rchit == WEBRTX_SHADER_UNUSED) {
    return;
  }
  _CRT_PARAM_SHADER_RECORD_WORD_OFFSET += SBT_HANDLE_SIZE_WORDS;
  float _crt_HitTEXT = _crt_RayTmaxEXT;
  _CRT_USER_RCHIT_TABLE
}

// TODO: unify hitgroup shaders: intersect, rchit, rahit
uint invokeShaderIndirect_anyHit(
    const uint sbtByteIndex, const vec3 _crt_WorldRayOriginEXT,
    const float _crt_RayTminEXT, const vec3 _crt_WorldRayDirectionEXT,
    const float _crt_RayTmaxEXT,  //  uint _gl_HitKindEXT,
    const int _crt_GeometryIndexEXT, const uint _crt_PrimitiveID,
    const float _CRT_PARAM_HIT_ATTRIBUTES[_CRT_HIT_ATTRIBUTES_MAX_WORDS]) {
  uint _CRT_PARAM_SHADER_RECORD_WORD_OFFSET = sbtByteIndex / 4;
  uint hitShaderGroupIdentifier =
      _CRT_SBT_BUFFER_NAME[_CRT_PARAM_SHADER_RECORD_WORD_OFFSET];
  uint rahit = RAHit(hitShaderGroupIdentifier);
  if (rahit == WEBRTX_SHADER_UNUSED) {
    // assume hit confirmed by default
    return _CRT_HIT_REPORT_CONFIRMED;
  }
  _CRT_PARAM_SHADER_RECORD_WORD_OFFSET += SBT_HANDLE_SIZE_WORDS;
  // TODO: alias
  const float _crt_HitTEXT = _crt_RayTmaxEXT;
  // assume hit confirmed by default
  uint _CRT_INOUT_PARAM_HIT_REPORT = _CRT_HIT_REPORT_CONFIRMED;
  // anyhit shaders take `inout _CRT_INOUT_PARAM_HIT_REPORT`.
  _CRT_USER_RAHIT_TABLE
  return _CRT_INOUT_PARAM_HIT_REPORT;
}

// TODO: move APIs to a central place
// BUG: this is different from vk api, it's only allowed to be called once and
// does not return a value (it's a statement not expression)
// https://forums.developer.nvidia.com/t/about-two-functions-rtpotentialintersection-and-rtreportintersection/30607/8
#define reportIntersectionEXT(hit_t, hit_kind) _CRT_POTENTIAL_HIT_T = (hit_t);

bool invokeShaderIndirect_intersect(
    const uint sbtByteIndex, const vec3 _crt_WorldRayOriginEXT,
    const float _crt_RayTminEXT, const vec3 _crt_WorldRayDirectionEXT,
    const float _crt_RayTmaxEXT,  // float _gl_RayTmaxEXT, uint _gl_HitKindEXT,
    const vec3 _crt_ObjectRayOriginEXT, out float _CRT_POTENTIAL_HIT_T,
    const vec3 _crt_ObjectRayDirectionEXT, const mat4x3 _crt_WorldToObjectEXT,
    const mat4x3 _crt_ObjectToWorldEXT, const int _crt_GeometryIndexEXT,
    const uint _crt_PrimitiveID,
    out float _CRT_PARAM_HIT_ATTRIBUTES[_CRT_HIT_ATTRIBUTES_MAX_WORDS]) {
  uint _CRT_PARAM_SHADER_RECORD_WORD_OFFSET = sbtByteIndex / 4;
  uint hitShaderGroupIdentifier =
      _CRT_SBT_BUFFER_NAME[_CRT_PARAM_SHADER_RECORD_WORD_OFFSET];
  uint rint = RInt(hitShaderGroupIdentifier);
  if (rint == WEBRTX_SHADER_UNUSED) {
    return false;
  }
  _CRT_PARAM_SHADER_RECORD_WORD_OFFSET += SBT_HANDLE_SIZE_WORDS;
  _CRT_POTENTIAL_HIT_T = _crt_RayTminEXT - 1.0;
  // TODO: uint potential_hit_kind;
  // intersection shaders take `inout _CRT_INOUT_PARAM_HIT_REPORT` and `const
  // _crt_RayTmaxEXT` and `out hit_attributes` and `out _CRT_POTENTIAL_HIT_T`.

  _CRT_USER_RINT_TABLE

  return (_CRT_POTENTIAL_HIT_T > _crt_RayTminEXT &&
          _CRT_POTENTIAL_HIT_T < _crt_RayTmaxEXT);
}

#endif  // _WEBRTX_SBT_