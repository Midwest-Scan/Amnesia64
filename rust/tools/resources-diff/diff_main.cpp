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

// Public API differential test for the Rust-owned cFileSearcher.
// Platform functions below are controlled native collaborators; actual
// FileSearcher and cString sources are linked in every variant. No private
// fields, allocation counts, raw pointer values or native container internals
// are observed. Stable returned references are checked by identity/value.
// Excluded native UB: directory cycles, empty recursive Windows paths,
// out-of-range platform filename buffers and invalidated references after clear.
#include "resources/FileSearcher.h"
#include "system/Platform.h"
#include "system/String.h"
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <map>
#include <stdexcept>
#include <string>
#include <vector>
#if HARNESS_RUST
#include "system/RustBridge.h"
#endif
using namespace hpl;
extern FILE* gLogOut;
static FILE* gOut;
struct Directory { std::vector<tWString> files, folders; };
static std::map<tWString, Directory> gDirs;
static std::map<tWString, tWString> gFullPaths;
static int gCalls[3], gTotalCalls[3], gQueries, gAdds, gExceptions, gActions;
static int gFailKind=-1, gFailAt=0, gActionKind=-1, gActionAt=0, gAction=0;
static cFileSearcher* gCurrent=NULL;
static cFileSearcher* gCopySource=NULL;
static tString* gLiveMask=NULL;
static bool gBackslash=false;
static uint64_t gRandom=0x57BA3CF1390DE21Full;
static uint32_t Next() { gRandom^=gRandom<<13; gRandom^=gRandom>>7; gRandom^=gRandom<<17; return (uint32_t)(gRandom>>19); }
static int Int(int lo,int hi) { return lo+(int)(Next()%(uint32_t)(hi-lo+1)); }
static tWString Wide(const tString& s) { return cString::To16Char(s); }
static void Bytes(const tString& s) { fprintf(gOut,"b%u:",(unsigned)s.size()); for(unsigned char c:s) fprintf(gOut,"%02x",(unsigned)c); }
static void WideOut(const tWString& s) { fprintf(gOut,"w%u:",(unsigned)s.size()); for(wchar_t c:s) fprintf(gOut,"%08x",(unsigned)c); }
static void Reset()
{
    gDirs.clear(); gFullPaths.clear(); memset(gCalls,0,sizeof(gCalls));
    gFailKind=-1; gFailAt=0; gActionKind=-1; gActionAt=0; gAction=0;
    gCurrent=NULL; gCopySource=NULL; gLiveMask=NULL; gBackslash=false;
}
static void Event(int kind)
{
    ++gCalls[kind]; ++gTotalCalls[kind];
    if(gActionKind==kind && gCalls[kind]==gActionAt) {
        int action=gAction; gActionKind=-1; ++gActions;
        fprintf(gOut,"action %d\n",action);
        if(action==1) *gLiveMask="*.txt";
        if(action==2) gCurrent->ClearDirectories();
        if(action==3) gCurrent->AddDirectory(L"Nested","*",false);
        if(action==4) *gCurrent=*gCopySource;
        if(action==5) { gCurrent->ClearDirectories(); gCurrent->AddDirectory(L"Nested","*",false); }
        if(action==6) { cFileSearcher* doomed=gCurrent; gCurrent=NULL; delete doomed; }
    }
    if(gFailKind==kind && gCalls[kind]==gFailAt) {
        gFailKind=-1; fprintf(gOut,"throw %d\n",kind); throw std::runtime_error("controlled platform failure");
    }
}
static bool Matches(const tWString& file,const tWString& mask)
{
    if(mask==L"*" || mask==L"*.*") return true;
    if(mask.size()>1 && mask[0]==L'*') {
        tWString suffix=mask.substr(1);
        if(file.size()<suffix.size()) return false;
        return file.substr(file.size()-suffix.size())==suffix;
    }
    return file==mask;
}
namespace hpl {
void cPlatform::FindFilesInDir(tWStringList& out,const tWString& dir,const tWString& mask,bool hidden)
{
    fputs("files ",gOut); WideOut(dir); fputc(' ',gOut); WideOut(mask); fprintf(gOut," hidden=%d\n",hidden);
    Event(0);
    auto it=gDirs.find(dir);
    if(it!=gDirs.end()) for(const auto& file:it->second.files) if(Matches(file,mask)) { out.push_back(file); fputs("file ",gOut); WideOut(file); fputc('\n',gOut); }
}
void cPlatform::FindFoldersInDir(tWStringList& out,const tWString& dir,bool hidden,bool up)
{
    fputs("folders ",gOut); WideOut(dir); fprintf(gOut," hidden=%d up=%d\n",hidden,up);
    Event(1);
    auto it=gDirs.find(dir);
    if(it!=gDirs.end()) for(const auto& folder:it->second.folders) { out.push_back(folder); fputs("folder ",gOut); WideOut(folder); fputc('\n',gOut); }
}
tWString cPlatform::GetFullFilePath(const tWString& input)
{
    fputs("full ",gOut); WideOut(input); fputc('\n',gOut);
    Event(2);
    auto it=gFullPaths.find(input);
    tWString result=it==gFullPaths.end() ? L"V:/"+input : it->second;
    if(gBackslash) for(auto& c:result) if(c==L'/') c=L'\\';
    fputs("resolved ",gOut); WideOut(result); fputc('\n',gOut); return result;
}
}
static void Add(cFileSearcher& s,const tWString& dir,const tString& mask,bool recursive)
{
    ++gAdds; fputs("add ",gOut); WideOut(dir); fputc(' ',gOut); Bytes(mask); fprintf(gOut," recursive=%d\n",recursive);
    try { s.AddDirectory(dir,mask,recursive); fputs("add-ok\n",gOut); }
    catch(const std::exception& e) { ++gExceptions; fprintf(gOut,"caught %s\n",e.what()); }
}
static void Query(cFileSearcher& s,const tString& name,bool withCount=true)
{
    ++gQueries; int count=-123456789;
    const tWString& path=s.GetFilePath(name,withCount ? &count : NULL);
    fputs("query ",gOut); Bytes(name); fprintf(gOut," count?=%d value=%d ",withCount,count); WideOut(path); fputc('\n',gOut);
}
static void Probe(cFileSearcher& s)
{
    const char* queries[]={"same.dat","A/same.dat","B/same.dat","a/same.dat","Root/A/same.dat","absent.dat","new.txt","Nested/same.dat","extless","extless/extless","double//A\\same.dat"};
    for(const char* q:queries) { Query(s,q,true); Query(s,q,false); }
}
static void BaseDirs()
{
    gDirs[L"Root/A"].files={L"same.dat",L"Unique.TXT",L"extless",L"another.dat"};
    gDirs[L"Root/B"].files={L"same.dat",L"new.txt",L"extless"};
    gDirs[L"Root"].files={L"root.dat"}; gDirs[L"Root"].folders={L"A",L"B"};
    gDirs[L"Nested"].files={L"same.dat",L"nested.dat"};
}
static void Fixed()
{
    fputs("FIXED\n",gOut); Reset(); BaseDirs(); cFileSearcher s;
    const tWString& missing=s.GetFilePath("missing.dat");
    Probe(s); Add(s,L"Root\\A","*",false); Probe(s);
    const tWString& first=s.GetFilePath("same.dat"); tWString firstValue=first;
    Add(s,L"Root/B","*",false); Add(s,L"Root\\A","*",false); Add(s,L"Root/B","*",false);
    fprintf(gOut,"first-stable=%d value=%d missing=%d\n",&first==&s.GetFilePath("same.dat"),first==firstValue,missing.empty());
    Probe(s);
    for(const char* q:{"SAME.DAT","UNIQUE.txt","Root/B/new.txt","wrong/new.txt","extless","Root/B/extless","/same.dat","\\same.dat","../same.dat","A/B/A/same.dat","same.dat/same.dat"}) { Query(s,q); Query(s,q,false); }
    cFileSearcher copy(s); cFileSearcher assigned; assigned=s;
    fprintf(gOut,"copy-distinct=%d assigned-distinct=%d\n",&copy.GetFilePath("same.dat")!=&first,&assigned.GetFilePath("same.dat")!=&first);
    const tWString& assignedNull=assigned.GetFilePath("missing.dat");
    assigned=assigned;
    fprintf(gOut,"self-assignment-null=%d\n",&assignedNull==&assigned.GetFilePath("missing.dat"));
    const tWString& selfRef=s.GetFilePath("same.dat"); s=s;
    fprintf(gOut,"self-assignment-found=%d value=%d\n",&selfRef==&s.GetFilePath("same.dat"),selfRef==firstValue);
    s.ClearDirectories(); fprintf(gOut,"null-after-clear=%d same-null=%d\n",missing.empty(),&missing==&s.GetFilePath("missing.dat"));
    Probe(s); Probe(copy); Probe(assigned);
    Add(s,L"Nested","*",false); Probe(copy); assigned=s; Probe(assigned); Probe(copy);
    fprintf(gOut,"null-after-assignment=%d same-null=%d\n",assignedNull.empty(),&assignedNull==&assigned.GetFilePath("missing.dat"));
    // Path component comparisons remain case-sensitive; only filenames fold case.
    Reset(); gDirs[L"Case"].files={L"SAME.DAT"}; gDirs[L"case"].files={L"same.dat"}; cFileSearcher casing;
    Add(casing,L"Case","*",false); Add(casing,L"case","*",false);
    for(const char* q:{"Case/same.dat","case/SAME.DAT","CASE/same.dat"}) Query(casing,q);
    // Exact resolution aliases distinguish first-entry suppression from normal
    // distinct directories; private duplicate multiplicity is not observed.
    Reset(); BaseDirs(); gFullPaths[L"Root/B/same.dat"]=L"V:/Alias/same.dat";
    cFileSearcher aliases; Add(aliases,L"Root/A","*",false); Add(aliases,L"Root/B","*",false); Add(aliases,L"Root/B","*",false); Probe(aliases);
    // Stored components include the filename; the reverse scan can match it.
    Reset(); gDirs[L"Score"].files={L"same.dat",L"extless"}; gFullPaths[L"Score/same.dat"]=L"X/a/b/same.dat";
    cFileSearcher scoring; Add(scoring,L"Score","*",false);
    for(const char* q:{"same.dat/same.dat","X/a/b/same.dat/gap/same.dat","b/a/same.dat","x/a/b/same.dat","extless","extless/extless","a/b/same.dat"}) Query(scoring,q);
    // One candidate matches the requested directory sequence despite skipped
    // available components; the other requires the opposite subsequence scan.
    gDirs[L"Score2"].files={L"same.dat"}; gFullPaths[L"Score2/same.dat"]=L"b/a/same.dat";
    Add(scoring,L"Score2","*",false); Query(scoring,"b/gap/a/same.dat"); Query(scoring,"X/a/b/same.dat");
    // The object behind a returned const reference is not const. A legal
    // const_cast edit changes subsequent duplicate-path comparisons, while
    // the entry's already-tokenized path components remain unchanged.
    Reset(); cFileSearcher liveText;
    gDirs[L"A"].files={L"same.dat"}; gDirs[L"B"].files={L"same.dat"}; gDirs[L"C"].files={L"same.dat"};
    Add(liveText,L"A","*",false);
    const tWString& mutableFirst=liveText.GetFilePath("same.dat");
    const_cast<tWString&>(mutableFirst)=L"V:/B/same.dat";
    Add(liveText,L"B","*",false); Query(liveText,"B/same.dat"); Query(liveText,"A/same.dat");
    const_cast<tWString&>(mutableFirst)=L"V:/A/same.dat";
    Add(liveText,L"B","*",false);
    const tWString& mutableSecond=liveText.GetFilePath("B/same.dat");
    const_cast<tWString&>(mutableSecond)=L"V:/C/same.dat";
    Add(liveText,L"C","*",false); Query(liveText,"C/same.dat"); Query(liveText,"B/same.dat");
    const tWString& mutableNull=liveText.GetFilePath("absent.dat");
    const_cast<tWString&>(mutableNull)=L"custom null";
    cFileSearcher copiedNull(liveText), assignedNullCopy; assignedNullCopy=liveText;
    liveText.ClearDirectories(); Query(liveText,"absent.dat"); Query(copiedNull,"absent.dat"); Query(assignedNullCopy,"absent.dat");
    fprintf(gOut,"mutated-null-stable=%d\n",&mutableNull==&liveText.GetFilePath("absent.dat"));
    // Long strings force native string/deque growth while earlier references live.
    Reset(); cFileSearcher growth; gDirs[L"Long"].files={L"same.dat"}; Add(growth,L"Long","*",false);
    const tWString& stable=growth.GetFilePath("same.dat"); const tWString stableCopy=stable;
    for(int i=0;i<2048;++i) gDirs[L"Growth"].files.push_back(Wide("file_"+std::to_string(i)+std::string(100,'x')+".dat"));
    Add(growth,L"Growth","*",false); Query(growth,"file_2047"+std::string(100,'x')+".dat");
    fprintf(gOut,"growth-stable=%d value=%d\n",&stable==&growth.GetFilePath("same.dat"),stable==stableCopy);
    // ASCII and Latin-1 keys remain byte strings. Wider filename code units
    // above 255 collapse to '_' on Windows, via the existing cString helper.
    Reset(); cFileSearcher encoding;
    gDirs[L"Encoding"].files={tWString(1,(wchar_t)0xe9)+L".dat",L"wide_\u0100.dat",L"wide__.dat",L"Mix.TXT"};
    Add(encoding,L"Encoding","*",false); Query(encoding,tString(1,(char)0xe9)+".dat"); Query(encoding,"wide__.dat"); Query(encoding,"MIX.txt");
}
static void RecursiveAndExceptions()
{
    fputs("RECURSION\n",gOut);
    for(int action=0;action<=5;++action) for(int kind=0;kind<3;++kind) {
        Reset(); BaseDirs(); cFileSearcher s, source;
        Add(source,L"Root/B","*",false); Add(s,L"Root/A","*",false);
        tString mask="*.dat"; memset(gCalls,0,sizeof(gCalls));
        gCurrent=&s; gCopySource=&source; gLiveMask=&mask; gActionKind=kind; gActionAt=1; gAction=action;
        fprintf(gOut,"reentry action=%d kind=%d\n",action,kind);
        Add(s,L"Root",mask,true); Probe(s); Query(s,"B/new.txt");
    }
    for(int kind=0;kind<3;++kind) for(int at=1;at<=5;++at) {
        Reset(); BaseDirs(); cFileSearcher s; Add(s,L"Nested","*",false);
        memset(gCalls,0,sizeof(gCalls)); gFailKind=kind; gFailAt=at;
        fprintf(gOut,"exception kind=%d at=%d\n",kind,at); Add(s,L"Root","*",true); Probe(s);
        gFailKind=-1; Add(s,L"Root","*",true); Probe(s);
    }
    // A search path may alias a stored result, but AddDirectory copies it
    // before platform callbacks. Clearing during listing is therefore defined.
    Reset(); BaseDirs(); cFileSearcher s; Add(s,L"Root/A","*",false);
    const tWString& alias=s.GetFilePath("same.dat"); gDirs[alias].files={L"same.dat"};
    gCurrent=&s; memset(gCalls,0,sizeof(gCalls)); gActionKind=0; gActionAt=1; gAction=2;
    s.AddDirectory(alias,"*",false); Query(s,"same.dat");
}
static void TerminalDeletion()
{
    // Deleting this from a callback is defined when the outer member never
    // touches it again. Each object below is heap-owned and deleted once by
    // the action. Path/mask arguments and native result lists live separately.
    fputs("TERMINAL-DELETION\n",gOut);
    for(int scenario=0;scenario<7;++scenario) {
        Reset(); gCurrent=new cFileSearcher();
        int kind=0; bool recursive=false; bool throws=false;
        if(scenario==1) recursive=true; // Empty files, then empty folders: no member access.
        if(scenario==2 || scenario==4) {
            kind=1; recursive=true; gDirs[L"Terminal"].files={L"same.dat"};
        }
        if(scenario>=3 && scenario<=5) throws=true;
        if(scenario==5) { kind=2; gDirs[L"Terminal"].files={L"same.dat"}; }
        if(scenario==6) {
            recursive=true; gDirs[L"Terminal"].folders={L"child"};
        }
        gActionKind=kind; gActionAt=scenario==6 ? 2 : 1; gAction=6;
        if(throws) { gFailKind=kind; gFailAt=1; }
        fprintf(gOut,"terminal scenario=%d kind=%d recursive=%d throws=%d\n",scenario,kind,recursive,throws);
        Add(*gCurrent,L"Terminal","*",recursive);
        // Do not use the former object/reference. Only inspect the independent
        // action slot to establish that the callback consumed ownership.
        fprintf(gOut,"terminal-deleted=%d\n",gCurrent==NULL);
        if(gCurrent) { delete gCurrent; gCurrent=NULL; }
    }
}
static void RandomCases()
{
    for(int test=0;test<2048;++test) {
        fprintf(gOut,"RANDOM %d\n",test); Reset(); cFileSearcher s;
        std::vector<tWString> dirs; int count=Int(2,7);
        for(int d=0;d<count;++d) {
            tString name="D"+std::to_string(d); tWString dir=Wide("Top/"+name); dirs.push_back(dir); gDirs[L"Top"].folders.push_back(Wide(name));
            for(int f=0,n=Int(1,12);f<n;++f) {
                tString file=(Int(0,3)==0 ? "FILE" : "file")+std::to_string(Int(0,6));
                if(Int(0,4)!=0) file+=Int(0,1) ? ".dat" : ".txt";
                gDirs[dir].files.push_back(Wide(file));
            }
        }
        gBackslash=Int(0,1)!=0;
        if(Int(0,1)) Add(s,L"Top",Int(0,1) ? "*" : "*.dat",true);
        for(int n=0;n<5;++n) {
            int d=Int(0,count-1); tWString dir=dirs[d]; if(Int(0,1)) for(auto& c:dir) if(c==L'/') c=L'\\';
            Add(s,dir,Int(0,3)==0 ? "*.txt" : "*",false);
        }
        for(int q=0;q<30;++q) {
            tString file=(Int(0,2)==0 ? "FILE" : "file")+std::to_string(Int(0,7));
            if(Int(0,4)!=0) file+=Int(0,1) ? ".dat" : ".txt";
            int d=Int(0,count); tString name=(Int(0,2)==0 ? "" : "D"+std::to_string(d)+(Int(0,1) ? "/" : "\\"))+file;
            Query(s,name,Int(0,3)!=0);
        }
        if(test%8==0) {
            cFileSearcher copy(s), assigned; assigned=copy; s.ClearDirectories();
            for(const char* name:{"file0.dat","D1/file1.txt","file2","missing.dat"}) { Query(copy,name); Query(assigned,name); Query(s,name); }
        }
    }
}
int main(int argc,char** argv)
{
    gOut=argc>1 ? fopen(argv[1],"wb") : stdout; if(!gOut) return 2; gLogOut=gOut;
#if HARNESS_RUST
    RustBridge_Init();
#endif
    Fixed(); RecursiveAndExceptions(); TerminalDeletion(); RandomCases();
    fprintf(gOut,"TOTAL adds=%d queries=%d files=%d folders=%d full=%d actions=%d exceptions=%d\n",gAdds,gQueries,gTotalCalls[0],gTotalCalls[1],gTotalCalls[2],gActions,gExceptions);
    if(gOut!=stdout) fclose(gOut); return 0;
}
