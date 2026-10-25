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
// cString implemented by the Rust port (rust/crates/hpl-system).
// Used when HPL2_RUST_SYSTEM is 1; otherwise String.cpp provides the original.
//////////////////////////////////////////////////////////////////////////

#include "system/RustBridge.h"

#if HPL2_RUST_SYSTEM

#include "system/String.h"

namespace hpl {

	//////////////////////////////////////////////////////////////////////////
	// ENCODINGS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	tWString cString::To16Char(const tString &asString)
	{
		tWString sOut;
		hpl_string_to_16char(RsStr(asString), RsAssign(sOut));
		return sOut;
	}

	tString cString::To8Char(const tWString &awsString)
	{
		tString sOut;
		hpl_string_to_8char(RsStr(awsString), RsAssign(sOut));
		return sOut;
	}

	tWString cString::UTF8ToWChar(const tString& asString)
	{
		tWString sOut;
		hpl_string_utf8_to_wchar(RsStr(asString), RsAssign(sOut));
		return sOut;
	}

	tString cString::S16BitToUTF8(const tWString& awsString)
	{
		tString sOut;
		hpl_string_s16bit_to_utf8(RsStr(awsString), RsAssign(sOut));
		return sOut;
	}

	tWString cString::Get16BitFromArray(const tString &asArray)
	{
		tWString sOut;
		hpl_string_get_16bit_from_array(RsStr(asArray), RsAssign(sOut));
		return sOut;
	}

