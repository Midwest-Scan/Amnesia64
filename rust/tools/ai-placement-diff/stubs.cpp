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

// Placement dependencies construct real C++ objects and native STL containers.
// World storage/physics/BV fixtures are bounded collaborators, not replacements
// for any placement, filtering, traversal or node insertion algorithm.
#include "ai/AINodeGenerator.h"
#include "system/LowLevelSystem.h"
#include "system/String.h"
#include "system/Platform.h"
#include "resources/Resources.h"
#include "physics/PhysicsBody.h"
#include <cstdarg>
#include <cstdio>
#include <cstdlib>
#include <new>

FILE* gLogOut=stdout;
bool gTraceAlloc=false;
int gFailAllocation=-1;
void (*gBeforeAllocation)()=NULL;
hpl::iPhysicsWorld* gPhysicsWorld=NULL;

void* operator new(std::size_t size)
{
    if(gBeforeAllocation) { auto hook=gBeforeAllocation; gBeforeAllocation=NULL; hook(); }
    if(gFailAllocation==0) {
        gFailAllocation=-1;
        if(gTraceAlloc) fprintf(gLogOut,"allocation-failed %llu\n",(unsigned long long)size);
        throw std::bad_alloc();
    }
    if(gFailAllocation>0) --gFailAllocation;
    void* data=std::malloc(size ? size : 1);
    if(!data) throw std::bad_alloc();
    if(gTraceAlloc) fprintf(gLogOut,"allocate %llu\n",(unsigned long long)size);
    return data;
}
void operator delete(void* data) noexcept
{
    if(gTraceAlloc && data) fputs("delete\n",gLogOut);
    std::free(data);
}
void operator delete(void* data,std::size_t) noexcept { ::operator delete(data); }

namespace hpl {
cWorld::cWorld(tString name,cGraphics*,cResources* resources,cSound*,cPhysics*,cScene*,cSystem* system,cAI*,cHaptic*) :
    msName(name),mpResources(resources),mpSystem(system),mpPhysicsWorld(NULL) {}
cWorld::~cWorld() { STLMapDeleteAll(m_mapTempNodes); }
iPhysicsWorld* cWorld::GetPhysicsWorld() { return gPhysicsWorld; }
// Same storage accessor as World.cpp; ownership is native and per world.
tTempAiNodeList* cWorld::GetAINodeList(const tString& type)
{
    cTempNodeContainer* container;
    auto it=m_mapTempNodes.find(type);
    if(it!=m_mapTempNodes.end()) container=it->second;
    else { container=new cTempNodeContainer(); m_mapTempNodes.insert(tTempNodeContainerMap::value_type(type,container)); }
    return &container->mlstNodes;
}
cResources::cResources(iLowLevelResources*,iLowLevelGraphics*) : iUpdateable("Resources") {}
cResources::~cResources() {}
cFileSearcher* cResources::GetFileSearcher() { return NULL; }
void cResources::Update(float) { std::abort(); }
iPhysicsWorld::iPhysicsWorld() : mpWorld(NULL),mbLogDebug(false),mbSaveContactPoints(false) {}
iPhysicsWorld::~iPhysicsWorld() {}
cPhysicsBodyIterator iPhysicsWorld::GetBodyIterator() { return cPhysicsBodyIterator(&mlstBodies); }
iEntity3D::iEntity3D(tString name) : msName(name) {}
iEntity3D::~iEntity3D() {}
cBoundingVolume* iEntity3D::GetBoundingVolume() { return &mBoundingVolume; }
iPhysicsBody::iPhysicsBody(const tString& name,iPhysicsWorld* world,iCollideShape* shape) :
    iEntity3D(name),mpWorld(world),mpShape(shape) {}
iPhysicsBody::~iPhysicsBody() {}
// Geometry is supplied directly by the fixture; no transform is applied.
cBoundingVolume::cBoundingVolume() {}
const cVector3f& cBoundingVolume::GetMin() { return mvWorldMin; }
const cVector3f& cBoundingVolume::GetMax() { return mvWorldMax; }
// File I/O is outside this slice. Empty world paths keep it unreachable.
tWString cString::SetFileExtW(const tWString&,const tWString&) { std::abort(); }
tString cString::To8Char(const tWString&) { std::abort(); }
tString cString::ToString(const char*,tString) { std::abort(); }
tString cString::ToString(int,int) { std::abort(); }
tString cString::ToString(float,int,bool) { std::abort(); }
int cString::ToInt(const char*,int) { std::abort(); }
cVector3f cString::ToVector3f(const char*,const cVector3f&) { std::abort(); }
FILE* cPlatform::OpenFile(const tWString&,const tWString) { std::abort(); }
bool cPlatform::FileExists(const tWString&) { std::abort(); }
cDate cPlatform::FileModifiedDate(const tWString&) { std::abort(); }
static void Print(const char* prefix,const char* format,va_list args) { fprintf(gLogOut,"%s",prefix); vfprintf(gLogOut,format,args); }
void Error(const char* f,...) { va_list a; va_start(a,f); Print("ERROR: ",f,a); va_end(a); }
void Warning(const char* f,...) { va_list a; va_start(a,f); Print("WARNING: ",f,a); va_end(a); }
void Log(const char* f,...) { va_list a; va_start(a,f); Print("LOG: ",f,a); va_end(a); }
}
