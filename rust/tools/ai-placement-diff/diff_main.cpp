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

// HPL2_RUST_AI selects production Generate placement and OnIntersect bodies.
// Normally constructed concrete world/body objects expose public virtual seams.
// Fixed BV data and controlled rays do not replace the placement algorithm.
// Tests never invalidate a live body/node iterator or compare iterators from
// different lists. Negative/zero/nonadvancing steps run only with bounded
// callback repair, or when the original outer loop cannot execute.
#include "ai/AINodeGenerator.h"
#include "physics/PhysicsBody.h"
#include "resources/Resources.h"
#include "system/LowLevelSystem.h"
#include <cassert>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <new>
#include <vector>
#if HARNESS_RUST
#include "system/RustBridge.h"
#endif
using namespace hpl;
extern FILE* gLogOut;
extern iPhysicsWorld* gPhysicsWorld;
extern bool gTraceAlloc;
extern int gFailAllocation;
extern void (*gBeforeAllocation)();
static FILE* gOut;
static cWorld* gWorld;
static cAINodeGenerator* gGenerator;
static cAINodeGeneratorParams* gParams;
static cAINodeGeneratorParams* gNestedParams;
static tTempAiNodeList* gNodes;
static unsigned gQueries=0,gVerticalRays=0,gWallRays=0,gHits=0,gExceptions=0,gNested=0;
static int gMode=0,gAction=0,gActionRay=1,gRayNumber=0,gWallNumber=0,gDepth=0,gBoundsAction=0,gMassAction=0;
static uint64_t gRandom=0x925B48F16137ACEDull;
static uint32_t Next() { gRandom^=gRandom<<13; gRandom^=gRandom>>7; gRandom^=gRandom<<17; return (uint32_t)(gRandom>>16); }
static int Int(int a,int b) { return a+(int)(Next()%(unsigned)(b-a+1)); }
static uint32_t Bits(float f) { uint32_t n; memcpy(&n,&f,4); return n; }
static float Float(uint32_t n) { float f; memcpy(&f,&n,4); return f; }
static void Vec(const cVector3f& v) { fprintf(gOut,"%08x,%08x,%08x",Bits(v.x),Bits(v.y),Bits(v.z)); }
static void Query(const char*,int,cWorld&,cAINodeGenerator&,cAINodeGeneratorParams&);
static cPhysicsRayParams* gCurrentHit=NULL;
static void Ray(iPhysicsRayCallback*,const cVector3f&,const cVector3f&,bool,bool,bool,bool);

