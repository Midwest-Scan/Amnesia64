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

// HPL2_RUST_AI selects native or Rust connection decisions in this corpus.
// Public Compile differential corpus. Graph/grid/iterator/FreePath/AddEdge and
// sorting all come from production AINodeContainer.cpp. A concrete mock physics
// world supplies visible/blocked rays to the real FreePath callback; no graph
// algorithm is replaced and no private test hooks or null-this calls are used.
// Empty grids, zero x/z query extents, unrepresentable float-to-int query casts,
// and NaN edge distances (invalid std::sort ordering) are excluded. Finite
// x/z positions, bounded query/extent ratios, and explicit anchor nodes ensure
// this. Nonfinite y is safe when filtering prevents any NaN edge from sorting.
#include "ai/AINodeContainer.h"
#include "scene/World.h"
#include "system/LowLevelSystem.h"
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <new>
#if HARNESS_RUST
#include "system/RustBridge.h"
#endif

using namespace hpl;
extern FILE* gLogOut;
extern bool gTraceAlloc;
extern int gFailAllocation;
extern iPhysicsWorld* gPhysicsWorld;
static FILE* gOut;
static cAINodeContainer* gContainer;
static cWorld* gWorld;
static uint64_t gRandom = 0x7E284C51A69D03BFull;
static uint32_t Next() { gRandom ^= gRandom << 13; gRandom ^= gRandom >> 7; gRandom ^= gRandom << 17; return (uint32_t)(gRandom >> 16); }
static int Int(int lo, int hi) { return lo + (int)(Next() % (uint32_t)(hi-lo+1)); }
static float Float(uint32_t value) { float f; memcpy(&f,&value,4); return f; }
static uint32_t Bits(float value) { uint32_t u; memcpy(&u,&value,4); return u; }
static int gRays, gBlockMode, gAction, gActionRay;
static unsigned gCompileCount, gTotalRays, gExceptionCount;
static void Compile(cAINodeContainer& c,const char* label,int sequence,int fail);

static void Position(cAINode* node, const cVector3f& value)
{
    // The original object is non-const; editing its public const reference view
    // preserves object lifetime and does not change vector/list membership.
    const_cast<cVector3f&>(node->GetPosition()) = value;
}

static void Ray(iPhysicsRayCallback* callback, const cVector3f& start, const cVector3f& end,
                bool distance, bool normal, bool point, bool prefilter)
{
    ++gRays; ++gTotalRays;
    fprintf(gOut,"ray %d %08x,%08x,%08x -> %08x,%08x,%08x flags=%d%d%d%d\n",gRays,
        Bits(start.x),Bits(start.y),Bits(start.z),Bits(end.x),Bits(end.y),Bits(end.z),distance,normal,point,prefilter);
    if(gAction && gRays == gActionRay) {
        int action = gAction; gAction = 0;
        fprintf(gOut,"ray-action %d\n",action);
        if(action == 1) {
            // AddEdge must recompute after visibility, and later candidates
            // must reread the source/candidate positions.
            Position(gContainer->GetNode(0),cVector3f(1.25f,0.5f,0.75f));
            Position(gContainer->GetNode(1),cVector3f(13.0f,2.0f,1.0f));
        } else if(action == 2) {
            gContainer->SetMaxEdgeDistance(2.0f); gContainer->SetMaxHeight(0.0f);
            gContainer->SetMaxEdges(2); gContainer->SetMinEdges(1);
        } else if(action == 3) {
            Warning("controlled visibility failure\n"); throw 1776;
        } else if(action == 4) {
            // Fifth successful ray arms the very next native edge allocation.
            // The attempted edge must remain absent, and later nodes untouched.
            gFailAllocation = 0;
        } else if(action == 5) {
            gContainer->SetMaxHeight(100.0f); gContainer->SetMaxEdgeDistance(24.0f);
            gContainer->SetMaxEdges(0); gContainer->SetMinEdges(20);
        } else if(action == 6) {
            Position(gContainer->GetNode(0),cVector3f(-1.0f,-1.0f,-1.0f));
            gContainer->SetNodeIsAtCenter(false);
        } else if(action == 7) {
            // Visibility callbacks may edit existing edges or append new ones.
            // Sorting/trimming must include this live vector, without snapshots.
            cAINode* source=gContainer->GetNode(0);
            source->AddEdge(gContainer->GetNode(3)); source->AddEdge(source);
            source->GetEdge(0)->mfDistance=0.25f; source->GetEdge(0)->mfSqrDistance=77.0f;
        } else if(action == 8) {
            // A separate container may compile recursively. Its caught native
            // exception must not poison the outer callback's exception context.
            cAINodeContainer* outer=gContainer; int outerRays=gRays, outerMode=gBlockMode;
            {
                cAINodeContainer inner("nested","node",gWorld,cVector3f(0));
                inner.AddNode("a",0,cVector3f(0)); inner.AddNode("b",1,cVector3f(1,0,1));
                inner.SetMaxEdgeDistance(4); inner.SetMaxHeight(2); inner.SetMaxEdges(0); inner.SetMinEdges(0);
                gAction=3; gActionRay=1; gBlockMode=0;
                Compile(inner,"nested",0,-1);
            }
            gContainer=outer; gRays=outerRays; gBlockMode=outerMode; gTraceAlloc=true;
        }
    }
    bool blocked = gBlockMode == 1 || (gBlockMode == 2 && ((Bits(end.x)^Bits(start.z)^(unsigned)gRays)%7 == 0))
        || (gBlockMode == 3 && gRays%5 == 0);
    fprintf(gOut,"visible %d\n",!blocked);
    // FreePath supplies no optional body callback. OnIntersect therefore sets
    // the real ray callback's intersected bit without dereferencing either arg.
    if(blocked) callback->OnIntersect(NULL,NULL);
}

