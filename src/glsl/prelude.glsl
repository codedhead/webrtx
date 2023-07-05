// prelude is guaranteed to be included only once

// TODO: see constant in .ts
layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

#define gl_RayFlagsNoneEXT (0U)
#define gl_RayFlagsOpaqueEXT (1U)
#define gl_RayFlagsNoOpaqueEXT (2U)
#define gl_RayFlagsTerminateOnFirstHitEXT (4U)
#define gl_RayFlagsSkipClosestHitShaderEXT (8U)
#define gl_RayFlagsCullBackFacingTrianglesEXT (16U)
#define gl_RayFlagsCullFrontFacingTrianglesEXT (32U)
#define gl_RayFlagsCullOpaqueEXT (64U)
#define gl_RayFlagsCullNoOpaqueEXT (128U)

#define gl_HitKindFrontFacingTriangleEXT (0xFEU)
#define gl_HitKindBackFacingTriangleEXT (0xFFU)

// Added built-in vars
uvec3 webrtx_LaunchIDEXT = gl_GlobalInvocationID;
#define gl_LaunchIDEXT webrtx_LaunchIDEXT
// see webrtx_LaunchSizeEXT in layout.glsl
#define gl_LaunchSizeEXT webrtx_LaunchSizeEXT

#define AccelerationStructureEXT uvec2

// shared names
#define _CRT_PARAM_HIT_ATTRIBUTES _crt_hattrs
#define _CRT_PARAM_SHADER_RECORD_WORD_OFFSET _crt_sr_wd_offset
#define _CRT_INOUT_PARAM_HIT_REPORT _crt_hit_report
#define _CRT_SBT_BUFFER_NAME _crt_sbt_buf
#define _CRT_POTENTIAL_HIT_T _crt_potential_hit_t

//! // uintBitsToFloat(uvec3) seems not working anymore
#define UNPACK_VEC3_FROM_UINT_BUFFER(buf_name, word_offset) \
  vec3(uintBitsToFloat(buf_name[(word_offset)]),            \
       uintBitsToFloat(buf_name[(word_offset) + 1]),        \
       uintBitsToFloat(buf_name[(word_offset) + 2]))

#define UNPACK_VEC3_FROM_SBT_BUFFER(buf_name, word_offset) \
  UNPACK_VEC3_FROM_UINT_BUFFER(                            \
      buf_name, (_CRT_PARAM_SHADER_RECORD_WORD_OFFSET + (word_offset)))

#define UNPACK_VEC2_FROM_UINT_BUFFER(buf_name, word_offset) \
  vec2(uintBitsToFloat(buf_name[(word_offset)]),            \
       uintBitsToFloat(buf_name[(word_offset) + 1]))

#define UNPACK_VEC2_FROM_SBT_BUFFER(buf_name, word_offset) \
  UNPACK_VEC2_FROM_UINT_BUFFER(                            \
      buf_name, (_CRT_PARAM_SHADER_RECORD_WORD_OFFSET + (word_offset)))

#define UNPACK_VEC4_FROM_UINT_BUFFER(buf_name, word_offset) \
  vec4(uintBitsToFloat(buf_name[(word_offset)]),            \
       uintBitsToFloat(buf_name[(word_offset) + 1]),        \
       uintBitsToFloat(buf_name[(word_offset) + 2]),        \
       uintBitsToFloat(buf_name[(word_offset) + 3]))

#define UNPACK_VEC4_FROM_SBT_BUFFER(buf_name, word_offset) \
  UNPACK_VEC4_FROM_UINT_BUFFER(                            \
      buf_name, (_CRT_PARAM_SHADER_RECORD_WORD_OFFSET + (word_offset)))

#define UNPACK_FLOAT_FROM_UINT_BUFFER(buf_name, word_offset) \
  uintBitsToFloat(buf_name[(word_offset)])

#define UNPACK_FLOAT_FROM_SBT_BUFFER(buf_name, word_offset) \
  UNPACK_FLOAT_FROM_UINT_BUFFER(                            \
      buf_name, (_CRT_PARAM_SHADER_RECORD_WORD_OFFSET + (word_offset)))

#define UNPACK_MAT4_FROM_UINT_BUFFER(buf_name, word_offset)       \
  mat4(UNPACK_VEC4_FROM_UINT_BUFFER(buf_name, (word_offset)),     \
       UNPACK_VEC4_FROM_UINT_BUFFER(buf_name, (word_offset) + 4), \
       UNPACK_VEC4_FROM_UINT_BUFFER(buf_name, (word_offset) + 8), \
       UNPACK_VEC4_FROM_UINT_BUFFER(buf_name, (word_offset) + 12))

#define UNPACK_MAT4_FROM_SBT_BUFFER(buf_name, word_offset) \
  UNPACK_MAT4_FROM_UINT_BUFFER(                            \
      buf_name, (_CRT_PARAM_SHADER_RECORD_WORD_OFFSET + (word_offset)))

#define UNPACK_FLOAT_FROM_FLOAT_BUFFER(buf_name, word_offset) \
  (buf_name[(word_offset)])

#define UNPACK_VEC2_FROM_FLOAT_BUFFER(buf_name, word_offset) \
  vec2(buf_name[(word_offset)], buf_name[(word_offset) + 1])

#define UNPACK_VEC3_FROM_FLOAT_BUFFER(buf_name, word_offset) \
  vec3(buf_name[(word_offset)], buf_name[(word_offset) + 1], \
       buf_name[(word_offset) + 2])

#define PACK_VEC2_INTO_FLOAT_BUFFER(buf_name, word_offset, data) \
  buf_name[(word_offset)] = (data)[0];                           \
  buf_name[(word_offset) + 1] = (data)[1];

#define PACK_VEC3_INTO_FLOAT_BUFFER(buf_name, word_offset, data) \
  buf_name[(word_offset)] = (data)[0];                           \
  buf_name[(word_offset) + 1] = (data)[1];                       \
  buf_name[(word_offset) + 2] = (data)[2];

#define _ge(a, b) step((b), (a))
#define _gt(a, b) (1.0 - step((a), (b)))

#define _CRT_HIT_REPORT_CONFIRMED 0
#define _CRT_HIT_REPORT_IGNORE 1
#define _CRT_HIT_REPORT_TERMINATE 2

// TODO: compile and group hit releated shaders (rint, rchit, rahit) together!!!
// so that control flow, e.g. terminateRayEXT easier
