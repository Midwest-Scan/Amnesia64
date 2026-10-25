/*
 * Copyright © 2009-2020 Frictional Games
 *
 * This file is part of Amnesia: The Dark Descent.
 *
 * Amnesia: The Dark Descent is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.

 * Amnesia: The Dark Descent is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Amnesia: The Dark Descent.  If not, see <https://www.gnu.org/licenses/>.
 */

//////////////////////////////////////////////////////////////////////////
// Bridge between the C++ engine and its Rust port (rust/, hpl2_rs.lib).
//
// Each ported module has a switch below. With the switch at 1 the C++
// functions of that module forward to Rust; at 0 the original C++ bodies
// are compiled instead (they stay in the same files for comparison).
//////////////////////////////////////////////////////////////////////////

#ifndef HPL_RUST_BRIDGE_H
#define HPL_RUST_BRIDGE_H

#ifndef HPL2_RUST_MATH
#define HPL2_RUST_MATH 1
#endif

// cString, cPreprocessParser, cLogicTimer, VideoComp
#ifndef HPL2_RUST_SYSTEM
#define HPL2_RUST_SYSTEM 1
#endif

// cVoxelMap algorithms (allocation and renderer stay in C++)
#ifndef HPL2_RUST_GENERATE
#define HPL2_RUST_GENERATE 1
#endif

// AStar, node/grid/connection helpers and state transitions (ownership stays C++)
#ifndef HPL2_RUST_AI
#define HPL2_RUST_AI 1
#endif

#include <cstddef>
#include <cstring>

#include "hpl2_rs.h"

#include "system/SystemTypes.h"
#include "math/MathTypes.h"
#include "math/MeshTypes.h"
#include "graphics/Color.h"

// System libraries needed by the Rust standard library. Listing them here
// means every project that links HPL2 picks them up automatically.
#ifdef _MSC_VER
	#pragma comment(lib, "kernel32.lib")
	#pragma comment(lib, "advapi32.lib")
	#pragma comment(lib, "ntdll.lib")
	#pragma comment(lib, "userenv.lib")
	#pragma comment(lib, "ws2_32.lib")
	#pragma comment(lib, "bcrypt.lib")
	#pragma comment(lib, "dbghelp.lib")
	#pragma comment(lib, "synchronization.lib")
#endif

namespace hpl {

	//------------------------------------------

	/**
	 * Connects Rust logging and panic reports to Log/Warning/Error, and
	 * checks that both sides agree on the size of wchar_t.
	 * Called by cEngine's constructor; safe to call more than once.
	 */
	void RustBridge_Init();

	//------------------------------------------
	// Layout checks: the Hpl* mirrors must match the engine classes exactly,
	// since arrays of engine objects are passed to Rust by pointer.

	// Numeric snapshots only: initialized members copied, never native class storage.
	static_assert(sizeof(HplAINodeGridDimensions) == 24, "AI grid dimensions layout");
	static_assert(alignof(HplAINodeGridDimensions) == alignof(float), "AI grid dimensions alignment");
	static_assert(offsetof(HplAINodeGridDimensions, minimum) == 0, "AI grid minimum offset");
	static_assert(offsetof(HplAINodeGridDimensions, maximum) == 8, "AI grid maximum offset");
	static_assert(offsetof(HplAINodeGridDimensions, map_size) == 16, "AI grid map size offset");
	// cLogicTimer numeric state:
	static_assert(sizeof(HplLogicTimerState) == 32, "logic timer state layout");
	static_assert(alignof(HplLogicTimerState) == alignof(double), "logic timer state alignment");
	static_assert(offsetof(HplLogicTimerState, local_time) == 0, "logic timer local time");
	static_assert(offsetof(HplLogicTimerState, local_time_add) == 8, "logic timer step");
	static_assert(offsetof(HplLogicTimerState, speed_mul) == 16, "logic timer speed");
	static_assert(offsetof(HplLogicTimerState, max_updates) == 24, "logic timer cap");
	static_assert(offsetof(HplLogicTimerState, update_count) == 28, "logic timer count");
	static_assert(sizeof(cVector2f) == sizeof(HplVector2f), "cVector2f layout");
	static_assert(sizeof(cVector2l) == sizeof(HplVector2l), "cVector2l layout");
	static_assert(sizeof(cVector3f) == sizeof(HplVector3f), "cVector3f layout");
	static_assert(sizeof(cVector3l) == sizeof(HplVector3l), "cVector3l layout");
	static_assert(sizeof(cMatrixf) == sizeof(HplMatrixf), "cMatrixf layout");
	static_assert(sizeof(cQuaternion) == sizeof(HplQuaternion), "cQuaternion layout");
	static_assert(offsetof(cQuaternion, w) == offsetof(HplQuaternion, w), "cQuaternion layout");
	static_assert(sizeof(cPlanef) == sizeof(HplPlanef), "cPlanef layout");
	static_assert(sizeof(cSpheref) == sizeof(HplSpheref), "cSpheref layout");
	static_assert(offsetof(cSpheref, center) == offsetof(HplSpheref, center), "cSpheref layout");
	static_assert(sizeof(cRect2f) == sizeof(HplRect2f), "cRect2f layout");
	static_assert(sizeof(cRect2l) == sizeof(HplRect2l), "cRect2l layout");
	static_assert(sizeof(cColor) == sizeof(HplColor), "cColor layout");
	static_assert(sizeof(cTriangleData) == sizeof(HplTriangleData), "cTriangleData layout");
	static_assert(offsetof(cTriangleData, facingLight) == offsetof(HplTriangleData, facing_light), "cTriangleData layout");
	static_assert(sizeof(cTriEdge) == sizeof(HplTriEdge), "cTriEdge layout");
	static_assert(offsetof(cTriEdge, invert_tri2) == offsetof(HplTriEdge, invert_tri2), "cTriEdge layout");