	tWString cString::ParseUnicodeString(const tString &asString)
	{
		tWString sOut;
		hpl_string_parse_unicode_string(RsStr(asString), RsAssign(sOut));
		return sOut;
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// SUB STRINGS AND PATHS
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	tString cString::Sub(const tString& asString,int alStart,int alCount)
	{
		tString sOut;
		hpl_string_sub(RsStr(asString), alStart, alCount, RsAssign(sOut));
		return sOut;
	}

	tWString cString::SubW(const tWString& asString,int alStart,int alCount)
	{
		tWString sOut;
		hpl_string_sub_w(RsStr(asString), alStart, alCount, RsAssign(sOut));
		return sOut;
	}

	//-----------------------------------------------------------------------

	tString cString::GetFileExt(const tString& aString)
	{
		tString sOut;
		hpl_string_get_file_ext(RsStr(aString), RsAssign(sOut));
		return sOut;
	}

	tWString cString::GetFileExtW(const tWString& aString)
	{
		tWString sOut;
		hpl_string_get_file_ext_w(RsStr(aString), RsAssign(sOut));
		return sOut;
	}

	tString cString::SetFileExt(const tString& aString,const tString& aExt)
	{
		tString sOut;
		hpl_string_set_file_ext(RsStr(aString), RsStr(aExt), RsAssign(sOut));
		return sOut;
	}

	tWString cString::SetFileExtW(const tWString& aString,const tWString& aExt)
	{
		tWString sOut;
		hpl_string_set_file_ext_w(RsStr(aString), RsStr(aExt), RsAssign(sOut));
		return sOut;
	}

	tString cString::GetFileName(const tString& aString)
	{
		tString sOut;
		hpl_string_get_file_name(RsStr(aString), RsAssign(sOut));
		return sOut;
	}

	tWString cString::GetFileNameW(const tWString& aString)
	{
		tWString sOut;
		hpl_string_get_file_name_w(RsStr(aString), RsAssign(sOut));
		return sOut;
	}

	tString cString::GetFilePath(const tString& aString)
	{
		tString sOut;
		hpl_string_get_file_path(RsStr(aString), RsAssign(sOut));
		return sOut;
	}

	tWString cString::GetFilePathW(const tWString& aString)
	{
		tWString sOut;
		hpl_string_get_file_path_w(RsStr(aString), RsAssign(sOut));
		return sOut;
	}

	tString cString::SetFilePath(const tString& aString,const tString& aPath)
	{
		tString sOut;
		hpl_string_set_file_path(RsStr(aString), RsStr(aPath), RsAssign(sOut));
		return sOut;
	}

	tWString cString::SetFilePathW(const tWString& aString,const tWString& aPath)
	{
		tWString sOut;
		hpl_string_set_file_path_w(RsStr(aString), RsStr(aPath), RsAssign(sOut));
		return sOut;
	}

	tString cString::GetRelativePath(const tString& aPath, const tString& aRelativeToPath)
	{
		tString sOut;
		hpl_string_get_relative_path(RsStr(aPath), RsStr(aRelativeToPath), RsAssign(sOut));
		return sOut;
	}

	tWString cString::GetRelativePathW(const tWString& aPath, const tWString& aRelativeToPath)
	{
		tWString sOut;
		hpl_string_get_relative_path_w(RsStr(aPath), RsStr(aRelativeToPath), RsAssign(sOut));
		return sOut;
	}

	tString cString::AddSlashAtEnd(const tString& asPath,char alSlash)
	{
		tString sOut;
		hpl_string_add_slash_at_end(RsStr(asPath), (uint8_t)alSlash, RsAssign(sOut));
		return sOut;
	}

	tWString cString::AddSlashAtEndW(const tWString& asPath,wchar_t alSlash)
	{
		tWString sOut;
		hpl_string_add_slash_at_end_w(RsStr(asPath), alSlash, RsAssign(sOut));
		return sOut;
	}

	tString cString::RemoveSlashAtEnd(const tString& asPath)
	{
		tString sOut;
		hpl_string_remove_slash_at_end(RsStr(asPath), RsAssign(sOut));
		return sOut;
	}

	tWString cString::RemoveSlashAtEndW(const tWString& asPath)
	{
		tWString sOut;
		hpl_string_remove_slash_at_end_w(RsStr(asPath), RsAssign(sOut));
		return sOut;
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// CASE AND REPLACING
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	tString cString::ToLowerCase(const tString& aString)
	{
		tString sOut;
		hpl_string_to_lower_case(RsStr(aString), RsAssign(sOut));
		return sOut;
	}

	tWString cString::ToLowerCaseW(const tWString& aString)
	{
		tWString sOut;
		hpl_string_to_lower_case_w(RsStr(aString), RsAssign(sOut));
		return sOut;
	}

	tString cString::ToUpperCase(const tString& aString)
	{
		tString sOut;
		hpl_string_to_upper_case(RsStr(aString), RsAssign(sOut));
		return sOut;
	}

	tWString cString::ToUpperCaseW(const tWString& aString)
	{
		tWString sOut;
		hpl_string_to_upper_case_w(RsStr(aString), RsAssign(sOut));
		return sOut;
	}

	tString cString::ReplaceCharTo(const tString& aString, const tString& asOldChar,const tString& asNewChar)
	{
		tString sOut;
		hpl_string_replace_char_to(RsStr(aString), RsStr(asOldChar), RsStr(asNewChar), RsAssign(sOut));
		return sOut;
	}

	tWString cString::ReplaceCharToW(const tWString& aString, const tWString& asOldChar,const tWString& asNewChar)
	{
		tWString sOut;
		hpl_string_replace_char_to_w(RsStr(aString), RsStr(asOldChar), RsStr(asNewChar), RsAssign(sOut));
		return sOut;
	}

	tString cString::ReplaceStringTo(const tString& asString, const tString& asOldString,const tString& asNewString)
	{
		tString sOut;
		hpl_string_replace_string_to(RsStr(asString), RsStr(asOldString), RsStr(asNewString), RsAssign(sOut));
		return sOut;
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// TEXT TO VALUES
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	tString cString::ToString(const char* asString,tString asDefault)
	{
		if(asString==NULL)return asDefault;
		return asString;
	}

	int cString::ToInt(const char* asString,int alDefault)
	{
		return hpl_string_to_int(RsCStr(asString), alDefault);
	}

	float cString::ToFloat(const char* asString,float afDefault)
	{
		return hpl_string_to_float(RsCStr(asString), afDefault);
	}

	bool cString::ToBool(const char* asString, bool abDefault)
	{
		return hpl_string_to_bool(RsCStr(asString), abDefault);
	}

	cColor cString::ToColor(const char* asString, const cColor& aDefault)
	{
		return FromRs(hpl_string_to_color(RsCStr(asString), ToRs(aDefault)));
	}

	cVector2f cString::ToVector2f(const char* asString, const cVector2f& avDefault)
	{
		return FromRs(hpl_string_to_vector2f(RsCStr(asString), ToRs(avDefault)));
	}

	cVector3f cString::ToVector3f(const char* asString, const cVector3f& avDefault)
	{
		return FromRs(hpl_string_to_vector3f(RsCStr(asString), ToRs(avDefault)));
	}

	cVector2l cString::ToVector2l(const char* asString, const cVector2l& avDefault)
	{
		return FromRs(hpl_string_to_vector2l(RsCStr(asString), ToRs(avDefault)));
	}

	cVector3l cString::ToVector3l(const char* asString, const cVector3l& avDefault)
	{
		return FromRs(hpl_string_to_vector3l(RsCStr(asString), ToRs(avDefault)));
	}

	cMatrixf cString::ToMatrixf(const char* asString, const cMatrixf& a_mtxDefault)
	{
		cMatrixf mtxOut;
		hpl_string_to_matrixf(RsCStr(asString), RsPtr(a_mtxDefault), RsPtr(mtxOut));
		return mtxOut;
	}

	//-----------------------------------------------------------------------

	tIntVec& cString::GetIntVec(const tString &asData, tIntVec& avVec,tString *apSeparators)
	{
		hpl_string_get_int_vec(RsStr(asData), RsStrPtr(apSeparators), RsAppend(avVec));
		return avVec;
	}

	tUIntVec& cString::GetUIntVec(const tString &asData, tUIntVec& avVec,tString *apSeparators)
	{
		hpl_string_get_int_vec(RsStr(asData), RsStrPtr(apSeparators), RsAppend(avVec));
		return avVec;
	}

	tFloatVec& cString::GetFloatVec(const tString &asData, tFloatVec& avVec,tString *apSeparators)
	{
		hpl_string_get_float_vec(RsStr(asData), RsStrPtr(apSeparators), RsAppend(avVec));
		return avVec;
	}

	tStringVec& cString::GetStringVec(const tString &asData, tStringVec& avVec,tString *apSeparators)
	{
		hpl_string_get_string_vec(RsStr(asData), RsStrPtr(apSeparators), RsPush(avVec));
		return avVec;
	}

	tWStringVec& cString::GetStringVecW(const tWString &asData, tWStringVec& avVec,tWString *apSeparators)
	{
		hpl_string_get_string_vec_w(RsStr(asData), RsStrPtr(apSeparators), false, RsPush(avVec));
		return avVec;
	}

	tWStringVec& cString::GetStringWVec(const tWString &asData, tWStringVec& avVec,tWString *apSeparators)
	{
		hpl_string_get_string_vec_w(RsStr(asData), RsStrPtr(apSeparators), true, RsPush(avVec));
		return avVec;
	}

	//-----------------------------------------------------------------------

	void cString::UIntStringToArray(unsigned int *apArray, const char* apString,int alSize)
	{
		hpl_string_uint_string_to_array(apArray, RsCStr(apString), alSize);
	}

	void cString::FloatStringToArray(float *apArray, const char* apString,int alSize)
	{
		hpl_string_float_string_to_array(apArray, RsCStr(apString), alSize);
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// VALUES TO TEXT
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	tString cString::ToString(int alX, int alPaddingZeros)
	{
		tString sOut;
		hpl_string_from_int(alX, alPaddingZeros, RsAssign(sOut));
		return sOut;
	}

	tString cString::ToString(unsigned int alX, int alPaddingZeros)
	{
		tString sOut;
		hpl_string_from_uint(alX, alPaddingZeros, RsAssign(sOut));
		return sOut;
	}

	tString cString::ToString(unsigned long alX, int alPaddingZeros)
	{
		tString sOut;
		hpl_string_from_ulong((uint64_t)alX, alPaddingZeros, RsAssign(sOut));
		return sOut;
	}

	tString cString::ToString(float afX, int alNumDecimals, bool abRemoveTrailingZeros)
	{
		tString sOut;
		hpl_string_from_float(afX, alNumDecimals, abRemoveTrailingZeros, RsAssign(sOut));
		return sOut;
	}

	tWString cString::ToStringW(int alX, int alPaddingZeros)
	{
		tWString sOut;
		hpl_string_from_int_w(alX, alPaddingZeros, RsAssign(sOut));
		return sOut;
	}

	tWString cString::ToStringW(unsigned long alX, int alPaddingZeros)
	{
		// The original prints this with "%ld", i.e. as a signed long.
		tWString sOut;
		hpl_string_from_long_w((int64_t)(long)alX, alPaddingZeros, RsAssign(sOut));
		return sOut;
	}

	tWString cString::ToStringW(float afX, int alNumDecimals, bool abRemoveTrailingZeros)
	{
		tWString sOut;
		hpl_string_from_float_w(afX, alNumDecimals, abRemoveTrailingZeros, RsAssign(sOut));
		return sOut;
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// SEARCHING
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	tString cString::GetLastChar(const tString& aString)
	{
		tString sOut;
		hpl_string_get_last_char(RsStr(aString), RsAssign(sOut));
		return sOut;
	}

	tWString cString::GetLastCharW(const tWString& aString)
	{
		tWString sOut;
		hpl_string_get_last_char_w(RsStr(aString), RsAssign(sOut));
		return sOut;
	}

	int cString::GetFirstStringPos(const tString& aString, const tString& aChar)
	{
		return hpl_string_get_first_string_pos(RsStr(aString), RsStr(aChar));
	}

	int cString::GetFirstStringPosW(const tWString& aString, const tWString& aChar)
	{
		return hpl_string_get_first_string_pos_w(RsStr(aString), RsStr(aChar));
	}

	int cString::GetLastStringPos(const tString& aString, const tString& aChar)
	{
		return hpl_string_get_last_string_pos(RsStr(aString), RsStr(aChar));
	}

	int cString::GetLastStringPosW(const tWString& aString, const tWString& aChar)
	{
		return hpl_string_get_last_string_pos_w(RsStr(aString), RsStr(aChar));
	}

	int cString::CountCharsInString(const tString& aString, const tString& aChar)
	{
		return hpl_string_count_chars_in_string(RsStr(aString), RsStr(aChar));
	}

	int cString::CountCharsInStringW(const tWString& aString, const tWString& aChar)
	{
		return hpl_string_count_chars_in_string_w(RsStr(aString), RsStr(aChar));
	}

	tString cString::GetNumericSuffix(const tString& aString, int* apIndex)
	{
		tString sOut;
		hpl_string_get_numeric_suffix(RsStr(aString), apIndex, RsAssign(sOut));
		return sOut;
	}

	tWString cString::GetNumericSuffixW(const tWString& aString, int* apIndex)
	{
		tWString sOut;
		hpl_string_get_numeric_suffix_w(RsStr(aString), apIndex, RsAssign(sOut));
		return sOut;
	}

	//-----------------------------------------------------------------------

	//////////////////////////////////////////////////////////////////////////
	// HASHES AND DATA
	//////////////////////////////////////////////////////////////////////////

	//-----------------------------------------------------------------------

	unsigned int cString::GetHash(const tString& asStr)
	{
		return hpl_string_get_hash(RsStr(asStr));
	}

	unsigned int cString::GetHashW(const tWString& asStr)
	{
		return hpl_string_get_hash_w(RsStr(asStr));
	}

	void cString::EncodeDataToTextString(const void *apData, size_t alSize, tString &asOutput)
	{
		hpl_string_encode_data_to_text_string(static_cast<const uint8_t*>(apData), alSize, RsAssign(asOutput));
	}

	void cString::DecodeDataFromTextString(const tString &asStr, void *apDest, size_t alSize)
	{
		hpl_string_decode_data_from_text_string(RsStr(asStr), static_cast<uint8_t*>(apDest), alSize);
	}

	//-----------------------------------------------------------------------

}

#endif // HPL2_RUST_SYSTEM
