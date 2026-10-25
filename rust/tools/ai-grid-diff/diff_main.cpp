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

// Public graph/grid differential test, selected by HPL2_RUST_AI.
// Compile the real AINodeContainer.cpp, retaining real node allocation, grid
// vectors, list state and iterator behavior. Unused XML/physics sections are
// eliminated where possible; abort-only unused dependencies satisfy MSVC linking.
// No grid or graph algorithm is replaced by a fixture.
// Empty BuildNodeGridMap is undefined in C++; zero-axis and nonfinite grid
// queries are excluded because division/cast is undefined. AddEdge has no such
// integer conversions and is tested with all floating-point edge cases.
#include "ai/AINodeContainer.h"
#include <cassert>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <new>
#if HARNESS_RUST
#include "system/RustBridge.h"
#endif

using namespace hpl;
extern FILE* gLogOut;
extern bool gTraceAlloc;
extern int gFailAllocation;
extern void (*gBeforeAllocation)();
static FILE* gOut;
static uint64_t gRandom = 0x5A17E8A9D3C462BFull;
static uint32_t Next() { gRandom ^= gRandom << 13; gRandom ^= gRandom >> 7; gRandom ^= gRandom << 17; return (uint32_t)(gRandom >> 16); }
static int Int(int lo, int hi) { return lo + (int)(Next() % (uint32_t)(hi - lo + 1)); }
static float Float(uint32_t value) { float f; memcpy(&f, &value, 4); return f; }
static uint32_t Bits(float value) { uint32_t u; memcpy(&u, &value, 4); return u; }
static float RandomFloat() { return Float(0x3f000000u | (Next() & 0x007fffffu)) * (float)Int(-32,32); }

