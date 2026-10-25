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

// Public AStar GetPath differential corpus. Physics/grid construction are fixture
// dependencies; HPL2_RUST_AI selects the production C++/Rust search core.
// They are not subjects of this port. A single valid grid uses the original
// iterator bodies below; FreePath supplies controlled visibility and traces.
// Nodes occupy one real array, so pointer ties have defined, reproducible order.
// MEMORY_MANAGER_ACTIVE exposes constructed search nodes through the existing
// engine observer; only their public, initialized fields are read after searches.
// Floats are serialized as bits. Bounds remain small; no signed counter overflow,
// invalid node pointers, edge-vector invalidation, or freed-parent reads are used.

#include "ai/AStar.h"
#include "ai/AINodeContainer.h"
#include "math/Math.h"
#include <algorithm>
#include <cassert>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <map>
#include <new>
#include <vector>

#if HARNESS_RUST
#include "system/RustBridge.h"
#endif

using namespace hpl;
extern FILE* gLogOut;
extern std::vector<cAStarNode*> gSearchNodes;
extern bool gFailNextAllocation;
extern std::size_t gFailAllocationSize;
extern cAStarNode* gOrphan;
static FILE* gOut;
static uint64_t gRandom = 0x7F4A7C159E3779B9ull;
static uint32_t Next() { gRandom ^= gRandom << 13; gRandom ^= gRandom >> 7; gRandom ^= gRandom << 17; return (uint32_t)(gRandom >> 16); }
static int Int(int lo, int hi) { return lo + (int)(Next() % (uint32_t)(hi - lo + 1)); }
static float Float(uint32_t u) { float f; memcpy(&f, &u, 4); return f; }
static uint32_t Bits(float f) { uint32_t u; memcpy(&u, &f, 4); return u; }

struct Fixture {
    cAINode* slab = NULL;
    size_t capacity = 0;
    std::vector<bool> starts, goals;
    const cVector3f* start = NULL;
    const cVector3f* goal = NULL;
    bool direct = false;
    bool nestedDirect = false;
};
static std::map<cAINodeContainer*, Fixture> gFixtures;

