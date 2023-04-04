
#ifndef _WEBRTX_SBT_BUFFER_
#define _WEBRTX_SBT_BUFFER_

// clang-format off
#include "layout.glsl"
// clang-format on

layout(set = RT_RESOURCES_BIND_SET, binding = BP_SBT) readonly buffer SBT {
  uint _CRT_SBT_BUFFER_NAME[];
};

#endif // _WEBRTX_SBT_BUFFER_