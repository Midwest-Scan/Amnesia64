// Differential test for the system module: calls every cString function and
// cPreprocessParser::Parse on fixed edge cases and thousands of random
// inputs, and prints the results. The output of the original C++ and of the
// Rust port must be byte-identical.
//
// Inputs are kept inside what the original handles without undefined
// behaviour on Linux (where this test runs): see the notes at each test.

#include "system/String.h"
#include "system/PreprocessParser.h"
#include "math/MathTypes.h"
#include "graphics/Color.h"

#include <cstdio>
#include <cstring>
#include <cstdint>
#include <string>
#include <vector>

#if HARNESS_RUST
#include "system/RustBridge.h"
#endif

using namespace hpl;

extern FILE* gLogOut;

//------------------------------------------
// Random numbers (same sequence for every build)

static uint64_t gState = 0x2545F4914F6CDD1Dull;
static uint32_t Next() { gState ^= gState << 13; gState ^= gState >> 7; gState ^= gState << 17; return (uint32_t)(gState >> 16); }
static int Int(int lo, int hi) { return lo + (int)(Next() % (uint32_t)(hi - lo + 1)); }
static bool Chance(int percent) { return Int(0, 99) < percent; }

static tString RandFrom(const char* alphabet, size_t alphabetLen, int maxLen)
{
	tString s;
	int len = Int(0, maxLen);
	for(int i=0; i<len; ++i) s += alphabet[Next() % alphabetLen];
	return s;
}

static const char kGeneral[] = "abcXYZ019 ./\\,\t\n\r-+e_[]u";
static const char kPath[] = "abc./\\";
static const char kNumbers[] = "0123456789 .,-+eExX\tinfa";
static const char kUnicode[] = "[]ubr0123456789xX ";

static tString RandGeneral(int maxLen = 16)
{
	tString s = RandFrom(kGeneral, sizeof(kGeneral) - 1, maxLen);
	if(Chance(10) && !s.empty()) s[Int(0, (int)s.size() - 1)] = '\0';
	if(Chance(15) && !s.empty()) s[Int(0, (int)s.size() - 1)] = (char)Int(128, 255);
	return s;
}

static tString RandAscii(int maxLen = 16)
{
	tString s = RandFrom(kGeneral, sizeof(kGeneral) - 1, maxLen);
	return s;
}

static tWString Widen(const tString& s)
{
	tWString w;
	for(size_t i=0; i<s.size(); ++i) w += (wchar_t)(unsigned char)s[i];
	return w;
}

static tWString RandWide(int maxLen = 16)
{
	tWString w = Widen(RandGeneral(maxLen));
	if(Chance(30) && !w.empty()) w[Int(0, (int)w.size() - 1)] = (wchar_t)Int(0x100, 0xFFFF);
	if(Chance(10) && !w.empty()) w[Int(0, (int)w.size() - 1)] = (wchar_t)Int(0x10000, 0x10FFFF);
	return w;
}

static float RandFloat()
{
	switch(Int(0, 5))
	{
	case 0: return (float)Int(-1000, 1000);
	case 1: return (float)Int(-100000, 100000) / 64.0f;
	case 2: { uint32_t b = Next() << 16 | Next(); float f; memcpy(&f, &b, 4); if(f != f) f = 0.5f; return f; }
	case 3: return (float)Int(-1000000, 1000000) * 1e-4f;
	case 4: return (float)Int(0, 9) * 0.125f;
	default: return (float)Int(-30, 30) * 1e10f;
	}
}

//------------------------------------------
// Output

static FILE* gOut;