class cFixturePhysics : public iPhysicsWorld
{
public:
    void CastRay(iPhysicsRayCallback* cb,const cVector3f& a,const cVector3f& b,bool d,bool n,bool p,bool f) override { Ray(cb,a,b,d,n,p,f); }
    // All non-ray operations are deliberately unavailable in this fixture.
    void Simulate(float) override { std::abort(); }
    void SetMaxTimeStep(float) override { std::abort(); }
    float GetMaxTimeStep() override { std::abort(); }
    void SetWorldSize(const cVector3f&,const cVector3f&) override { std::abort(); }
    cVector3f GetWorldSizeMin() override { std::abort(); }
    cVector3f GetWorldSizeMax() override { std::abort(); }
    void SetGravity(const cVector3f&) override { std::abort(); }
    cVector3f GetGravity() override { std::abort(); }
    void SetAccuracyLevel(ePhysicsAccuracy) override { std::abort(); }
    ePhysicsAccuracy GetAccuracyLevel() override { std::abort(); }
    void SetNumberOfThreads(int) override { std::abort(); }
    int GetNumberOfThreads() override { std::abort(); }
    iCollideShape* CreateNullShape() override { std::abort(); }
    iCollideShape* CreateBoxShape(const cVector3f&,cMatrixf*) override { std::abort(); }
    iCollideShape* CreateSphereShape(const cVector3f&,cMatrixf*) override { std::abort(); }
    iCollideShape* CreateCylinderShape(float,float,cMatrixf*) override { std::abort(); }
    iCollideShape* CreateCapsuleShape(float,float,cMatrixf*) override { std::abort(); }
    iCollideShape* CreateMeshShape(iVertexBuffer*) override { std::abort(); }
    iCollideShape* LoadMeshShapeFromBuffer(cBinaryBuffer*) override { std::abort(); }
    void SaveMeshShapeToBuffer(iCollideShape*,cBinaryBuffer*) override { std::abort(); }
    iCollideShape* CreateCompundShape(tCollideShapeVec&) override { std::abort(); }
    iCollideShape* CreateStaticSceneShape(tCollideShapeVec&,tMatrixfVec*) override { std::abort(); }
    iPhysicsJointBall* CreateJointBall(const tString&,const cVector3f&,const cVector3f&,iPhysicsBody*,iPhysicsBody*) override { std::abort(); }
    iPhysicsJointHinge* CreateJointHinge(const tString&,const cVector3f&,const cVector3f&,iPhysicsBody*,iPhysicsBody*) override { std::abort(); }
    iPhysicsJointSlider* CreateJointSlider(const tString&,const cVector3f&,const cVector3f&,iPhysicsBody*,iPhysicsBody*) override { std::abort(); }
    iPhysicsJointScrew* CreateJointScrew(const tString&,const cVector3f&,const cVector3f&,iPhysicsBody*,iPhysicsBody*) override { std::abort(); }
    iPhysicsMaterial* CreateMaterial(const tString&) override { std::abort(); }
    iPhysicsBody* CreateBody(const tString&,iCollideShape*) override { std::abort(); }
    iCharacterBody* CreateCharacterBody(const tString&,const cVector3f&) override { std::abort(); }
    void GetBodiesInBV(cBoundingVolume*,std::vector<iPhysicsBody*>*) override { std::abort(); }
    iPhysicsRope* CreateRope(const tString&,const cVector3f&,const cVector3f&) override { std::abort(); }
    iPhysicsController* CreateController(const tString&) override { std::abort(); }
    void RenderShapeDebugGeometry(iCollideShape*,const cMatrixf&,iLowLevelGraphics*,const cColor&) override { std::abort(); }
    void RenderDebugGeometry(iLowLevelGraphics*,const cColor&) override { std::abort(); }
    bool CheckShapeCollision(iCollideShape*,const cMatrixf&,iCollideShape*,const cMatrixf&,cCollideData&,int,bool) override { std::abort(); }
};

