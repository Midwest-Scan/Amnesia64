/*
 * Copyright © 2009-2020 Frictional Games
 * 
 * This file is part of Amnesia: The Dark Descent.
 * 
 * Amnesia: The Dark Descent is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version. 

 * Amnesia: The Dark Descent is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Amnesia: The Dark Descent.  If not, see <https://www.gnu.org/licenses/>.
 */

// Public C++ API differential test for VoxelMap (HPL2_RUST_GENERATE).
// The actual iLowLevelGraphics interface is subclassed; no engine header is
// replaced. Every sphere center, radius, color, order and count is recorded.

#include "generate/VoxelMap.h"
#include "generate/Generate.h"
#include "capture_graphics.h"
#include <climits>
#include <cstdint>
#include <cstdio>
#include <cstring>

#if HARNESS_RUST
#include "system/RustBridge.h"
#endif

using namespace hpl;
extern FILE* gLogOut;
extern bool gTraceArrays;
static FILE* gOut;
static uint64_t gState = 0x6a09e667f3bcc909ull;
static int gRender;
static uint32_t Next() { gState ^= gState << 13; gState ^= gState >> 7; gState ^= gState << 17; return (uint32_t)(gState >> 16); }
static int Int(int lo, int hi) { return lo + (int)(Next() % (uint32_t)(hi - lo + 1)); }
static float FloatBits(uint32_t u) { float f; memcpy(&f, &u, sizeof(f)); return f; }
static uint32_t Bits(float f) { uint32_t b; memcpy(&b, &f, sizeof(b)); return b; }
static char CharBits(unsigned char u) { char c; memcpy(&c, &u, sizeof(c)); return c; }

void CaptureGraphics::DrawSphere(const cVector3f& pos, float radius, cColor col)
{
	fprintf(gOut, "sphere %d %d %08x,%08x,%08x r=%08x c=%08x,%08x,%08x,%08x\n", gRender, calls++,
		Bits(pos.x), Bits(pos.y), Bits(pos.z), Bits(radius), Bits(col.r), Bits(col.g), Bits(col.b), Bits(col.a));
	// Renderer callbacks may legally call public map setters. The original
	// reloads state/buffer/loop bounds after each callback, and color is a live
	// const reference to the caller's non-const object (never a const_cast).
	if(calls == 1 && mutation != 0)
	{
		fprintf(gOut, "callback-mutation %d\n", mutation);
		if(mutation == 1)
		{
			map->SetPosition(cVector3f(-7.5f, 4.25f, -0.0f));
			map->SetVoxelSize(-0.375f);
			*color = cColor(0.25f, -0.0f, 8.0f, 0.125f);
		}
		if(mutation == 2)
		{
			map->SetVoxel(cVector3l(1,0,0), 0);
			map->SetVoxel(cVector3l(2,0,0), 1);
			map->SetVoxel(cVector3l(0,1,0), 1);
		}
		if(mutation == 3)
		{
			map->SetSize(cVector3l(2,3,2));
			map->SetVoxel(cVector3l(1,0,0), 1);
			map->SetVoxel(cVector3l(0,2,1), 1);
		}
		if(mutation == 4) map->SetSize(cVector3l(0,0,0));
		if(mutation == 5)
		{
			map->SetSize(cVector3l(1,1,1));
			map->SetVoxel(cVector3l(0,0,0), 1);
		}
		if(mutation == 6)
		{
			map->SetPosition(cVector3f(8,9,10));
			map->SetVoxel(cVector3l(1,0,0), 0);
			throw 1776;
		}
	}
}

static void State(const char* label, int sequence, int step, cVoxelMap& map)
{
	const cVector3l& size = map.GetSize();
	const cVector3f& pos = map.GetPosition();
	fprintf(gOut, "%s %d %d size=%d,%d,%d voxel=%08x pos=%08x,%08x,%08x\n", label, sequence, step,
		size.x, size.y, size.z, Bits(map.GetVoxelSize()), Bits(pos.x), Bits(pos.y), Bits(pos.z));
}

static void Render(cVoxelMap& map, cColor& color, int mutation = 0)
{
	CaptureGraphics graphics;
	graphics.map = &map;
	graphics.color = &color;
	graphics.mutation = mutation;
	fprintf(gOut, "render %d\n", gRender);
	map.DebugRender(&graphics, color);
	fprintf(gOut, "render-end %d count=%d\n", gRender++, graphics.calls);
}

