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

// HPL2_RUST_AI selects native/Rust GetPath and the earlier AI slices.
// AStar, graph/grid/iterator/AddEdge/FreePath all use production implementations.
// A normal concrete physics world supplies controlled rays. A monotonic aligned
// graph allocation arena preserves MSVC's native address tie order across runs.
// Nonempty grids have finite x/z extents and bounded casts. No freed parent,
// invalidated grid/edge iterator or uninitialized search cost is read.
#include "ai/AStar.h"
#include "ai/AINodeContainer.h"
#include "scene/World.h"
#include "system/LowLevelSystem.h"
#include <algorithm>
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
extern std::vector<cAStarNode*> gSearchNodes;
extern bool gTraceAlloc,gAllocateGraphNode;
extern std::size_t gFailSize,gHookSize,gLastAllocationSize;
extern int gFailCountdown,gFailRequireGoal;
extern void (*gAllocationHook)();
extern void (*gSearchDeleteHook)();
extern void ResetNodeArena();
static FILE* gOut;
static cAINodeContainer* gContainer;
static cAStarHandler* gHandler;
static const cVector3f* gStart;
static const cVector3f* gGoal;
static bool gStartAllowed[128],gGoalAllowed[128],gDirect=false;
static int gRayInPath=0,gPathKind=0,gPathNode=-1,gAction=0,gActionGroup=0,gGroups=0,gBlockedRay=1;
static bool gPathBlocked=false;
static std::size_t gListSize,gTreeSize;
static unsigned gQueries=0,gTotalRays=0,gExceptions=0,gNested=0;
static uint64_t gRandom=0x6137ACED925B48F1ull;
static uint32_t Next() { gRandom^=gRandom<<13; gRandom^=gRandom>>7; gRandom^=gRandom<<17; return (uint32_t)(gRandom>>16); }
static int Int(int a,int b) { return a+(int)(Next()%(unsigned)(b-a+1)); }
static uint32_t Bits(float f) { uint32_t n; memcpy(&n,&f,4); return n; }
static float Float(uint32_t n) { float f; memcpy(&f,&n,4); return f; }
static bool Same(const cVector3f& a,const cVector3f& b) { return a.x==b.x && a.y==b.y && a.z==b.z; }
static void Position(cAINode* node,const cVector3f& value) { const_cast<cVector3f&>(node->GetPosition())=value; }
static void Query(const char*,int,cAINodeContainer&,cAStarHandler&,const cVector3f&,const cVector3f&,tAINodeList*);

