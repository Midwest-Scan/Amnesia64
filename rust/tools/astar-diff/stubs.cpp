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

// Log and allocation observers for the normal MEMORY_MANAGER_ACTIVE interface.
// Shared by both HPL2_RUST_AI variants of the public AStar differential test.
// Node fields are public; no private production state or test hooks are used.
#include "ai/AStar.h"
#include "ai/AINodeContainer.h"
#include "system/LowLevelSystem.h"
#include <algorithm>
#include <cstdarg>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <new>
#include <vector>

FILE* gLogOut = stdout;
std::vector<hpl::cAStarNode*> gSearchNodes;
bool gFailNextAllocation = false;
std::size_t gFailAllocationSize = 0;
hpl::cAStarNode* gOrphan = NULL;

void* operator new(std::size_t size)
{
    if(gFailNextAllocation && (!gFailAllocationSize || gFailAllocationSize == size)) {
        gFailNextAllocation = false; gFailAllocationSize = 0; throw std::bad_alloc();
    }
    void* data = std::malloc(size ? size : 1);
    if(!data) throw std::bad_alloc();
    return data;
}
void operator delete(void* data) noexcept { std::free(data); }
void operator delete(void* data, std::size_t) noexcept { std::free(data); }

namespace hpl {

cAllocatedPointer::cAllocatedPointer(void* data, const std::string& file, int line, size_t memory)
    : msFile(file), mlLine(line), mlMemory(memory), mpData(data) {}

void* cMemoryManager::AddPointer(const cAllocatedPointer& allocation)
{
    // The standalone source set allocates only cAStarNode through hplNew.
    if(allocation.mlMemory != sizeof(cAStarNode) || allocation.msFile.find("AStar.cpp") == std::string::npos)
        throw 9001;
    cAStarNode* node = static_cast<cAStarNode*>(allocation.mpData);
    gSearchNodes.push_back(node);
    fprintf(gLogOut, "allocate %d\n", node->mpAINode->GetID());
    return node;
}

bool cMemoryManager::RemovePointer(void* data, const char*, int)
{
    cAStarNode* node = static_cast<cAStarNode*>(data);
    std::vector<cAStarNode*>::iterator it = std::find(gSearchNodes.begin(), gSearchNodes.end(), node);
    if(it == gSearchNodes.end()) throw 9002;
    // Duplicate temporary nodes have uninitialized cost/distance: never read them.
    fprintf(gLogOut, "delete %d\n", node->mpAINode->GetID());
    gSearchNodes.erase(it);
    return true;
}

static void Print(const char* prefix, const char* fmt, va_list ap)
{
    fprintf(gLogOut, "%s", prefix);
    vfprintf(gLogOut, fmt, ap);
}
void Error(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("ERROR: ", fmt, ap); va_end(ap); }
void Warning(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("WARNING: ", fmt, ap); va_end(ap); }
void Log(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("LOG: ", fmt, ap); va_end(ap); }

}