namespace hpl {

// These node bodies are identical to AINodeContainer.cpp.
cAINode::cAINode() {}
cAINode::~cAINode() {}
void cAINode::AddEdge(cAINode* node)
{
    cAINodeEdge edge;
    edge.mpNode = node;
    edge.mfDistance = cMath::Vector3Dist(mvPosition, node->mvPosition);
    edge.mfSqrDistance = cMath::Vector3DistSqr(mvPosition, node->mvPosition);
    mvEdges.push_back(edge);
}

// The fixture deliberately replaces world/physics setup with a single grid.
// ReserveSpace owns one contiguous node array; AddNode retains normal public
// graph identities, positions and insertion order. Container APIs are otherwise
// only used by the unchanged GetPath candidate-enumeration code.
cAINodeContainer::cAINodeContainer(const tString& name, const tString& nodeName,
    cWorld* world, const cVector3f& size)
{
    msName = name; msNodeName = nodeName; mpWorld = world; mvSize = size;
    mpRayCallback = NULL;
    mlMaxNodeEnds = 5; mlMinNodeEnds = 2; mfMaxEndDistance = 1000.0f; mfMaxHeight = 1000.0f;
    mlNodesPerGrid = 6; mbNodeIsAtCenter = true;
    mvGridMapSize = cVector2l(0); mvGridSize = cVector2f(4096.0f); mvMinGridPos = cVector2f(-2048.0f);
    mvMaxGridPos = cVector2f(2048.0f); mvGrids.resize(1);
    gFixtures[this] = Fixture();
}
cAINodeContainer::~cAINodeContainer()
{
    delete[] gFixtures[this].slab;
    gFixtures.erase(this);
}
void cAINodeContainer::ReserveSpace(size_t count)
{
    Fixture& fixture = gFixtures[this];
    assert(!fixture.slab && mvNodes.empty());
    fixture.slab = new cAINode[count]; fixture.capacity = count;
    fixture.starts.resize(count); fixture.goals.resize(count); mvNodes.reserve(count);
}
void cAINodeContainer::AddNode(const tString& name, int id, const cVector3f& position, void* data)
{
    Fixture& fixture = gFixtures[this];
    assert(mvNodes.size() < fixture.capacity);
    cAINode* node = fixture.slab + mvNodes.size();
    node->msName = name; node->mlID = id; node->mvPosition = position; node->mpUserData = data;
    mvNodes.push_back(node); mvGrids[0].mlstNodes.push_back(node);
    m_mapNodesByName.insert(tAINodeNameMap::value_type(name, node));
    m_mapNodesByID.insert(tAINodeIDMap::value_type(id, node));
}
int cAINodeContainer::GetNodeNum() const { return (int)mvNodes.size(); }
cVector2l cAINodeContainer::GetGridPosFromLocal(const cVector2f&) { return cVector2l(0); }
cAIGridNode* cAINodeContainer::GetGrid(const cVector2l&) { return &mvGrids[0]; }
cAINodeIterator cAINodeContainer::GetNodeIterator(const cVector3f& position, float radius)
{
    return cAINodeIterator(this, position, radius);
}
bool cAINodeContainer::FreePath(const cVector3f& start, const cVector3f& end, int rays,
    tAIFreePathFlag flags, iAIFreePathCallback* callback)
{
    Fixture& fixture = gFixtures[this];
    int id = -1;
    for(cAINode* node : mvNodes) if(&end == &node->GetPosition()) { id = node->GetID(); break; }
    bool fromStart = &start == fixture.start;
    bool result = id < 0 ? fixture.direct || fixture.nestedDirect
        : fromStart ? fixture.starts[id] : fixture.goals[id];
    fprintf(gOut, "free %c %d rays=%d flags=%u callback=%d result=%d %08x,%08x,%08x -> %08x,%08x,%08x\n",
        fromStart ? 'S' : 'G', id, rays, flags, callback != NULL, result,
        Bits(start.x), Bits(start.y), Bits(start.z), Bits(end.x), Bits(end.y), Bits(end.z));
    return result;
}

// Original iterator algorithm, kept here to avoid linking the physics engine.
cAINodeIterator::cAINodeIterator(cAINodeContainer* container, const cVector3f& position, float radius)
{
    mpContainer = container; mvPosition = position; mfRadius = radius;
    cVector2f local(mvPosition.x, mvPosition.z); local -= mpContainer->mvMinGridPos;
    cVector2f localStart = local - cVector2f(radius), localEnd = local + cVector2f(radius);
    mvStartGridPos = mpContainer->GetGridPosFromLocal(localStart);
    mvEndGridPos = mpContainer->GetGridPosFromLocal(localEnd); mvGridPos = mvStartGridPos;
    mpNodeList = &mpContainer->GetGrid(mvGridPos)->mlstNodes;
    while(mpNodeList->empty()) {
        if(IncGridPos()) mpNodeList = &mpContainer->GetGrid(mvGridPos)->mlstNodes;
        else { mpNodeList = NULL; break; }
    }
    if(mpNodeList) mNodeIt = mpNodeList->begin();
}
bool cAINodeIterator::HasNext() { return !(mpNodeList == NULL || mpNodeList->empty()); }
cAINode* cAINodeIterator::Next()
{
    cAINode* node = *mNodeIt;
    ++mNodeIt;
    if(mNodeIt == mpNodeList->end()) {
        if(IncGridPos()) {
            mpNodeList = &mpContainer->GetGrid(mvGridPos)->mlstNodes;
            while(mpNodeList->empty()) {
                if(IncGridPos()) mpNodeList = &mpContainer->GetGrid(mvGridPos)->mlstNodes;
                else { mpNodeList = NULL; break; }
            }
        } else mpNodeList = NULL;
        if(mpNodeList) mNodeIt = mpNodeList->begin();
    }
    return node;
}
bool cAINodeIterator::IncGridPos()
{
    ++mvGridPos.x;
    if(mvGridPos.x > mvEndGridPos.x) {
        mvGridPos.x = mvStartGridPos.x; ++mvGridPos.y;
        if(mvGridPos.y > mvEndGridPos.y) return false;
    }
    return true;
}

} // namespace hpl