static void P(const char* label) { fprintf(gOut, "\n%s:", label); }
static void P(int i) { fprintf(gOut, " %d", i); }
static void P(unsigned int i) { fprintf(gOut, " %u", i); }
static void P(bool b) { fputs(b ? " T" : " F", gOut); }
static void P(float f) { uint32_t b; memcpy(&b, &f, 4); fprintf(gOut, " %08x", b); }
static void P(const tString& s)
{
	fputs(" \"", gOut);
	for(size_t i=0; i<s.size(); ++i)
	{
		unsigned char c = (unsigned char)s[i];
		if(c >= 0x21 && c <= 0x7e && c != '\\' && c != '"') fputc(c, gOut);
		else fprintf(gOut, "\\x%02x", c);
	}
	fputs("\"", gOut);
}
static void P(const tWString& s)
{
	fputs(" L[", gOut);
	for(size_t i=0; i<s.size(); ++i) fprintf(gOut, "%s%x", i ? "," : "", (unsigned int)s[i]);
	fputs("]", gOut);
}
static void P(const cVector2f& v) { P(v.x); P(v.y); }
static void P(const cVector3f& v) { P(v.x); P(v.y); P(v.z); }
static void P(const cVector2l& v) { P(v.x); P(v.y); }
static void P(const cVector3l& v) { P(v.x); P(v.y); P(v.z); }
static void P(const cColor& c) { P(c.r); P(c.g); P(c.b); P(c.a); }
static void P(const cMatrixf& m) { for(int i=0; i<16; ++i) P(m.v[i]); }
template<class T> static void P(const std::vector<T>& v) { fprintf(gOut, " #%d", (int)v.size()); for(size_t i=0; i<v.size(); ++i) P(v[i]); }

//------------------------------------------
// Tests

static void TestEncodings()
{
	// To16Char: ASCII only (glibc's mbstowcs fails on other bytes in the
	// "C" locale and the original then throws). Embedded NULs stop it.
	for(int i=0; i<500; ++i)
	{
		tString s = RandAscii();
		if(Chance(10) && !s.empty()) s[Int(0, (int)s.size() - 1)] = '\0';
		P("To16Char"); P(s); P(cString::To16Char(s));
	}
	for(int i=0; i<1000; ++i)
	{
		tWString w = RandWide();
		P("To8Char"); P(w); P(cString::To8Char(w));
		P("S16BitToUTF8"); P(cString::S16BitToUTF8(w));
	}
	// Large and negative wchar_t values (32-bit on Linux)
	{
		const unsigned int vals[] = { 0x7F, 0x80, 0x7FF, 0x800, 0xFFFF, 0x10000, 0x1FFFFF, 0x200000, 0x3FFFFFF, 0x4000000, 0x7FFFFFFF, 0x80000000u, 0xFFFFFFFFu, 0x12345678 };
		for(size_t i=0; i<sizeof(vals)/sizeof(vals[0]); ++i)
		{
			tWString w(1, (wchar_t)vals[i]);
			P("S16BitToUTF8"); P(w); P(cString::S16BitToUTF8(w));
			P("To8Char"); P(cString::To8Char(w));
		}
	}

	// UTF8ToWChar: random bytes, but only strings where the decoder never
	// reads past the terminating NUL (the original reads beyond the string
	// for a truncated sequence at the very end).
	for(int i=0, done=0; done<1500 && i<100000; ++i)
	{
		tString s;
		int len = Int(0, 12);
		for(int j=0; j<len; ++j)
		{
			switch(Int(0, 5))
			{
			case 0: s += (char)Int(0x20, 0x7e); break;
			case 1: s += (char)Int(0x80, 0xBF); break;
			case 2: s += (char)Int(0xC0, 0xDF); break;
			case 3: s += (char)Int(0xE0, 0xEF); break;
			case 4: s += (char)Int(0xF0, 0xFF); break;
			default: s += (char)Int(0, 0x7f); break;
			}
		}
		size_t pos = 0, maxRead = 0;
		while(pos < s.size())
		{
			unsigned char c = (unsigned char)s[pos];
			size_t n = 1;
			if((c & 0x80) == 0) n = 1;
			else if((c & 0xe0) == 0xc0) n = 2;
			else if((c & 0xf0) == 0xe0) n = 3;
			else if(sizeof(wchar_t) == 4 && (c & 0xf8) == 0xf0) n = 4;
			else if(sizeof(wchar_t) == 4 && (c & 0xfc) == 0xf8) n = 5;
			else if(sizeof(wchar_t) == 4 && (c & 0xfe) == 0xfc) n = 6;
			if(pos + n - 1 > maxRead) maxRead = pos + n - 1;
			pos += n;
		}
		if(maxRead > s.size()) continue;
		++done;
		P("UTF8ToWChar"); P(s); P(cString::UTF8ToWChar(s));
	}
	{
		const char* valid[] = { "h\xc3\xa9llo", "\xe2\x82\xac", "\xf0\x9f\x98\x80x", "\xf8\x88\x80\x80\x80y", "\xfc\x84\x80\x80\x80\x80z", "" };
		for(size_t i=0; i<sizeof(valid)/sizeof(valid[0]); ++i)
		{
			tString s(valid[i]);
			P("UTF8ToWChar"); P(s); P(cString::UTF8ToWChar(s));
			P("roundtrip"); P(cString::S16BitToUTF8(cString::UTF8ToWChar(s)));
		}
	}

	for(int i=0; i<1000; ++i)
	{
		tString s = RandFrom(kUnicode, sizeof(kUnicode) - 1, 20);
		if(Chance(20)) s = "a[br]b[u233]c[u" + cString::ToString(Int(-100000, 2000000)) + "]";
		P("ParseUnicodeString"); P(s); P(cString::ParseUnicodeString(s));
	}
	for(int i=0; i<300; ++i)
	{
		tString s = RandFrom(kNumbers, sizeof(kNumbers) - 1, 20);
		P("Get16BitFromArray"); P(s); P(cString::Get16BitFromArray(s));
	}
}

