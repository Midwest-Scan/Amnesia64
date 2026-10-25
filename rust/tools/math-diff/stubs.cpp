// Minimal definitions of engine symbols the math module links against, so the
// math sources can be built standalone for the differential test.

#include "system/String.h"
#include "system/SerializeClass.h"
#include <cstdarg>
#include <cstdio>

namespace hpl {

	static void Print(const char* prefix, const char* fmt, va_list ap)
	{
		fprintf(stderr, "%s", prefix);
		vfprintf(stderr, fmt, ap);
	}

	void Error(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("ERROR: ", fmt, ap); va_end(ap); }
	void Warning(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("WARNING: ", fmt, ap); va_end(ap); }
	void Log(const char* fmt, ...) { va_list ap; va_start(ap, fmt); Print("", fmt, ap); va_end(ap); }

	cSerializeClass::cSerializeClass(const char*, const char*, cSerializeMemberField*, size_t, iSerializable* (*)()) {}

	// Copied from sources/system/String.cpp
	tString cString::Sub(const tString& asString,int alStart,int alCount)
	{
		int lStringSize = (int)asString.length();
		if(alStart >= lStringSize) return "";
		if(alStart + alCount > lStringSize) alCount = lStringSize - alStart;

		if(alCount<0)	return asString.substr(alStart);
		else			return asString.substr(alStart, alCount);
	}

	tWString cString::SubW(const tWString& asString,int alStart,int alCount)
	{
		int lStringSize = (int)asString.length();
		if(lStringSize==0) return _W("");
		if(alStart >= lStringSize) return _W("");
		if(alStart + alCount > lStringSize) alCount = lStringSize - alStart;

		if(alCount<0)	return asString.substr(alStart);
		else			return asString.substr(alStart, alCount);
	}
}
