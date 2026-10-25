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
// Switches of the Rust port (rust/). With a switch at 1 that module runs
// in Rust; at 0 the original C++ is compiled instead (it stays in place).
//
// Only macros live here, so public engine headers can include this file
// to pick their class layout (phase 2, see rust/PHASE2.md) without
// depending on hpl2_rs.h. HPL2, Lux and the tools must all see the same
// values: change them here rather than per project.
//////////////////////////////////////////////////////////////////////////

#ifndef HPL_RUST_CONFIG_H
#define HPL_RUST_CONFIG_H

// cMath, cQuaternion, cFrustum, cBoundingVolume, cCRC
#ifndef HPL2_RUST_MATH
#define HPL2_RUST_MATH 1
#endif

// cString, cPreprocessParser, cLogicTimer, VideoComp
#ifndef HPL2_RUST_SYSTEM
#define HPL2_RUST_SYSTEM 1
#endif

// cVoxelMap algorithms (allocation and renderer stay in C++)
#ifndef HPL2_RUST_GENERATE
#define HPL2_RUST_GENERATE 1
#endif

// AI. Navigation (cAINodeContainer, cAINode, cAINodeIterator, cAStarHandler)
// is owned by Rust behind C++ facades; the state machine and node generator
// forward their algorithms (C++ keeps those objects).
#ifndef HPL2_RUST_AI
#define HPL2_RUST_AI 1
#endif

#endif // HPL_RUST_CONFIG_H
