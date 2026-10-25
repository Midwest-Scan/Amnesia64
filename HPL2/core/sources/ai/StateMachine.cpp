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

#include "ai/StateMachine.h"

#include "system/LowLevelSystem.h"
#include "system/RustBridge.h"

#if HPL2_RUST_AI
#include <exception>
#endif


namespace hpl {

	//////////////////////////////////////////////////////////////////////////
	// STATE
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------
	
	iAIState::iAIState()
	{
		mfTimeCount = 0;
	}

	//-----------------------------------------------------------------------

	void iAIState::Sleep(float afTime)
	{

	}

	//-----------------------------------------------------------------------

	void iAIState::Update(float afTime)
	{
#if HPL2_RUST_AI
		struct cStateScheduleContext
		{
			iAIState *mpState;
			std::exception_ptr mException;
		};
		cStateScheduleContext context = { this, std::exception_ptr() };
		HplAIStateScheduleHost host = {
			&context,
			[](void *apCtx) -> float {
				return static_cast<cStateScheduleContext*>(apCtx)->mpState->mfTimeCount;
			},
			[](void *apCtx, float afCount) {
				static_cast<cStateScheduleContext*>(apCtx)->mpState->mfTimeCount = afCount;
			},
			[](void *apCtx) -> float {
				return static_cast<cStateScheduleContext*>(apCtx)->mpState->mfUpdateStep;
			},
			[](void *apCtx, float afStep) -> bool {
				cStateScheduleContext *pCtx = static_cast<cStateScheduleContext*>(apCtx);
				try { pCtx->mpState->OnUpdate(afStep); return true; }
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			}
		};
		hpl_ai_state_schedule(afTime, &host);
		if(context.mException) std::rethrow_exception(context.mException);
#else
		//Update according to the time step
		mfTimeCount	+= afTime;
		while(mfTimeCount >= mfUpdateStep)
		{
			OnUpdate(mfUpdateStep);
			mfTimeCount -= mfUpdateStep;
		}
#endif
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// CONSTRUCTORS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	cStateMachine::cStateMachine()
	{
		mbActive = true;

		mpCurrentState = NULL;
	}

	//-----------------------------------------------------------------------

	cStateMachine::~cStateMachine()
	{
		STLMapDeleteAll(m_mapStates);
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// PUBLIC METHODS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------
	
	void cStateMachine::Update(float afTime)
	{
#if HPL2_RUST_AI
		struct cStateUpdateContext
		{
			cStateMachine *mpMachine;
			std::exception_ptr mException;
		};
		cStateUpdateContext context = { this, std::exception_ptr() };
		HplAIStateUpdateHost host = {
			&context,
			[](void *apCtx) -> bool {
				return RsBool(static_cast<cStateUpdateContext*>(apCtx)->mpMachine->mbActive);
			},
			[](void *apCtx) -> void* {
				return static_cast<cStateUpdateContext*>(apCtx)->mpMachine->mpCurrentState;
			},
			[](void *apCtx, void *apState, float afTimeStep) -> bool {
				cStateUpdateContext *pCtx = static_cast<cStateUpdateContext*>(apCtx);
				try
				{
					// Keep the selected state through its entire backlog, including reentry.
					static_cast<iAIState*>(apState)->Update(afTimeStep);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			}
		};
		hpl_ai_state_update(afTime, &host);
		if(context.mException) std::rethrow_exception(context.mException);
#else
		if(mbActive== false || mpCurrentState==NULL) return;
		
		mpCurrentState->Update(afTime);
#endif
	}

	//-----------------------------------------------------------------------

	void cStateMachine::AddState(iAIState *apState, const tString& asName, int alId, float afUpdateStep)
	{
		apState->SetStateMachine(this);
		apState->mlId = alId;
		apState->msName = asName;
		apState->mfUpdateStep = afUpdateStep;

		if(m_mapStates.empty()) mpCurrentState = apState;

		m_mapStates.insert(tAIStateMap::value_type(alId, apState));
	}

	//-----------------------------------------------------------------------

	void cStateMachine::ChangeState(int alId)
	{
#if HPL2_RUST_AI
		struct cStateTransitionContext
		{
			cStateMachine *mpMachine;
			std::exception_ptr mException;
		};
		cStateTransitionContext context = { this, std::exception_ptr() };
		HplAIStateTransitionHost host = {
			&context,
			[](void *apCtx) -> void* {
				return static_cast<cStateTransitionContext*>(apCtx)->mpMachine->mpCurrentState;
			},
			[](void *, void *apState) -> int32_t {
				return static_cast<iAIState*>(apState)->GetId();
			},
			[](void *apCtx, int32_t alStateId) -> void* {
				return static_cast<cStateTransitionContext*>(apCtx)->mpMachine->GetState(alStateId);
			},
			[](void *apCtx, int32_t alStateId) -> bool {
				cStateTransitionContext *pCtx = static_cast<cStateTransitionContext*>(apCtx);
				try
				{
					Warning("State %d does not exist!\n",alStateId);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx, void *apState, int32_t alNextId) -> bool {
				cStateTransitionContext *pCtx = static_cast<cStateTransitionContext*>(apCtx);
				try
				{
					static_cast<iAIState*>(apState)->OnLeaveState(alNextId);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx, void *apState, int32_t alPreviousId) -> bool {
				cStateTransitionContext *pCtx = static_cast<cStateTransitionContext*>(apCtx);
				try
				{
					static_cast<iAIState*>(apState)->OnEnterState(alPreviousId);
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx, void *apState) {
				static_cast<cStateTransitionContext*>(apCtx)->mpMachine->mpCurrentState = static_cast<iAIState*>(apState);
			}
		};
		hpl_ai_state_change(alId, &host);
		if(context.mException) std::rethrow_exception(context.mException);
#else
		if(alId == mpCurrentState->GetId()) return;
		
		iAIState *pState = GetState(alId);
		if(pState==NULL){
			Warning("State %d does not exist!\n",alId); return;
		}

		if(mpCurrentState) mpCurrentState->OnLeaveState(pState->GetId());
		pState->OnEnterState(mpCurrentState==NULL ? -1 : mpCurrentState->GetId());

		mpCurrentState = pState;
#endif
	}
	
	//-----------------------------------------------------------------------
	
	iAIState* cStateMachine::GetState(int alId)
	{
		tAIStateMapIt it = m_mapStates.find(alId);
		if(it == m_mapStates.end()) return NULL;

		return it->second;
	}
	//-----------------------------------------------------------------------
	
	iAIState* cStateMachine::CurrentState()
	{
		return mpCurrentState;
	}

	//-----------------------------------------------------------------------
}
