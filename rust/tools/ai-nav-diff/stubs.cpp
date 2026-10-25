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

// Engine pieces outside the AI module, the same in every variant.
//
// Allocation: the original orders A* ties by cAINode address. Both the
// original nodes and the Rust-mode facades come from hplNew in AddNode, so
// with a monotonic allocator (never reusing memory) both orders equal the
// insertion order and ties resolve the same way. A general heap would make
// that order depend on earlier frees, differently per variant.
#include "system/LowLevelSystem.h"
#include "system/String.h"
#include "system/Platform.h"
#include "scene/World.h"
#include "physics/PhysicsWorld.h"
#include "math/Math.h"
#include <cstdarg>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <new>
#include <sys/mman.h>

FILE* gLogOut = stdout;
hpl::iPhysicsWorld* gPhysicsWorld = NULL;

static char* gArena = NULL;
static size_t gArenaUsed = 0;
static const size_t kArenaSize = (size_t)16 << 30; // reserved, not committed

void* operator new(std::size_t size)
{
    if(gArena == NULL) {
        void* p = mmap(NULL, kArenaSize, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS|MAP_NORESERVE, -1, 0);
        if(p == MAP_FAILED) std::abort();
        gArena = static_cast<char*>(p);
    }
    size_t aligned = (size + 15) & ~(size_t)15;
    if(aligned == 0) aligned = 16;
    if(gArenaUsed + aligned > kArenaSize) std::abort();
    void* data = gArena + gArenaUsed;
    gArenaUsed += aligned;
    return data;
}
void* operator new[](std::size_t size) { return ::operator new(size); }
void operator delete(void*) noexcept {}
void operator delete[](void*) noexcept {}
void operator delete(void*, std::size_t) noexcept {}
void operator delete[](void*, std::size_t) noexcept {}

namespace hpl {
// A real cWorld whose physics world is the fixture.
cWorld::cWorld(tString asName, cGraphics*, cResources*, cSound*, cPhysics*, cScene*, cSystem*, cAI*, cHaptic*) :
    msName(asName), mpPhysicsWorld(NULL) {}
cWorld::~cWorld() {}
iPhysicsWorld* cWorld::GetPhysicsWorld() { return gPhysicsWorld; }
iPhysicsWorld::iPhysicsWorld() : mpWorld(NULL), mbLogDebug(false), mbSaveContactPoints(false) {}
iPhysicsWorld::~iPhysicsWorld() {}

// The original Vector3Cross expression (Math.cpp is out of scope here).
cVector3f cMath::Vector3Cross(const cVector3f& a, const cVector3f& b)
{
    cVector3f result;
    result.x = a.y * b.z - a.z * b.y;
    result.y = a.z * b.x - a.x * b.z;
    result.z = a.x * b.y - a.y * b.x;
    return result;
}

// The string functions SaveToFile/LoadFromFile use. Simple, exact versions:
// what matters is that every variant reads back what it wrote.
tString cString::To8Char(const tWString& s) { tString r; for(size_t i=0;i<s.size();++i) r += (char)s[i]; return r; }
tString cString::ToString(const char* s, tString def) { return s ? tString(s) : def; }
tString cString::ToString(int x, int) { char b[32]; snprintf(b,sizeof(b),"%d",x); return b; }
tString cString::ToString(float x, int, bool) { char b[64]; snprintf(b,sizeof(b),"%.9g",x); return b; }
int cString::ToInt(const char* s, int def) { return s ? atoi(s) : def; }
float cString::ToFloat(const char* s, float def) { return s ? strtof(s, NULL) : def; }

FILE* cPlatform::OpenFile(const tWString& asFile, const tWString asMode)
{
    return fopen(cString::To8Char(asFile).c_str(), cString::To8Char(asMode).c_str());
}

static void Print(const char* prefix, const char* format, va_list args)
{
    fprintf(gLogOut, "%s", prefix);
    vfprintf(gLogOut, format, args);
}
void Error(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("ERROR: ", fmt, ap); va_end(ap); }
void Warning(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("WARNING: ", fmt, ap); va_end(ap); }
void Log(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("LOG: ", fmt, ap); va_end(ap); }
}
