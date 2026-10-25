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

#include "system/LogicTimer.h"

#include "system/LowLevelSystem.h"
#include "system/Platform.h"
#include "system/RustBridge.h"

namespace hpl {

#if HPL2_RUST_SYSTEM
	// Clock reads stay lazy: a check past the update cap must not read time.
	static double RsLogicTimerTime(void *)
	{
		return (double)cPlatform::GetApplicationTime();
	}

	static const HplLogicTimerHost gRsLogicTimerHost = { NULL, &RsLogicTimerTime };

	HplLogicTimerState cLogicTimer::ToRustState() const
	{
		HplLogicTimerState state = { mlLocalTime, mlLocalTimeAdd, mfSpeedMul, mlMaxUpdates, mlUpdateCount };
		return state;
	}

	void cLogicTimer::FromRustState(const HplLogicTimerState& aState)
	{
		mlLocalTime = aState.local_time;
		mlLocalTimeAdd = aState.local_time_add;
		mfSpeedMul = aState.speed_mul;
		mlMaxUpdates = aState.max_updates;
		mlUpdateCount = aState.update_count;
	}
#endif

	//////////////////////////////////////////////////////////////////////////
	// CONSTRUCTORS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	cLogicTimer::cLogicTimer(int alUpdatesPerSec, iLowLevelSystem *apLowLevelSystem)
	{
#if HPL2_RUST_SYSTEM
		mpLowLevelSystem = apLowLevelSystem;
		HplLogicTimerState state;
		hpl_logic_timer_init(alUpdatesPerSec, gRsLogicTimerHost, &state);
		FromRustState(state);
#else
		mlMaxUpdates = alUpdatesPerSec/10;
		mlUpdateCount =0;
		
		mpLowLevelSystem = apLowLevelSystem;

		mfSpeedMul = 1.0f;

		SetUpdatesPerSec(alUpdatesPerSec);
#endif
	}

	//-----------------------------------------------------------------------

	cLogicTimer::~cLogicTimer()
	{
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// PUBLIC METHODS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------
	void cLogicTimer::Reset()
	{
#if HPL2_RUST_SYSTEM
		HplLogicTimerState state = ToRustState();
		hpl_logic_timer_reset(&state, gRsLogicTimerHost);
		FromRustState(state);
#else
		mlLocalTime = (double)cPlatform::GetApplicationTime();
#endif
	}

	//-----------------------------------------------------------------------

	bool cLogicTimer::WantUpdate()
	{
#if HPL2_RUST_SYSTEM
		HplLogicTimerState state = ToRustState();
		bool bWantUpdate = hpl_logic_timer_want_update(&state, gRsLogicTimerHost);
		FromRustState(state);
		return bWantUpdate;
#else
		++mlUpdateCount;
		if(mlUpdateCount > mlMaxUpdates) return false;

		if(mlLocalTime< (double)cPlatform::GetApplicationTime())
		{
			Update();
			return true;
		}
		return false;
#endif
	}
	
	//-----------------------------------------------------------------------

	void cLogicTimer::EndUpdateLoop()
	{
#if HPL2_RUST_SYSTEM
		HplLogicTimerState state = ToRustState();
		hpl_logic_timer_end_update_loop(&state, gRsLogicTimerHost);
		FromRustState(state);
#else
		if(mlUpdateCount > mlMaxUpdates){
			Reset();
		}

		mlUpdateCount=0;
#endif
	}

	//-----------------------------------------------------------------------

	void cLogicTimer::SetUpdatesPerSec(int alUpdatesPerSec)
	{
#if HPL2_RUST_SYSTEM
		HplLogicTimerState state = ToRustState();
		hpl_logic_timer_set_updates_per_sec(&state, alUpdatesPerSec, gRsLogicTimerHost);
		FromRustState(state);
#else
		mlLocalTimeAdd = 1000.0/((double)alUpdatesPerSec);
		Reset();
#endif
	}

	//-----------------------------------------------------------------------

	void cLogicTimer::SetMaxUpdates(int alMax)
	{
#if HPL2_RUST_SYSTEM
		HplLogicTimerState state = ToRustState();
		hpl_logic_timer_set_max_updates(&state, alMax);
		FromRustState(state);
#else
		mlMaxUpdates = alMax;
#endif
	}

	//-----------------------------------------------------------------------

	int cLogicTimer::GetUpdatesPerSec()
	{
#if HPL2_RUST_SYSTEM
		HplLogicTimerState state = ToRustState();
		return hpl_logic_timer_get_updates_per_sec(&state);
#else
		return (int)(1000.0 / ((double)mlLocalTimeAdd));
#endif
	}
	
	//-----------------------------------------------------------------------

	float cLogicTimer::GetStepSize()
	{
#if HPL2_RUST_SYSTEM
		HplLogicTimerState state = ToRustState();
		return hpl_logic_timer_get_step_size(&state);
#else
		return ((float)mlLocalTimeAdd)/1000.0f;
#endif
	}

	//-----------------------------------------------------------------------
	
	//////////////////////////////////////////////////////////////////////////
	// PRIVATE METHODS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	void cLogicTimer::Update()
	{
#if HPL2_RUST_SYSTEM
		HplLogicTimerState state = ToRustState();
		hpl_logic_timer_update(&state);
		FromRustState(state);
#else
		mlLocalTime += mlLocalTimeAdd/mfSpeedMul;
#endif
	}

	//-----------------------------------------------------------------------

}
