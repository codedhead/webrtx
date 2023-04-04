// prelude can only be included only once

const uint _CRT_USER_NEXT_UNUSED_BIND_SET = 0;

// Calulated from all rint/rchit/rahit shaders. Minimum is 2.
const uint _CRT_HIT_ATTRIBUTES_MAX_WORDS = 5;

#define _CRT_USER_BVH_GEOM_BUFFERS_INITIALIZER_LIST \
  {                                                 \
    { 0, 0, 0, 0, 0, 0, 0, 0 }                      \
  }

#define _CRT_USER_DEFINE_GEO_BUFFERS DEFINE_GEO_BUFFER_x1
#define _CRT_USER_GEO_BUFFERS_ACCESSOR_CASES(wordIndex) \
  _GET_FROM_BUFFER_CASE_x1(wordIndex)

// DUMMY: no actual shader functions
#define _CRT_USER_RMISS_TABLE
#define _CRT_USER_RGEN_TABLE
#define _CRT_USER_RINT_TABLE
#define _CRT_USER_RCHIT_TABLE
#define _CRT_USER_RAHIT_TABLE