static void TestSubAndSearch()
{
	for(int i=0; i<3000; ++i)
	{
		tString s = RandGeneral(12);
		int len = (int)s.size();
		int start = Int(0, len + 2), count = Int(-3, len + 3);
		P("Sub"); P(s); P(start); P(count); P(cString::Sub(s, start, count));
		P("Sub1"); P(cString::Sub(s, start));
		tWString w = RandWide(12);
		int wstart = Int(0, (int)w.size() + 2);
		P("SubW"); P(w); P(wstart); P(count); P(cString::SubW(w, wstart, count));
		P("SubW-neg"); P(cString::SubW(tWString(), Int(-5, -1), count));

		tString pat = RandFrom("abc./", 5, 3);
		P("FirstPos"); P(pat); P(cString::GetFirstStringPos(s, pat)); P(cString::GetFirstStringPosW(Widen(s), Widen(pat)));
		P("LastPos"); P(cString::GetLastStringPos(s, pat)); P(cString::GetLastStringPosW(Widen(s), Widen(pat)));
		P("Count"); P(cString::CountCharsInString(s, pat)); P(cString::CountCharsInStringW(w, Widen(pat)));
		P("LastChar"); P(cString::GetLastChar(s)); P(cString::GetLastCharW(w));

		tString num = RandFrom("ab0123456789\xb0", 13, 8);
		int index = -7;
		tString suffix = cString::GetNumericSuffix(num, &index);
		P("NumSuffix"); P(num); P(suffix); P(index);
		index = -7;
		tWString wsuffix = cString::GetNumericSuffixW(Widen(num), &index);
		P(wsuffix); P(index);
		P(cString::GetNumericSuffix(num, NULL));
	}
}

