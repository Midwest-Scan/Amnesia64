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
// cAINode, cAINodeIterator and cAINodeContainer as facades over the Rust
// navigation graph (rust/crates/hpl-ai/src/nav.rs, rust/PHASE2.md).
// Used when HPL2_RUST_AI is 1; otherwise AINodeContainer.cpp provides the
// original. The ray callback and FreePath are in AINodeContainer.cpp and
// used in both modes.
//////////////////////////////////////////////////////////////////////////

#include "system/RustBridge.h"

#if HPL2_RUST_AI

#include "ai/AINodeContainer.h"

#include "system/String.h"
#include "system/LowLevelSystem.h"
#include "system/Platform.h"

#include "impl/tinyXML/tinyxml.h"

#include <algorithm>
#include <exception>

namespace hpl {

	// Edges are stored in Rust with this class's layout; GetEdge returns them in place.
	static_assert(sizeof(cAINodeEdge) == sizeof(HplAINodeEdge), "cAINodeEdge layout");
	static_assert(offsetof(cAINodeEdge, mfDistance) == offsetof(HplAINodeEdge, distance), "cAINodeEdge distance");
	static_assert(offsetof(cAINodeEdge, mfSqrDistance) == offsetof(HplAINodeEdge, sqr_distance), "cAINodeEdge squared distance");
	static_assert(offsetof(cAINodeEdge, mpNode) == offsetof(HplAINodeEdge, node), "cAINodeEdge node");

	// The iterator keeps the Rust grid cursor in its int[8] storage.
	static_assert(sizeof(HplAINodeIterState) == 8*sizeof(int), "cAINodeIterator state size");
	static_assert(alignof(HplAINodeIterState) <= alignof(int), "cAINodeIterator state alignment");

