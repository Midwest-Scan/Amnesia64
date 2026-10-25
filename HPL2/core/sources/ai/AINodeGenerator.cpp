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

#include "ai/AINodeGenerator.h"

#include "scene/World.h"

#include "system/LowLevelSystem.h"
#include "system/System.h"
#include "system/String.h"
#include "system/Platform.h"
#include "system/RustBridge.h"

#if HPL2_RUST_AI
#include <exception>
#endif

#include "resources/Resources.h"
#include "resources/FileSearcher.h"

#include "physics/PhysicsWorld.h"
#include "physics/PhysicsBody.h"

#include "impl/tinyXML/tinyxml.h"


namespace hpl {

	//////////////////////////////////////////////////////////////////////////
	// PARAMS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------
	
	cAINodeGeneratorParams::cAINodeGeneratorParams()
	{
		msNodeType = "node";

		mfHeightFromGround = 0.1f;
		mfMinWallDist = 0.4f;

		mvMinPos = cVector3f(-10000, -10000, -10000);
		mvMaxPos = cVector3f(10000, 10000, 10000);

		mfGridSize = 0.4f;
	}

	//-----------------------------------------------------------------------
	
	//-----------------------------------------------------------------------

	class cCollideRayCallback : public iPhysicsRayCallback
	{
	public:
		bool OnIntersect(iPhysicsBody *pBody,cPhysicsRayParams *apParams)
		{
			if(pBody->GetMass()!=0) return true;

			mbIntersected = true;
			mvPos = apParams->mvPoint;
			mfDist = apParams->mfDist;

			return false;
		}

		bool mbIntersected;
		cVector3f mvPos;
		float mfDist;
	};

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// CONSTRUCTORS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------
	
	cAINodeGenerator::cAINodeGenerator()
	{

	}
	
	cAINodeGenerator::~cAINodeGenerator()
	{

	}
	
	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// PUBLIC METHODS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------
	
	static cCollideRayCallback gCollideRayCallback;

	//-----------------------------------------------------------------------