static void TestPaths()
{
	for(int i=0; i<3000; ++i)
	{
		tString s = RandFrom(kPath, sizeof(kPath) - 1, 12);
		tString t = RandFrom(kPath, sizeof(kPath) - 1, 12);
		tString ext = RandFrom("x.", 2, 3);
		tWString ws = Widen(s), wt = Widen(t);
		P("Path"); P(s); P(t); P(ext);
		P(cString::GetFileExt(s)); P(cString::GetFileExtW(ws));
		P(cString::SetFileExt(s, ext)); P(cString::SetFileExtW(ws, Widen(ext)));
		P(cString::GetFileName(s)); P(cString::GetFileNameW(ws));
		P(cString::GetFilePath(s)); P(cString::GetFilePathW(ws));
		P(cString::SetFilePath(s, t)); P(cString::SetFilePathW(ws, wt));
		P(cString::GetRelativePath(s, t)); P(cString::GetRelativePathW(ws, wt));
		P(cString::AddSlashAtEnd(s)); P(cString::AddSlashAtEnd(s, '\\')); P(cString::AddSlashAtEndW(ws)); P(cString::AddSlashAtEndW(ws, (wchar_t)0x2216));
		P(cString::RemoveSlashAtEnd(s)); P(cString::RemoveSlashAtEndW(ws));
	}
}

static void TestCaseAndReplace()
{
	for(int i=0; i<3000; ++i)
	{
		tString s = RandGeneral();
		tWString w = RandWide();
		P("Case"); P(s); P(cString::ToLowerCase(s)); P(cString::ToUpperCase(s));
		P(w); P(cString::ToLowerCaseW(w)); P(cString::ToUpperCaseW(w));

		tString a = RandFrom("abc.", 4, 2), b = RandFrom("xyz", 3, 2);
		P("ReplaceChar"); P(a); P(b); P(cString::ReplaceCharTo(s, a, b)); P(cString::ReplaceCharToW(w, Widen(a), Widen(b)));
		P(cString::ReplaceCharTo(s, tString(), b));

		// ReplaceStringTo with an empty old string never returns in the original.
		tString o = RandFrom("abc.", 4, 3);
		if(o.empty()) o = "a";
		tString n = RandFrom("xyz", 3, 3);
		P("ReplaceString"); P(o); P(n); P(cString::ReplaceStringTo(s, o, n));
	}
}

static void TestSplit()
{
	for(int i=0; i<3000; ++i)
	{
		tString s = RandGeneral(20);
		tString seps = RandFrom(" ,a/\n", 5, 3);
		tWString w = RandWide(20);
		tWString wseps = Widen(seps);
		P("Split"); P(s); P(seps);
		{ tStringVec v; v.push_back("keep"); P(cString::GetStringVec(s, v)); }
		{ tStringVec v; P(cString::GetStringVec(s, v, &seps)); }
		{ tWStringVec v; P(w); P(cString::GetStringVecW(w, v)); }
		{ tWStringVec v; P(cString::GetStringVecW(w, v, &wseps)); }
		{ tWStringVec v; P(cString::GetStringWVec(w, v)); }
		{ tWStringVec v; P(cString::GetStringWVec(w, v, &wseps)); }

		tString nums = RandFrom(kNumbers, sizeof(kNumbers) - 1, 24);
		P("Numbers"); P(nums);
		{ tIntVec v; v.push_back(-1); P(cString::GetIntVec(nums, v)); }
		{ tIntVec v; P(cString::GetIntVec(nums, v, &seps)); }
		{ tUIntVec v; P(cString::GetUIntVec(nums, v)); }
		{ tFloatVec v; P(cString::GetFloatVec(nums, v)); }
		{ tFloatVec v; P(cString::GetFloatVec(nums, v, &seps)); }
	}
}

