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

#include "generate/VoxelMap.h"

#include "system/LowLevelSystem.h"
#include "graphics/LowLevelGraphics.h"
#include "system/RustBridge.h"

#include <cstring>
#if HPL2_RUST_GENERATE
#include <exception>
#endif

namespace hpl {

	//////////////////////////////////////////////////////////////////////////
	// CONSTRUCTORS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	cVoxelMap::cVoxelMap(const cVector3l& avSize)
	{
		mpData = NULL;
		SetSize(avSize);

		mfVoxelSize = 0.1f;
		mvPosition = 0;
	}

	//-----------------------------------------------------------------------

	cVoxelMap::~cVoxelMap()
	{
		hplDeleteArray(mpData);
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// PUBLIC METHODS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	void cVoxelMap::SetVoxel(const cVector3l &avPos, char alVal)
	{
#if HPL2_RUST_GENERATE
		hpl_voxel_map_set_voxel(mpData, ToRs(mvSize), ToRs(avPos), static_cast<unsigned char>(alVal));
#else
		mpData[avPos.z * mvSize.x*mvSize.y + avPos.y * mvSize.x + avPos.x] = alVal;
#endif
	}
	
	//-----------------------------------------------------------------------

	void cVoxelMap::SetSize(const cVector3l& avSize)
	{
#if HPL2_RUST_GENERATE
		// Allocation stays in C++: preserve hplNewArray tracking and exceptions.
		if(mpData) hplDeleteArray(mpData);
		mvSize = avSize;
		size_t lBytes = hpl_voxel_map_set_size(ToRs(mvSize));
		mpData = hplNewArray(unsigned char, lBytes);
		memset(mpData, 0, lBytes);
#else
		if(mpData) hplDeleteArray(mpData);
		
        mvSize = avSize;

		mpData = hplNewArray(unsigned char, mvSize.x*mvSize.y*mvSize.z*sizeof(unsigned char));
		memset(mpData, 0, mvSize.x*mvSize.y*mvSize.z*sizeof(unsigned char));
#endif
	}

	//-----------------------------------------------------------------------

	void cVoxelMap::DebugRender(iLowLevelGraphics *apLowGfx, const cColor &aCol)
	{
#if HPL2_RUST_GENERATE
		struct cRenderContext
		{
			cVoxelMap *mpMap;
			iLowLevelGraphics *mpGraphics;
			const cColor *mpColor;
			std::exception_ptr mException;
		};
		cRenderContext context = { this, apLowGfx, &aCol, std::exception_ptr() };

		// Read live members, never a captured buffer: DrawSphere may resize or
		// edit this map and the caller's color before the next iteration.
		HplVoxelRenderHost host = {
			&context,
			[](void *apCtx) -> HplVector3l {
				return ToRs(static_cast<cRenderContext*>(apCtx)->mpMap->mvSize);
			},
			[](void *apCtx, size_t alIndex) -> uint8_t {
				return static_cast<cRenderContext*>(apCtx)->mpMap->mpData[alIndex];
			},
			[](void *apCtx) -> HplVector3f {
				return ToRs(static_cast<cRenderContext*>(apCtx)->mpMap->mvPosition);
			},
			[](void *apCtx) -> float {
				return static_cast<cRenderContext*>(apCtx)->mpMap->mfVoxelSize;
			},
			[](void *apCtx, HplVector3f avCenter, float afRadius) -> bool {
				cRenderContext *pCtx = static_cast<cRenderContext*>(apCtx);
				try
				{
					pCtx->mpGraphics->DrawSphere(FromRs(avCenter), afRadius, *pCtx->mpColor);
					return true;
				}
				catch(...)
				{
					// Preserve C++ exceptions without unwinding through Rust.
					pCtx->mException = std::current_exception();
					return false;
				}
			}
		};
		hpl_voxel_map_debug_render(&host);
		if(context.mException) std::rethrow_exception(context.mException);
#else
		for(int z=0; z<mvSize.z; ++z)
		for(int y=0; y<mvSize.y; ++y)
		for(int x=0; x<mvSize.x; ++x)
		{
			if(mpData[z * mvSize.x*mvSize.y + y * mvSize.x + x]==1)
			{
				apLowGfx->DrawSphere(mvPosition + cVector3f((float)x, (float)y, (float)z)*mfVoxelSize + cVector3f(mfVoxelSize/2), mfVoxelSize/2, aCol);
			}
		}
#endif
	}

	//-----------------------------------------------------------------------

}
