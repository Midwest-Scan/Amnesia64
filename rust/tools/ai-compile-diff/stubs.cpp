/*
 * Copyright © 2009-2020 Frictional Games
 *
 * This file is part of Amnesia: The Dark Descent.
 *
 * Amnesia: The Dark Descent is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Amnesia: The Dark Descent is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Amnesia: The Dark Descent. If not, see <https://www.gnu.org/licenses/>.
 */

// Allocation and log traces shared by both HPL2_RUST_AI variants.
// Real node/container/iterator methods are linked from AINodeContainer.cpp.
#include "system/LowLevelSystem.h"
#include "system/String.h"
#include "system/Platform.h"
#include "scene/World.h"
#include "physics/PhysicsWorld.h"
#include "math/Math.h"
#include <cstdarg>
#include <cstdio>
#include <cstdlib>
#include <new>

FILE* gLogOut = stdout;
bool gTraceAlloc = false;
int gFailAllocation = -1;
void (*gBeforeAllocation)() = NULL;
hpl::iPhysicsWorld* gPhysicsWorld = NULL;

void* operator new(std::size_t size)
{
    if(gBeforeAllocation) { auto action=gBeforeAllocation; gBeforeAllocation=NULL; action(); }
    if(gFailAllocation == 0) {
        gFailAllocation = -1;
        if(gTraceAlloc) fprintf(gLogOut, "allocation-failed %llu\n", (unsigned long long)size);
        throw std::bad_alloc();
    }
    if(gFailAllocation > 0) --gFailAllocation;
    void* data = std::malloc(size ? size : 1);
    if(!data) throw std::bad_alloc();
    if(gTraceAlloc) fprintf(gLogOut, "allocate %llu\n", (unsigned long long)size);
    return data;
}
void operator delete(void* data) noexcept
{
    if(gTraceAlloc && data) fputs("delete\n", gLogOut);
    std::free(data);
}
void operator delete(void* data, std::size_t) noexcept { ::operator delete(data); }

namespace hpl {
// Construct actual C++ dependency objects: never call GetPhysicsWorld through a
// null/fabricated cWorld. All automatic STL/value members have normal lifetimes.
cWorld::cWorld(tString asName, cGraphics*, cResources*, cSound*, cPhysics*, cScene*, cSystem*, cAI*, cHaptic*) :
    msName(asName), mpPhysicsWorld(NULL) {}
cWorld::~cWorld() {}
iPhysicsWorld* cWorld::GetPhysicsWorld() { return gPhysicsWorld; }
iPhysicsWorld::iPhysicsWorld() : mpWorld(NULL), mbLogDebug(false), mbSaveContactPoints(false) {}
iPhysicsWorld::~iPhysicsWorld() {}

// This out-of-scope math dependency is the original Vector3Cross expression.
// FreePath itself, its five ray offsets and callback behavior remain production.
cVector3f cMath::Vector3Cross(const cVector3f& a, const cVector3f& b)
{
    cVector3f result;
    result.x = a.y * b.z - a.z * b.y;
    result.y = a.z * b.x - a.x * b.z;
    result.z = a.x * b.y - a.y * b.x;
    return result;
}
// MSVC diagnoses undefined references before unused function elimination. These
// out-of-scope dependencies must never execute in the graph/grid corpus.
tString cString::To8Char(const tWString&) { std::abort(); }
tString cString::ToString(const char*, tString) { std::abort(); }
tString cString::ToString(int, int) { std::abort(); }
tString cString::ToString(float, int, bool) { std::abort(); }
int cString::ToInt(const char*, int) { std::abort(); }
float cString::ToFloat(const char*, float) { std::abort(); }
FILE* cPlatform::OpenFile(const tWString&, const tWString) { std::abort(); }

static void Print(const char* prefix, const char* format, va_list args)
{
    fprintf(gLogOut, "%s", prefix);
    vfprintf(gLogOut, format, args);
}
void Error(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("ERROR: ", fmt, ap); va_end(ap); }
void Warning(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("WARNING: ", fmt, ap); va_end(ap); }
void Log(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("LOG: ", fmt, ap); va_end(ap); }
}
