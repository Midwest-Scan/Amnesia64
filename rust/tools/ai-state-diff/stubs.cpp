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

// HPL2_RUST_AI selects the transition/update implementation in both variants.
// The production state machine and ordinary derived callbacks supply all state.
#include "system/LowLevelSystem.h"
#include <cstdarg>
#include <cstdio>

FILE* gLogOut = stdout;
bool gThrowWarning = false;

namespace hpl {
static void Print(const char* prefix, const char* format, va_list args)
{
    fprintf(gLogOut, "%s", prefix);
    vfprintf(gLogOut, format, args);
}
void Error(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("ERROR: ", fmt, ap); va_end(ap); }
void Log(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("LOG: ", fmt, ap); va_end(ap); }
void Warning(const char* fmt, ...)
{
    va_list ap; va_start(ap, fmt); Print("WARNING: ", fmt, ap); va_end(ap);
    if(gThrowWarning) { gThrowWarning=false; throw 606; }
}
}