	void cAINodeGenerator::Generate(cWorld* apWorld,cAINodeGeneratorParams *apParams)
	{
		mpWorld = apWorld;
		mpParams = apParams;

		mlIDCount =0;

		iPhysicsWorld *pPhysicsWorld = apWorld->GetPhysicsWorld();

		bool mbLoadFromFile=false;
		
		cSystem *pSystem = apWorld->GetSystem();
		cResources *pResources = apWorld->GetResources();
		cFileSearcher *pFileSearcher = pResources->GetFileSearcher();

		mpNodeList = apWorld->GetAINodeList(mpParams->msNodeType);

		if(mpWorld->GetFilePath() != _W(""))
		{
			tWString sPath = mpWorld->GetFilePath();
			tWString sSaveFile = cString::SetFileExtW(sPath,_W("ainodes"));

			if(sPath != _W("") && cPlatform::FileExists(sSaveFile))
			{
				cDate mapDate = cPlatform::FileModifiedDate( sPath);
				cDate saveDate = cPlatform::FileModifiedDate(sSaveFile);

				//If the save file is newer than the map load from it.
				if(saveDate > mapDate)
				{
					LoadFromFile();
					return;
				}
			}
		}
        		
		
#if HPL2_RUST_AI
		cPhysicsBodyIterator bodyIt = pPhysicsWorld->GetBodyIterator();
		struct cPlacementContext
		{
			cAINodeGenerator *mpGenerator;
			iPhysicsWorld *mpPhysicsWorld;
			cAINodeGeneratorParams *mpOriginalParams;
			cPhysicsBodyIterator *mpBodyIt;
			tTempAiNodeListIt mNodeIt;
			bool mbNodeStarted;
			std::exception_ptr mException;
		};
		cPlacementContext context;
		context.mpGenerator = this;
		context.mpPhysicsWorld = pPhysicsWorld;
		context.mpOriginalParams = apParams;
		context.mpBodyIt = &bodyIt;
		context.mbNodeStarted = false;
		HplAINodePlacementHost host = {
			&context,
			[](void *apCtx, bool *apPresent, HplVector3f *apMin, HplVector3f *apMax) -> bool {
				cPlacementContext *pCtx = static_cast<cPlacementContext*>(apCtx);
				try
				{
					*apPresent = pCtx->mpBodyIt->HasNext();
					if(!*apPresent) return true;
					iPhysicsBody *pBody = pCtx->mpBodyIt->Next();
					// Faithful: two virtual bounds calls, copying min before the second.
					cVector3f vMin = pBody->GetBoundingVolume()->GetMin();
					cVector3f vMax = pBody->GetBoundingVolume()->GetMax();
					*apMin = ToRs(vMin);
					*apMax = ToRs(vMax);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx) -> float {
				return static_cast<cPlacementContext*>(apCtx)->mpGenerator->mpParams->mfGridSize;
			},
			[](void *apCtx) -> HplVector3f {
				return ToRs(static_cast<cPlacementContext*>(apCtx)->mpGenerator->mpParams->mvMinPos);
			},
			[](void *apCtx) -> HplVector3f {
				return ToRs(static_cast<cPlacementContext*>(apCtx)->mpGenerator->mpParams->mvMaxPos);
			},
			[](void *apCtx, HplVector3f avStart, HplVector3f avEnd) -> bool {
				cPlacementContext *pCtx = static_cast<cPlacementContext*>(apCtx);
				try
				{
					pCtx->mpPhysicsWorld->CastRay(pCtx->mpGenerator,FromRs(avStart),FromRs(avEnd),false,false,true);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx) -> float {
				// Faithful: grid steps use the original argument even after reentry.
				return static_cast<cPlacementContext*>(apCtx)->mpOriginalParams->mfGridSize;
			},
			[](void *apCtx) -> void* {
				cPlacementContext *pCtx = static_cast<cPlacementContext*>(apCtx);
				if(!pCtx->mbNodeStarted)
				{
					pCtx->mNodeIt = pCtx->mpGenerator->mpNodeList->begin();
					pCtx->mbNodeStarted = true;
				}
				else ++pCtx->mNodeIt;
				// Faithful: advance only after all rays, observing newly appended nodes.
				return pCtx->mNodeIt == pCtx->mpGenerator->mpNodeList->end() ? NULL : &*pCtx->mNodeIt;
			},
			[](void *apCtx) -> float {
				return static_cast<cPlacementContext*>(apCtx)->mpGenerator->mpParams->mfMinWallDist;
			},
			[](void *apCtx, void *apNode, HplVector3f avOffset, bool *apHit, float *apDistance) -> bool {
				cPlacementContext *pCtx = static_cast<cPlacementContext*>(apCtx);
				try
				{
					cTempAiNode &Node = *static_cast<cTempAiNode*>(apNode);
					gCollideRayCallback.mbIntersected = false;
					// Keep the native start reference and the shared callback for reentry.
					pCtx->mpPhysicsWorld->CastRay(&gCollideRayCallback,Node.mvPos,Node.mvPos + FromRs(avOffset),true,false,true);
					*apHit = gCollideRayCallback.mbIntersected;
					if(*apHit) *apDistance = gCollideRayCallback.mfDist;
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *, void *apNode) -> HplVector3f {
				return ToRs(static_cast<cTempAiNode*>(apNode)->mvPos);
			},
			[](void *, void *apNode, HplVector3f avPosition) {
				static_cast<cTempAiNode*>(apNode)->mvPos = FromRs(avPosition);
			}
		};
		hpl_ai_node_generator_place(&host);
		if(context.mException) std::rethrow_exception(context.mException);
#else
		/////////////////////////////////
		// Get the size of the world
		cPhysicsBodyIterator it = pPhysicsWorld->GetBodyIterator();
		cVector3f vWorldMax(-100000,-100000,-100000);
		cVector3f vWorldMin( 100000, 100000, 100000);

		while(it.HasNext())
		{
			iPhysicsBody *pBody = it.Next();

			cVector3f vMin = pBody->GetBoundingVolume()->GetMin();
			cVector3f vMax = pBody->GetBoundingVolume()->GetMax();
			
			//X
			if(vWorldMin.x > vMin.x) vWorldMin.x = vMin.x;
			if(vWorldMax.x < vMax.x) vWorldMax.x = vMax.x;

			//Y
			if(vWorldMin.y > vMin.y) vWorldMin.y = vMin.y;
			if(vWorldMax.y < vMax.y) vWorldMax.y = vMax.y;

			//Z
			if(vWorldMin.z > vMin.z) vWorldMin.z = vMin.z;
			if(vWorldMax.z < vMax.z) vWorldMax.z = vMax.z;
		}
		
		//Make the world small according to grid size.
		vWorldMin.x += mpParams->mfGridSize;
		vWorldMin.z += mpParams->mfGridSize;
		vWorldMax.x -= mpParams->mfGridSize;
		vWorldMax.z -= mpParams->mfGridSize;
		
		/////////////////////////////////////////
		//Check against the user set min and max
		if(vWorldMin.x < mpParams->mvMinPos.x) vWorldMin.x = mpParams->mvMinPos.x;
		if(vWorldMax.x > mpParams->mvMaxPos.x) vWorldMax.x = mpParams->mvMaxPos.x;

		if(vWorldMin.y < mpParams->mvMinPos.y) vWorldMin.y = mpParams->mvMinPos.y;
		if(vWorldMax.y > mpParams->mvMaxPos.y) vWorldMax.y = mpParams->mvMaxPos.y;

		if(vWorldMin.z < mpParams->mvMinPos.z) vWorldMin.z = mpParams->mvMinPos.z;
		if(vWorldMax.z > mpParams->mvMaxPos.z) vWorldMax.z = mpParams->mvMaxPos.z;
				

		/////////////////////////////////////////
		//Place the nodes in the world
		cVector3f vPos(vWorldMin.x,0,vWorldMin.z);

		while(vPos.z <= vWorldMax.z)
		{
			cVector3f vStart(vPos.x, vWorldMax.y, vPos.z);
			cVector3f vEnd(vPos.x, vWorldMin.y, vPos.z);
			
			pPhysicsWorld->CastRay(this,vStart,vEnd,false,false,true);

			//Log("Pos: %s Min: %s Max: %s\n",vPos.ToString().c_str(),
			//								vWorldMin.ToString().c_str(),
			//								vWorldMax.ToString().c_str());

			vPos.x += apParams->mfGridSize;
			if(vPos.x > vWorldMax.x)
			{
				vPos.x = vWorldMin.x;
				vPos.z +=  apParams->mfGridSize;
			}
		}

		/////////////////////////////////////////
		//Check so that the nodes are not too close to walls
		cVector3f vEnds[4] = {	cVector3f(mpParams->mfMinWallDist,0,0),
									cVector3f(-mpParams->mfMinWallDist,0,0),
									cVector3f(0,0,mpParams->mfMinWallDist),
									cVector3f(0,0,-mpParams->mfMinWallDist)
							};

		cVector3f vPushBackDirs[4] = {	cVector3f(-1,0,0),
										cVector3f(1,0,0),
										cVector3f(0,0,-1),
										cVector3f(0,0,1)
								};

		tTempAiNodeListIt nodeIt = mpNodeList->begin();
		for(; nodeIt != mpNodeList->end(); ++nodeIt)
		{
			cTempAiNode &Node = *nodeIt;
		
			//Check if there are any walls close by.
			for(int i=0; i<4; ++i)
			{
				gCollideRayCallback.mbIntersected = false;
				pPhysicsWorld->CastRay(&gCollideRayCallback,Node.mvPos,Node.mvPos + vEnds[i],true,false,true);
				
				if(gCollideRayCallback.mbIntersected)
				{
					//Log("Walldistance %f : Add: (%s) Push (%s) Min: %f\n",gCollideRayCallback.mfDist, 
					//											vEnds[i].ToString().c_str(),
					//											vPushBackDirs[i].ToString().c_str(),
					//											mpParams->mfMinWallDist);
					if(gCollideRayCallback.mfDist < mpParams->mfMinWallDist)
					{
						Node.mvPos += vPushBackDirs[i] * (mpParams->mfMinWallDist - gCollideRayCallback.mfDist);
					}
				}
			}
		}

#endif

		///////////////////////////////////////////
		// Save to file

		SaveToFile();
	}
	
	//////////////////////////////////////////////////////////////////////////
	// PRIVATE
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------
	
	bool cAINodeGenerator::OnIntersect(iPhysicsBody *pBody,cPhysicsRayParams *apParams)
	{
#if HPL2_RUST_AI
		struct cIntersectionContext
		{
			cAINodeGenerator *mpGenerator;
			iPhysicsBody *mpBody;
			cPhysicsRayParams *mpRayParams;
			std::exception_ptr mException;
		};
		cIntersectionContext context = {this, pBody, apParams, std::exception_ptr()};
		HplAINodeIntersectionHost host = {
			&context,
			[](void *apCtx, float *apMass) -> bool {
				cIntersectionContext *pCtx = static_cast<cIntersectionContext*>(apCtx);
				try { *apMass = pCtx->mpBody->GetMass(); return true; }
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx) -> HplVector3f {
				return ToRs(static_cast<cIntersectionContext*>(apCtx)->mpRayParams->mvPoint);
			},
			[](void *apCtx) -> float {
				return static_cast<cIntersectionContext*>(apCtx)->mpGenerator->mpParams->mfHeightFromGround;
			},
			[](void *apCtx, HplVector3f avPosition) -> bool {
				cIntersectionContext *pCtx = static_cast<cIntersectionContext*>(apCtx);
				try
				{
					cAINodeGenerator *pGenerator = pCtx->mpGenerator;
					// Faithful: capture ID before allocation, then increment the live ID.
					pGenerator->mpNodeList->push_back(cTempAiNode(FromRs(avPosition),"",pGenerator->mlIDCount));
					pGenerator->mlIDCount++;
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			}
		};
		bool bResult = hpl_ai_node_generator_intersect(&host);
		if(context.mException) std::rethrow_exception(context.mException);
		return bResult;
#else
		if(pBody->GetMass() != 0) return true;

		iPhysicsWorld *pPhysicsWorld = mpWorld->GetPhysicsWorld();

		cVector3f vPosition = apParams->mvPoint + cVector3f(0,mpParams->mfHeightFromGround,0);
		
		mpNodeList->push_back(cTempAiNode(vPosition,"",mlIDCount));
		mlIDCount++;

		return true;
#endif
	}

	//-----------------------------------------------------------------------

	void cAINodeGenerator::SaveToFile()
	{
		if(mpWorld->GetFilePath() == _W("")) return;

		cSystem *pSystem = mpWorld->GetSystem();
		cResources *pResources = mpWorld->GetResources();
		cFileSearcher *pFileSearcher = pResources->GetFileSearcher();

		tWString sMapPath = mpWorld->GetFilePath();
		tWString sSaveFile = cString::SetFileExtW(sMapPath,_W("ainodes"));
		
		TiXmlDocument* pXmlDoc = hplNew( TiXmlDocument, () );

		TiXmlElement *pRootElem = static_cast<TiXmlElement*>(pXmlDoc->InsertEndChild(TiXmlElement("AiNodes")));
		
		tTempAiNodeListIt nodeIt = mpNodeList->begin();
		for(; nodeIt != mpNodeList->end(); ++nodeIt)
		{
			cTempAiNode &Node = *nodeIt;
			TiXmlElement *pNodeElem = static_cast<TiXmlElement*>(pRootElem->InsertEndChild(TiXmlElement("Node")));
			
			tString sPos =	cString::ToString(Node.mvPos.x)+" " + 
							cString::ToString(Node.mvPos.y)+" " +
							cString::ToString(Node.mvPos.z);
			pNodeElem->SetAttribute("Pos",sPos.c_str());
			pNodeElem->SetAttribute("Name", Node.msName.c_str());
			pNodeElem->SetAttribute("Name", cString::ToString(Node.mlID).c_str());
		}
		
		FILE *pFile = cPlatform::OpenFile(sSaveFile,_W("w+"));
		if(pFile==NULL || pXmlDoc->SaveFile(pFile)==false)
		{
			Error("Couldn't save XML file %s\n",sSaveFile.c_str());
			hplDelete(pXmlDoc);
			return;
		}
		fclose(pFile);
		hplDelete(pXmlDoc);
	}

	//-----------------------------------------------------------------------

	void cAINodeGenerator::LoadFromFile()
	{
		if(mpWorld->GetFilePath() == _W("")) return;

		cSystem *pSystem = mpWorld->GetSystem();
		cResources *pResources = mpWorld->GetResources();
		cFileSearcher *pFileSearcher = pResources->GetFileSearcher();

		tWString sMapPath = mpWorld->GetFilePath();
		tWString sSaveFile = cString::SetFileExtW(sMapPath,_W("ainodes"));

		FILE *pFile = cPlatform::OpenFile(sSaveFile, _W("rb"));
		if (!pFile) {
			Warning("Couldn't open XML file %s\n",cString::To8Char(sSaveFile).c_str());
			return;
		}
		TiXmlDocument* pXmlDoc = hplNew( TiXmlDocument, () );
		if(pXmlDoc->LoadFile(pFile)==false)
		{
			Warning("Couldn't open XML file %s\n",cString::To8Char(sSaveFile).c_str());
			fclose(pFile);
			hplDelete(pXmlDoc);
			return;
		}
		fclose(pFile);

		TiXmlElement *pRootElem = pXmlDoc->RootElement();

		TiXmlElement *pNodeElem = pRootElem->FirstChildElement("Node");
		for(; pNodeElem != NULL; pNodeElem = pNodeElem->NextSiblingElement("Node"))
		{
			cVector3f vPos = cString::ToVector3f(pNodeElem->Attribute("Pos"),0);
			tString sName = cString::ToString(pNodeElem->Attribute("Name"),"");
			int alID = cString::ToInt(pNodeElem->Attribute("ID"),-1);

			mpNodeList->push_back(cTempAiNode(vPos,sName,alID));
		}

		hplDelete(pXmlDoc);
	}
	
	//-----------------------------------------------------------------------
}