static const char* kNumberStrings[] = {
	"", " ", "0", "-0", "+5", "  42", "\t\n-17abc", "2147483647", "2147483648", "-2147483648", "-2147483649",
	"99999999999999999999", "-99999999999999999999", "9223372036854775807", "9223372036854775808",
	"1.5", "-.25e1", "1e", "1e+", "1.e2", ".", "-.", "e5", "0x1p4", "0x.8", "0x", "0X1A", "0x1.fffffffffffff8p0",
	"0x1p-1074", "0x1p-1075", "0x1.8p-1074", "0x1p1024", "inf", "-INFINITY", "infinit", "nan", "nan(123)", "-nan(", "NaNx",
	"1e400", "-1e400", "1e-400", "4.9406564584124654e-324", "2.4703282292062328e-324", "2.4703282292062327e-324",
	"3.4028235e38", "3.4028236e38", "1.17549435e-38", "0.1", "0.30000000000000004", "123456789012345678901234567890",
	"1,2", "1 2", "1 2 3", "1 2 3 4", "true", "TRUE", "True ", "yes", "1 0 0 1", "0.5 0.5 0.5 1",
	"1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16", "1 2 3 4 5 6 7 8 9 10 11 12 13 14 15", "1e-45 1e-46 7e-46",
	"16777217 0.1 1e38", "0x10 0x.1p4",
	// Hex ties at double precision that land on a float tie after the
	// (float) cast, so double-level rounding errors show up in ToFloat.
	"0x1.00000100000008p0", "0x1.000001000000081p0", "0x1.00000300000008p0", "0x1.0000010000000800000000001p0",
};

static void TestFromText()
{
	const size_t n = sizeof(kNumberStrings) / sizeof(kNumberStrings[0]);
	for(size_t i=0; i<n + 2000; ++i)
	{
		tString s = i < n ? tString(kNumberStrings[i]) : RandFrom(kNumbers, sizeof(kNumbers) - 1, 30);
		const char* c = s.c_str();
		P("FromText"); P(s);
		P(cString::ToInt(c, -9)); P(cString::ToFloat(c, -9.5f)); P(cString::ToBool(c, true));
		P(cString::ToString(c, "def"));
		P(cString::ToColor(c, cColor(9, 8, 7, 6)));
		P(cString::ToVector2f(c, cVector2f(-1, -2))); P(cString::ToVector3f(c, cVector3f(-1, -2, -3)));
		P(cString::ToVector2l(c, cVector2l(-1, -2))); P(cString::ToVector3l(c, cVector3l(-1, -2, -3)));
		P(cString::ToMatrixf(c, cMatrixf::Identity));
	}
	P("FromTextNull");
	P(cString::ToInt(NULL, -9)); P(cString::ToFloat(NULL, -9.5f)); P(cString::ToBool(NULL, true)); P(cString::ToBool(NULL, false));
	P(cString::ToString(NULL, "def")); P(cString::ToColor(NULL, cColor(1, 2, 3, 4)));
	P(cString::ToVector2f(NULL, cVector2f(1, 2))); P(cString::ToVector3f(NULL, cVector3f(1, 2, 3)));
	P(cString::ToVector2l(NULL, cVector2l(1, 2))); P(cString::ToVector3l(NULL, cVector3l(1, 2, 3)));
	P(cString::ToMatrixf(NULL, cMatrixf::Identity));

	// The array functions keep reading until they have alSize values, so
	// only ask for as many as the string has (and tokens of < 20 chars).
	for(int i=0; i<2000; ++i)
	{
		tString s = RandFrom("0123456789.-e  ", 15, 40);
		int tokens = 0;
		bool tooLong = false;
		{
			int len = 0;
			for(size_t j=0; j<=s.size(); ++j)
			{
				if(j == s.size() || s[j] == ' ') { if(len) ++tokens; if(len > 19) tooLong = true; len = 0; }
				else ++len;
			}
		}
		if(tooLong) continue;
		int size = Int(0, tokens);
		unsigned int vU[32]; float vF[32];
		for(int j=0; j<32; ++j) { vU[j] = 0xDEADBEEF; vF[j] = -1.25f; }
		cString::UIntStringToArray(vU, s.c_str(), size);
		cString::FloatStringToArray(vF, s.c_str(), size);
		P("ToArray"); P(s); P(size);
		for(int j=0; j<=size && j<32; ++j) { P(vU[j]); P(vF[j]); }
	}
}