static void Nodes(cAINodeContainer& container, int count)
{
    container.ReserveSpace(count);
    for(int i=0; i<count; ++i) {
        char name[24]; snprintf(name, sizeof(name), "node%d", i);
        container.AddNode(name, i, cVector3f((float)i, 0, 0));
    }
}
static void Edge(cAINodeContainer& container, int from, int to, float distance)
{
    cAINode* node = container.GetNode(from);
    node->AddEdge(container.GetNode(to));
    node->GetEdge(node->GetEdgeNum()-1)->mfDistance = distance;
    node->GetEdge(node->GetEdgeNum()-1)->mfSqrDistance = distance*distance;
}
static void Position(cAINode* node, const cVector3f& position)
{
    // The original object is non-const; modifying this public const-reference
    // view via const_cast is defined. No container iterator is active here.
    const_cast<cVector3f&>(node->GetPosition()) = position;
}
static void Dump()
{
    std::vector<cAStarNode*> ordered = gSearchNodes;
    std::sort(ordered.begin(), ordered.end(), [](cAStarNode* a, cAStarNode* b) { return a->mpAINode < b->mpAINode; });
    for(cAStarNode* node : ordered)
        fprintf(gOut, "search %d distance=%08x cost=%08x parent=%d\n", node->mpAINode->GetID(),
            Bits(node->mfDistance), Bits(node->mfCost), node->mpParent ? node->mpParent->mpAINode->GetID() : -1);
}

struct Callback : iAStarCallback {
    cAStarHandler* handler;
    cAINodeContainer* container;
    int mode, count = 0, rejectMask = 0;
    float mutation = 3.25f, mutation2 = 0, mutation3 = 0;
    Callback* replacement = NULL;
    Callback(cAStarHandler& h, cAINodeContainer& c, int m) : handler(&h), container(&c), mode(m) {}
    bool CanAddNode(cAINode* parent, cAINode* child) override
    {
        ++count;
        fprintf(gOut, "callback mode=%d count=%d %d->%d\n", mode, count, parent->GetID(), child->GetID());
        bool accepted = !(rejectMask && ((parent->GetID()*7 + child->GetID()*3 + count) & rejectMask) == 0);
        if(count != 1) return accepted;
        if(mode == 1) handler->SetMaxIterations(1);
        if(mode == 2) handler->SetMaxIterations(-1);
        if(mode == 3) handler->SetCallback(NULL);
        if(mode == 4) handler->SetCallback(replacement);
        if(mode == 5) {
            // Edit in-place: vector storage and the current edge pointer stay valid.
            cAINodeEdge* edge = parent->GetEdge(0);
            edge->mpNode = container->GetNode(container->GetNodeNum()-1);
            edge->mfDistance = mutation;
        }
        if(mode == 6) {
            for(cAStarNode* node : gSearchNodes) if(node->mpAINode == parent) node->mfDistance = mutation;
        }
        if(mode == 7) {
            Position(child, cVector3f(child->GetPosition().x, mutation, child->GetPosition().z));
        }
        if(mode == 8) {
            // Nested direct searches retain the old sets; the outer parent stays valid.
            Fixture& fixture = gFixtures[container]; fixture.nestedDirect = true;
            cVector3f start(0), goal(0); tAINodeList output;
            output.push_back(parent);
            bool result = handler->GetPath(start, goal, &output);
            fprintf(gOut, "nested-direct result=%d size=%llu\n", result, (unsigned long long)output.size());
            fixture.nestedDirect = false;
        }
        if(mode == 9) throw 1776;
        if(mode == 10) {
            // Nested full search deletes the old search parent. Return false and
            // use a single edge on this parent so the outer algorithm never reads
            // that freed pointer. Graph nodes themselves stay alive throughout.
            handler->SetCallback(NULL);
            handler->SetMaxIterations(0);
            Fixture& fixture = gFixtures[container];
            const cVector3f* previousStart = fixture.start;
            const cVector3f* previousGoal = fixture.goal;
            cVector3f start(2,0,0), goal(3,0,0); fixture.start = &start; fixture.goal = &goal;
            bool result = handler->GetPath(start, goal, NULL);
            fprintf(gOut, "nested-full result=%d\n", result);
            fixture.start = previousStart; fixture.goal = previousGoal;
            return false;
        }
        if(mode == 11) {
            for(cAStarNode* node : gSearchNodes) if(node->mpAINode == parent) node->mfDistance = mutation;
            parent->GetEdge(0)->mfDistance = mutation2;
        }
        if(mode == 12) {
            parent->GetEdge(0)->mfDistance = mutation;
            Position(parent, cVector3f(parent->GetPosition().x, mutation2, parent->GetPosition().z));
        }
        if(mode == 13) Position(child, cVector3f(mutation, mutation2, mutation3));
        if(mode == 14) {
            // hplNew may first allocate its __FILE__ std::string on MSVC. Target
            // the node itself, not metadata evaluated before its constructor.
            gFailAllocationSize = sizeof(cAStarNode); gFailNextAllocation = true;
        }
        if(mode == 15) {
            // Node 1 is the next candidate after this rejected edge. Its closed
            // set insertion allocates after GetBestNode has erased the open entry.
            for(cAStarNode* node : gSearchNodes) if(node->mpAINode->GetID() == 1) gOrphan = node;
            gFailAllocationSize = 0; gFailNextAllocation = true;
            return false;
        }
        if(mode == 16) {
            Position(parent, cVector3f(parent->GetPosition().x, mutation2, parent->GetPosition().z));
            Position(child, cVector3f(child->GetPosition().x, mutation, child->GetPosition().z));
        }
        return accepted;
    }
};