class cFixturePhysics : public iPhysicsWorld
{
public:
    void Add(iPhysicsBody* body) { mlstBodies.push_back(body); }
    void Clear() { mlstBodies.clear(); }
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

static cFixturePhysics* gFixturePhysics;
static iPhysicsBody* gAppendBody;

class cFixtureBody : public iPhysicsBody
{
public:
    float mass;
    int id;
    int boundReads=0;
    cFixtureBody(int ident,cFixturePhysics* physics,float value,const cVector3f& low,const cVector3f& high) :
        iPhysicsBody("body",physics,NULL),mass(value),id(ident) { mBoundingVolume.mvWorldMin=low; mBoundingVolume.mvWorldMax=high; }
    cBoundingVolume* GetBoundingVolume() override {
        ++boundReads; fprintf(gOut,"bounds %d %d ",id,boundReads); Vec(mBoundingVolume.mvWorldMin); fputc(' ',gOut); Vec(mBoundingVolume.mvWorldMax); fputc('\n',gOut);
        if(gBoundsAction && ((gBoundsAction!=3 && gBoundsAction!=5) || boundReads==2)) {
            int action=gBoundsAction; gBoundsAction=0; fprintf(gOut,"bounds-action %d\n",action);
            if(action==1) { mBoundingVolume.mvWorldMax.x+=0.75f; gParams->mfGridSize=0.75f; }
            if(action==2 || action==5) { Warning("controlled bounding-volume failure\n"); throw 101; }
            if(action==3) { mBoundingVolume.mvWorldMin.x-=0.5f; mBoundingVolume.mvWorldMax.z+=0.5f; }
            if(action==4) gFixturePhysics->Add(gAppendBody);
        }
        return &mBoundingVolume;
    }
    float GetMass() const override {
        fprintf(gOut,"mass %d %08x\n",id,Bits(mass));
        if(gMassAction) {
            int action=gMassAction; gMassAction=0; fprintf(gOut,"mass-action %d\n",action);
            if(action==1) gParams->mfHeightFromGround=0.625f;
            if(action==2) { Warning("controlled mass failure\n"); throw 102; }
            if(action==3) { gCurrentHit->mvPoint=cVector3f(-0.0f,0.75f,-0.0f); gParams->mfHeightFromGround=-0.0f; }
        }
        return mass;
    }
    void SetMaterial(iPhysicsMaterial* apMaterial) override { std::abort(); }
    void SetLinearVelocity(const cVector3f &avVel) override { std::abort(); }
    cVector3f GetLinearVelocity() const override { std::abort(); }
    void SetAngularVelocity(const cVector3f &avVel) override { std::abort(); }
    cVector3f GetAngularVelocity() const override { std::abort(); }
    void SetLinearDamping(float afDamping) override { std::abort(); }
    float GetLinearDamping() const override { std::abort(); }
    void SetAngularDamping(float afDamping) override { std::abort(); }
    float GetAngularDamping() const override { std::abort(); }
    void SetMaxLinearSpeed(float afSpeed) override { std::abort(); }
    float GetMaxLinearSpeed() const override { std::abort(); }
    void SetMaxAngularSpeed(float afDamping) override { std::abort(); }
    float GetMaxAngularSpeed() const override { std::abort(); }
    cVector3f GetInertiaVector()  override { std::abort(); }
    cMatrixf GetInertiaMatrix()  override { std::abort(); }
    void  SetMass(float afMass) override { std::abort(); }
    void  SetMassCentre(const cVector3f& avCentre) override { std::abort(); }
    cVector3f GetMassCentre() const override { std::abort(); }
    void AddForce(const cVector3f &avForce) override { std::abort(); }
    void AddForceAtPosition(const cVector3f &avForce, const cVector3f &avPos) override { std::abort(); }
    void AddTorque(const cVector3f &avTorque) override { std::abort(); }
    void AddImpulse(const cVector3f &avImpulse) override { std::abort(); }
    void AddImpulseAtPosition(const cVector3f &avImpulse, const cVector3f &avPos) override { std::abort(); }
    void Enable() override { std::abort(); }
    bool GetEnabled() const override { std::abort(); }
    void SetAutoDisable(bool abEnabled) override { std::abort(); }
    bool GetAutoDisable() const override { std::abort(); }
    void SetAutoDisableLinearThreshold(float afThresold) override { std::abort(); }
    float GetAutoDisableLinearThreshold() const override { std::abort(); }
    void SetAutoDisableAngularThreshold(float afThresold) override { std::abort(); }
    float GetAutoDisableAngularThreshold() const override { std::abort(); }
    void SetAutoDisableNumSteps(int alNum) override { std::abort(); }
    int GetAutoDisableNumSteps() const override { std::abort(); }
    void SetContinuousCollision(bool abOn) override { std::abort(); }
    bool GetContinuousCollision() override { std::abort(); }
    void SetGravity(bool abEnabled) override { std::abort(); }
    bool GetGravity() const override { std::abort(); }
    void RenderDebugGeometry(iLowLevelGraphics *apLowLevel,const cColor &aColor) override { std::abort(); }
    void DeleteLowLevel() override { std::abort(); }
};

static cFixtureBody* gHitBodies[4];
static bool Emit(iPhysicsRayCallback* cb,cFixtureBody* body,const cVector3f& point,float distance)
{
    cPhysicsRayParams params; params.mvPoint=point; params.mfDist=distance; params.mvNormal=cVector3f(0,1,0); params.mfT=0.5f;
    ++gHits; fprintf(gOut,"hit %d ",body->id); Vec(point); fprintf(gOut," dist=%08x\n",Bits(distance));
    auto previous=gCurrentHit; gCurrentHit=&params;
    try { bool result=cb->OnIntersect(body,&params); gCurrentHit=previous; fprintf(gOut,"hit-return %d\n",result); return result; }
    catch(...) { gCurrentHit=previous; throw; }
}
static void AllocateEdit()
{
    fputs("allocation-edit-height\n",gOut); gParams->mfHeightFromGround=5.0f;
}
static void AllocateNestedHit()
{
    fputs("allocation-nested-hit\n",gOut); ++gNested;
    Emit(static_cast<iPhysicsRayCallback*>(gGenerator),gHitBodies[0],cVector3f(-0.0f,0.5f,-0.0f),0.0f);
}
static void Ray(iPhysicsRayCallback* cb,const cVector3f& start,const cVector3f& end,bool d,bool n,bool p,bool f)
{
    int number=d ? ++gWallNumber : ++gRayNumber;
    if(d) ++gWallRays; else ++gVerticalRays;
    fprintf(gOut,"ray %d %c %d ",gDepth,d ? 'w':'v',number); Vec(start); fputs(" -> ",gOut); Vec(end); fprintf(gOut," options=%d%d%d%d\n",d,n,p,f);
    int action=0;
    if(gAction && number==gActionRay && ((gAction>=20)==d)) { action=gAction; gAction=0; fprintf(gOut,"ray-action %d\n",action); }
    if(action==1) { gParams->mfGridSize=0.75f; gParams->mfHeightFromGround=0.3f; gParams->mvMinPos=cVector3f(90); gParams->mvMaxPos=cVector3f(91); }
    if(action==2 || action==20) { Warning("controlled ray failure before hits\n"); throw 103; }
    if(action==4) gFailAllocation=0;
    if(action==5) gBeforeAllocation=AllocateEdit;
    if(action==6) gBeforeAllocation=AllocateNestedHit;
    if(action==7) gParams->mfGridSize=1.0f; // rescue an initially zero/negative/nonadvancing step
    if(action==24) gNodes->push_back(cTempAiNode(cVector3f(2,0.5f,3),"appended",-8));
    if(action==21) { gParams->mfMinWallDist=0.875f; const_cast<cVector3f&>(start)+=cVector3f(0.125f,-0.0f,-0.25f); }
    if(!d) {
        int count=gMode==1 ? 0 : gMode==2 ? 1 : 4;
        for(int i=0;i<count;++i) {
            cVector3f point(start.x,0.25f+(float)i*0.375f,start.z);
            if(gMode==3 && i==0) point=cVector3f(-0.0f,-0.0f,-0.0f);
            if(!Emit(cb,gHitBodies[i],point,(float)i*0.5f)) break;
        }
    } else {
        // Keep zero/NaN/infinite distances: comparisons and all three vector
        // products must retain native semantics. Bounds/steps stay finite.
        static const float distances[]={0.0f,0.125f,0.4f,0.75f,-0.125f,Float(0x7fc12345),Float(0x7f800000),Float(0xff800000),-0.0f};
        int which=(gMode+number)%9;
        if(gMode!=1) {
            if(Emit(cb,gHitBodies[1],end,0.1f)) // dynamic must not terminate the ray
                Emit(cb,gHitBodies[0],end,distances[which]);
        }
    }
    if(action==3 || action==22) { Warning("controlled ray failure after hits\n"); throw 104; }
    if(action==8 || action==23) {
        // Same world/type/list preserves all native iterators. Nested bounds are
        // inverted in z, so it appends nothing; existing nodes may be adjusted.
        int oldRays=gRayNumber,oldWalls=gWallNumber,oldMode=gMode;
        auto oldParams=gParams; ++gDepth; ++gNested; gMode=1;
        Query("nested",action,*gWorld,*gGenerator,*gNestedParams);
        --gDepth; gParams=oldParams; gRayNumber=oldRays; gWallNumber=oldWalls; gMode=oldMode;
    }
    // The vertical argument is a local vector, wall start is a live node field.
    // Trace after callbacks so alias preservation is visible to the comparator.
    fputs("ray-after ",gOut); Vec(start); fputc('\n',gOut);
}

static void Dump(tTempAiNodeList& nodes)
{
    fprintf(gOut,"nodes %llu\n",(unsigned long long)nodes.size());
    for(const auto& node:nodes) { fprintf(gOut,"node %d name=%s ",node.mlID,node.msName.c_str()); Vec(node.mvPos); fputc('\n',gOut); }
}
static void Query(const char* label,int index,cWorld& world,cAINodeGenerator& generator,cAINodeGeneratorParams& params)
{
    ++gQueries; gWorld=&world; gGenerator=&generator; gParams=&params; gNodes=world.GetAINodeList(params.msNodeType); gRayNumber=0; gWallNumber=0;
    fprintf(gOut,"query %s %d params=%08x,%08x,%08x low=",label,index,Bits(params.mfGridSize),Bits(params.mfHeightFromGround),Bits(params.mfMinWallDist));
    Vec(params.mvMinPos); fputs(" high=",gOut); Vec(params.mvMaxPos); fputc('\n',gOut);
    bool priorTrace=gTraceAlloc; gTraceAlloc=true;
    try { generator.Generate(&world,&params); fputs("generate-return\n",gOut); }
    catch(const std::bad_alloc&) { ++gExceptions; fputs("exception bad_alloc\n",gOut); }
    catch(int code) { ++gExceptions; fprintf(gOut,"exception %d\n",code); }
    gTraceAlloc=priorTrace;
    if(gFailAllocation>=0) fprintf(gOut,"unconsumed-fault %d\n",gFailAllocation);
    gFailAllocation=-1; gBeforeAllocation=NULL;
    Dump(*gNodes); fprintf(gOut,"query-end rays=%d,%d action=%d\n",gRayNumber,gWallNumber,gAction);
}
static void Case(const char* label,int index,int scenario,bool seeded)
{
    cFixturePhysics physics; gPhysicsWorld=&physics; gFixturePhysics=&physics;
    cResources resources(NULL,NULL); cWorld world("placement",NULL,&resources,NULL,NULL,NULL,NULL,NULL,NULL);
    cAINodeGenerator generator; cAINodeGeneratorParams params,nested;
    nested.mfGridSize=0.25f; nested.mfHeightFromGround=1.25f; nested.mvMinPos.z=10; nested.mvMaxPos.z=9; nested.mfMinWallDist=0.2f;
    gNestedParams=&nested; gAction=0; gActionRay=1; gBoundsAction=0; gMassAction=0; gMode=0;
    params.mfGridSize=0.5f; params.mfMinWallDist=0.4f;
    cVector3f low(-1,-2,-1),high(1,3,1);
    if(seeded) {
        params.mfGridSize=(float)Int(1,4)*0.125f;
        low=cVector3f((float)Int(-6,0)*0.25f,(float)Int(-6,0)*0.5f,(float)Int(-6,0)*0.25f);
        high=low+cVector3f(params.mfGridSize*(float)Int(1,5),(float)Int(0,8)*0.25f,params.mfGridSize*(float)Int(1,5));
        params.mfHeightFromGround=(float)Int(-4,5)*0.125f; params.mfMinWallDist=(float)Int(-2,6)*0.125f;
        params.mvMinPos=cVector3f((float)Int(-5,1)*0.5f,(float)Int(-3,0),(float)Int(-5,1)*0.5f);
        params.mvMaxPos=cVector3f((float)Int(1,6)*0.5f,(float)Int(0,3),(float)Int(1,6)*0.5f);
        gMode=Int(0,7); if(index%13==0) gAction=1; if(index%19==0) gAction=21;
    }
    cFixtureBody bodies[]={cFixtureBody(0,&physics,0.0f,low,high),cFixtureBody(1,&physics,1.0f,low,high),
        cFixtureBody(2,&physics,-1.0f,low,high),cFixtureBody(3,&physics,-0.0f,low,high)};
    for(int i=0;i<4;++i) gHitBodies[i]=&bodies[i];
    gAppendBody=&bodies[2];
    physics.Add(&bodies[0]); physics.Add(&bodies[1]);
    auto nodes=world.GetAINodeList(params.msNodeType);
    if(!seeded || index%3==0) nodes->push_back(cTempAiNode(cVector3f(-0.0f,0.25f,-0.0f),"existing",-7));
    if(!seeded) {
        if(scenario==1) gMode=1;
        if(scenario==2) { params.mvMinPos.x=2; params.mvMaxPos.x=1; }
        if(scenario==3) { params.mvMinPos.z=2; params.mvMaxPos.z=1; }
        if(scenario==4) { params.mvMinPos.y=5; params.mvMaxPos.y=-4; }
        if(scenario==5) params.mfHeightFromGround=-0.0f;
        if(scenario==6) { params.mfMinWallDist=-0.0f; gMode=3; }
        if(scenario==7) params.mfMinWallDist=-0.5f;
        if(scenario==8) params.mfMinWallDist=Float(0x7fc12345);
        if(scenario==9) { gAction=1; gMassAction=1; }
        if(scenario==10) gAction=21;
        if(scenario==11) gBoundsAction=1;
        if(scenario==12) gBoundsAction=2;
        if(scenario==13) gMassAction=2;
        if(scenario>=14 && scenario<=21) gAction=scenario-13;
        if(scenario>=22 && scenario<=25) gAction=scenario-2;
        if(scenario==26) { params.mfGridSize=0; gAction=7; }
        if(scenario==27) { params.mfGridSize=-0.125f; gAction=7; }
        if(scenario==28) { params.mfGridSize=Float(1); gAction=7; }
        if(scenario==29) params.mfGridSize=Float(0x7fc12345);
        if(scenario==30) params.mfGridSize=Float(0x7f800000);
        if(scenario==31) { bodies[0].mass=Float(0x7fc12345); bodies[3].mass=0.0f; }
        if(scenario==32) { params.mfHeightFromGround=Float(0x7f800000); }
        if(scenario==33) physics.Clear();
        if(scenario==34) gBoundsAction=4;
        if(scenario==35) { gAction=24; params.mvMinPos.z=2; params.mvMaxPos.z=1; }
        if(scenario==36) gMassAction=3;
        if(scenario==37) gBoundsAction=3;
        if(scenario==38) { gMode=8; params.mfMinWallDist=0; params.mvMinPos.z=2; params.mvMaxPos.z=1; }
        if(scenario==39) { params.mvMinPos.x=Float(0x7fc12345); params.mvMaxPos.z=Float(0x7fc12345); }
        if(scenario==40) { bodies[0].GetBoundingVolume()->mvWorldMin=cVector3f(Float(0x7fc12345)); }
        if(scenario==41) { bodies[0].GetBoundingVolume()->mvWorldMax=cVector3f(Float(0x7fc12345)); }
        if(scenario==42) { bodies[1].GetBoundingVolume()->mvWorldMin=cVector3f(-1.25f,-3,-1.25f); bodies[1].GetBoundingVolume()->mvWorldMax=cVector3f(1.25f,4,1.25f); }
        if(scenario==43) { params.mvMinPos=cVector3f(-200000); params.mvMaxPos=cVector3f(200000); physics.Clear(); }
        if(scenario==44) params.mfMinWallDist=Float(0x7f800000);
        if(scenario==45) params.mfMinWallDist=Float(0xff800000);
        if(scenario==46) gBoundsAction=5;
    }
    Query(label,index,world,generator,params);
    if(!seeded && scenario==0) Query("repeat",index,world,generator,params);
    if(!seeded && scenario==17) {
        // Failed append must not increment ID. Invoke through the public base
        // interface after initialization, then examine the inserted native node.
        gTraceAlloc=true; Emit(static_cast<iPhysicsRayCallback*>(&generator),&bodies[0],cVector3f(7,8,9),0); gTraceAlloc=false; Dump(*nodes);
    }
}
int main(int argc,char** argv)
{
    if(argc!=2) return 2;
    gOut=fopen(argv[1],"wb"); if(!gOut) return 3; gLogOut=gOut;
#if HARNESS_RUST
    RustBridge_Init();
#endif
    for(int scenario=0;scenario<=46;++scenario) Case("fixed",scenario,scenario,false);
    for(int index=0;index<2048;++index) Case("seeded",index,0,true);
    fprintf(gOut,"summary queries=%u vertical=%u wall=%u hits=%u exceptions=%u nested=%u seeded=2048\n",gQueries,gVerticalRays,gWallRays,gHits,gExceptions,gNested);
    fclose(gOut); gLogOut=stdout; return 0;
}
