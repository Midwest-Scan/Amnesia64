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

// Public C++ API differential test for cLogicTimer and VideoComp.
// All floating results are printed as bits, including NaNs and signed zero.
// The original's signed update-counter overflow is excluded: each sequence
// makes at most 128 WantUpdate calls before destroying/resetting its timer.

#include "system/LogicTimer.h"
#include "system/Platform.h"
#include "graphics/GraphicsTypes.h"
#include <algorithm>
#include <climits>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <vector>

#if HARNESS_RUST
#include "system/RustBridge.h"
#endif

using namespace hpl;
extern FILE* gLogOut;
extern uint32_t gClock, gClockStep, gClockReads;
static FILE* gOut;
static uint64_t gState = 0x2545F4914F6CDD1Dull;
static uint32_t Next() { gState ^= gState << 13; gState ^= gState >> 7; gState ^= gState << 17; return (uint32_t)(gState >> 16); }
static int Int(int lo, int hi) { return lo + (int)(Next() % (uint32_t)(hi - lo + 1)); }
static int SignedBits() { uint32_t u = Next(); int i; memcpy(&i, &u, sizeof(i)); return i; }
static float FloatBits(uint32_t u) { float f; memcpy(&f, &u, sizeof(f)); return f; }
static uint32_t Bits(float f) { uint32_t b; memcpy(&b, &f, sizeof(b)); return b; }
static uint64_t Bits(double f) { uint64_t b; memcpy(&b, &f, sizeof(b)); return b; }

static void State(const char* label, int sequence, int step, cLogicTimer& timer, int result = -1)
{
	double rate = 1000.0 / timer.GetLocalTimeAdd();
	fprintf(gOut, "%s %d %d result=%d clock=%08x reads=%u local=%016llx add=%016llx step=%08x rate=",
		label, sequence, step, result, gClock, gClockReads,
		(unsigned long long)Bits(timer.GetLocalTime()), (unsigned long long)Bits(timer.GetLocalTimeAdd()), Bits(timer.GetStepSize()));
	// C++ conversion to int is undefined outside this range. No such getter
	// call is made even when a deliberately extreme constructor/setter is tested.
	if(std::isfinite(rate) && rate >= (double)INT_MIN && rate < (double)INT_MAX + 1.0)
		fprintf(gOut, "%d\n", timer.GetUpdatesPerSec());
	else
		fputs("undefined-cast-skipped\n", gOut);
}

static void FixedTimers()
{
	const int rates[] = {0, 1, -1, 9, 10, 11, 30, 37, 60, 144, 1000, -37, -1000, INT_MIN, INT_MAX};
	const uint32_t speeds[] = {0x3f800000, 0x40000000, 0x3f000000, 0xbf800000, 0,
		0x80000000, 0x7f800000, 0xff800000, 0x7fc00000, 1, 0x7f7fffff, 0x3faaaaab};
	int sequence = 0;
	for(int rate : rates)
	for(uint32_t speed : speeds)
	{
		gClock = (sequence % 2) ? 0xfffffff0u : 1000u;
		gClockStep = 0;
		gClockReads = 0;
		cLogicTimer timer(rate, NULL);
		State("construct", sequence, rate, timer);
		timer.SetSpeedMul(FloatBits(speed));
		fprintf(gOut, "speed %d %08x\n", sequence, speed);
		// First run retains the constructor's cap, including zero/negative caps.
		for(int step=0; step<12; ++step)
		{
			if(step == 1) ++gClock;
			if(step == 4) gClock += 100;
			State("initial-want", sequence, step, timer, timer.WantUpdate());
		}
		timer.EndUpdateLoop();
		State("initial-end", sequence, 0, timer);
		timer.SetMaxUpdates(4);
		State("set-max", sequence, 4, timer);
		for(int loop=0; loop<4; ++loop)
		{
			gClock += 1001;
			gClockStep = (uint32_t)loop;
			for(int step=0; step<8; ++step)
				State("want", sequence, loop*8+step, timer, timer.WantUpdate());
			timer.EndUpdateLoop();
			State("end", sequence, loop, timer);
		}
		// Changing frequency resets time but retains cap and update count.
		timer.WantUpdate();
		timer.SetUpdatesPerSec(rate == 37 ? 144 : 37);
		State("set-rate", sequence, 0, timer);
		gClock += 500;
		for(int step=0; step<7; ++step)
			State("after-rate", sequence, step, timer, timer.WantUpdate());
		timer.Reset();
		State("reset", sequence, 0, timer);
		timer.EndUpdateLoop();
		State("final-end", sequence, 0, timer);
		++sequence;
	}

	// Strict equality, failed requests consuming the cap, and Reset not clearing it.
	gClock = 5000; gClockStep = 0; gClockReads = 0;
	cLogicTimer timer(1000, NULL);
	timer.SetMaxUpdates(2);
	for(int step=0; step<4; ++step) State("equal", 0, step, timer, timer.WantUpdate());
	gClock = 9000;
	timer.Reset();
	State("reset-capped", 0, 0, timer, timer.WantUpdate());
	timer.EndUpdateLoop();
	State("reset-end", 0, 0, timer);
	for(int cap : {-2, -1, 0, 1, 2, INT_MAX})
	{
		timer.SetMaxUpdates(cap);
		gClockStep = 1;
		for(int step=0; step<6; ++step) State("cap", cap, step, timer, timer.WantUpdate());
		timer.EndUpdateLoop();
		State("cap-end", cap, 0, timer);
	}
}