static void Query(const char* label, int sequence, int step, cAINodeContainer& container,
    cAStarHandler& handler, cVector3f& start, cVector3f& goal, tAINodeList* output)
{
    Fixture& fixture = gFixtures[&container]; fixture.start = &start; fixture.goal = &goal;
    fprintf(gOut, "query %s %d %d output=%d\n", label, sequence, step, output != NULL);
    try { fprintf(gOut, "result %d\n", handler.GetPath(start, goal, output)); }
    catch(int value) { fprintf(gOut, "exception %d\n", value); }
    catch(const std::bad_alloc&) { fputs("exception bad_alloc\n", gOut); }
    if(gFailNextAllocation) {
        // A deliberately planted selection/loop bug may avoid the intended next
        // insertion. Report it as output, instead of failing in Dump's own vector
        // allocation or incorrectly freeing a node still owned by a native set.
        gFailNextAllocation = false; gFailAllocationSize = 0; gOrphan = NULL;
        fputs("allocation-failure-was-not-consumed\n", gOut);
    }
    if(output) {
        fputs("path", gOut);
        for(cAINode* node : *output) fprintf(gOut, " %d", node->GetID());
        fputc('\n', gOut);
    }
    Dump();
}

static void Fixed()
{
    {
        // 1 + 2^-24 rounds back to 1 in f32 before multiplication. Promoting
        // the whole cost expression to double changes the stored cost by 1 ULP.
        cAINodeContainer container("precision", "node", NULL, cVector3f(1)); Nodes(container, 2);
        Position(container.GetNode(1), cVector3f(1, Float(0x33800000), 0));
        cAStarHandler handler(&container); Callback callback(handler, container, 0); handler.SetCallback(&callback);
        Fixture& fixture = gFixtures[&container]; fixture.starts[0] = true; fixture.goals[1] = true;
        Edge(container, 0, 1, Float(0x3f800001));
        cVector3f start(0), goal = container.GetNode(1)->GetPosition(); tAINodeList output;
        Query("cost-f32-rounding", 0, 0, container, handler, start, goal, &output);
    }
    for(int limit : {-3, -1, 0, 1, 2, 3, 4, 8}) {
        cAINodeContainer container("fixed", "node", NULL, cVector3f(1)); Nodes(container, 5);
        cAStarHandler handler(&container); handler.SetMaxIterations(limit);
        Callback callback(handler, container, 0); handler.SetCallback(&callback);
        Fixture& fixture = gFixtures[&container]; fixture.starts[0] = true; fixture.goals[4] = true;
        Edge(container, 0, 1, 8); Edge(container, 0, 2, 1); Edge(container, 0, 0, 0);
        Edge(container, 2, 1, 1); Edge(container, 1, 3, 1); Edge(container, 3, 4, 1); Edge(container, 4, 1, 0);
        cVector3f start(0), goal(4,0,0); tAINodeList output; output.push_back(container.GetNode(3));
        Query("no-relax", limit, 0, container, handler, start, goal, &output);
        fixture.direct = true;
        Query("direct-retains", limit, 1, container, handler, start, goal, &output);
        fixture.direct = false;
        Query("null-output", limit, 2, container, handler, start, goal, NULL);
    }
    for(int mode=0; mode<=10; ++mode) {
        cAINodeContainer container("callback", "node", NULL, cVector3f(1)); Nodes(container, 5);
        cAStarHandler handler(&container); handler.SetMaxIterations(mode == 2 ? 1 : -1);
        Callback callback(handler, container, mode), replacement(handler, container, 0);
        callback.replacement = &replacement; replacement.rejectMask = 1; handler.SetCallback(&callback);
        Fixture& fixture = gFixtures[&container]; fixture.starts[0] = true; fixture.goals[4] = true;
        Edge(container, 0, 1, 2);
        if(mode != 10) { Edge(container, 0, 2, 1); Edge(container, 0, 3, 7); }
        Edge(container, 1, 2, 1); Edge(container, 2, 4, 1); Edge(container, 3, 4, 1);
        cVector3f start(0), goal(4,0,0); tAINodeList output;
        Query("callback", mode, 0, container, handler, start, goal, &output);
        handler.SetCallback(NULL); handler.SetMaxIterations(-1);
        Query("callback-recovery", mode, 1, container, handler, start, goal, &output);
    }
    for(int mode : {14,15}) {
        cAINodeContainer container("allocation", "node", NULL, cVector3f(1)); Nodes(container, 3);
        cAStarHandler handler(&container); Callback callback(handler, container, mode); handler.SetCallback(&callback);
        Fixture& fixture = gFixtures[&container]; fixture.starts[0] = true; fixture.starts[1] = true; fixture.goals[2] = true;
        Edge(container, 0, 2, 1);
        cVector3f start(0), goal(2,0,0); tAINodeList output;
        Query("allocation-failure", mode, 0, container, handler, start, goal, &output);
        assert(!gFailNextAllocation);
        if(gOrphan) {
            // Original GetBestNode loses ownership if closed.insert throws after
            // open.erase. Record that exact state, then reclaim the known orphan
            // through the existing public memory observer to keep the test clean.
            hplDelete(gOrphan); gOrphan = NULL;
        }
        handler.SetCallback(NULL);
        Query("allocation-recovery", mode, 1, container, handler, start, goal, &output);
    }
    const uint32_t values[] = {0,0x80000000,1,0x80000001,0x3f800000,0xbf800000,0x3f800001,
        0x3f7fffff,0x4b800000,0x7f7fffff,0xff7fffff,0x7f800000,0xff800000,0x7fc12345,0xffc54321,0x7f812345};
    int sequence = 0;
    for(uint32_t a : values) for(uint32_t b : values) {
        cAINodeContainer container("float", "node", NULL, cVector3f(1)); Nodes(container, 5);
        cAStarHandler handler(&container); Callback callback(handler, container, 0); handler.SetCallback(&callback);
        Fixture& fixture = gFixtures[&container]; fixture.starts[0] = true; fixture.goals[4] = true;
        // Non-finite edge distances are defined: they enter only float arithmetic
        // and comparisons. Candidate node coordinates remain finite for the grid.
        Edge(container, 0, 1, Float(a)); Edge(container, 0, 2, Float(b)); Edge(container, 0, 3, 1);
        Edge(container, 1, 4, 1); Edge(container, 2, 4, 1); Edge(container, 3, 4, 1);
        Position(container.GetNode(1), cVector3f(2,1,0)); Position(container.GetNode(2), cVector3f(2,-1,0));
        cVector3f start(0), goal(4,0,0); tAINodeList output;
        Query("float-edges", sequence++, 0, container, handler, start, goal, &output);
    }
    for(int mode : {6,7}) for(uint32_t value : values) {
        cAINodeContainer container("float-callback", "node", NULL, cVector3f(1)); Nodes(container, 4);
        cAStarHandler handler(&container); Callback callback(handler, container, mode); callback.mutation = Float(value);
        handler.SetCallback(&callback); Fixture& fixture = gFixtures[&container]; fixture.starts[0] = true; fixture.goals[3] = true;
        Edge(container, 0, 1, 1); Edge(container, 0, 2, 1); Edge(container, 1, 3, 1); Edge(container, 2, 3, 1);
        cVector3f start(0), goal(3,0,0); tAINodeList output;
        Query("float-callback", sequence++, mode, container, handler, start, goal, &output);
    }
    const uint32_t nans[] = {0x7fc12345,0xffc54321,0x7f812345,0xff854321};
    for(int mode : {11,12,13,16}) for(uint32_t a : nans) for(uint32_t b : nans) {
        cAINodeContainer container("nan-pair", "node", NULL, cVector3f(1)); Nodes(container, 4);
        cAStarHandler handler(&container); Callback callback(handler, container, mode);
        callback.mutation = Float(a); callback.mutation2 = mode == 13 ? 0 : Float(b);
        callback.mutation3 = Float(b); handler.SetCallback(&callback);
        Fixture& fixture = gFixtures[&container]; fixture.starts[0] = true; fixture.goals[3] = true;
        Edge(container, 0, 1, 1); Edge(container, 0, 2, 1); Edge(container, 1, 3, 1); Edge(container, 2, 3, 1);
        cVector3f start(0), goal(3,0,0); tAINodeList output;
        Query("nan-pair", sequence++, mode, container, handler, start, goal, &output);
    }
    for(uint32_t a : nans) for(uint32_t b : nans) for(uint32_t c : nans) {
        cAINodeContainer container("nan-xyz", "node", NULL, cVector3f(1)); Nodes(container, 3);
        cAStarHandler handler(&container); Callback callback(handler, container, 13);
        callback.mutation = Float(a); callback.mutation2 = Float(b); callback.mutation3 = Float(c);
        handler.SetCallback(&callback); Fixture& fixture = gFixtures[&container];
        fixture.starts[0] = true; fixture.goals[2] = true;
        Edge(container, 0, 1, 1); Edge(container, 1, 2, 1);
        cVector3f start(0), goal(2,0,0); tAINodeList output;
        Query("nan-xyz", sequence++, 13, container, handler, start, goal, &output);
    }
    for(int count : {0,1,2,8}) {
        cAINodeContainer container("empty", "node", NULL, cVector3f(1)); Nodes(container, count);
        cAStarHandler handler(&container); cVector3f start(0), goal(1); tAINodeList output;
        Query("empty-starts", count, 0, container, handler, start, goal, &output);
        Fixture& fixture = gFixtures[&container];
        std::fill(fixture.starts.begin(), fixture.starts.end(), true);
        Query("empty-goals", count, 1, container, handler, start, goal, &output);
        std::fill(fixture.goals.begin(), fixture.goals.end(), true);
        Query("all-goals", count, 2, container, handler, start, goal, &output);
    }
}

