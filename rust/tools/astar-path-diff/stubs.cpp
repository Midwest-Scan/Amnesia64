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

// Native AStar/path fixture observers. HPL2_RUST_AI selects production bodies.
// Objects, sets, grids and paths retain their real constructors and containers.
#include "ai/AStar.h"
#include "ai/AINodeContainer.h"
#include "system/LowLevelSystem.h"
#include "system/String.h"
#include "system/Platform.h"
#include "scene/World.h"
#include "physics/PhysicsWorld.h"
#include "math/Math.h"
#include <algorithm>
#include <cassert>
#include <cstdint>
#include <cstdarg>
#include <cstdio>
#include <cstdlib>
#include <new>
#include <type_traits>
#include <vector>

FILE* gLogOut=stdout;
hpl::iPhysicsWorld* gPhysicsWorld=NULL;
std::vector<hpl::cAStarNode*> gSearchNodes;
bool gTraceAlloc=false, gAllocateGraphNode=false;
std::size_t gFailSize=0, gHookSize=0, gLastAllocationSize=0;
int gFailCountdown=-1, gFailRequireGoal=-1;
void (*gAllocationHook)()=NULL;
void (*gSearchDeleteHook)()=NULL;
static std::aligned_storage<sizeof(hpl::cAINode),alignof(hpl::cAINode)>::type gNodeArena[128];
static unsigned gArenaNext=0, gArenaLive=0;
void ResetNodeArena() { assert(gArenaLive==0); gArenaNext=0; }

void* operator new(std::size_t size)
{
    gLastAllocationSize=size;
    if(gAllocationHook && size==gHookSize) { auto hook=gAllocationHook; gAllocationHook=NULL; hook(); }
    if(gFailCountdown>=0 && size==gFailSize) {
        // Only armed main-time faults inspect this vector. Normal global
        // allocations must work before construction and after destruction.
        bool ready=gFailRequireGoal<0;
        if(!ready) for(auto node:gSearchNodes) if(node->mpAINode->GetID()==gFailRequireGoal) ready=true;
        if(ready && gFailCountdown--==0) {
            gFailCountdown=-1; fprintf(gLogOut,"allocation-failed %llu\n",(unsigned long long)size); throw std::bad_alloc();
        }
    }
    if(gAllocateGraphNode && size==sizeof(hpl::cAINode)) {
        gAllocateGraphNode=false; assert(gArenaNext<128); ++gArenaLive;
        return &gNodeArena[gArenaNext++];
    }
    void* data=std::malloc(size ? size : 1); if(!data) throw std::bad_alloc();
    return data;
}
void operator delete(void* data) noexcept
{
    const uintptr_t p=(uintptr_t)data, begin=(uintptr_t)&gNodeArena[0], end=(uintptr_t)&gNodeArena[128];
    if(p>=begin && p<end) { assert(gArenaLive>0); --gArenaLive; return; }
    std::free(data);
}
void operator delete(void* data,std::size_t) noexcept { ::operator delete(data); }

namespace hpl {
cAllocatedPointer::cAllocatedPointer(void* data,const std::string& file,int line,size_t memory)
    : msFile(file),mlLine(line),mlMemory(memory),mpData(data) {}
void* cMemoryManager::AddPointer(const cAllocatedPointer& allocation)
{
    if(allocation.mlMemory==sizeof(cAStarNode) && allocation.msFile.find("AStar.cpp")!=std::string::npos) {
        cAStarNode* node=static_cast<cAStarNode*>(allocation.mpData);
        gSearchNodes.push_back(node); // Capacity is reserved before any query.
        if(gTraceAlloc) fprintf(gLogOut,"search-add %d\n",node->mpAINode->GetID());
    }
    return allocation.mpData;
}
bool cMemoryManager::RemovePointer(void* data,const char*,int)
{
    auto it=std::find(gSearchNodes.begin(),gSearchNodes.end(),data);
    if(it!=gSearchNodes.end()) {
        cAStarNode* node=*it;
        if(gTraceAlloc) fprintf(gLogOut,"search-delete %d\n",node->mpAINode->GetID());
        if(gSearchDeleteHook) { auto hook=gSearchDeleteHook; gSearchDeleteHook=NULL; hook(); }
        gSearchNodes.erase(it);
    }
    return true;
}
// Normally constructed dependencies, with unused operations deliberately absent.
cWorld::cWorld(tString name,cGraphics*,cResources*,cSound*,cPhysics*,cScene*,cSystem*,cAI*,cHaptic*) :
    msName(name),mpPhysicsWorld(NULL) {}
cWorld::~cWorld() {}
iPhysicsWorld* cWorld::GetPhysicsWorld() { return gPhysicsWorld; }
iPhysicsWorld::iPhysicsWorld() : mpWorld(NULL),mbLogDebug(false),mbSaveContactPoints(false) {}
iPhysicsWorld::~iPhysicsWorld() {}
cVector3f cMath::Vector3Cross(const cVector3f& a,const cVector3f& b)
{
    cVector3f result; result.x=a.y*b.z-a.z*b.y; result.y=a.z*b.x-a.x*b.z; result.z=a.x*b.y-a.y*b.x; return result;
}
tString cString::To8Char(const tWString&) { std::abort(); }
tString cString::ToString(const char*,tString) { std::abort(); }
tString cString::ToString(int,int) { std::abort(); }
tString cString::ToString(float,int,bool) { std::abort(); }
int cString::ToInt(const char*,int) { std::abort(); }
float cString::ToFloat(const char*,float) { std::abort(); }
FILE* cPlatform::OpenFile(const tWString&,const tWString) { std::abort(); }
static void Print(const char* prefix,const char* format,va_list args) { fprintf(gLogOut,"%s",prefix); vfprintf(gLogOut,format,args); }
void Error(const char* f,...) { va_list a; va_start(a,f); Print("ERROR: ",f,a); va_end(a); }
void Warning(const char* f,...) { va_list a; va_start(a,f); Print("WARNING: ",f,a); va_end(a); }
void Log(const char* f,...) { va_list a; va_start(a,f); Print("LOG: ",f,a); va_end(a); }
}