static void Write(cVoxelMap& map, const cVector3l& pos, unsigned char value)
{
	fprintf(gOut, "write %d,%d,%d=%02x\n", pos.x, pos.y, pos.z, value);
	map.SetVoxel(pos, CharBits(value));
}

static bool ValidFlatIndex(const cVector3l& size, const cVector3l& pos)
{
	// Every signed intermediate in the original must fit int, and the final
	// index must address the allocation. Individual axis bounds are NOT
	// checked: e.g. (-1,1,0) aliases the last byte of the preceding row.
	int64_t zx = (int64_t)pos.z * size.x, zxy = zx * size.y;
	int64_t yx = (int64_t)pos.y * size.x, sum = zxy + yx, index = sum + pos.x;
	int64_t count = (int64_t)size.x * size.y * size.z;
	return zx >= INT_MIN && zx <= INT_MAX && zxy >= INT_MIN && zxy <= INT_MAX &&
		yx >= INT_MIN && yx <= INT_MAX && sum >= INT_MIN && sum <= INT_MAX && index >= 0 && index < count;
}

static void FixedCases()
{
	cColor color(0.25f, 0.5f, 0.75f, 1.0f);
	cGenerate generate;
	generate.Init(NULL, NULL);
	cVoxelMap* factoryMap = generate.CreateVoxelMap(cVector3l(3,2,4));
	State("factory", 0, 0, *factoryMap);
	Render(*factoryMap, color);
	// Dense non-cubic map exposes x/y/z indexing, traversal, and half-size offsets.
	for(int z=0; z<4; ++z) for(int y=0; y<2; ++y) for(int x=0; x<3; ++x)
		Write(*factoryMap, cVector3l(x,y,z), 1);
	factoryMap->SetPosition(cVector3f(-1.125f, 2.75f, -13.0f));
	factoryMap->SetVoxelSize(0.3f);
	Render(*factoryMap, color);
	factoryMap->SetSize(factoryMap->GetSize()); // aliased size argument, same size still clears
	State("same-size", 0, 0, *factoryMap);
	Render(*factoryMap, color);
	generate.DestroyVoxelMap(factoryMap);

	cVoxelMap single(cVector3l(1,1,1));
	for(int value=0; value<256; ++value)
	{
		Write(single, cVector3l(0,0,0), (unsigned char)value);
		Render(single, color); // exactly 1 renders; all other byte patterns do not
	}

	const cVector3l sizes[] = {cVector3l(0,0,0), cVector3l(0,3,4), cVector3l(2,0,4), cVector3l(2,3,0),
		cVector3l(1,1,1), cVector3l(2,3,4), cVector3l(4,2,3),
		cVector3l(-2,-3,4), cVector3l(-2,3,-4), cVector3l(2,-3,-4), cVector3l(-2,0,4)};
	for(int sequence=0; sequence<(int)(sizeof(sizes)/sizeof(*sizes)); ++sequence)
	{
		// Small products are representable signed ints and nonnegative. Two
		// negative dimensions can allocate valid storage but render no cells.
		cVoxelMap map(sizes[sequence]);
		State("dimensions", sequence, 0, map);
		Render(map, color);
		const cVector3l aliases[] = {cVector3l(0,0,0), cVector3l(-1,1,0), cVector3l(0,-1,1),
			cVector3l(7,-1,0), cVector3l(17,2,-1), cVector3l(-5,3,0), cVector3l(23,0,0)};
		for(int step=0; step<(int)(sizeof(aliases)/sizeof(*aliases)); ++step)
		{
			if(!ValidFlatIndex(sizes[sequence], aliases[step])) continue;
			Write(map, aliases[step], 1);
			Render(map, color);
		}
		map.SetSize(cVector3l(2,3,1));
		State("resize", sequence, 0, map);
		Render(map, color);
	}

	const uint32_t floats[] = {0, 0x80000000, 0x3dcccccd, 0x3eaaaaab, 0x3f800000, 0xbf800000,
		0x00000001, 0x007fffff, 0x00800000, 0x4b000001, 0x7f7fffff, 0x7f800000, 0xff800000, 0x7fc00000,
		0x7fc12345, 0xffc54321, 0x7f812345, 0xff854321};
	const int floatCount = (int)(sizeof(floats)/sizeof(*floats));
	cVoxelMap map(cVector3l(3,2,2));
	Write(map, cVector3l(0,0,0), 1);
	Write(map, cVector3l(1,0,1), 1);
	Write(map, cVector3l(2,1,1), 1);
	// Include distinct signed quiet and signaling NaN payloads in both inputs;
	// raw-bit comparison exposes compiler-dependent operand selection/quieting.
	for(int sizeIndex=0; sizeIndex<floatCount; ++sizeIndex)
	for(int posIndex=0; posIndex<floatCount; ++posIndex)
	{
		map.SetVoxelSize(FloatBits(floats[sizeIndex]));
		map.SetPosition(cVector3f(FloatBits(floats[posIndex]), -13.25f, FloatBits(floats[(posIndex+3)%floatCount])));
		color = cColor(FloatBits(floats[(posIndex+1)%floatCount]), -0.0f, 0.33333334f, FloatBits(0x7fc12345));
		State("float-edges", sizeIndex, posIndex, map);
		Render(map, color);
	}

	for(int mutation=1; mutation<=5; ++mutation)
	{
		map.SetSize(cVector3l(3,2,2));
		map.SetPosition(cVector3f(1,2,3));
		map.SetVoxelSize(0.2f);
		color = cColor(1,0,0,1);
		Write(map, cVector3l(0,0,0), 1);
		Write(map, cVector3l(1,0,0), 1);
		Write(map, cVector3l(2,1,1), 1);
		Render(map, color, mutation);
		State("after-callback", mutation, 0, map);
		Render(map, color);
	}

	// Exceptions from the C++ renderer must preserve the completed draw prefix
	// and map edits, stop traversal, then propagate back to the C++ caller.
	map.SetSize(cVector3l(3,1,1));
	map.SetPosition(cVector3f(0,0,0));
	map.SetVoxelSize(0.25f);
	for(int x=0; x<3; ++x) Write(map, cVector3l(x,0,0), 1);
	try
	{
		Render(map, color, 6);
		fputs("exception-missing\n", gOut);
	}
	catch(int value)
	{
		fprintf(gOut, "exception-caught %d\n", value);
	}
	State("after-exception", 0, 0, map);
	Render(map, color);
}

