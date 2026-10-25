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

#include "ai/AStar.h"

#include "ai/AINodeContainer.h"

#include "math/Math.h"

#include "system/LowLevelSystem.h"
#include "system/RustBridge.h"

#if HPL2_RUST_AI
#include <exception>
#include <new>
#endif


namespace hpl {

	//////////////////////////////////////////////////////////////////////////
	// NODE
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	cAStarNode::cAStarNode(cAINode *apAINode)
	{
		mpParent = NULL;
		mpAINode = apAINode;
	}

	//-----------------------------------------------------------------------

	bool cAStarNodeCompare::operator()(cAStarNode* apNodeA,cAStarNode* apNodeB) const
	{
		return apNodeA->mpAINode < apNodeB->mpAINode;
	}


	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// CONSTRUCTORS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	cAStarHandler::cAStarHandler(cAINodeContainer *apContainer)
	{
		mlMaxIterations = -1;

		mpContainer = apContainer;

		mpCallback = NULL;
	}

	//-----------------------------------------------------------------------

	cAStarHandler::~cAStarHandler()
	{
		STLDeleteAll(m_setClosedList);
		STLDeleteAll(m_setOpenList);
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// PUBLIC METHODS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	bool cAStarHandler::GetPath(const cVector3f& avStart, const cVector3f& avGoal,tAINodeList *apNodeList)
	{
#if HPL2_RUST_AI
		// Lazy stack storage preserves native iterator lifetime without allocating
		// or requiring a newer C++ language standard than the existing projects.
		struct cPathIterator
		{
			alignas(cAINodeIterator) unsigned char mvStorage[sizeof(cAINodeIterator)];
			cAINodeIterator *mpIterator;
			cPathIterator() : mpIterator(NULL) {}
			~cPathIterator() { if(mpIterator) mpIterator->~cAINodeIterator(); }
			void Create(cAINodeContainer *apContainer, const cVector3f &avPosition, float afRadius)
			{
				mpIterator = new(static_cast<void*>(mvStorage)) cAINodeIterator(apContainer->GetNodeIterator(avPosition,afRadius));
			}
		};
		struct cPathContext
		{
			cAStarHandler *mpHandler;
			const cVector3f *mpStart;
			const cVector3f *mpGoal;
			tAINodeList *mpOutput;
			cPathIterator mStartIterator;
			cPathIterator mGoalIterator;
			std::exception_ptr mException;
		};
		cPathContext context;
		context.mpHandler = this;
		context.mpStart = &avStart;
		context.mpGoal = &avGoal;
		context.mpOutput = apNodeList;
		// Do not read the handler's uninitialized goal pointer on a first call.
		HplAStarPathHost host = {
			&context,
			[](void *apCtx) -> float {
				return static_cast<cPathContext*>(apCtx)->mpHandler->mpContainer->GetMaxHeight();
			},
			[](void *apCtx, int32_t alEndpoint) -> HplVector3f {
				cPathContext *pCtx = static_cast<cPathContext*>(apCtx);
				return ToRs(alEndpoint == 0 ? *pCtx->mpStart : *pCtx->mpGoal);
			},
			[](void *apCtx, bool *apVisible) -> bool {
				cPathContext *pCtx = static_cast<cPathContext*>(apCtx);
				try
				{
					*apVisible = pCtx->mpHandler->mpContainer->FreePath(*pCtx->mpStart,*pCtx->mpGoal,-1,eAIFreePathFlag_SkipDynamic);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx) {
				static_cast<cPathContext*>(apCtx)->mpHandler->mpGoalNode = NULL;
			},
			[](void *apCtx) -> bool {
				cPathContext *pCtx = static_cast<cPathContext*>(apCtx);
				try
				{
					cAStarHandler *pHandler = pCtx->mpHandler;
					STLDeleteAll(pHandler->m_setClosedList);
					STLDeleteAll(pHandler->m_setOpenList);
					pHandler->m_setGoalNodes.clear();
					pHandler->mpGoalNode = NULL;
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx, HplVector3f avPosition) {
				static_cast<cPathContext*>(apCtx)->mpHandler->mvGoal = FromRs(avPosition);
			},
			[](void *apCtx) -> float {
				return static_cast<cPathContext*>(apCtx)->mpHandler->mpContainer->GetMaxEdgeDistance();
			},
			[](void *apCtx, int32_t alEndpoint, float afRadius) -> bool {
				cPathContext *pCtx = static_cast<cPathContext*>(apCtx);
				try
				{
					cPathIterator &iterator = alEndpoint == 0 ? pCtx->mStartIterator : pCtx->mGoalIterator;
					iterator.Create(pCtx->mpHandler->mpContainer,alEndpoint == 0 ? *pCtx->mpStart : *pCtx->mpGoal,afRadius);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx, int32_t alEndpoint) -> void* {
				cPathContext *pCtx = static_cast<cPathContext*>(apCtx);
				cAINodeIterator *pIterator = alEndpoint == 0 ? pCtx->mStartIterator.mpIterator : pCtx->mGoalIterator.mpIterator;
				return pIterator->HasNext() ? pIterator->Next() : NULL;
			},
			[](void *, void *apNode) -> HplVector3f {
				return ToRs(static_cast<cAINode*>(apNode)->GetPosition());
			},
			[](void *apCtx, int32_t alEndpoint, void *apNode, bool *apVisible) -> bool {
				cPathContext *pCtx = static_cast<cPathContext*>(apCtx);
				try
				{
					*apVisible = pCtx->mpHandler->mpContainer->FreePath(alEndpoint == 0 ? *pCtx->mpStart : *pCtx->mpGoal,
						static_cast<cAINode*>(apNode)->GetPosition(),-1,eAIFreePathFlag_SkipDynamic);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx, void *apNode, float afDistance) -> bool {
				cPathContext *pCtx = static_cast<cPathContext*>(apCtx);
				try
				{
					pCtx->mpHandler->AddOpenNode(static_cast<cAINode*>(apNode),NULL,afDistance);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx, void *apNode) -> bool {
				cPathContext *pCtx = static_cast<cPathContext*>(apCtx);
				try
				{
					pCtx->mpHandler->m_setGoalNodes.insert(static_cast<cAINode*>(apNode));
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx) -> bool {
				cPathContext *pCtx = static_cast<cPathContext*>(apCtx);
				try { pCtx->mpHandler->IterateAlgorithm(); return true; }
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx) -> void* {
				return static_cast<cPathContext*>(apCtx)->mpHandler->mpGoalNode;
			},
			[](void *apCtx) -> bool {
				return static_cast<cPathContext*>(apCtx)->mpOutput != NULL;
			},
			[](void *apCtx, void *apSearch) -> bool {
				cPathContext *pCtx = static_cast<cPathContext*>(apCtx);
				try
				{
					// Preserve the field reference through allocation inside push_back.
					pCtx->mpOutput->push_back(static_cast<cAStarNode*>(apSearch)->mpAINode);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *, void *apSearch) -> void* {
				return static_cast<cAStarNode*>(apSearch)->mpParent;
			}
		};
		bool bResult = hpl_astar_get_path(&host);
		if(context.mException) std::rethrow_exception(context.mException);
		return bResult;
#else
		float fMaxHeight = mpContainer->GetMaxHeight()*1.5f;

		/////////////////////////////////////////////////
		// check if there is free path from start to goal
		float fHeight = fabs(avStart.y - avGoal.y);
		if(fHeight <= fMaxHeight && mpContainer->FreePath(avStart,avGoal,-1,eAIFreePathFlag_SkipDynamic))
		{
			mpGoalNode = NULL;
			return true;
		}

		////////////////////////////////////////////////
		//Reset all variables
		//These should just be cleared and pools used instead.
		STLDeleteAll(m_setClosedList);
		STLDeleteAll(m_setOpenList);
		m_setGoalNodes.clear();
		mpGoalNode=NULL;

		//Set goal position
		mvGoal = avGoal;
		
		
		////////////////////////////////////////////////
		//Find nodes reachable from the start and goal position (use double 2*2 distance)
		float fMaxDist = mpContainer->GetMaxEdgeDistance()*2; //float fMaxDist = mpContainer->GetMaxEdgeDistance()*mpContainer->GetMaxEdgeDistance()*4;
		
		/////////////////////
		//Check with Start
		//Log(" Get Start\n");
		cAINodeIterator startNodeIt =  mpContainer->GetNodeIterator(avStart,fMaxDist);
		while(startNodeIt.HasNext())
		{
			cAINode *pAINode = startNodeIt.Next();
			//Log("Check node: %s\n",pAINode->GetName().c_str());

			float fHeight = fabs(avStart.y - pAINode->GetPosition().y);
			float fDist = cMath::Vector3Dist(avStart,pAINode->GetPosition()); //float fDist = cMath::Vector3DistSqr(avStart,pAINode->GetPosition());
			if(fDist < fMaxDist && fHeight <= fMaxHeight)
			{
				//Check if path is clear
				if(mpContainer->FreePath(avStart,pAINode->GetPosition(),-1,	eAIFreePathFlag_SkipDynamic))
				{
					AddOpenNode(pAINode,NULL,fDist);
				}
			}
		}
		//Log(" Found start\n");

		////////////////////////////////
		//Check with Goal
		//Log(" Get Goal\n");
		cAINodeIterator goalNodeIt =  mpContainer->GetNodeIterator(avGoal,fMaxDist);
		while(goalNodeIt.HasNext())
		{
			cAINode *pAINode = goalNodeIt.Next();
			//Log("Check node: %s\n",pAINode->GetName().c_str());
			
			float fHeight = fabs(avGoal.y - pAINode->GetPosition().y);
			float fDist = cMath::Vector3Dist(avGoal,pAINode->GetPosition()); //fDist = cMath::Vector3DistSqr(avGoal,pAINode->GetPosition());
			if(fDist < fMaxDist && fHeight <= fMaxHeight)
			{
				//Check if path is clear
				if(mpContainer->FreePath(avGoal,pAINode->GetPosition(),-1, eAIFreePathFlag_SkipDynamic))
				{
					m_setGoalNodes.insert(pAINode);
				}
			}
		}
		//Log(" Found goal\n");
		
		/*for(int i=0; i<mpContainer->GetNodeNum(); ++i)
		{
			cAINode *pAINode = mpContainer->GetNode(i);

			////////////////////////////////
			//Check with Start
			float fHeight = fabs(avStart.y - pAINode->GetPosition().y);
			float fDist = cMath::Vector3Dist(avStart,pAINode->GetPosition());
			//float fDist = cMath::Vector3DistSqr(avStart,pAINode->GetPosition());
			if(fDist < fMaxDist && fHeight <= fMaxHeight)
			{
				//Check if path is clear
				if(mpContainer->FreePath(avStart,pAINode->GetPosition(),-1))
				{
					AddOpenNode(pAINode,NULL,fDist);
				}
			}

			////////////////////////////////
			//Check with Goal
			fHeight = fabs(avGoal.y - pAINode->GetPosition().y);
			fDist = cMath::Vector3Dist(avGoal,pAINode->GetPosition());
			//fDist = cMath::Vector3DistSqr(avGoal,pAINode->GetPosition());
			if(fDist < fMaxDist && fHeight <= fMaxHeight)
			{
				//Check if path is clear
				if(mpContainer->FreePath(avGoal,pAINode->GetPosition(),3))
				{
					m_setGoalNodes.insert(pAINode);
				}
			}
		}*/

		////////////////////////////////////////////////
		//Iterate the algorithm
		IterateAlgorithm();

		////////////////////////////////////////////////
		//Check if goal was found, if so build path.
		if(mpGoalNode)
		{
			if(apNodeList)
			{
				cAStarNode *pParentNode = mpGoalNode;
				while(pParentNode != NULL)
				{
					apNodeList->push_back(pParentNode->mpAINode);
					pParentNode = pParentNode->mpParent;
				}
			}

			return true;
		}
		else
		{
			 return false;
		}
#endif
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// PRIVATE METHODS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	void cAStarHandler::IterateAlgorithm()
	{
#if HPL2_RUST_AI
		struct cSearchContext
		{
			cAStarHandler *mpHandler;
			std::exception_ptr mException;
		};
		cSearchContext context = { this, std::exception_ptr() };
		// Keep raw handles and live reads: callbacks can edit edges, limits,
		// callbacks and search state. Each nested call owns its exception slot.
		HplAStarSearchHost host = {
			&context,
			[](void *apCtx) -> bool {
				return static_cast<cSearchContext*>(apCtx)->mpHandler->m_setOpenList.empty();
			},
			[](void *apCtx) -> int32_t {
				return static_cast<cSearchContext*>(apCtx)->mpHandler->mlMaxIterations;
			},
			[](void *apCtx) -> void* {
				cSearchContext *pCtx = static_cast<cSearchContext*>(apCtx);
				try { return pCtx->mpHandler->GetBestNode(); }
				catch(...) { pCtx->mException = std::current_exception(); return NULL; }
			},
			[](void *, void *apSearch) -> void* {
				return static_cast<cAStarNode*>(apSearch)->mpAINode;
			},
			[](void *apCtx, void *apNode) -> bool {
				return static_cast<cSearchContext*>(apCtx)->mpHandler->IsGoalNode(static_cast<cAINode*>(apNode));
			},
			[](void *apCtx, void *apSearch) {
				static_cast<cSearchContext*>(apCtx)->mpHandler->mpGoalNode = static_cast<cAStarNode*>(apSearch);
			},
			[](void *, void *apNode) -> int32_t {
				return static_cast<cAINode*>(apNode)->GetEdgeNum();
			},
			[](void *, void *apNode, int32_t alIndex) -> void* {
				return static_cast<cAINode*>(apNode)->GetEdge(alIndex);
			},
			[](void *, void *apEdge) -> void* {
				return static_cast<cAINodeEdge*>(apEdge)->mpNode;
			},
			[](void *apCtx, void *apParent, void *apChild, bool *apAllowed) -> bool {
				cSearchContext *pCtx = static_cast<cSearchContext*>(apCtx);
				try
				{
					cAStarHandler *pHandler = pCtx->mpHandler;
					bool bAllowed = pHandler->mpCallback == NULL ||
						pHandler->mpCallback->CanAddNode(static_cast<cAINode*>(apParent), static_cast<cAINode*>(apChild));
					if(apAllowed) *apAllowed = bAllowed;
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *, void *apEdge) -> float {
				return static_cast<cAINodeEdge*>(apEdge)->mfDistance;
			},
			[](void *, void *apSearch) -> float {
				return static_cast<cAStarNode*>(apSearch)->mfDistance;
			},
			[](void *apCtx, void *apNode, void *apParent, float afDistance) -> bool {
				cSearchContext *pCtx = static_cast<cSearchContext*>(apCtx);
				try
				{
					pCtx->mpHandler->AddOpenNode(static_cast<cAINode*>(apNode), static_cast<cAStarNode*>(apParent), afDistance);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			}
		};
		hpl_astar_iterate_algorithm(&host);
		if(context.mException) std::rethrow_exception(context.mException);
#else
		int lIterationCount=0;
		while(m_setOpenList.empty()==false && (mlMaxIterations <0 || lIterationCount < mlMaxIterations))
		{
			cAStarNode *pNode = GetBestNode();
			cAINode *pAINode = pNode->mpAINode;

			//////////////////////
			// Check if current node can reach goal
			if(IsGoalNode(pAINode))
			{
				mpGoalNode = pNode;
				break;
			}

			/////////////////////
			//Add nodes connected to current
			int lEdgeCount = pAINode->GetEdgeNum();
			for(int i=0; i< lEdgeCount; ++i)
			{
				cAINodeEdge *pEdge = pAINode->GetEdge(i);

				if(mpCallback == NULL || mpCallback->CanAddNode(pAINode, pEdge->mpNode))
				{
					AddOpenNode(pEdge->mpNode, pNode, pNode->mfDistance + pEdge->mfDistance);
					//AddOpenNode(pEdge->mpNode, pNode, pNode->mfDistance + pEdge->mfSqrDistance);
				}
			}

			++lIterationCount;
		}
#endif
	}


	//-----------------------------------------------------------------------

	void cAStarHandler::AddOpenNode(cAINode *apAINode, cAStarNode *apParent, float afDistance)
	{
		//TODO: free path check with dynamic objects here.

		//TODO: Some pooling here would be good.
		cAStarNode *pNode = hplNew( cAStarNode, (apAINode) );

		//Check if it is in closed list.
		tAStarNodeSetIt it = m_setClosedList.find(pNode);
		if(it != m_setClosedList.end()){
			hplDelete(pNode);
			return;
		}

		//Try to add it to the open list
		std::pair<tAStarNodeSetIt, bool> testPair = m_setOpenList.insert(pNode);
		if(testPair.second == false){
			hplDelete(pNode);
			return;
		}

		pNode->mfDistance = afDistance;
		pNode->mfCost = Cost(afDistance,apAINode,apParent) + Heuristic(pNode->mpAINode->GetPosition(), mvGoal);
		pNode->mpParent = apParent;
	}

	//-----------------------------------------------------------------------

	cAStarNode* cAStarHandler::GetBestNode()
	{
#if HPL2_RUST_AI
		struct cOpenContext
		{
			tAStarNodeSetIt mNext;
			tAStarNodeSetIt mEnd;
		};
		cOpenContext context = { m_setOpenList.begin(), m_setOpenList.end() };
		HplAStarOpenHost host = {
			&context,
			[](void *apCtx, float *apCost) -> void* {
				cOpenContext *pCtx = static_cast<cOpenContext*>(apCtx);
				if(pCtx->mNext == pCtx->mEnd) return NULL;
				cAStarNode *pNode = *pCtx->mNext++;
				if(apCost) *apCost = pNode->mfCost;
				return pNode;
			}
		};
		cAStarNode *pBestNode = static_cast<cAStarNode*>(hpl_astar_get_best_node(&host));
		// Faithful: native pointer ordering, and erase before the potentially
		// throwing closed-set insertion. Allocation exceptions never cross Rust frames.
		m_setOpenList.erase(m_setOpenList.find(pBestNode));
		m_setClosedList.insert(pBestNode);
		return pBestNode;
#else
		tAStarNodeSetIt it = m_setOpenList.begin();
		tAStarNodeSetIt bestIt = it;
		cAStarNode* pBestNode = *it; ++it;

		//Iterate open list and find the best node.
		for(; it != m_setOpenList.end(); ++it)
		{
			cAStarNode *pNode = *it;
			if(pBestNode->mfCost > pNode->mfCost)
			{
				pBestNode = pNode;
				bestIt = it;
			}
		}

		//Remove node from open
		m_setOpenList.erase(bestIt);

		//Add to closed list
		m_setClosedList.insert(pBestNode);

		return pBestNode;
#endif
	}

	//-----------------------------------------------------------------------
	
	float cAStarHandler::Cost(float afDistance, cAINode *apAINode, cAStarNode *apParent)
	{
#if HPL2_RUST_AI
		// Faithful: a root cost does not read either node's position.
		if(apParent)
			return hpl_astar_cost(afDistance, apAINode->GetPosition().y, true, apParent->mpAINode->GetPosition().y);
		return hpl_astar_cost(afDistance, 0.0f, false, 0.0f);
#else
		if(apParent)
		{
			float fHeight = (1+fabs(apAINode->GetPosition().y - apParent->mpAINode->GetPosition().y));
			return afDistance * fHeight;
		}
		else
			return afDistance;
#endif
	}

	//-----------------------------------------------------------------------

	float cAStarHandler::Heuristic(const cVector3f& avStart, const cVector3f& avGoal)
	{
#if HPL2_RUST_AI
		return hpl_astar_heuristic(ToRs(avStart), ToRs(avGoal));
#else
		//return cMath::Vector3DistSqr(avStart, avGoal);
		return cMath::Vector3Dist(avStart, avGoal);
#endif
	}

	//-----------------------------------------------------------------------

	bool cAStarHandler::IsGoalNode(cAINode *apAINode)
	{
		tAINodeSetIt it = m_setGoalNodes.find(apAINode);
		if(it == m_setGoalNodes.end()) return false;

		return true;
	}

	//-----------------------------------------------------------------------
}