static void Ray(iPhysicsRayCallback* callback,const cVector3f& start,const cVector3f& end,bool d,bool n,bool p,bool f)
{
    ++gTotalRays;
    if(gRayInPath==0) {
        ++gGroups; gPathNode=-1;
        for(int i=0;i<gContainer->GetNodeNum();++i) if(Same(end,gContainer->GetNode(i)->GetPosition())) { gPathNode=i; break; }
        gPathKind=Same(start,*gStart) && Same(end,*gGoal) ? 0 : Same(start,*gStart) ? 1 : 2;
        gPathBlocked=gPathKind==0 ? !gDirect : gPathNode<0 || !(gPathKind==1 ? gStartAllowed[gPathNode] : gGoalAllowed[gPathNode]);
        fprintf(gOut,"visibility group=%d kind=%d node=%d blocked=%d\n",gGroups,gPathKind,gPathNode,gPathBlocked);
    }
    ++gRayInPath;
    fprintf(gOut,"ray %d %d %08x,%08x,%08x -> %08x,%08x,%08x options=%d%d%d%d\n",gGroups,gRayInPath,
        Bits(start.x),Bits(start.y),Bits(start.z),Bits(end.x),Bits(end.y),Bits(end.z),d,n,p,f);
    if(gAction && gGroups==gActionGroup && gRayInPath==1) {
        int action=gAction; gAction=0; fprintf(gOut,"ray-action %d\n",action);
        if(action==1) { *const_cast<cVector3f*>(gStart)=cVector3f(0.75f,0.125f,0.5f); *const_cast<cVector3f*>(gGoal)=cVector3f(4.75f,0.25f,1.5f); }
        if(action==2) { gContainer->SetMaxHeight(-1); gContainer->SetMaxEdgeDistance(1.25f); }
        if(action==3) { gContainer->SetMaxHeight(100); gContainer->SetMaxEdgeDistance(30); }
        if(action==4 && gPathNode>=0) Position(gContainer->GetNode(gPathNode),cVector3f(13,0.5f,2));
        if(action==5) { Warning("controlled path visibility failure\n"); throw 1776; }
        if(action==6 || action==7) {
            // Nested setup never touches graph/grid storage, so the outer
            // candidate iterator remains valid. Restore only fixture ray state.
            const cVector3f *outerStart=gStart,*outerGoal=gGoal;
            int outerRay=gRayInPath,outerKind=gPathKind,outerNode=gPathNode,outerGroups=gGroups;
            bool outerBlocked=gPathBlocked,outerDirect=gDirect;
            cVector3f a(-0.75f,0,0.25f),b(2.75f,0,1.25f);
            gDirect=action==6; gHandler->SetMaxIterations(0); ++gNested;
            Query("nested",action,*gContainer,*gHandler,a,b,NULL);
            gStart=outerStart; gGoal=outerGoal; gRayInPath=outerRay; gPathKind=outerKind;
            gPathNode=outerNode; gGroups=outerGroups; gPathBlocked=outerBlocked; gDirect=outerDirect;
            gHandler->SetMaxIterations(-1);
        }
        if(action==8) { gFailSize=sizeof(cAStarNode); gFailCountdown=0; }
        if(action==9) { gFailSize=gTreeSize; gFailCountdown=0; }
    }
    bool blocked=gPathBlocked && gRayInPath==gBlockedRay;
    if(blocked) callback->OnIntersect(NULL,NULL);
    if(blocked || gRayInPath==5) gRayInPath=0;
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


static void Add(cAINodeContainer& c,const cVector3f& p)
{
    int id=c.GetNodeNum(); char name[24]; snprintf(name,sizeof(name),"node%d",id);
    gAllocateGraphNode=true; c.AddNode(name,id,p); assert(!gAllocateGraphNode);
    fprintf(gOut,"node %d %08x,%08x,%08x\n",id,Bits(p.x),Bits(p.y),Bits(p.z));
}
static void Edge(cAINodeContainer& c,int a,int b,float distance)
{
    c.GetNode(a)->AddEdge(c.GetNode(b)); cAINodeEdge* edge=c.GetNode(a)->GetEdge(c.GetNode(a)->GetEdgeNum()-1);
    edge->mfDistance=distance; edge->mfSqrDistance=distance*distance;
    fprintf(gOut,"edge %d %d %08x\n",a,b,Bits(distance));
}
static void Setup(cAINodeContainer& c,int count=8)
{
    for(int i=0;i<count-2;++i) Add(c,cVector3f((float)i,0,(float)(i%3)*0.5f));
    Add(c,cVector3f(-30,0,-30)); Add(c,cVector3f(30,0,30));
    c.BuildNodeGridMap(); c.SetMaxHeight(1); c.SetMaxEdgeDistance(2);
    std::fill(gStartAllowed,gStartAllowed+128,false); std::fill(gGoalAllowed,gGoalAllowed+128,false);
    gStartAllowed[0]=true; gGoalAllowed[count-3]=true;
    gDirect=false; gBlockedRay=1; gAction=0; gActionGroup=0;
}
static void Cleanup()
{
    // Original AddOpen/GetBest can leak a node after a failed native set insert.
    // The handler has already destroyed all owned nodes; reclaim only leftovers.
    while(!gSearchNodes.empty()) { fprintf(gOut,"reclaim-orphan %d\n",gSearchNodes.back()->mpAINode->GetID()); hplDelete(gSearchNodes.back()); }
}
static void DumpSearch(bool initialized)
{
    auto ordered=gSearchNodes;
    std::sort(ordered.begin(),ordered.end(),[](cAStarNode* a,cAStarNode* b) { return a->mpAINode->GetID()<b->mpAINode->GetID(); });
    for(cAStarNode* node:ordered) {
        fprintf(gOut,"search %d parent=%d",node->mpAINode->GetID(),node->mpParent ? node->mpParent->mpAINode->GetID() : -1);
        if(initialized) fprintf(gOut," distance=%08x cost=%08x",Bits(node->mfDistance),Bits(node->mfCost));
        fputc('\n',gOut);
    }
}
static void Query(const char* name,int sequence,cAINodeContainer& c,cAStarHandler& h,const cVector3f& start,const cVector3f& goal,tAINodeList* output)
{
    ++gQueries; gContainer=&c; gHandler=&h; gStart=&start; gGoal=&goal; gRayInPath=0; gGroups=0;
    fprintf(gOut,"query %s %d output=%d direct=%d limits=%08x,%08x start=%08x,%08x,%08x goal=%08x,%08x,%08x\n",
        name,sequence,output!=NULL,gDirect,Bits(c.GetMaxHeight()),Bits(c.GetMaxEdgeDistance()),
        Bits(start.x),Bits(start.y),Bits(start.z),Bits(goal.x),Bits(goal.y),Bits(goal.z));
    gTraceAlloc=true; bool initialized=true;
    try { bool result=h.GetPath(start,goal,output); fprintf(gOut,"result %d\n",result); }
    catch(const std::bad_alloc&) { ++gExceptions; initialized=false; fputs("exception bad_alloc\n",gOut); }
    catch(int value) { ++gExceptions; fprintf(gOut,"exception %d\n",value); }
    if(gFailCountdown>=0) fprintf(gOut,"allocation-fault-unconsumed %d\n",gFailCountdown);
    gFailCountdown=-1; gFailRequireGoal=-1; gAllocationHook=NULL; gSearchDeleteHook=NULL; gRayInPath=0;
    if(output) { fputs("path",gOut); for(cAINode* node:*output) fprintf(gOut," %d",node->GetID()); fputc('\n',gOut); }
    DumpSearch(initialized);
    fprintf(gOut,"query-end groups=%d action=%d\n",gGroups,gAction); gAction=0;
}
struct cAllowEdges : iAStarCallback {
    int mode=0,count=0;
    bool CanAddNode(cAINode* a,cAINode* b) override {
        ++count; fprintf(gOut,"callback %d %d %d->%d\n",mode,count,a->GetID(),b->GetID());
        if(mode==2 && count==1) throw 909;
        return mode!=1 || (a->GetID()+b->GetID()+count)%3!=0;
    }
};
static void OutputMutationNode()
{
    for(cAStarNode* node:gSearchNodes) if(node->mpAINode->GetID()==5) {
        // Node 6 is an unused higher-address anchor. Every comparison against
        // another stored search node retains its outcome, so set order is valid.
        node->mpAINode=gContainer->GetNode(6); fputs("output-node-reference-mutated\n",gOut); return;
    }
    // cAStarNode and list nodes can have the same native size. Wait until
    // the fixed chain goal has been constructed, then the next match is output.
    gAllocationHook=OutputMutationNode;
}
static void OutputMutationParent()
{
    for(cAStarNode* node:gSearchNodes) if(node->mpAINode->GetID()==5) {
        node->mpParent=NULL; fputs("output-parent-mutated\n",gOut); return;
    }
    gAllocationHook=OutputMutationParent;
}
static void OutputNestedDirect()
{
    bool found=false; for(cAStarNode* node:gSearchNodes) if(node->mpAINode->GetID()==5) found=true;
    if(!found) { gAllocationHook=OutputNestedDirect; return; }
    const cVector3f *outerStart=gStart,*outerGoal=gGoal;
    int outerGroups=gGroups; bool outerDirect=gDirect;
    cVector3f a(-0.75f,0,0.25f),b(2.75f,0,1.25f);
    gDirect=true; ++gNested;
    Query("nested-output-direct",0,*gContainer,*gHandler,a,b,NULL);
    gStart=outerStart; gGoal=outerGoal; gGroups=outerGroups; gDirect=outerDirect;
}
static void DeleteFailure() { Warning("controlled reset failure\n"); throw 1945; }
static void DeleteMutation()
{
    *const_cast<cVector3f*>(gGoal)=cVector3f(5.25f,0,1.25f);
    gContainer->SetMaxEdgeDistance(3); fputs("reset-callback-mutated-goal-and-radius\n",gOut);
}
static void Fixed(cWorld& world,cFixturePhysics& physics)
{
    for(int scenario=0;scenario<38;++scenario) {
        ResetNodeArena(); cAINodeContainer c("fixed","node",&world,cVector3f(0)); Setup(c);
        for(int i=0;i<5;++i) Edge(c,i,i+1,1.0f);
        {
            cAStarHandler h(&c); cAllowEdges callback; h.SetCallback(&callback);
            cVector3f start(-0.25f,0,0.125f),goal(5.25f,0,1.125f); tAINodeList output; output.push_back(c.GetNode(7));
            gPhysicsWorld=&physics;
            if(scenario<8) h.SetMaxIterations(scenario-2);
            if(scenario==8) gDirect=true;
            if(scenario==9) gPhysicsWorld=NULL;
            if(scenario==10) std::fill(gStartAllowed,gStartAllowed+128,false);
            if(scenario==11) std::fill(gGoalAllowed,gGoalAllowed+128,false);
            if(scenario==12) { gStartAllowed[1]=true; gStartAllowed[2]=true; gGoalAllowed[3]=true; }
            if(scenario==13) callback.mode=1;
            if(scenario==14) callback.mode=2;
            if(scenario>=15 && scenario<23) { gAction=scenario-14; gActionGroup=1; }
            if(scenario>=23 && scenario<31) { gAction=scenario-22; gActionGroup=2; }
            if(scenario==31) gBlockedRay=5;
            if(scenario==32) { gHookSize=gListSize; gAllocationHook=OutputMutationNode; }
            if(scenario==33) { gHookSize=gListSize; gAllocationHook=OutputMutationParent; }
            if(scenario==34 || scenario==35) { gFailSize=gListSize; gFailCountdown=scenario-34; gFailRequireGoal=5; }
            if(scenario==36) { gAction=9; gActionGroup=9; }
            if(scenario==37) { gHookSize=gListSize; gAllocationHook=OutputNestedDirect; }
            Query("fixed",scenario,c,h,start,goal,scenario==7 ? NULL : &output);
            h.SetMaxIterations(-1); h.SetCallback(NULL); gDirect=true;
            Query("direct-retains",scenario,c,h,start,goal,&output);
            gDirect=false; gPhysicsWorld=&physics; c.SetMaxHeight(1); c.SetMaxEdgeDistance(2);
            Query("repeat",scenario,c,h,start,goal,NULL);
        }
        Cleanup();
    }
    for(int stage=1;stage<=5;++stage) {
        ResetNodeArena(); cAINodeContainer c("live","node",&world,cVector3f(0)); Setup(c);
        for(int i=0;i<5;++i) Edge(c,i,i+1,1);
        {
            cAStarHandler h(&c); cVector3f start(-0.25f,0,0.125f),goal(5.25f,0,1.125f); tAINodeList output;
            gPhysicsWorld=&physics; Query("warm",stage,c,h,start,goal,&output);
            if(stage==1) gSearchDeleteHook=DeleteMutation;
            if(stage==2) { gStartAllowed[1]=true; gAction=1; gActionGroup=3; }
            if(stage==3) { gAction=5; gActionGroup=3; }
            if(stage==4) { gStartAllowed[1]=true; gAction=7; gActionGroup=3; }
            if(stage==5) gSearchDeleteHook=DeleteFailure;
            Query("live",stage,c,h,start,goal,&output);
        }
        Cleanup();
    }
    for(int alias=0;alias<6;++alias) {
        ResetNodeArena(); cAINodeContainer c("aliases","node",&world,cVector3f(0)); Setup(c);
        for(int i=0;i<5;++i) Edge(c,i,i+1,1);
        {
            cAStarHandler h(&c); cVector3f value(-0.25f,0,0.125f),goal(5.25f,0,1.125f); tAINodeList output;
            const cVector3f& a=alias<2 ? value : c.GetNode(0)->GetPosition();
            const cVector3f& b=alias<2 ? value : alias<4 ? goal : c.GetNode(5)->GetPosition();
            // Aliases refer to live, non-const objects. Visibility edits preserve
            // graph membership, initialized fields and bounded grid conversions.
            gPhysicsWorld=&physics; gAction=alias%2 ? 4 : 1; gActionGroup=alias%2 ? 2 : 1;
            Query("endpoint-alias",alias,c,h,a,b,&output);
            Query("endpoint-alias-repeat",alias,c,h,a,b,NULL);
        }
        Cleanup();
    }
    const float distances[]={-1.0f,-0.0f,0.0f,Float(0x3f7fffff),1.0f,Float(0x3f800001),2.0f,10.0f};
    const float heights[]={-1.0f,-0.0f,0.0f,Float(0x3effffff),0.5f,Float(0x3f000001),Float(0x7fc12345),Float(0x7f800000)};
    for(int i=0;i<128;++i) {
        ResetNodeArena(); cAINodeContainer c("boundaries","node",&world,cVector3f(0)); Setup(c);
        Position(c.GetNode(0),cVector3f(0,0.75f,0)); Position(c.GetNode(1),cVector3f(2,0,0));
        Edge(c,0,1,2); c.SetMaxHeight(heights[(i/8)%8]); c.SetMaxEdgeDistance(distances[i%8]);
        gGoalAllowed[1]=true; gGoalAllowed[5]=false; gDirect=i>=64;
        {
            cAStarHandler h(&c); cVector3f start(0,0,0),goal(2,0.75f,0); tAINodeList output;
            gPhysicsWorld=&physics; Query("boundaries",i,c,h,start,goal,&output);
        }
        Cleanup();
    }
    for(int i=0;i<33;++i) {
        ResetNodeArena(); cAINodeContainer c("precision","node",&world,cVector3f(0)); Setup(c);
        Position(c.GetNode(0),cVector3f(Float(0x3f7ffff0u+i),0.000244140625f,0.000244140625f));
        // The last vector rounds to distance 1 in f32 but to the next float
        // when squared/summed in double; that is exactly the excluded cutoff.
        if(i==32) Position(c.GetNode(0),cVector3f(Float(0x3f7ffff1),Float(0x3a800000),Float(0x3a800000)));
        c.SetMaxHeight(1); c.SetMaxEdgeDistance(i==32 ? Float(0x3f000001) : 0.5f);
        gGoalAllowed[0]=true; gGoalAllowed[5]=false;
        {
            cAStarHandler h(&c); cVector3f start(0),goal(1.25f,0,0.125f); tAINodeList output;
            gPhysicsWorld=&physics; Query("precision",i,c,h,start,goal,&output);
        }
        Cleanup();
    }
}
static void Random(cWorld& world,cFixturePhysics& physics)
{
    for(int trial=0;trial<2048;++trial) {
        ResetNodeArena(); cAINodeContainer c("random","node",&world,cVector3f(0)); int count=Int(5,14); Setup(c,count);
        for(int i=0;i<count-2;++i) {
            Position(c.GetNode(i),cVector3f((float)Int(-48,48)*0.125f,(float)Int(-8,8)*0.125f,(float)Int(-48,48)*0.125f));
            gStartAllowed[i]=Int(0,2)==0; gGoalAllowed[i]=Int(0,2)==0;
        }
        // Rebuild preserves prior lists by design, exercising duplicate candidates.
        c.BuildNodeGridMap();
        for(int i=0;i<count-2;++i) for(int edge=0;edge<Int(1,5);++edge) Edge(c,i,Int(0,count-3),(float)Int(0,32)*0.125f);
        c.SetMaxHeight((float)Int(-1,12)*0.125f); c.SetMaxEdgeDistance((float)Int(0,80)*0.125f);
        {
            cAStarHandler h(&c); cAllowEdges callback; callback.mode=trial%2; h.SetCallback(&callback);
            cVector3f start((float)Int(-40,40)*0.125f+0.03125f,(float)Int(-6,6)*0.125f,(float)Int(-40,40)*0.125f+0.03125f);
            cVector3f goal((float)Int(-40,40)*0.125f+0.0625f,(float)Int(-6,6)*0.125f,(float)Int(-40,40)*0.125f+0.0625f);
            tAINodeList output; output.push_back(c.GetNode(count-1));
            gPhysicsWorld=trial%31==0 ? NULL : &physics; gDirect=trial%5==0;
            h.SetMaxIterations(trial%9-2); Query("random",trial,c,h,start,goal,&output);
            h.SetMaxIterations(-1); gDirect=trial%3==0; Query("random-repeat",trial,c,h,start,goal,trial%2 ? NULL : &output);
        }
        Cleanup();
    }
}
int main(int argc,char** argv)
{
    if(argc!=2) return 2; gOut=fopen(argv[1],"wb"); if(!gOut) return 3; gLogOut=gOut;
    gSearchNodes.reserve(1024);
    { tAINodeList list; list.push_back(NULL); gListSize=gLastAllocationSize; }
    { tAINodeSet set; set.insert(NULL); gTreeSize=gLastAllocationSize; }
#if HARNESS_RUST
    RustBridge_Init();
#endif
    { cWorld world("fixture",NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL); cFixturePhysics physics; Fixed(world,physics); Random(world,physics); }
    assert(gSearchNodes.empty());
    fprintf(gOut,"totals queries=%u rays=%u exceptions=%u nested=%u\n",gQueries,gTotalRays,gExceptions,gNested);
    fclose(gOut); return 0;
}
