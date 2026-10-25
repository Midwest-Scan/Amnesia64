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

// Deterministic host clock and log capture for LogicTimer / VideoComp.
// The clock wraps at 32 bits on every host, matching Windows GetApplicationTime.
#include "system/LowLevelSystem.h"
#include "system/Platform.h"
#include <cstdint>
#include <cstdio>

FILE* gLogOut = stdout;
uint32_t gClock = 0;
uint32_t gClockStep = 0;
uint32_t gClockReads = 0;

namespace hpl {

	static void Print(const char* prefix, const char* fmt, va_list ap)
	{
		fprintf(gLogOut, "%s", prefix);
		vfprintf(gLogOut, fmt, ap);
	}

	void Error(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("ERROR: ", fmt, ap); va_end(ap); }
	void Warning(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("WARNING: ", fmt, ap); va_end(ap); }
	void Log(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("LOG: ", fmt, ap); va_end(ap); }

	unsigned long cPlatform::GetApplicationTime()
	{
		uint32_t value = gClock;
		gClock += gClockStep;
		++gClockReads;
		return value;
	}

	// Platform.cpp also contains message-box forwarding; no GUI is needed here.
	void cPlatform::CreateMessageBoxBase(eMsgBoxType, const wchar_t*, const wchar_t*, va_list) {}
}