static void TestToText()
{
	const int ints[] = { 0, 1, -1, 7, -7, 2147483647, -2147483647 - 1, 1000000 };
	const int pads[] = { 0, 1, 3, 12, -1, -5 };
	for(size_t i=0; i<sizeof(ints)/sizeof(ints[0]); ++i)
		for(size_t j=0; j<sizeof(pads)/sizeof(pads[0]); ++j)
		{
			P("IntToText"); P(ints[i]); P(pads[j]);
			P(cString::ToString(ints[i], pads[j])); P(cString::ToString((unsigned int)ints[i], pads[j]));
			P(cString::ToString((unsigned long)(unsigned int)ints[i], pads[j])); P(cString::ToString((unsigned long)(long)ints[i], pads[j]));
			P(cString::ToStringW(ints[i], pads[j]));
			P(cString::ToStringW((unsigned long)(unsigned int)ints[i], pads[j])); P(cString::ToStringW((unsigned long)(long)ints[i], pads[j]));
		}
	P("Defaults"); P(cString::ToString(5)); P(cString::ToString(5u)); P(cString::ToString(5ul)); P(cString::ToString(1.5f)); P(cString::ToStringW(5)); P(cString::ToStringW(5ul)); P(cString::ToStringW(1.5f));

	const float specials[] = { 0.0f, -0.0f, 0.5f, 1.5f, 2.5f, 0.125f, 0.375f, 1e-10f, 1e-45f, 3.4028235e38f, -3.4028235e38f, 0.1f, 1.0f/3.0f, 123456.789f, 1e30f };
	const int decimals[] = { 0, 1, 2, 3, 6, 9, 20, -1, -5, 40 };
	for(int i=0; i<1500; ++i)
	{
		float f = i < (int)(sizeof(specials)/sizeof(specials[0])) ? specials[i] : RandFloat();
		int dec = decimals[Int(0, sizeof(decimals)/sizeof(decimals[0]) - 1)];
		bool rem = Chance(50);
		P("FloatToText"); P(f); P(dec); P(rem);
		P(cString::ToString(f, dec, rem)); P(cString::ToStringW(f, dec, rem));
		P(cString::ToString(f, dec, !rem));
	}
	// inf / nan
	{
		float inf = 1e38f * 1e38f;
		float nan = inf - inf;
		P("FloatSpecial"); P(cString::ToString(inf, 2, true)); P(cString::ToString(-inf)); P(cString::ToStringW(inf, 3));
		P(cString::ToString(nan, 2)); P(cString::ToString(-nan, 2)); P(cString::ToStringW(nan, 2));
	}
}

static void TestHashAndData()
{
	// GetHash is not tested here: on Linux (4-byte wchar_t) the original
	// reads past the string. The Rust port implements the Windows version
	// and is checked against reference values in its unit tests.
	for(int i=0; i<1000; ++i)
	{
		tWString w = RandWide(20);
		P("HashW"); P(w); P(cString::GetHashW(w));
	}
	for(int i=0; i<500; ++i)
	{
		std::vector<unsigned char> data(Int(0, 7) * 4 + (Chance(10) ? 1 : 0));
		for(size_t j=0; j<data.size(); ++j) data[j] = (unsigned char)Next();
		tString text = "untouched";
		cString::EncodeDataToTextString(data.empty() ? NULL : &data[0], data.size(), text);
		P("Encode"); P((int)data.size()); P(text);

		std::vector<unsigned char> back(data.size() + 4, 0xAB);
		tString in = text;
		if(Chance(20) && !in.empty()) in.resize(in.size() - 1);
		if(Chance(20)) in += "x";
		cString::DecodeDataFromTextString(in, &back[0], data.size());
		P("Decode"); P(in);
		for(size_t j=0; j<back.size(); ++j) P((int)back[j]);
	}
}

//------------------------------------------
// Preprocessor

