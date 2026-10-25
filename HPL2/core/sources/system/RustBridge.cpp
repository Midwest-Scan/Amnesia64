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

#include "system/RustBridge.h"

#include "system/LowLevelSystem.h"

namespace hpl {

	//-----------------------------------------------------------------------

	static void RustLogCallback(int32_t alLevel, const char* asMessage)
	{
		switch(alLevel)
		{
		case HPL_LOG_LEVEL_ERROR:	Error("%s\n", asMessage);	break;
		case HPL_LOG_LEVEL_WARNING:	Warning("%s\n", asMessage);	break;
		default:					Log("%s\n", asMessage);		break;
		}
	}

	//-----------------------------------------------------------------------

	void RustBridge_Init()
	{
		hpl_rs_init(&RustLogCallback);

		if(hpl_rs_wchar_size() != sizeof(wchar_t))
		{
			Error("Rust bridge: wchar_t is %d bytes in C++ but %d bytes in hpl2_rs.lib!\n",
					(int)sizeof(wchar_t), (int)hpl_rs_wchar_size());
		}
	}

	//-----------------------------------------------------------------------

}