	//------------------------------------------
	// Value conversions (engine type <-> Hpl* mirror)

	inline HplVector2f ToRs(const cVector2f& v) { HplVector2f r = { v.x, v.y }; return r; }
	inline HplVector2l ToRs(const cVector2l& v) { HplVector2l r = { v.x, v.y }; return r; }
	inline HplVector3f ToRs(const cVector3f& v) { HplVector3f r = { v.x, v.y, v.z }; return r; }
	inline HplVector3l ToRs(const cVector3l& v) { HplVector3l r = { v.x, v.y, v.z }; return r; }
	inline HplColor ToRs(const cColor& c) { HplColor r = { c.r, c.g, c.b, c.a }; return r; }
	inline HplPlanef ToRs(const cPlanef& p) { HplPlanef r = { p.a, p.b, p.c, p.d }; return r; }
	inline HplRect2f ToRs(const cRect2f& a) { HplRect2f r = { a.x, a.y, a.w, a.h }; return r; }
	inline HplRect2l ToRs(const cRect2l& a) { HplRect2l r = { a.x, a.y, a.w, a.h }; return r; }
	inline HplSpheref ToRs(const cSpheref& s) { HplSpheref r = { s.r, ToRs(s.center) }; return r; }
	inline HplQuaternion ToRs(const cQuaternion& q) { HplQuaternion r = { ToRs(q.v), q.w }; return r; }

	inline cVector2f FromRs(const HplVector2f& v) { return cVector2f(v.x, v.y); }
	inline cVector2l FromRs(const HplVector2l& v) { return cVector2l(v.x, v.y); }
	inline cVector3f FromRs(const HplVector3f& v) { return cVector3f(v.x, v.y, v.z); }
	inline cVector3l FromRs(const HplVector3l& v) { return cVector3l(v.x, v.y, v.z); }
	inline cColor FromRs(const HplColor& c) { return cColor(c.r, c.g, c.b, c.a); }
	inline cPlanef FromRs(const HplPlanef& p) { return cPlanef(p.a, p.b, p.c, p.d); }
	inline cRect2f FromRs(const HplRect2f& a) { return cRect2f(a.x, a.y, a.w, a.h); }
	inline cRect2l FromRs(const HplRect2l& a) { return cRect2l(a.x, a.y, a.w, a.h); }
	inline cSpheref FromRs(const HplSpheref& s) { return cSpheref(FromRs(s.center), s.r); }
	inline cQuaternion FromRs(const HplQuaternion& q) { return cQuaternion(q.w, q.v.x, q.v.y, q.v.z); }
	inline cMatrixf FromRs(const HplMatrixf& m) { cMatrixf r; r.FromVec(&m.m[0][0]); return r; }

	//------------------------------------------
	// Pointer views (no copy). Valid because of the layout checks above.

