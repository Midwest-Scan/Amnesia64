/* Stand-in for SDL_stdinc.h: String.cpp only needs Uint32 and
   SDL_COMPILE_TIME_ASSERT. */
#ifndef SDL_STDINC_STUB_H
#define SDL_STDINC_STUB_H
#include <stdint.h>
typedef uint32_t Uint32;
#define SDL_COMPILE_TIME_ASSERT(name, x) typedef int SDL_compile_time_assert_##name[(x) * 2 - 1]
#endif
