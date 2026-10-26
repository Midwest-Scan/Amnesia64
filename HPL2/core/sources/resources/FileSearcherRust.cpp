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
// cFileSearcher facade over Rust-owned resource lookup and scan decisions.
// HPL2_RUST_RESOURCES selects this file; FileSearcher.cpp keeps the original.
//////////////////////////////////////////////////////////////////////////

#include "system/RustBridge.h"

#if HPL2_RUST_RESOURCES

#include "resources/FileSearcher.h"
#include "system/Platform.h"
#include "system/String.h"
#include <exception>

namespace hpl {

	cFileSearcher::cFileSearcher()
	{
		mpRs = hpl_resources_file_searcher_new();
	}

	//-----------------------------------------------------------------------

	cFileSearcher::~cFileSearcher()
	{
		hpl_resources_file_searcher_free(mpRs);
	}

	//-----------------------------------------------------------------------

	cFileSearcher::cFileSearcher(const cFileSearcher& aOther)
		: mvPaths(aOther.mvPaths), msNull(aOther.msNull)
	{
		// Clone after all throwing native copies have finished.
		mpRs = hpl_resources_file_searcher_clone(aOther.mpRs);
	}

	//-----------------------------------------------------------------------

	cFileSearcher& cFileSearcher::operator=(const cFileSearcher& aOther)
	{
		if(this == &aOther) return *this;
		std::deque<tWString> vPaths(aOther.mvPaths);
		tWString sNull(aOther.msNull);
		// Keep the Rust object itself alive if assignment occurs during a scan.
		hpl_resources_file_searcher_assign(mpRs, aOther.mpRs);
		mvPaths.swap(vPaths);
		msNull.swap(sNull);
		return *this;
	}

	//-----------------------------------------------------------------------

	void cFileSearcher::AddDirectory(const tWString& asSearchPath, const tString& asMask, bool abAddSubDirectories)
	{
		struct cContext
		{
			cFileSearcher *mpSearcher;
			const tString *mpMask;
			std::exception_ptr mException;
		};
		cContext context = { this, &asMask, std::exception_ptr() };
		HplFileSearchHost host = {
			&context,
			[](void *apCtx, HplWStr aPath, bool abFolders, HplWStrOut aOut) -> bool {
				cContext *pCtx = static_cast<cContext*>(apCtx);
				try {
					tWString sPath(aPath.ptr, aPath.len);
					tWStringList lstNames;
					if(abFolders) cPlatform::FindFoldersInDir(lstNames, sPath, false);
					else cPlatform::FindFilesInDir(lstNames, sPath, cString::To16Char(*pCtx->mpMask));
					for(tWStringListIt it = lstNames.begin(); it != lstNames.end(); ++it)
						aOut.sink(aOut.ctx, it->c_str(), it->size());
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx, HplWStr aPath, HplWStrOut aOut) -> bool {
				cContext *pCtx = static_cast<cContext*>(apCtx);
				try {
					tWString sPath = cPlatform::GetFullFilePath(tWString(aPath.ptr, aPath.len));
					aOut.sink(aOut.ctx, sPath.c_str(), sPath.size());
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx, HplWStr aPath, size_t *apIndex) -> bool {
				cContext *pCtx = static_cast<cContext*>(apCtx);
				try {
					pCtx->mpSearcher->mvPaths.push_back(tWString(aPath.ptr, aPath.len));
					*apIndex = pCtx->mpSearcher->mvPaths.size() - 1;
					return true;
				}
				catch(...) { pCtx->mException = std::current_exception(); return false; }
			},
			[](void *apCtx, size_t alIndex) -> HplWStr {
				cContext *pCtx = static_cast<cContext*>(apCtx);
				return RsStr(pCtx->mpSearcher->mvPaths[alIndex]);
			}
		};
		hpl_resources_file_searcher_add_directory(mpRs, RsStr(asSearchPath), abAddSubDirectories, &host);
		if(context.mException) std::rethrow_exception(context.mException);
	}

	//-----------------------------------------------------------------------

	void cFileSearcher::ClearDirectories()
	{
		hpl_resources_file_searcher_clear(mpRs);
		mvPaths.clear();
	}

	//-----------------------------------------------------------------------

	const tWString& cFileSearcher::GetFilePath(const tString& asFileNameAndPath, int *apEqualCount)
	{
		size_t lIndex = hpl_resources_file_searcher_get_file_path(mpRs, RsStr(asFileNameAndPath), apEqualCount);
		return lIndex == static_cast<size_t>(-1) ? msNull : mvPaths[lIndex];
	}

	//-----------------------------------------------------------------------
}

#endif // HPL2_RUST_RESOURCES
