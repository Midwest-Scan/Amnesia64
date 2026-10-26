/*
 * Copyright © 2009-2020 Frictional Games
 *
 * This file is part of Amnesia: The Dark Descent.
 *
 * Amnesia: The Dark Descent is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Amnesia: The Dark Descent is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Amnesia: The Dark Descent. If not, see <https://www.gnu.org/licenses/>.
 */

// Windows filesystem smoke: links the real PlatformWin32.cpp directory and
// full-path methods. Files are created only under each variant's output folder.
// The current directory prefix is removed from traces so variants compare.
#include "resources/FileSearcher.h"
#include "system/Platform.h"
#include <cstdio>
#include <stdexcept>
#define NOMINMAX
#include <windows.h>
#if HARNESS_RUST
#include "system/RustBridge.h"
#endif
using namespace hpl;
extern FILE* gLogOut;
static FILE* gOut;
static tWString gRoot;
static void Folder(const wchar_t* path)
{
    if(!CreateDirectoryW(path,NULL) && GetLastError()!=ERROR_ALREADY_EXISTS) throw std::runtime_error("create test folder");
}
static void File(const wchar_t* path,bool hidden=false)
{
    HANDLE f=CreateFileW(path,GENERIC_WRITE,FILE_SHARE_READ,NULL,OPEN_ALWAYS,FILE_ATTRIBUTE_NORMAL,NULL);
    if(f==INVALID_HANDLE_VALUE) throw std::runtime_error("create test file");
    CloseHandle(f); if(hidden && !SetFileAttributesW(path,FILE_ATTRIBUTE_HIDDEN)) throw std::runtime_error("hide test file");
}
static void Query(cFileSearcher& s,const char* name)
{
    int count=-123456789; const tWString& result=s.GetFilePath(name,&count);
    tWString path=result;
    if(path.compare(0,gRoot.size(),gRoot)==0) path=L"ROOT"+path.substr(gRoot.size());
    fprintf(gOut,"query %s count=%d path=",name,count);
    for(wchar_t c:path) fprintf(gOut,"%04x",(unsigned)c);
    fputc('\n',gOut);
}
int main(int argc,char** argv)
{
    gOut=argc>1 ? fopen(argv[1],"wb") : stdout; if(!gOut) return 2; gLogOut=gOut;
#if HARNESS_RUST
    RustBridge_Init();
#endif
    Folder(L"native-files"); Folder(L"native-files/Alpha"); Folder(L"native-files/Beta"); Folder(L"native-files/Alpha/Deep");
    File(L"native-files/root.dat"); File(L"native-files/Alpha/Shared.dat"); File(L"native-files/Beta/Shared.dat");
    File(L"native-files/Alpha/Deep/Leaf.dat"); File(L"native-files/Beta/extra.txt"); File(L"native-files/secret.dat",true);
    gRoot=cPlatform::GetFullFilePath(L"native-files"); for(auto& c:gRoot) if(c==L'\\') c=L'/';
    cFileSearcher s; s.AddDirectory(L"native-files","*.dat",true);
    for(const char* name:{"root.dat","Alpha/Shared.dat","Beta/Shared.dat","Deep/Leaf.dat","SHARED.DAT","secret.dat","extra.txt","absent.dat"}) Query(s,name);
    const tWString& stable=s.GetFilePath("Alpha/Shared.dat"); tWString snapshot=stable;
    s.AddDirectory(L"native-files","*.txt",true);
    fprintf(gOut,"reference-stable=%d value=%d\n",&stable==&s.GetFilePath("Alpha/Shared.dat"),stable==snapshot);
    Query(s,"extra.txt"); cFileSearcher copy(s), assigned; assigned=s; s.ClearDirectories();
    Query(s,"Shared.dat"); Query(copy,"Alpha/Shared.dat"); Query(assigned,"Beta/Shared.dat");
    s.AddDirectory(L"native-files\\Alpha","*.*",false); Query(s,"Shared.dat"); Query(s,"Leaf.dat");
    if(gOut!=stdout) fclose(gOut); return 0;
}