static float RandomFloat()
{
	// All finite bit patterns, including subnormals, with occasional infinities
	// and canonical NaN. No float-to-int casts occur in the tested C++ methods.
	uint32_t value = Next();
	if((value & 31) == 0) return FloatBits(0x7fc00000);
	if((value & 31) == 1) return FloatBits(0x7f800000);
	return FloatBits(value & 0xfeffffffu);
}

static void RandomCases()
{
	for(int sequence=0; sequence<2048; ++sequence)
	{
		// Read RNG outputs in separate statements to avoid constructor-argument
		// evaluation order differences between C++ compilers.
		int sx=Int(1,5), sy=Int(1,5), sz=Int(1,5);
		cVoxelMap map(cVector3l(sx,sy,sz));
		float r=RandomFloat(), g=RandomFloat(), b=RandomFloat(), a=RandomFloat();
		cColor color(r,g,b,a);
		State("random-create", sequence, 0, map);
		for(int step=0; step<32; ++step)
		{
			int operation = Int(0,11);
			if(operation <= 6)
			{
				const cVector3l& size = map.GetSize();
				int index=Int(0,size.x*size.y*size.z-1);
				int y=Int(-4,6), z=Int(-4,6);
				int x=index - z*size.x*size.y - y*size.x;
				// All intermediates are bounded by a few hundred, and the
				// deliberately aliased coordinates resolve to a valid flat index.
				unsigned char value = operation < 4 ? 1 : (unsigned char)Next();
				Write(map, cVector3l(x,y,z), value);
			}
			else if(operation == 7)
			{
				sx=Int(1,5); sy=Int(1,5); sz=Int(1,5);
				map.SetSize(cVector3l(sx,sy,sz));
			}
			else if(operation == 8) map.SetSize(map.GetSize());
			else if(operation == 9) map.SetVoxelSize(RandomFloat());
			else if(operation == 10)
			{
				float x=RandomFloat(), y=RandomFloat(), z=RandomFloat();
				map.SetPosition(cVector3f(x,y,z));
			}
			else
			{
				r=RandomFloat(); g=RandomFloat(); b=RandomFloat(); a=RandomFloat();
				color=cColor(r,g,b,a);
			}
			State("random", sequence, step*16+operation, map);
			if(step % 4 == 0) Render(map, color);
		}
		Render(map, color);
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
	gTraceArrays = true;
	FixedCases();
	RandomCases();
	gTraceArrays = false;
	if(gOut != stdout && fclose(gOut) != 0) return 3;
	return 0;
}