static int Index(cAINodeContainer& container, cAINode* node)
{
    for(int i=0; i<container.GetNodeNum(); ++i) if(container.GetNode(i) == node) return i;
    return -1;
}
static void Position(cAINode* node, const cVector3f& value)
{
    // The underlying object is non-const; this public reference view permits
    // defined position edits between builds without any private test hook.
    const_cast<cVector3f&>(node->GetPosition()) = value;
}
static void Add(cAINodeContainer& container, const cVector3f& position)
{
    int index = container.GetNodeNum(); char name[32];
    snprintf(name,sizeof(name),"node%d",index);
    container.AddNode(name,index,position);
    fprintf(gOut,"node %d %08x,%08x,%08x\n",index,Bits(position.x),Bits(position.y),Bits(position.z));
}
static bool Build(cAINodeContainer& container, const char* label, int sequence, int fail = -1)
{
    fprintf(gOut,"build %s %d nodes=%d fail=%d\n",label,sequence,container.GetNodeNum(),fail);
    gTraceAlloc = true; gFailAllocation = fail;
    bool result = true;
    try { container.BuildNodeGridMap(); fputs("built\n",gOut); }
    catch(const std::bad_alloc&) { fputs("exception bad_alloc\n",gOut); result = false; }
    // A mutant may skip the targeted allocation; record this instead of letting
    // unrelated output allocations trigger the fault later.
    if(gFailAllocation >= 0) fprintf(gOut,"allocation-fault-unconsumed %d\n",gFailAllocation);
    gFailAllocation = -1; gTraceAlloc = false;
    return result;
}
static void Query(cAINodeContainer& container, const cVector3f& position, float radius, int id)
{
    fprintf(gOut,"query %d %08x,%08x,%08x radius=%08x\n",id,Bits(position.x),Bits(position.y),Bits(position.z),Bits(radius));
    cAINodeIterator it = container.GetNodeIterator(position,radius);
    int count = 0;
    while(it.HasNext()) {
        cAINode* node = it.Next();
        fprintf(gOut,"found %d %d\n",count++,Index(container,node));
        if(count > 100000) throw 9001; // protects against a looping mutant
    }
    fprintf(gOut,"end %d has-next=%d\n",count,it.HasNext());
}
static void Queries(cAINodeContainer& container, int sequence)
{
    Query(container,cVector3f(0),200.0f,0);
    Query(container,cVector3f(-100,-800,-100),0.0f,1);
    Query(container,cVector3f(200,900,200),0.0f,2);
    Query(container,cVector3f(8,0,7),-4.0f,3);
    for(int q=0;q<12;++q) {
        float x=RandomFloat()*2, z=RandomFloat()*2, radius=(float)Int(-6,70)+Float(0x3eaaaaab);
        Query(container,cVector3f(x,(float)sequence,z),radius,q+4);
    }
}
static cAINodeContainer* gGrowingContainer;
static void AddDuringResize()
{
    // The allocator callback adds one node strictly inside the already-computed
    // bounds. No iterator/reference into mvNodes survives the allocation. The
    // original placement loop must observe the larger live node count.
    Add(*gGrowingContainer,cVector3f(5,0,5));
}
static void Edges()
{
    const uint32_t values[] = {0,0x80000000,1,0x80000001,0x00800000,0x80800000,
        0x3f800000,0xbf800000,0x3f800001,0x3f7fffff,0x4b800000,0x7f7fffff,0xff7fffff,
        0x7f800000,0xff800000,0x7fc12345,0xffc54321,0x7f812345,0xff854321};
    int sequence = 0;
    for(uint32_t a : values) for(uint32_t b : values) for(int mode=0;mode<6;++mode) {
        cAINodeContainer container("edge","node",NULL,cVector3f(1));
        cVector3f from(Float(a),Float(b),Float(a)), to(Float(b),Float(a),Float(b));
        if(mode==1) { from=cVector3f(Float(a),0,0); to=cVector3f(Float(b),0,0); }
        if(mode==2) { from=cVector3f(0,Float(a),0); to=cVector3f(0,Float(b),0); }
        if(mode==3) { from=cVector3f(Float(a),Float(b),0); to=cVector3f(0); }
        if(mode==4) { from=cVector3f(Float(a),0,Float(b)); to=cVector3f(0); }
        if(mode==5) { from=cVector3f(0,Float(a),Float(b)); to=cVector3f(0); }
        fprintf(gOut,"edge-case %d mode=%d\n",sequence++,mode);
        Add(container,from); Add(container,to);
        for(int repeat=0;repeat<3;++repeat) {
            cAINode* first=container.GetNode(0);
            first->AddEdge(container.GetNode(repeat==2?0:1));
            cAINodeEdge* edge=first->GetEdge(first->GetEdgeNum()-1);
            fprintf(gOut,"edge %d destination=%d distance=%08x square=%08x count=%d\n",repeat,
                Index(container,edge->mpNode),Bits(edge->mfDistance),Bits(edge->mfSqrDistance),first->GetEdgeNum());
        }
    }
    for(int sequence=0;sequence<4096;++sequence) {
        cAINodeContainer container("random-edge","node",NULL,cVector3f(1));
        float ax=RandomFloat(), ay=RandomFloat(), az=RandomFloat();
        float bx=RandomFloat(), by=RandomFloat(), bz=RandomFloat();
        Add(container,cVector3f(ax,ay,az)); Add(container,cVector3f(bx,by,bz));
        container.GetNode(0)->AddEdge(container.GetNode(1));
        cAINodeEdge* edge=container.GetNode(0)->GetEdge(0);
        fprintf(gOut,"random-edge %d %08x %08x\n",sequence,Bits(edge->mfDistance),Bits(edge->mfSqrDistance));
    }
}
static void Fixed()
{
    const int sizes[] = {1,2,3,5,6,7,12,13,14,15,16,36,37,38,39,40,72,73,74,75,120,121,122,123,216};
    for(int count:sizes) {
        cAINodeContainer container("size","node",NULL,cVector3f(1));
        for(int i=0;i<count;++i) Add(container,cVector3f((float)(i%7),0,(float)((i*3)%11)));
        if(count>1) Position(container.GetNode(1),cVector3f(12,0,10));
        Build(container,"dimension",count);
        if(count>1) Queries(container,count);
        Build(container,"repeat",count);
        if(count>1) Queries(container,count);
        if(count>1) {
            for(int i=0;i<count;++i) {
                cVector3f p=container.GetNode(i)->GetPosition();
                Position(container.GetNode(i),cVector3f(p.x*0.5f+2.25f,p.y,p.z*2.0f-1.75f));
            }
            Build(container,"moved",count); Queries(container,count);
        }
    }
    for(int mode=0;mode<4;++mode) {
        cAINodeContainer container("degenerate","node",NULL,cVector3f(1));
        for(int i=0;i<8;++i) Add(container,cVector3f(mode==0 ? 4.0f : (float)i,0,mode==1 ? 7.0f : (float)i));
        if(mode==2) for(int i=0;i<8;++i) Position(container.GetNode(i),cVector3f(-0.0f,0,0.0f));
        if(mode==3) Position(container.GetNode(0),cVector3f(Float(0x7fc12345),0,Float(0xffc54321)));
        // These builds are defined: nonpositive/NaN cell sizes skip casts.
        // Do not query until positive finite extents have been restored.
        Build(container,"degenerate",mode);
        for(int i=0;i<8;++i) Position(container.GetNode(i),cVector3f((float)i,0,(float)(7-i)));
        Build(container,"restored-extents",mode); Queries(container,mode);
    }
    for(uint32_t bits : {0x00800000u,0x33800001u,0x3a800001u,0x3f800000u,0x4b000001u}) {
        float scale=Float(bits);
        cAINodeContainer container("scaled","node",NULL,cVector3f(1));
        Add(container,cVector3f(0)); Add(container,cVector3f(scale,700,scale*2.0f));
        Add(container,cVector3f(scale*0.25f,-900,scale*0.75f));
        Build(container,"scaled",(int)bits);
        // The smallest cell is still a nonzero normal/subnormal. Keep all
        // query-to-cell ratios small and finite so the original casts are defined.
        Query(container,cVector3f(scale,0,scale*2.0f),0.0f,0);
        Query(container,cVector3f(scale*0.25f,100,scale*0.75f),scale*0.125f,1);
        Query(container,cVector3f(0),scale*2.0f,2);
        Query(container,cVector3f(scale,0,scale),-scale*0.5f,3);
    }
    {
        cAINodeContainer container("allocation-callback","node",NULL,cVector3f(1));
        Add(container,cVector3f(0)); Add(container,cVector3f(10,0,10));
        gGrowingContainer=&container; gBeforeAllocation=AddDuringResize;
        Build(container,"live-node-count",0);
        assert(gBeforeAllocation==NULL);
        Query(container,cVector3f(5,0,5),20,0);
        gGrowingContainer=NULL;
    }
    for(int fail : {0,1,3,10}) {
        cAINodeContainer container("allocation","node",NULL,cVector3f(1));
        Add(container,cVector3f(0)); Add(container,cVector3f(10,0,10));
        Build(container,"new-grid-failure",fail,fail);
        Build(container,"new-grid-recovery",fail); Queries(container,fail);
        Build(container,"append-failure",fail,fail%2);
        Queries(container,fail); // initialized grid remains valid after partial append
        Build(container,"append-recovery",fail); Queries(container,fail);
    }
}
static void Random()
{
    for(int sequence=0;sequence<2048;++sequence) {
        cAINodeContainer container("random","node",NULL,cVector3f(1));
        int count=Int(2,64); container.ReserveSpace(count+12);
        Add(container,cVector3f(-32,0,-48)); Add(container,cVector3f(64,0,96));
        for(int i=2;i<count;++i) { float x=RandomFloat(), y=RandomFloat(), z=RandomFloat(); Add(container,cVector3f(x,y,z)); }
        Build(container,"random",sequence); Queries(container,sequence);
        if(sequence%4==0) {
            for(int i=0;i<count;++i) {
                cVector3f p=container.GetNode(i)->GetPosition();
                Position(container.GetNode(i),cVector3f(p.x+3.25f,p.y,p.z-7.5f));
            }
            if(sequence%8==0) for(int i=0;i<12;++i) Add(container,cVector3f((float)i,0,(float)(i*2)));
            Build(container,"random-rebuild",sequence); Queries(container,sequence);
        }
    }
}
int main(int argc,char** argv)
{
    gOut=argc>1 ? fopen(argv[1],"wb") : stdout;
    if(!gOut) return 2;
    gLogOut=gOut;
#if HARNESS_RUST
    RustBridge_Init();
#endif
    Edges(); Fixed(); Random();
    if(gOut!=stdout && fclose(gOut)!=0) return 3;
    return 0;
}
