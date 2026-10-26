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

// Native logging/registration dependencies of the original cString source.
#include "system/LowLevelSystem.h"
#include "system/SerializeClass.h"
#include <cstdarg>
#include <cstdio>
FILE* gLogOut=stdout;
namespace hpl {
static void Print(const char* prefix,const char* format,va_list args) { fprintf(gLogOut,"%s",prefix); vfprintf(gLogOut,format,args); }
void Error(const char* f,...) { va_list a; va_start(a,f); Print("ERROR: ",f,a); va_end(a); }
void Warning(const char* f,...) { va_list a; va_start(a,f); Print("WARNING: ",f,a); va_end(a); }
void Log(const char* f,...) { va_list a; va_start(a,f); Print("LOG: ",f,a); va_end(a); }
cSerializeClass::cSerializeClass(const char*,const char*,cSerializeMemberField*,size_t,iSerializable* (*)()) {}
}

#if HARNESS_NATIVE_FILES
// These unused PlatformWin32 timer imports must link even though the smoke
// exercises only its real file/directory functions. Abort if reached.
#include <cstdlib>
extern "C" unsigned int __cdecl SDL_GetTicks(void) { std::abort(); }
extern "C" void __cdecl SDL_Delay(unsigned int) { std::abort(); }
#endif
