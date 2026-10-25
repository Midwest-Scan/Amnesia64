// Minimal definitions of engine symbols the system module links against, so
// String.cpp and PreprocessParser.cpp can be built standalone for the
// differential test. Log output goes to the test's output file, so error
// and warning messages are compared too.

#include "system/LowLevelSystem.h"
#include "system/Platform.h"
#include "system/SerializeClass.h"
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <map>

FILE* gLogOut = stdout;

namespace hpl {

	static void Print(const char* prefix, const char* fmt, va_list ap)
	{
		fprintf(gLogOut, "%s", prefix);
		vfprintf(gLogOut, fmt, ap);
	}

	void Error(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("ERROR: ", fmt, ap); va_end(ap); }
	void Warning(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("WARNING: ", fmt, ap); va_end(ap); }
	void Log(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("LOG: ", fmt, ap); va_end(ap); }

	cSerializeClass::cSerializeClass(const char*, const char*, cSerializeMemberField*, size_t, iSerializable* (*)()) {}

	//------------------------------------------
	// In-memory files for @include

	static std::map<tWString, tString>& Files()
	{
		static std::map<tWString, tString> files;
		if(files.empty())
		{
			files[L"inc.glsl"] = "INC($A)\n";
			files[L"shaders/inc.glsl"] = "SHADERS_INC\n";
			files[L"shaders/empty.glsl"] = "";
			files[L"dir\\inc.glsl"] = "BACKSLASH_INC\n";
		}
		return files;
	}

	bool cPlatform::FileExists(const tWString& asFileName)
	{
		return Files().count(asFileName) != 0;
	}

	unsigned long cPlatform::GetFileSize(const tWString& asFileName)
	{
		return (unsigned long)Files()[asFileName].size();
	}

	bool cPlatform::CopyFileToBuffer(const tWString& asFileName, void *apBuffer, unsigned long alSize)
	{
		const tString& sData = Files()[asFileName];
		if(alSize > sData.size()) return false;
		if(alSize) memcpy(apBuffer, sData.data(), alSize);
		return true;
	}
}
