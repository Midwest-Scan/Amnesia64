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

// Engine log capture; no GPU or OS resources are created by this harness.
#include "system/LowLevelSystem.h"
#include <cstdarg>
#include <cstdio>
#include <cstdlib>
#include <new>

FILE* gLogOut = stdout;
bool gTraceArrays = false;

// The tested unsigned-char buffers have no array cookie. Record requested
// bytes, never addresses, so over-allocation is observable too. Instrumentation
// is enabled only during the corpus, avoiding process/runtime startup noise.
void* operator new[](std::size_t size)
{
	void* data = std::malloc(size ? size : 1);
	if(!data) throw std::bad_alloc();
	if(gTraceArrays) fprintf(gLogOut, "array-new %llu\n", (unsigned long long)size);
	return data;
}

void operator delete[](void* data) noexcept
{
	if(gTraceArrays && data) fputs("array-delete\n", gLogOut);
	std::free(data);
}

void operator delete[](void* data, std::size_t) noexcept
{
	operator delete[](data);
}

namespace hpl {
	static void Print(const char* prefix, const char* fmt, va_list ap)
	{
		fprintf(gLogOut, "%s", prefix);
		vfprintf(gLogOut, fmt, ap);
	}
	void Error(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("ERROR: ", fmt, ap); va_end(ap); }
	void Warning(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("WARNING: ", fmt, ap); va_end(ap); }
	void Log(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("LOG: ", fmt, ap); va_end(ap); }
}