static void Add(cAINodeContainer& c,const cVector3f& position)
{
    int i = c.GetNodeNum(); char name[32]; snprintf(name,sizeof(name),"node%d",i);
    c.AddNode(name,i,position);
    fprintf(gOut,"node %d %08x,%08x,%08x\n",i,Bits(position.x),Bits(position.y),Bits(position.z));
}
static void Settings(cAINodeContainer& c,float distance,float height,int maximum,int minimum,bool center=true)
{
    c.SetMaxEdgeDistance(distance); c.SetMaxHeight(height); c.SetMaxEdges(maximum); c.SetMinEdges(minimum); c.SetNodeIsAtCenter(center);
    fprintf(gOut,"settings %08x %08x %d %d %d\n",Bits(distance),Bits(height),maximum,minimum,center);
}
static void Dump(cAINodeContainer& c)
{
    for(int i=0;i<c.GetNodeNum();++i) {
        cAINode* node = c.GetNode(i); const cVector3f& pos = node->GetPosition();
        fprintf(gOut,"result %d %08x,%08x,%08x edges=%d\n",i,Bits(pos.x),Bits(pos.y),Bits(pos.z),node->GetEdgeNum());
        for(int j=0;j<node->GetEdgeNum();++j) {
            cAINodeEdge* edge=node->GetEdge(j);
            fprintf(gOut,"edge %d %d %d %08x %08x\n",i,j,edge->mpNode->GetID(),Bits(edge->mfDistance),Bits(edge->mfSqrDistance));
        }
    }
}
static void Compile(cAINodeContainer& c,const char* label,int sequence,int fail=-1)
{
    fprintf(gOut,"compile %s %d fault=%d mode=%d action=%d/%d\n",label,sequence,fail,gBlockMode,gAction,gActionRay);
    ++gCompileCount; gContainer=&c; gRays=0; gTraceAlloc=true; gFailAllocation=fail;
    try { c.Compile(); fputs("compiled\n",gOut); }
    catch(const std::bad_alloc&) { ++gExceptionCount; fputs("exception bad_alloc\n",gOut); }
    catch(int value) { ++gExceptionCount; fprintf(gOut,"exception %d\n",value); }
    if(gFailAllocation>=0) fprintf(gOut,"allocation-fault-unconsumed %d\n",gFailAllocation);
    gFailAllocation=-1; gTraceAlloc=false;
    fprintf(gOut,"compile-end rays=%d action=%d\n",gRays,gAction);
    gAction=0; Dump(c);
}
static void Anchors(cAINodeContainer& c)
{
    Add(c,cVector3f(0,0,0)); Add(c,cVector3f(1,0,1));
    Add(c,cVector3f(30,0,30)); Add(c,cVector3f(-30,0,-30));
}