static const char* kPreTokens[] = {
	"text", " ", "  ", "\n", "\r\n", "\t", "$", "$A", "$B$", "$C", "$Missing", "$1x", "$$", "@", "@ifdef ", "@elseif ",
	"@else", "@endif", "@define ", "@include ", "&&", " && ", " || ", "||", "A", "B", "C", "D", "Var1", "x", ";",
	"(", ")", "inc.glsl", "missing.glsl", "\\inc.glsl", "@define D 5\n", "@ifdef A\n", "@ifdef Q\n", "@endif\n",
	"@else\n", "@elseif B\n", "@include inc.glsl\n", "vec3 $A;", "x=$D;\n", "# ", "@ifdef A && B\n", "@ifdef Q || C\n",
};

static void TestPreprocess()
{
	cPreprocessParser parser;
	parser.GetEnvVarContainer()->Add("C", "envC");
	parser.GetEnvVarContainer()->Add("E");
	parser.GetEnvVarContainer()->Add("Num", 42);

	cParserVarContainer vars;
	vars.Add("A", "1");
	vars.Add("B", "");
	vars.Add("C", "shadowed");

	const char* fixed[] = {
		"",
		"plain text",
		"@ifdef A\na\n@elseif B\nb\n@else\nc\n@endif\nend\n",
		"@ifdef Q\na\n@elseif B\nb\n@else\nc\n@endif\nend\n",
		"@ifdef Q\na\n@elseif R\nb\n@else\nc\n@endif\nend\n",
		"@ifdef Q\n@ifdef A\nx\n@else\ny\n@endif\n@else\nz\n@endif\nend\n",
		"\t  @define Foo bar\n$Foo-$Foo$\nend\n",
		"@include inc.glsl\nafter\n",
		"@include missing.glsl\nafter\n",
		"@define\nend\n",
		"@ifdef\nend\n",
		"@include\nend\n",
		"a\n@endif\nb\n",
		"@else\nend\n",
		"x && y\n@ifdef A && Q\nno\n@endif\n@ifdef Q || B\nyes\n@endif\nend\n",
		"$C $E $Num $Missing\nend\n",
		"@ifdef A\n@define D 1\n@include missing.glsl\n$D\n@endif\nend\n",
		"@ifdef A\n@ifdef\n$A\n@endif\nend\n",
		"@ifdef Q\n@ifdef A\n@else\n@endif\n@elseif A\nok\n@endif\nend\n",
	};
	const wchar_t* dirs[] = { L"", L"shaders", L"shaders/", L"dir\\" };

	const size_t nFixed = sizeof(fixed) / sizeof(fixed[0]);
	for(size_t i=0; i<nFixed + 3000; ++i)
	{
		tString input;
		if(i < nFixed)
		{
			input = fixed[i];
		}
		else
		{
			int count = Int(1, 14);
			for(int j=0; j<count; ++j) input += kPreTokens[Next() % (sizeof(kPreTokens) / sizeof(kPreTokens[0]))];
			// End with a line of text so no keyword is the last symbol: the
			// original dereferences a null symbol there.
			input += "\nend\n";
		}
		tWString dir = dirs[i % 4];
		cParserVarContainer* pVars = (i % 5 == 0) ? NULL : &vars;

		P("Preprocess"); P(input); P(dir); P(pVars != NULL);
		fputs("\n", gOut);
		tString output = "PRE:";
		bool ok = parser.Parse(&input, &output, pVars, dir);
		P(ok); P(output);
		tParseVarMap* pDefined = parser.GetParsingVarContainer()->GetMapPtr();
		P((int)pDefined->size());
		for(tParseVarMapIt it = pDefined->begin(); it != pDefined->end(); ++it) { P(it->first); P(it->second); }
	}
}

int main(int argc, char** argv)
{
	gOut = argc > 1 ? fopen(argv[1], "wb") : stdout;
	if(!gOut) return 1;
	gLogOut = gOut;
#if HARNESS_RUST
	RustBridge_Init();
#endif

	TestEncodings();
	TestSubAndSearch();
	TestPaths();
	TestCaseAndReplace();
	TestSplit();
	TestFromText();
	TestToText();
	TestHashAndData();
	TestPreprocess();

	fputs("\n", gOut);
	fclose(gOut);
	return 0;
}
