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

// Differential test of AI navigation through its public C++ API only:
// cAINodeContainer (AddNode, lookups, Compile, iterators, FreePath,
// SaveToFile/LoadFromFile), cAINode (positions, edges, AddEdge) and
// cAStarHandler (GetPath with iAStarCallback, iteration limits, output
// lists). Built against the original sources and against the Rust-owned
// phase 2 facades; every result, callback and ray is printed.
//
// A fixture physics world blocks rays that cross wall segments (in xz) and
// can act at a chosen ray: move nodes, change properties, add edges or
// nodes, rebuild the grid, run a nested compile or search, or throw.
//
// Excluded, because the original has undefined behaviour there: empty
// containers (Compile/LoadFromFile read mvNodes[0]); non-finite positions
// (invalid float-to-int grid casts and NaN sort keys); AddNode during
// Compile and AddEdge during CanAddNode (iterator/pointer invalidation);
// a nested search on the same handler that resets the outer search; and
// searches over edges whose target is NULL (stale .nodes files).
#include "ai/AINodeContainer.h"
#include "ai/AStar.h"
#include "scene/World.h"
#include "system/LowLevelSystem.h"
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>
#if HARNESS_RUST
#include "system/RustBridge.h"
#endif

using namespace hpl;
extern FILE* gLogOut;
extern iPhysicsWorld* gPhysicsWorld;

static FILE* gOut;
static uint64_t gRandom = 0x5DEECE66D1234567ull;
static uint32_t Next() { gRandom ^= gRandom << 13; gRandom ^= gRandom >> 7; gRandom ^= gRandom << 17; return (uint32_t)(gRandom >> 16); }
static int Int(int lo, int hi) { return lo + (int)(Next() % (uint32_t)(hi - lo + 1)); }
static bool Chance(int percent) { return Int(0, 99) < percent; }
static uint32_t Bits(float value) { uint32_t u; memcpy(&u, &value, 4); return u; }
static int Id(cAINode* node) { return node ? node->GetID() : -1; }

//------------------------------------------
// Fixture physics

struct cWall { float x0, z0, x1, z1; };
static std::vector<cWall> gWalls;
static int gRays, gTotalRays;

// Ray actions run inside FreePath's CastRay; callback actions inside
// iAStarCallback::CanAddNode.
enum eAction {
    eAction_None,
    eAction_Move, eAction_Properties, eAction_AddEdge, eAction_AddNode,
    eAction_RebuildGrid, eAction_NestedCompile, eAction_Throw,
    eAction_NestedDirectPath, eAction_CallbackLimit, eAction_CallbackClear, eAction_CallbackThrow,
    eAction_LastEnum
};
static bool IsRayAction(int a) { return a > eAction_None && a < eAction_NestedDirectPath; }
static bool gPrintRays;
static int gAction, gActionAt;
static cAINodeContainer* gContainer;
static cAStarHandler* gHandler;
static cWorld* gWorld;
static void RunAction(const char* where);

static float Cross(float ax, float az, float bx, float bz) { return ax*bz - az*bx; }

static bool Crosses(const cVector3f& a, const cVector3f& b, const cWall& w)
{
    float d1 = Cross(w.x1-w.x0, w.z1-w.z0, a.x-w.x0, a.z-w.z0);
    float d2 = Cross(w.x1-w.x0, w.z1-w.z0, b.x-w.x0, b.z-w.z0);
    float d3 = Cross(b.x-a.x, b.z-a.z, w.x0-a.x, w.z0-a.z);
    float d4 = Cross(b.x-a.x, b.z-a.z, w.x1-a.x, w.z1-a.z);
    return ((d1 > 0) != (d2 > 0)) && ((d3 > 0) != (d4 > 0)) && d1 != 0 && d2 != 0;
}