static void Fixed(cWorld& world,cFixturePhysics& physics)
{
    const float distances[]={-2.0f,-0.0f,0.0f,Float(0x3fffffffu),2.0f,Float(0x40000001u),8.0f,60.0f};
    const float heights[]={-1.0f,-0.0f,0.0f,Float(0x3fffffffu),2.0f,Float(0x40000001u),100.0f};
    const int maxima[]={-3,0,1,2,4,40}; const int minima[]={-2,0,1,2,5,40};
    for(int i=0;i<168;++i) {
        cAINodeContainer c("fixed","node",&world,cVector3f(0.8f,1.8f,0.8f));
        Anchors(c);
        Add(c,cVector3f(4,0,0)); Add(c,cVector3f(Float(0x407fffffu),0,0)); Add(c,cVector3f(Float(0x40800001u),0,0));
        Add(c,cVector3f(0,2,0)); Add(c,cVector3f(0,Float(0x3fffffffu),0)); Add(c,cVector3f(0,Float(0x40000001u),0));
        Add(c,cVector3f(0,-2,0)); Add(c,cVector3f(-0.0f,0,-0.0f)); Add(c,cVector3f(0,0,0));
        Settings(c,distances[i%8],heights[(i/8)%7],maxima[i%6],minima[(i/6)%6],i%2==0);
        gPhysicsWorld=i%9==0 ? NULL : &physics; gBlockMode=i%4;
        Compile(c,"fixed",i);
        if(i%7==0) Compile(c,"fixed-repeat",i);
    }
    // Existing edges may be negative, infinite or signed zero without violating
    // std::sort's strict weak order. NaNs are deliberately not included.
    const float weights[]={-10.0f,-0.0f,0.0f,1.0f,2.0f,Float(0x40000001u),8.0f,Float(0x7f800000u)};
    for(int i=0;i<36;++i) {
        cAINodeContainer c("existing","node",&world,cVector3f(0)); Anchors(c);
        for(int j=7;j>=0;--j) {
            c.GetNode(0)->AddEdge(c.GetNode(j%4));
            cAINodeEdge* edge=c.GetNode(0)->GetEdge(7-j); edge->mfDistance=weights[j]; edge->mfSqrDistance=(float)(101+j);
        }
        Settings(c,2,0,maxima[i%6],minima[i/6]); gPhysicsWorld=&physics; gBlockMode=i%2;
        Compile(c,"existing",i); Compile(c,"existing-repeat",i);
    }
    // Precision-sensitive distances with nonzero y/z terms and tiny/large but
    // representable grid ratios. This covers sqrt-before-comparison rounding.
    const float scales[]={0.0001f,1.0f,10000.0f};
    for(int k=0;k<3;++k) for(int i=0;i<32;++i) {
        cAINodeContainer c("precision","node",&world,cVector3f(0)); float scale=scales[k];
        Add(c,cVector3f(0)); Add(c,cVector3f(10*scale,0,10*scale));
        float x=Float(0x3f7ffff0u+i)*scale;
        Add(c,cVector3f(x,0.000244140625f*scale,0.000244140625f*scale));
        Add(c,cVector3f(-x,0.000345266977f*scale,-0.000345266977f*scale));
        Settings(c,0.5f*scale,scale,0,8); gPhysicsWorld=&physics; gBlockMode=0;
        Compile(c,"precision",k*32+i);
    }
    const uint32_t specialY[]={0x7fc01234u,0xffc05678u,0x7f800000u,0xff800000u};
    const uint32_t specialHeight[]={0x7fc01234u,0x7f800000u,0xff800000u,0x00000000u};
    for(int i=0;i<16;++i) {
        cAINodeContainer c("special-height","node",&world,cVector3f(0)); Anchors(c);
        Add(c,cVector3f(1,Float(specialY[i%4]),1));
        Add(c,cVector3f(2,Float(specialY[i%4]),2));
        Settings(c,10,Float(specialHeight[i/4]),0,10); gPhysicsWorld=&physics; gBlockMode=0;
        Compile(c,"special-height",i);
    }
}