static void RandomTimers()
{
	const int rates[] = {0, -1, 1, 7, 10, 29, 37, 59, 60, 144, 1000, 1000003, -144, INT_MIN, INT_MAX};
	const uint32_t speeds[] = {0, 0x80000000, 0x3f800000, 0x40000000, 0x3f000000,
		0xbf800000, 0x7f800000, 0xff800000, 0x7fc00000, 1, 0x3eaaaaab, 0x7f7fffff};
	for(int sequence=0; sequence<1000; ++sequence)
	{
		gClock = Next(); gClockStep = Next() % 5; gClockReads = 0;
		int rate = rates[Next() % (sizeof(rates)/sizeof(*rates))];
		cLogicTimer timer(rate, NULL);
		State("random-construct", sequence, rate, timer);
		for(int step=0; step<96; ++step)
		{
			int operation = Int(0, 11), result = -1;
			switch(operation)
			{
			case 0: timer.Reset(); break;
			case 1: case 2: timer.EndUpdateLoop(); break;
			case 3:
				rate = rates[Next() % (sizeof(rates)/sizeof(*rates))];
				timer.SetUpdatesPerSec(rate);
				fprintf(gOut, "random-rate %d %d %d\n", sequence, step, rate);
				break;
			case 4:
				result = Int(-3, 15); timer.SetMaxUpdates(result); break;
			case 5:
				{ uint32_t speed = speeds[Next() % (sizeof(speeds)/sizeof(*speeds))];
				  timer.SetSpeedMul(FloatBits(speed));
				  fprintf(gOut, "random-speed %d %d %08x\n", sequence, step, speed); }
				break;
			case 6: gClock += (uint32_t)Int(0, 2000); break;
			case 7: gClock = Next(); gClockStep = Next() % 7; break;
			default: result = timer.WantUpdate(); break;
			}
			State("random", sequence, step*16+operation, timer, result);
		}
	}
}

static cVideoMode Mode(int display, int width, int height, int bpp, int refresh, bool custom)
{
	cVideoMode mode(display, cVector2l(width, height), bpp, refresh);
	mode.mbCustom = custom;
	return mode;
}

static void PrintMode(const cVideoMode& mode)
{
	fprintf(gOut, "%d,%d,%d,%d,%d,%d", mode.mlDisplay, mode.mvScreenSize.x, mode.mvScreenSize.y,
		mode.mlBitsPerPixel, mode.mlRefreshRate, (int)mode.mbCustom);
}

static void Pair(const cVideoMode& a, const cVideoMode& b, int id)
{
	fprintf(gOut, "compare %d ", id); PrintMode(a); fputc(' ', gOut); PrintMode(b);
	fprintf(gOut, " %d %d %d %d\n", (int)VideoComp()(a,b), (int)VideoComp()(b,a),
		(int)VideoComp()(a,a), (int)VideoComp()(b,b));
}

static void VideoModes()
{
	const int values[] = {INT_MIN, -1000, -1, 0, 1, 480, 1080, 1920, INT_MAX};
	std::vector<cVideoMode> modes;
	for(int value : values)
	{
		modes.push_back(Mode(value, 1920, 1080, 32, 60, false));
		modes.push_back(Mode(0, value, 1080, 16, 144, true));
		modes.push_back(Mode(0, 1920, value, 24, -1, false));
		modes.push_back(Mode(0, 1920, 1080, value, value, true));
	}
	int id = 0;
	for(const cVideoMode& a : modes) for(const cVideoMode& b : modes) Pair(a,b,id++);
	for(int i=0; i<12000; ++i)
	{
		// Read RNG results in separate statements to avoid argument evaluation order.
		int display = SignedBits(), width = SignedBits(), height = SignedBits();
		int bpp = SignedBits(), refresh = SignedBits();
		bool custom = (Next() & 1) != 0;
		cVideoMode a = Mode(display, width, height, bpp, refresh, custom), b = a;
		if(i % 4 == 0) b.mlDisplay = SignedBits();
		if(i % 4 <= 1) b.mvScreenSize.x = SignedBits();
		if(i % 4 <= 2) b.mvScreenSize.y = SignedBits();
		b.mlBitsPerPixel = SignedBits(); b.mlRefreshRate = SignedBits(); b.mbCustom = !custom;
		Pair(a,b,id++);
	}
	for(int sequence=0; sequence<100; ++sequence)
	{
		modes.clear();
		for(int i=0; i<48; ++i)
		{
			int display = Int(-1, 2), width = Int(0, 4)*640, height = Int(0, 3)*480;
			// BPP stores the input index, exposing stable order for equal sort keys.
			modes.push_back(Mode(display, width, height, i, 144-i, (i & 1) != 0));
		}
		std::stable_sort(modes.begin(), modes.end(), VideoComp());
		for(const cVideoMode& mode : modes)
		{ fprintf(gOut, "sorted %d ", sequence); PrintMode(mode); fputc('\n', gOut); }
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
	FixedTimers();
	RandomTimers();
	VideoModes();
	if(gOut != stdout && fclose(gOut) != 0) return 3;
	return 0;
}
