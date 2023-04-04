#ifndef _WEBRTX_INTERSECT_
#define _WEBRTX_INTERSECT_

#include "common.glsl"

// https://tavianator.com/2011/ray_box.html
// https://medium.com/@bromanz/another-view-on-the-classic-ray-aabb-intersection-algorithm-for-bvh-traversal-41125138b525
bool intersect_aabb(const vec3 rayOrigin, const vec3 invRayDir,
                    const float ray_tmin, const float ray_tmax,
                    const AABB aabb) {
  vec3 t0 = (aabb.min - rayOrigin) * invRayDir;
  vec3 t1 = (aabb.max - rayOrigin) * invRayDir;
  vec3 tmin = min(t0, t1);
  vec3 tmax = max(t0, t1);
  return max(ray_tmin, max(max(tmin.x, tmin.y), tmin.z)) <=
         min(ray_tmax, min(min(tmax.x, tmax.y), tmax.z));
}

/*
 * Copyright (c) 1993 - 2010 NVIDIA Corporation.  All rights reserved.
 *
 * NOTICE TO USER:
 *
 * This source code is subject to NVIDIA ownership rights under U.S. and
 * international Copyright laws.  Users and possessors of this source code
 * are hereby granted a nonexclusive, royalty-free license to use this code
 * in individual and commercial software.
 *
 * NVIDIA MAKES NO REPRESENTATION ABOUT THE SUITABILITY OF THIS SOURCE
 * CODE FOR ANY PURPOSE.  IT IS PROVIDED "AS IS" WITHOUT EXPRESS OR
 * IMPLIED WARRANTY OF ANY KIND.  NVIDIA DISCLAIMS ALL WARRANTIES WITH
 * REGARD TO THIS SOURCE CODE, INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY, NONINFRINGEMENT, AND FITNESS FOR A PARTICULAR PURPOSE.
 * IN NO EVENT SHALL NVIDIA BE LIABLE FOR ANY SPECIAL, INDIRECT, INCIDENTAL,
 * OR CONSEQUENTIAL DAMAGES, OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS
 * OF USE, DATA OR PROFITS,  WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE
 * OR OTHER TORTIOUS ACTION,  ARISING OUT OF OR IN CONNECTION WITH THE USE
 * OR PERFORMANCE OF THIS SOURCE CODE.
 *
 * U.S. Government End Users.   This source code is a "commercial item" as
 * that term is defined at  48 C.F.R. 2.101 (OCT 1995), consisting  of
 * "commercial computer  software"  and "commercial computer software
 * documentation" as such terms are  used in 48 C.F.R. 12.212 (SEPT 1995)
 * and is provided to the U.S. Government only as a commercial end item.
 * Consistent with 48 C.F.R.12.212 and 48 C.F.R. 227.7202-1 through
 * 227.7202-4 (JUNE 1995), all U.S. Government End Users acquire the
 * source code with only those rights set forth herein.
 *
 * Any use of this source code in individual and commercial software must
 * include, in the user documentation and internal comments to the code,
 * the above Disclaimer and U.S. Government End Users Notice.
 */
bool intersect_triangle_branchless(const vec3 ray_origin, const float ray_tmin,
                                   const vec3 ray_dir, const float ray_tmax,
                                   const vec3 p0, const vec3 p1, const vec3 p2,
                                   out vec3 n, out float t, out float beta,
                                   out float gamma) {
  const vec3 e0 = p1 - p0;
  const vec3 e1 = p0 - p2;
  n = cross(e1, e0);

  const vec3 e2 = (1.f / dot(n, ray_dir)) * (p0 - ray_origin);
  const vec3 i = cross(ray_dir, e2);

  beta = dot(i, e1);
  gamma = dot(i, e0);
  t = dot(n, e2);

  return ((t < ray_tmax) && (t > ray_tmin) && (beta >= 0.f) && (gamma >= 0.f) &&
          (beta + gamma <= 1));
}

bool intersect_triangle_earlyexit(Ray ray, vec3 p0, vec3 p1, vec3 p2,
                                  out vec3 n, out float t, out float beta,
                                  out float gamma) {
  vec3 e0 = p1 - p0;
  vec3 e1 = p0 - p2;
  n = cross(e0, e1);

  float v = dot(n, ray.direction);
  float r = 1.f / v;

  vec3 e2 = p0 - ray.origin;
  float va = dot(n, e2);
  t = r * va;

  // Initialize these to reduce their liveness when we leave the function
  // without computing their value.
  beta = 0;
  gamma = 0;

  if (t < ray.tmax && t > ray.tmin) {
    vec3 i = cross(e2, ray.direction);
    float v1 = dot(i, e1);
    beta = r * v1;
    if (beta >= 0.f) {
      float v2 = dot(i, e0);
      gamma = r * v2;
      n = -n;
      return ((v1 + v2) * v <= v * v && gamma >= 0.f);
    }
  }
  return false;
}

#endif // _WEBRTX_INTERSECT_