static void Callbacks(cWorld& world,cFixturePhysics& physics)
{
    for(int action=1;action<=8;++action) for(int late=0;late<2;++late) {
        cAINodeContainer c("callback","node",&world,cVector3f(0.8f,1.8f,0.8f)); Anchors(c);
        Add(c,cVector3f(2,0,1)); Add(c,cVector3f(3,1,1)); Add(c,cVector3f(4,-1,2));
        Settings(c,10,2,3,2); gPhysicsWorld=&physics; gBlockMode=0;
        gAction=action; gActionRay=action==4 ? (late ? 10 : 5) : (late ? 7 : 1);
        Compile(c,"callback",action*2+late); Compile(c,"callback-recovery",action*2+late);
    }
    for(int fail=0;fail<40;++fail) {
        cAINodeContainer c("allocation","node",&world,cVector3f(0)); Anchors(c);
        Add(c,cVector3f(2,0,1)); Add(c,cVector3f(3,0,1));
        Settings(c,20,2,3,2); gPhysicsWorld=&physics; gBlockMode=0;
        Compile(c,"allocation",fail,fail); Compile(c,"allocation-recovery",fail);
    }
}

static void Random(cWorld& world,cFixturePhysics& physics)
{
    for(int iteration=0;iteration<2048;++iteration) {
        cAINodeContainer c("random","node",&world,cVector3f(0.625f,1.875f,0.625f));
        // Anchors ensure both query extents remain nonzero, including after
        // bounded position edits and duplicate positions in seeded cases.
        Add(c,cVector3f(-20,0,-20)); Add(c,cVector3f(20,0,20));
        int count=Int(3,14);
        for(int j=2;j<count;++j) {
            cVector3f p((float)Int(-160,160)*0.125f,(float)Int(-24,24)*0.125f,(float)Int(-160,160)*0.125f);
            if(j>2 && j%5==0) p=c.GetNode(j-1)->GetPosition();
            Add(c,p);
        }
        float distance=(float)Int(0,240)*0.125f; float height=(float)Int(-1,24)*0.125f;
        Settings(c,distance,height,Int(-2,10),Int(-2,12),iteration%2==0);
        gPhysicsWorld=iteration%13==0 ? NULL : &physics; gBlockMode=iteration%4;
        if(iteration%3==0) {
            // Pre-existing self/duplicate edges are accepted by AddEdge. Compile
            // appends rather than clearing them, even with zero/new visibility.
            c.GetNode(0)->AddEdge(c.GetNode(0)); c.GetNode(0)->AddEdge(c.GetNode(count-1));
            c.GetNode(0)->AddEdge(c.GetNode(count-1));
        }
        Compile(c,"random",iteration);
        if(iteration%8==0) {
            Position(c.GetNode(2),cVector3f(0.25f,0.5f,-0.75f));
            Settings(c,distance*0.5f,height+0.5f,iteration%7,iteration%5);
            Compile(c,"random-repeat",iteration);
        }
    }
}

int main(int argc,char** argv)
{
    if(argc!=2) return 2;
    gOut=fopen(argv[1],"wb"); if(!gOut) return 3; gLogOut=gOut;
#if HARNESS_RUST
    RustBridge_Init();
#endif
    {
        cWorld world("fixture",NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL);
        gWorld=&world;
        cFixturePhysics physics;
        Fixed(world,physics); Callbacks(world,physics); Random(world,physics);
    }
    fprintf(gOut,"totals compiles=%u rays=%u exceptions=%u\n",gCompileCount,gTotalRays,gExceptionCount);
    fclose(gOut); return 0;
}
