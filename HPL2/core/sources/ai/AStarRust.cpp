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

//////////////////////////////////////////////////////////////////////////
// cAStarHandler as a facade over the Rust search (rust/crates/hpl-ai/src/
// nav.rs, rust/PHASE2.md). Used when HPL2_RUST_AI is 1; otherwise
// AStar.cpp provides the original.
//////////////////////////////////////////////////////////////////////////

#include "system/RustBridge.h"

#if HPL2_RUST_AI

#include "ai/AStar.h"
#include "ai/AINodeContainer.h"

#include <exception>

namespace hpl {

	//////////////////////////////////////////////////////////////////////////
	// CONSTRUCTORS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	cAStarHandler::cAStarHandler(cAINodeContainer *apContainer)
	{
		mpContainer = apContainer;
		mpCallback = NULL;

		// The iteration limit starts at -1 (unlimited) in Rust.
		mpRs = hpl_ai_astar_new();
	}

	//-----------------------------------------------------------------------

	cAStarHandler::~cAStarHandler()
	{
		hpl_ai_astar_free(mpRs);
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// PUBLIC METHODS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	bool cAStarHandler::GetPath(const cVector3f& avStart, const cVector3f& avGoal,tAINodeList *apNodeList)
	{
		// Rust runs the search; physics, the user callback and the output
		// list stay native.
		struct cContext
		{
			cAStarHandler *mpHandler;
			tAINodeList *mpNodeList;
			std::exception_ptr mException;
		};
		cContext context = { this, apNodeList, std::exception_ptr() };

		HplAStarPathHost host = {
			&context,
			[](void *apCtx, HplVector3f aStart, HplVector3f aEnd, bool *apVisible) -> bool {
				cContext *pCtx = static_cast<cContext*>(apCtx);
				try {
					*apVisible = pCtx->mpHandler->mpContainer->FreePath(FromRs(aStart), FromRs(aEnd), -1, eAIFreePathFlag_SkipDynamic);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx, void *apParent, void *apChild, bool *apAllowed) -> bool {
				cContext *pCtx = static_cast<cContext*>(apCtx);
				try {
					iAStarCallback *pCallback = pCtx->mpHandler->mpCallback;
					*apAllowed = pCallback == NULL ||
								 pCallback->CanAddNode(static_cast<cAINode*>(apParent), static_cast<cAINode*>(apChild));
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx, void *apNode) -> bool {
				cContext *pCtx = static_cast<cContext*>(apCtx);
				try {
					pCtx->mpNodeList->push_back(static_cast<cAINode*>(apNode));
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			}
		};

		bool bResult = hpl_ai_astar_get_path(mpRs, mpContainer->mpRs, ToRs(avStart), ToRs(avGoal), apNodeList != NULL, &host);
		if(context.mException) std::rethrow_exception(context.mException);
		return bResult;
	}

	//-----------------------------------------------------------------------

	void cAStarHandler::SetMaxIterations(int alX)
	{
		hpl_ai_astar_set_max_iterations(mpRs, alX);
	}

	//-----------------------------------------------------------------------
}

#endif // HPL2_RUST_AI