static float RandomFinite()
{
    // Keep node positions within one finite grid; edge weights separately include
    // infinities/NaNs. Random mantissas expose rounding and f32/f64 changes.
    float magnitude = Float(0x3f000000u | (Next() & 0x007fffffu));
    return magnitude * (float)Int(-16,16);
}
static void Random()
{
    const int limits[] = {-7,-1,0,1,2,3,5,9,32};
    for(int sequence=0; sequence<2048; ++sequence) {
        int count = Int(2,20);
        cAINodeContainer container("random", "node", NULL, cVector3f(1)); Nodes(container, count);
        for(int i=0; i<count; ++i) {
            float x=RandomFinite(), y=RandomFinite(), z=RandomFinite();
            Position(container.GetNode(i), cVector3f(x,y,z));
        }
        for(int i=0; i<count; ++i) {
            int edges = Int(0,7);
            for(int e=0; e<edges; ++e) {
                int target = Int(0,count-1); float distance = RandomFinite();
                if(sequence % 7 == 0) distance = (float)Int(-2,2);
                Edge(container, i, target, distance);
            }
        }
        cAStarHandler handler(&container);
        Callback callback(handler, container, 0); handler.SetCallback(&callback);
        Fixture& fixture = gFixtures[&container];
        for(int i=0; i<count; ++i) { fixture.starts[i] = Int(0,5)==0; fixture.goals[i] = Int(0,4)==0; }
        fixture.starts[Int(0,count-1)] = true; fixture.goals[Int(0,count-1)] = true;
        tAINodeList output; output.push_back(container.GetNode(Int(0,count-1)));
        float sx=RandomFinite(), sy=RandomFinite(), sz=RandomFinite();
        float gx=RandomFinite(), gy=RandomFinite(), gz=RandomFinite();
        cVector3f start(sx,sy,sz), goal(gx,gy,gz);
        for(int step=0; step<3; ++step) {
            int limit = limits[Int(0,8)]; handler.SetMaxIterations(limit);
            callback.rejectMask = step == 1 ? 3 : 0;
            fixture.direct = step == 2 && sequence % 3 == 0;
            Query("random", sequence, step, container, handler, start, goal, step == 1 ? NULL : &output);
        }
    }
}

int main(int argc, char** argv)
{
    gOut = argc > 1 ? fopen(argv[1], "wb") : stdout;
    if(!gOut) return 2;
    gLogOut = gOut;
#if HARNESS_RUST
    RustBridge_Init();
#endif
    Fixed(); Random();
    if(!gSearchNodes.empty() || !gFixtures.empty()) return 4;
    if(gOut != stdout && fclose(gOut) != 0) return 3;
    return 0;
}