	static HplAINodeIterState* RsIterState(int *apState)
	{
		return reinterpret_cast<HplAINodeIterState*>(apState);
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// AI NODE
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	cAINode::cAINode()
	{
		mpRs = NULL;
	}

	//-----------------------------------------------------------------------

	cAINode::~cAINode()
	{
	}

	//-----------------------------------------------------------------------

	void cAINode::AddEdge(cAINode *pNode)
	{
		hpl_ai_node_add_edge(mpRs, pNode->mpRs);
	}

	int cAINode::GetEdgeNum() const
	{
		return hpl_ai_node_edge_num(mpRs);
	}

	cAINodeEdge* cAINode::GetEdge(int alIdx)
	{
		return reinterpret_cast<cAINodeEdge*>(hpl_ai_node_edge(mpRs, alIdx));
	}

	const cVector3f& cAINode::GetPosition()
	{
		return *reinterpret_cast<const cVector3f*>(hpl_ai_node_position(mpRs));
	}

	int cAINode::GetID()
	{
		return hpl_ai_node_id(mpRs);
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// NODE ITERATOR
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	cAINodeIterator::cAINodeIterator(cAINodeContainer *apContainer, const cVector3f &avPos, float afRadius)
	{
		mpContainer = apContainer;
		hpl_ai_nav_iter_begin(mpContainer->mpRs, ToRs(avPos), afRadius, RsIterState(mlState));
	}

	//-----------------------------------------------------------------------

	bool cAINodeIterator::HasNext()
	{
		return hpl_ai_nav_iter_has_next(mpContainer->mpRs, RsIterState(mlState));
	}

	//-----------------------------------------------------------------------

	cAINode *cAINodeIterator::Next()
	{
		return static_cast<cAINode*>(hpl_ai_nav_iter_next(mpContainer->mpRs, RsIterState(mlState)));
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// CONSTRUCTORS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	cAINodeContainer::cAINodeContainer(	const tString& asName, const tString &asNodeName, 
										cWorld *apWorld, const cVector3f &avCollideSize)
	{
		mpWorld = apWorld;
		mvSize = avCollideSize;
		msName = asName;
		msNodeName = asNodeName;

		mpRayCallback = hplNew( cAINodeRayCallback, () );

		mbNodeIsAtCenter = true;

		// Max/min edges (5, 2), max edge distance (3), max height (0.1) and
		// nodes per grid (6) start at the original defaults in Rust.
		mpRs = hpl_ai_nav_new();
	}

	//-----------------------------------------------------------------------

	cAINodeContainer::~cAINodeContainer()
	{
		hplDelete(mpRayCallback);

		for(int i=0; i<GetNodeNum(); ++i)
		{
			cAINode *pNode = GetNode(i);
			hplDelete(pNode);
		}
		hpl_ai_nav_free(mpRs);
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// PUBLIC METHODS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	void cAINodeContainer::ReserveSpace(size_t alReserveSpace)
	{
		hpl_ai_nav_reserve(mpRs, alReserveSpace);
	}

	//-----------------------------------------------------------------------

	void cAINodeContainer::AddNode(const tString &asName, int alID, const cVector3f &avPosition, void *apUserData)
	{
		cAINode *pNode = hplNew( cAINode, () );
		pNode->msName = asName;
		pNode->mpRs = hpl_ai_nav_add_node(mpRs, RsStr(asName), alID, ToRs(avPosition), apUserData, pNode);
	}

	//-----------------------------------------------------------------------

	int cAINodeContainer::GetNodeNum() const
	{
		return hpl_ai_nav_node_num(mpRs);
	}

	cAINode* cAINodeContainer::GetNode(int alIdx)
	{
		return static_cast<cAINode*>(hpl_ai_nav_node(mpRs, alIdx));
	}

	//-----------------------------------------------------------------------

	cAINode* cAINodeContainer::GetNodeFromName(const tString &asName)
	{
		return static_cast<cAINode*>(hpl_ai_nav_node_from_name(mpRs, RsStr(asName)));
	}

	//-----------------------------------------------------------------------

	cAINode* cAINodeContainer::GetNodeFromID(int alID)
	{
		return static_cast<cAINode*>(hpl_ai_nav_node_from_id(mpRs, alID));
	}

	//-----------------------------------------------------------------------

	class cSortEndNodes
	{
	public:
		bool operator()(const cAINodeEdge &aEndA, const cAINodeEdge &aEndB)
		{
			return aEndA.mfDistance < aEndB.mfDistance;
		}
	};

	void cAINodeContainer::Compile()
	{
		// Rust runs the original algorithm; physics and the platform's
		// std::sort (whose tie order is observable) stay native.
		struct cContext
		{
			cAINodeContainer *mpContainer;
			std::exception_ptr mException;
		};
		cContext context = { this, std::exception_ptr() };

		HplAINavCompileHost host = {
			&context,
			[](void *apCtx, HplVector3f aStart, HplVector3f aEnd, bool *apVisible) -> bool {
				cContext *pCtx = static_cast<cContext*>(apCtx);
				try {
					tAIFreePathFlag flag = eAIFreePathFlag_SkipDynamic | eAIFreePathFlag_SkipVolatile;
					*apVisible = pCtx->mpContainer->FreePath(FromRs(aStart), FromRs(aEnd), -1, flag);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx, HplAINodeEdge *apEdges, size_t alCount) -> bool {
				cContext *pCtx = static_cast<cContext*>(apCtx);
				try {
					cAINodeEdge *pEdges = reinterpret_cast<cAINodeEdge*>(apEdges);
					std::sort(pEdges, pEdges + alCount, cSortEndNodes());
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			}
		};
		hpl_ai_nav_compile(mpRs, &host);
		if(context.mException) std::rethrow_exception(context.mException);
	}

	//-----------------------------------------------------------------------

	void cAINodeContainer::BuildNodeGridMap()
	{
		hpl_ai_nav_build_grid(mpRs);
	}

	//-----------------------------------------------------------------------

	cAINodeIterator cAINodeContainer::GetNodeIterator(const cVector3f &avPosition, float afRadius)
	{
		return cAINodeIterator(this,avPosition, afRadius);
	}

	//-----------------------------------------------------------------------

	void cAINodeContainer::SetMaxEdges(int alX){ hpl_ai_nav_set_max_edges(mpRs, alX); }
	void cAINodeContainer::SetMinEdges(int alX){ hpl_ai_nav_set_min_edges(mpRs, alX); }
	void cAINodeContainer::SetMaxEdgeDistance(float afX){ hpl_ai_nav_set_max_edge_distance(mpRs, afX); }
	float cAINodeContainer::GetMaxEdgeDistance() const { return hpl_ai_nav_max_edge_distance(mpRs); }
	void cAINodeContainer::SetMaxHeight(float afX){ hpl_ai_nav_set_max_height(mpRs, afX); }
	float cAINodeContainer::GetMaxHeight() const { return hpl_ai_nav_max_height(mpRs); }

	//-----------------------------------------------------------------------

	void cAINodeContainer::SaveToFile(const tWString &asFile)
	{
		TiXmlDocument* pXmlDoc = hplNew( TiXmlDocument,() );
		
		TiXmlElement *pRootElem = static_cast<TiXmlElement*>(pXmlDoc->InsertEndChild(TiXmlElement("AINodes")));

		for(int i=0; i< GetNodeNum(); ++i)
		{
			cAINode * pNode = GetNode(i);
			TiXmlElement *pNodeElem = static_cast<TiXmlElement*>(pRootElem->InsertEndChild(TiXmlElement("Node")));

			pNodeElem->SetAttribute("Name", pNode->GetName().c_str());
			pNodeElem->SetAttribute("ID", cString::ToString(pNode->GetID()).c_str());

			for(int edge =0; edge < pNode->GetEdgeNum(); ++edge)
			{
				cAINodeEdge *pEdge = pNode->GetEdge(edge);
				TiXmlElement *pEdgeElem = static_cast<TiXmlElement*>(pNodeElem->InsertEndChild(TiXmlElement("Edge")));

				pEdgeElem->SetAttribute("Node", pEdge->mpNode->GetName().c_str());
				tString sDistance = cString::ToString(pEdge->mfDistance);
				pEdgeElem->SetAttribute("Distance", sDistance.c_str());
			}
		}

		FILE *pFile = cPlatform::OpenFile(asFile, _W("w+"));
		if(pFile==NULL || pXmlDoc->SaveFile(pFile)==false)
		{
			Error("Couldn't save XML file %s\n",asFile.c_str());
			hplDelete(pXmlDoc);
			return;
		}
		fclose(pFile);
		hplDelete(pXmlDoc);
	}

	//-----------------------------------------------------------------------

	void cAINodeContainer::LoadFromFile(const tWString &asFile)
	{
		BuildNodeGridMap();

		FILE *pFile = cPlatform::OpenFile(asFile, _W("rb"));
		if(pFile==NULL) return;
		
		TiXmlDocument* pXmlDoc = hplNew( TiXmlDocument, () );
		if(pXmlDoc->LoadFile(pFile)==false)
		{
			Warning("Couldn't open XML file %s\n",cString::To8Char(asFile).c_str());
			fclose(pFile);
			hplDelete(pXmlDoc);
			return;
		}
		fclose(pFile);

		TiXmlElement *pRootElem = pXmlDoc->RootElement();

		TiXmlElement *pNodeElem = pRootElem->FirstChildElement("Node");
		for(; pNodeElem != NULL; pNodeElem = pNodeElem->NextSiblingElement("Node"))
		{
			tString sName = cString::ToString(pNodeElem->Attribute("Name"),"");
			int alID = cString::ToInt(pNodeElem->Attribute("ID"),-1);

			cAINode *pNode = GetNodeFromID(alID);

			// Deviation: the original dereferences NULL for an ID that is not in
			// the container (a stale .nodes file); that node's edges are skipped.
			if(pNode==NULL) continue;
            
			TiXmlElement *pEdgeElem = pNodeElem->FirstChildElement("Edge");
			for(; pEdgeElem != NULL; pEdgeElem = pEdgeElem->NextSiblingElement("Edge"))
			{
				tString sNodeName = cString::ToString(pEdgeElem->Attribute("Node"),"");			
				cAINode *pEdgeNode = GetNodeFromName(sNodeName);

				cAINodeEdge Edge;
				Edge.mpNode = pEdgeNode;
				Edge.mfDistance = cString::ToFloat(pEdgeElem->Attribute("Distance"),0);
				Edge.mfSqrDistance = Edge.mfDistance*Edge.mfDistance;

				// The target is NULL if the file names a node that does not exist.
				hpl_ai_node_push_edge(pNode->mpRs, Edge.mpNode, Edge.mfDistance, Edge.mfSqrDistance);
			}
		}

		hplDelete(pXmlDoc);
	}

	//-----------------------------------------------------------------------
}

#endif // HPL2_RUST_AI
