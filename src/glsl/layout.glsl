
#ifndef _WEBRTX_RESOURCES_
#define _WEBRTX_RESOURCES_

#define RT_RESOURCES_BIND_SET _CRT_USER_NEXT_UNUSED_BIND_SET
const uint RT_UNIFORM_PARAMS_NUM_BIND_LOCATIONS = 1;

// bind points
const uint BP_RT_UNIFORM_PARAMS = 0;
const uint BP_SBT = BP_RT_UNIFORM_PARAMS + RT_UNIFORM_PARAMS_NUM_BIND_LOCATIONS;
const uint BP_TLAS_BVH_TREE_NODES = BP_SBT + 1;
const uint BP_BLASES_BVH_TREE_NODES = BP_TLAS_BVH_TREE_NODES + 1;

// user defined offsets
const uint BP_GEOM_BUFFERS_START = BP_BLASES_BVH_TREE_NODES + 1;

layout(std140, set = RT_RESOURCES_BIND_SET,
       binding = BP_RT_UNIFORM_PARAMS) uniform RtUniforms {
  uint sbtStartRayGen;
  uint sbtStartRayMiss;
  uint sbtStrideRayMiss;
  uint sbtStartHit;
  uint sbtStrideHit;
  uint sbtStartCallable;
  uint sbtStrideCallable;
  uvec3 webrtx_NumWorkGroups;
}
rtUniformParams;

// Added built-in vars
uvec3 webrtx_LaunchSizeEXT =
    rtUniformParams.webrtx_NumWorkGroups * gl_WorkGroupSize;

#endif // _WEBRTX_RESOURCES_