static void Ray(iPhysicsRayCallback* callback, const cVector3f& start, const cVector3f& end)
{
    ++gRays; ++gTotalRays;
    bool blocked = false;
    for(size_t i=0; i<gWalls.size(); ++i) if(Crosses(start, end, gWalls[i])) blocked = true;
    if(gPrintRays)
        fprintf(gOut, "ray %08x,%08x,%08x %08x,%08x,%08x %s\n", Bits(start.x), Bits(start.y), Bits(start.z),
                Bits(end.x), Bits(end.y), Bits(end.z), blocked ? "blocked" : "free");
    if(IsRayAction(gAction) && gRays == gActionAt) RunAction("ray");
    // FreePath passes no body callback here: OnIntersect only sets the flag.
    if(blocked) callback->OnIntersect(NULL, NULL);
}

class cFixturePhysics : public iPhysicsWorld
{
public:
    void CastRay(iPhysicsRayCallback* cb,const cVector3f& a,const cVector3f& b,bool,bool,bool,bool) override { Ray(cb,a,b); }
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
static cFixturePhysics* gPhysics;

//------------------------------------------
// A* callback

class cFixtureAStarCallback : public iAStarCallback
{
public:
    int mlCalls;
    uint32_t mlRejectMask;
    bool CanAddNode(cAINode *apParentNode, cAINode *apChildNode) override
    {
        ++mlCalls;
        fprintf(gOut, "can-add %d %d", Id(apParentNode), Id(apChildNode));
        bool bAllowed = ((uint32_t)(Id(apParentNode)*31 + Id(apChildNode)*17) % 7) >= mlRejectMask;
        fprintf(gOut, " %d\n", bAllowed);
        if(gAction >= eAction_CallbackLimit && mlCalls == gActionAt) RunAction("can-add");
        return bAllowed;
    }
};
static cFixtureAStarCallback gCallback;

//------------------------------------------
// Printing

static void Dump(cAINodeContainer& c, const char* label)
{
    fprintf(gOut, "dump %s nodes=%d %08x %08x\n", label, c.GetNodeNum(), Bits(c.GetMaxEdgeDistance()), Bits(c.GetMaxHeight()));
    for(int i=0; i<c.GetNodeNum(); ++i)
    {
        cAINode* node = c.GetNode(i);
        const cVector3f& p = node->GetPosition();
        fprintf(gOut, " node %d %d %s %08x,%08x,%08x edges=%d:", i, node->GetID(), node->GetName().c_str(),
                Bits(p.x), Bits(p.y), Bits(p.z), node->GetEdgeNum());
        for(int j=0; j<node->GetEdgeNum(); ++j)
        {
            cAINodeEdge* e = node->GetEdge(j);
            fprintf(gOut, " %d/%08x/%08x", Id(e->mpNode), Bits(e->mfDistance), Bits(e->mfSqrDistance));
        }
        fputs("\n", gOut);
    }
}

static void Iterate(cAINodeContainer& c, const cVector3f& pos, float radius)
{
    fprintf(gOut, "iterate %08x,%08x,%08x r=%08x:", Bits(pos.x), Bits(pos.y), Bits(pos.z), Bits(radius));
    cAINodeIterator it = c.GetNodeIterator(pos, radius);
    cAINodeIterator copy = it; // value type: copies iterate independently
    int count = 0;
    while(it.HasNext()) { fprintf(gOut, " %d", Id(it.Next())); ++count; }
    int copied = 0;
    while(copy.HasNext()) { copy.Next(); ++copied; }
    fprintf(gOut, " | copy %d\n", copied == count);
}

//------------------------------------------
// Actions inside callbacks

static void RunAction(const char* where)
{
    int action = gAction;
    gAction = 0;
    fprintf(gOut, "action %d at %s\n", action, where);
    cAINodeContainer& c = *gContainer;
    switch(action)
    {
    case eAction_Move: {
        // The original returns a reference to its own member; editing it moves
        // the node in place (grid membership does not change).
        cAINode* node = c.GetNode(Int(0, c.GetNodeNum()-1));
        cVector3f& p = const_cast<cVector3f&>(node->GetPosition());
        p.x += 0.75f; p.y += 0.25f;
        break; }
    case eAction_Properties:
        c.SetMaxEdgeDistance(c.GetMaxEdgeDistance()*0.5f);
        c.SetMaxHeight(c.GetMaxHeight() + 0.5f);
        c.SetMaxEdges(Int(-1, 4));
        c.SetMinEdges(Int(-1, 3));
        break;
    case eAction_AddEdge: {
        cAINode* a = c.GetNode(Int(0, c.GetNodeNum()-1));
        cAINode* b = c.GetNode(Int(0, c.GetNodeNum()-1));
        a->AddEdge(b);
        break; }
    case eAction_AddNode: {
        char name[32]; snprintf(name, sizeof(name), "late%d", c.GetNodeNum());
        c.AddNode(name, 1000 + c.GetNodeNum(), cVector3f(1.5f, 0, -2.5f));
        break; }
    case eAction_RebuildGrid:
        c.BuildNodeGridMap();
        break;
    case eAction_NestedCompile: {
        cAINodeContainer inner("inner", "node", gWorld, cVector3f(0.5f, 1.5f, 0.5f));
        inner.AddNode("a", 0, cVector3f(0, 0, 0));
        inner.AddNode("b", 1, cVector3f(1, 0, 1));
        inner.AddNode("c", 2, cVector3f(2, 0, 0));
        cAINodeContainer* outer = gContainer;
        int rays = gRays;
        inner.Compile();
        gContainer = outer; gRays = rays;
        Dump(inner, "inner");
        break; }
    case eAction_NestedDirectPath: {
        // Same handler; the direct free-path check ends it before any reset.
        tAINodeList lst;
        cVector3f p(0.25f, 0, 0.25f);
        bool r = gHandler->GetPath(p, p, &lst);
        fprintf(gOut, "nested-path %d %d\n", r, (int)lst.size());
        break; }
    case eAction_Throw:
        Warning("controlled exception\n");
        throw 1776;
    case eAction_CallbackLimit:
        gHandler->SetMaxIterations(Int(0, 3));
        break;
    case eAction_CallbackClear:
        gHandler->SetCallback(NULL);
        break;
    case eAction_CallbackThrow:
        throw 1984;
    }
}

//------------------------------------------
// Scenarios

static void Compile(cAINodeContainer& c, const char* label)
{
    gContainer = &c; gRays = 0;
    fprintf(gOut, "compile %s action=%d@%d\n", label, gAction, gActionAt);
    try { c.Compile(); fprintf(gOut, "compiled rays=%d\n", gRays); }
    catch(int e) { fprintf(gOut, "exception %d rays=%d\n", e, gRays); }
    gAction = 0;
    Dump(c, label);
}

static void Path(cAStarHandler& h, cAINodeContainer& c, const cVector3f& start, const cVector3f& goal, bool output, bool prefill)
{
    gContainer = &c; gHandler = &h; gRays = 0; gCallback.mlCalls = 0;
    fprintf(gOut, "path %08x,%08x,%08x -> %08x,%08x,%08x out=%d pre=%d action=%d@%d\n",
            Bits(start.x), Bits(start.y), Bits(start.z), Bits(goal.x), Bits(goal.y), Bits(goal.z), output, prefill, gAction, gActionAt);
    tAINodeList lst;
    if(prefill) lst.push_back(c.GetNode(0));
    try {
        bool r = h.GetPath(start, goal, output ? &lst : NULL);
        fprintf(gOut, "result %d:", r);
    }
    catch(int e) { fprintf(gOut, "exception %d:", e); }
    for(tAINodeListIt it = lst.begin(); it != lst.end(); ++it) fprintf(gOut, " %d", Id(*it));
    fprintf(gOut, " rays=%d calls=%d\n", gRays, gCallback.mlCalls);
    gAction = 0;
}

static cVector3f RandomPosition(float extent, bool lattice)
{
    if(lattice) return cVector3f((float)Int(-6, 6), Chance(80) ? 0.0f : (float)Int(-2, 2)*0.25f, (float)Int(-6, 6));
    return cVector3f((float)Int(-800, 800)*extent/800.0f, Chance(70) ? 0.0f : (float)Int(-40, 40)*0.0625f, (float)Int(-800, 800)*extent/800.0f);
}

static void RandomWalls()
{
    gWalls.clear();
    int count = Int(0, 4);
    for(int i=0; i<count; ++i)
    {
        cWall w = { (float)Int(-60, 60)*0.1f, (float)Int(-60, 60)*0.1f, (float)Int(-60, 60)*0.1f, (float)Int(-60, 60)*0.1f };
        gWalls.push_back(w);
    }
}

static void Fill(cAINodeContainer& c, bool lattice, float extent)
{
    int count = Int(1, 45);
    c.ReserveSpace(Int(0, 50));
    for(int i=0; i<count; ++i)
    {
        char name[32];
        int id = i;
        if(i > 0 && Chance(5)) snprintf(name, sizeof(name), "n%d", Int(0, i-1)); // duplicate name
        else snprintf(name, sizeof(name), "n%d", i);
        if(i > 0 && Chance(4)) id = Int(0, i-1); // duplicate ID
        cVector3f p = RandomPosition(extent, lattice);
        if(i > 0 && Chance(6)) p = c.GetNode(Int(0, i-1))->GetPosition(); // duplicate position
        c.AddNode(name, id, p, NULL);
    }
}

static void Properties(cAINodeContainer& c)
{
    const float distances[] = { 0.0f, 0.5f, 1.0f, 1.5f, 2.0f, 3.0f, 5.0f };
    const float heights[] = { 0.0f, 0.1f, 0.41f, 1.0f, 3.0f };
    c.SetMaxEdgeDistance(distances[Int(0, 6)]);
    c.SetMaxHeight(heights[Int(0, 4)]);
    c.SetMaxEdges(Int(-1, 8));
    c.SetMinEdges(Int(-1, 4));
    c.SetNodeIsAtCenter(Chance(70));
    fprintf(gOut, "properties %08x %08x center=%d\n", Bits(c.GetMaxEdgeDistance()), Bits(c.GetMaxHeight()), c.GetNodeIsAtCenter());
}

static void Lookups(cAINodeContainer& c)
{
    fprintf(gOut, "lookups num=%d:", c.GetNodeNum());
    for(int k=0; k<6; ++k)
    {
        char name[32]; int i = Int(-2, c.GetNodeNum()+2);
        snprintf(name, sizeof(name), "n%d", i);
        fprintf(gOut, " name(%s)=%d id(%d)=%d", name, Id(c.GetNodeFromName(name)), i, Id(c.GetNodeFromID(i)));
    }
    fputs("\n", gOut);
}

static void SaveLoad(cAINodeContainer& c, int scenario)
{
    char file[128]; snprintf(file, sizeof(file), "%s.nodes", getenv("NAV_DIFF_TMP") ? getenv("NAV_DIFF_TMP") : "/tmp/ai-nav-diff");
    tWString wfile; for(const char* p=file; *p; ++p) wfile += (wchar_t)*p;
    c.SaveToFile(wfile);
    cAINodeContainer loaded("loaded", "node", gWorld, c.GetCollideSize());
    for(int i=0; i<c.GetNodeNum(); ++i)
    {
        cAINode* n = c.GetNode(i);
        // Rename some nodes: their incoming edges load with a NULL target.
        tString name = (scenario % 3 == 0 && i % 4 == 1) ? "renamed" + n->GetName() : n->GetName();
        loaded.AddNode(name, n->GetID(), n->GetPosition(), NULL);
    }
    loaded.LoadFromFile(wfile);
    Dump(loaded, "loaded");
    Iterate(loaded, c.GetNode(0)->GetPosition(), 2.0f);

    tWString missing = wfile + L".missing";
    cAINodeContainer empty("missing", "node", gWorld, c.GetCollideSize());
    empty.AddNode("x", 0, cVector3f(0), NULL);
    empty.LoadFromFile(missing);
    Dump(empty, "missing-file");
}

static void RandomScenario(int scenario)
{
    bool lattice = Chance(50);
    float extent = Chance(80) ? 6.0f : 40.0f;
    cAINodeContainer c("random", "node", gWorld, cVector3f(0.5f + Int(0, 4)*0.125f, 1.5f + Int(0, 4)*0.125f, 0.5f));
    fprintf(gOut, "scenario %d lattice=%d\n", scenario, lattice);
    Fill(c, lattice, extent);
    Properties(c);
    RandomWalls();
    gPhysicsWorld = Chance(5) ? NULL : gPhysics;

    // Some containers start with hand-added edges (Compile keeps them).
    if(Chance(15)) c.GetNode(0)->AddEdge(c.GetNode(c.GetNodeNum()-1));

    if(Chance(85)) Compile(c, "random");
    else { c.BuildNodeGridMap(); Dump(c, "grid-only"); }
    if(Chance(10)) { Properties(c); Compile(c, "repeat"); } // grid rebuilt: lists keep old entries

    Lookups(c);
    for(int k=0; k<4; ++k) Iterate(c, RandomPosition(extent, false), (float)Int(0, 40)*0.25f);

    cAStarHandler h(&c);
    if(Chance(50)) { gCallback.mlRejectMask = Int(0, 3); h.SetCallback(&gCallback); }
    if(Chance(20)) h.SetMaxIterations(Int(0, 6));
    for(int k=0; k<6; ++k)
    {
        cVector3f start = RandomPosition(extent, false), goal = RandomPosition(extent, false);
        if(Chance(40)) start = c.GetNode(Int(0, c.GetNodeNum()-1))->GetPosition();
        if(Chance(40)) goal = c.GetNode(Int(0, c.GetNodeNum()-1))->GetPosition();
        Path(h, c, start, goal, !Chance(10), Chance(30));
    }

    if(gPhysicsWorld && Chance(30)) SaveLoad(c, scenario);
}

static void ActionScenario(int scenario)
{
    int action = 1 + scenario % (eAction_LastEnum - 1);
    cAINodeContainer c("action", "node", gWorld, cVector3f(0.5f, 1.5f, 0.5f));
    fprintf(gOut, "action-scenario %d action=%d\n", scenario, action);
    Fill(c, true, 6.0f);
    c.SetMaxEdgeDistance(2.0f); c.SetMaxHeight(1.0f); c.SetMaxEdges(Int(2, 6)); c.SetMinEdges(Int(0, 2));
    RandomWalls();
    gPhysicsWorld = gPhysics;

    // AddNode during Compile invalidates the original's node iterator.
    gPrintRays = true;
    if(action != eAction_AddNode && IsRayAction(action))
    {
        gAction = action; gActionAt = Int(1, 12);
        Compile(c, "action");
    }
    else Compile(c, "plain");

    cAStarHandler h(&c);
    gCallback.mlRejectMask = 1;
    for(int k=0; k<3; ++k)
    {
        cVector3f start = c.GetNode(Int(0, c.GetNodeNum()-1))->GetPosition() + cVector3f(0.1f, 0, 0.1f);
        cVector3f goal = c.GetNode(Int(0, c.GetNodeNum()-1))->GetPosition() + cVector3f(-0.1f, 0, 0.1f);
        gAction = action;
        gActionAt = IsRayAction(action) ? Int(1, 10) : Int(1, 6);
        if(action == eAction_NestedDirectPath)
        {
            // A second search on the same handler from inside CanAddNode,
            // while the outer search is live. It ends at its direct free-path
            // check (start == goal), which only clears the goal.
            struct cNested : public iAStarCallback {
                int mlCalls;
                bool CanAddNode(cAINode *a, cAINode *b) override {
                    fprintf(gOut, "can-add %d %d\n", Id(a), Id(b));
                    if(++mlCalls == gActionAt && gAction) RunAction("can-add");
                    return true;
                }
            } nested;
            nested.mlCalls = 0;
            h.SetCallback(&nested);
            Path(h, c, start, goal, true, false);
        }
        else
        {
            h.SetCallback(&gCallback);
            Path(h, c, start, goal, true, Chance(30));
        }
        h.SetMaxIterations(-1);
        Dump(c, "after-path");
    }
    Dump(c, "after-paths");
}

// Lattices behind walls: most paths need the graph search, and the many
// equal costs exercise the address-order tie break.
static void PathScenario(int scenario)
{
    int w = Int(3, 10), d = Int(3, 10);
    cAINodeContainer c("paths", "node", gWorld, cVector3f(0.4f, 1.6f, 0.4f));
    fprintf(gOut, "path-scenario %d %dx%d\n", scenario, w, d);
    int id = 0;
    for(int x=0; x<w; ++x) for(int z=0; z<d; ++z)
    {
        if(Chance(12)) continue; // holes
        char name[32]; snprintf(name, sizeof(name), "p%d", id);
        float y = Chance(85) ? 0.0f : (float)Int(1, 4)*0.25f;
        c.AddNode(name, id++, cVector3f((float)x, y, (float)z), NULL);
    }
    if(c.GetNodeNum() == 0) c.AddNode("p0", 0, cVector3f(0), NULL);
    c.SetMaxEdgeDistance(Chance(50) ? 1.0f : 1.5f);
    c.SetMaxHeight(Chance(80) ? 0.5f : 0.3f);
    c.SetMaxEdges(Int(4, 8)); c.SetMinEdges(Int(1, 3));
    c.SetNodeIsAtCenter(Chance(50));

    // One to three walls across the lattice, each with a gap.
    gWalls.clear();
    int walls = Int(1, 3);
    for(int i=0; i<walls; ++i)
    {
        bool alongX = Chance(50);
        float at = (float)Int(1, (alongX ? d : w) - 1) - 0.5f;
        float length = (float)(alongX ? w : d);
        float gap = (float)Int(0, (int)length - 1);
        cWall a, b;
        if(alongX) { cWall wa = { -1, at, gap - 0.3f, at }; cWall wb = { gap + 0.7f, at, length + 1, at }; a = wa; b = wb; }
        else { cWall wa = { at, -1, at, gap - 0.3f }; cWall wb = { at, gap + 0.7f, at, length + 1 }; a = wa; b = wb; }
        gWalls.push_back(a);
        if(Chance(85)) gWalls.push_back(b); // otherwise an open side
    }
    gPhysicsWorld = gPhysics;
    Compile(c, "lattice");

    cAStarHandler h(&c);
    if(Chance(40)) { gCallback.mlRejectMask = Int(0, 2); h.SetCallback(&gCallback); }
    if(Chance(15)) h.SetMaxIterations(Int(1, 12));
    for(int k=0; k<8; ++k)
    {
        cVector3f start = c.GetNode(Int(0, c.GetNodeNum()-1))->GetPosition() + cVector3f(0.2f, 0, 0.1f);
        cVector3f goal = c.GetNode(Int(0, c.GetNodeNum()-1))->GetPosition() + cVector3f(-0.1f, 0, 0.2f);
        Path(h, c, start, goal, true, Chance(20));
    }
}

int main(int argc, char** argv)
{
    if(argc != 2) return 2;
    gOut = fopen(argv[1], "wb"); if(!gOut) return 3;
    gLogOut = gOut;
#if HARNESS_RUST
    RustBridge_Init();
#endif
    cWorld world("fixture", NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL);
    gWorld = &world;
    cFixturePhysics physics;
    gPhysics = &physics;

    gPrintRays = false;
    for(int i=0; i<1500; ++i) RandomScenario(i);
    for(int i=0; i<1000; ++i) PathScenario(i);
    for(int i=0; i<600; ++i) ActionScenario(i);

    fprintf(gOut, "totals rays=%d\n", gTotalRays);
    fclose(gOut);
    return 0;
}