	inline const HplMatrixf* RsPtr(const cMatrixf& m) { return reinterpret_cast<const HplMatrixf*>(&m); }
	inline HplMatrixf* RsPtr(cMatrixf& m) { return reinterpret_cast<HplMatrixf*>(&m); }
	inline const HplVector3f* RsPtr(const cVector3f* v) { return reinterpret_cast<const HplVector3f*>(v); }
	inline HplVector3f* RsPtr(cVector3f* v) { return reinterpret_cast<HplVector3f*>(v); }
	inline const HplPlanef* RsPtr(const cPlanef* p) { return reinterpret_cast<const HplPlanef*>(p); }
	inline HplPlanef* RsPtr(cPlanef* p) { return reinterpret_cast<HplPlanef*>(p); }
	inline HplColor* RsPtr(cColor* c) { return reinterpret_cast<HplColor*>(c); }
	inline HplQuaternion* RsPtr(cQuaternion* q) { return reinterpret_cast<HplQuaternion*>(q); }

	/**
	 * A C++ bool member that may never have been initialised can hold any
	 * byte, which is undefined behaviour for a Rust bool. This reads it as a
	 * byte and normalises it.
	 */
	inline bool RsBool(const bool& b) { unsigned char c; memcpy(&c, &b, 1); return c != 0; }

	//------------------------------------------
	// Strings. Inputs are passed as views (pointer + length), outputs come
	// back through a sink: a callback that assigns, appends or pushes the
	// data into a C++ container. E.g.
	//   tString sOut; hpl_string_get_file_name(RsStr(sIn), RsAssign(sOut));

	inline HplStr RsStr(const tString& s) { HplStr r = { reinterpret_cast<const uint8_t*>(s.data()), s.size() }; return r; }
	inline HplWStr RsStr(const tWString& s) { HplWStr r = { s.data(), s.size() }; return r; }

	/** A C string; NULL gives a null view. */
	inline HplStr RsCStr(const char* s) { HplStr r = { reinterpret_cast<const uint8_t*>(s), s ? strlen(s) : 0 }; return r; }

	/** An optional string; NULL gives a null view. */
	inline HplStr RsStrPtr(const tString* s) { if(s) return RsStr(*s); HplStr r = { NULL, 0 }; return r; }
	inline HplWStr RsStrPtr(const tWString* s) { if(s) return RsStr(*s); HplWStr r = { NULL, 0 }; return r; }

	inline void RsSinkAssign(void* ctx, const uint8_t* d, size_t n) { static_cast<tString*>(ctx)->assign(reinterpret_cast<const char*>(d), n); }
	inline void RsSinkAppend(void* ctx, const uint8_t* d, size_t n) { static_cast<tString*>(ctx)->append(reinterpret_cast<const char*>(d), n); }
	inline void RsSinkPush(void* ctx, const uint8_t* d, size_t n) { static_cast<tStringVec*>(ctx)->push_back(tString(reinterpret_cast<const char*>(d), n)); }
	inline void RsSinkAssignW(void* ctx, const HplWChar* d, size_t n) { static_cast<tWString*>(ctx)->assign(d, n); }
	inline void RsSinkPushW(void* ctx, const HplWChar* d, size_t n) { static_cast<tWStringVec*>(ctx)->push_back(tWString(d, n)); }
	inline void RsSinkAppendInts(void* ctx, const int32_t* d, size_t n) { tIntVec* v = static_cast<tIntVec*>(ctx); v->insert(v->end(), d, d + n); }
	inline void RsSinkAppendUInts(void* ctx, const int32_t* d, size_t n) { tUIntVec* v = static_cast<tUIntVec*>(ctx); for(size_t i=0; i<n; ++i) v->push_back((unsigned int)d[i]); }
	inline void RsSinkAppendFloats(void* ctx, const float* d, size_t n) { tFloatVec* v = static_cast<tFloatVec*>(ctx); v->insert(v->end(), d, d + n); }

	inline HplStrOut RsAssign(tString& s) { HplStrOut o = { &RsSinkAssign, &s }; return o; }
	inline HplStrOut RsAppend(tString& s) { HplStrOut o = { &RsSinkAppend, &s }; return o; }
	inline HplStrOut RsPush(tStringVec& v) { HplStrOut o = { &RsSinkPush, &v }; return o; }
	inline HplWStrOut RsAssign(tWString& s) { HplWStrOut o = { &RsSinkAssignW, &s }; return o; }
	inline HplWStrOut RsPush(tWStringVec& v) { HplWStrOut o = { &RsSinkPushW, &v }; return o; }
	inline HplI32Out RsAppend(tIntVec& v) { HplI32Out o = { &RsSinkAppendInts, &v }; return o; }
	inline HplI32Out RsAppend(tUIntVec& v) { HplI32Out o = { &RsSinkAppendUInts, &v }; return o; }
	inline HplF32Out RsAppend(tFloatVec& v) { HplF32Out o = { &RsSinkAppendFloats, &v }; return o; }

	//------------------------------------------

};
#endif // HPL_RUST_BRIDGE_H
