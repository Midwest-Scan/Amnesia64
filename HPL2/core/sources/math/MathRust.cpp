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
// cMath implemented by the Rust port (rust/crates/hpl-math).
// Used when HPL2_RUST_MATH is 1; otherwise Math.cpp provides the original.
//////////////////////////////////////////////////////////////////////////

#include "system/RustBridge.h"

#if HPL2_RUST_MATH

#include "math/Math.h"

#include "math/Frustum.h"
#include "graphics/VertexBuffer.h"

#include <stdio.h>
#include <vector>

namespace hpl {

	static char mpTempChar[1024];

	//-----------------------------------------------------------------------

	static HplFrustumProjection RsProjection(cFrustum *apFrustum)
	{
		HplFrustumProjection p;
		p.proj = *RsPtr(apFrustum->GetProjectionMatrix());
		p.view = *RsPtr(apFrustum->GetViewMatrix());
		p.near_plane = apFrustum->GetNearPlane();
		p.far_plane = apFrustum->GetFarPlane();
		p.fov = apFrustum->GetFOV();
		p.aspect = apFrustum->GetAspect();
		p.projection_type = (int32_t)apFrustum->GetProjectionType();
		return p;
	}

	static std::vector<uint32_t> RsWideChars(const tWString& asString)
	{
		std::vector<uint32_t> vChars;
		vChars.reserve(asString.size());
		for(size_t i=0; i<asString.size(); ++i)
			vChars.push_back((uint32_t)asString[i]);
		return vChars;
	}

	static const uint32_t* RsData(const std::vector<uint32_t>& avChars)
	{
		return avChars.empty() ? NULL : &avChars[0];
	}

	//////////////////////////////////////////////////////////////////////////
	// RANDOM GENERATION
	//////////////////////////////////////////////////////////////////////////

	int cMath::RandRectl(int alMin, int alMax) { return hpl_math_rand_rect_l(alMin, alMax); }
	float cMath::RandRectf(float afMin, float afMax) { return hpl_math_rand_rect_f(afMin, afMax); }

	cVector2f cMath::RandRectVector2f(const cVector2f &avMin,const cVector2f &avMax)
	{
		return FromRs(hpl_math_rand_rect_vector2f(ToRs(avMin), ToRs(avMax)));
	}

	cVector3f cMath::RandRectVector3f(const cVector3f &avMin,const cVector3f &avMax)
	{
		return FromRs(hpl_math_rand_rect_vector3f(ToRs(avMin), ToRs(avMax)));
	}

	cColor cMath::RandRectColor(const cColor &aMin,const cColor &aMax)
	{
		return FromRs(hpl_math_rand_rect_color(ToRs(aMin), ToRs(aMax)));
	}

	cVector3f cMath::RandomSphereSurfacePoint(float afRadius) { return FromRs(hpl_math_random_sphere_surface_point(afRadius)); }
	cVector3f cMath::RandomSphereInsidePoint(float afRadius) { return FromRs(hpl_math_random_sphere_inside_point(afRadius)); }
	void cMath::Randomize(int alSeed) { hpl_math_randomize(alSeed); }

	//////////////////////////////////////////////////////////////////////////
	// BIT OPERATIONS AND COLOR CONVERSION
	//////////////////////////////////////////////////////////////////////////

	int cMath::GetFlagBit(int alBitNum) { return hpl_math_get_flag_bit(alBitNum); }

	cVector3f cMath::RGBToHSB(const cColor& aRGB)
	{
		cVector3f vHSB;
		RGBToHSBHelper(aRGB, vHSB);
		return vHSB;
	}

	void cMath::RGBToHSBHelper(const cColor& aRGB, cVector3f& avX)
	{
		hpl_math_rgb_to_hsb_into(ToRs(aRGB), RsPtr(&avX));
	}

	cColor cMath::HSBToRGB(const cVector3f& avHSB)
	{
		cColor c;
		HSBToRGBHelper(avHSB, c);
		return c;
	}

	void cMath::HSBToRGBHelper(const cVector3f& avHSB, cColor& aX)
	{
		hpl_math_hsb_to_rgb_into(ToRs(avHSB), RsPtr(&aX));
	}

	cColor cMath::HexToRGB(const tString& asHex) { cColor c; HexToRGBHelper(asHex, c); return c; }
	cColor cMath::HexWToRGB(const tWString& asHex) { cColor c; HexWToRGBHelper(asHex, c); return c; }
	cColor cMath::HexToRGBA(const tString& asHex) { cColor c; HexToRGBAHelper(asHex, c); return c; }
	cColor cMath::HexWToRGBA(const tWString& asHex) { cColor c; HexWToRGBAHelper(asHex, c); return c; }

	void cMath::HexToRGBHelper(const tString& asHex, cColor& aCol)
	{
		hpl_math_hex_to_color((const uint8_t*)asHex.c_str(), asHex.size(), RsPtr(&aCol), false);
	}

	void cMath::HexWToRGBHelper(const tWString& asHex, cColor& aCol)
	{
		std::vector<uint32_t> vChars = RsWideChars(asHex);
		hpl_math_hex_to_color_wide(RsData(vChars), vChars.size(), RsPtr(&aCol), false);
	}

	void cMath::HexToRGBAHelper(const tString& asHex, cColor& aCol)
	{
		hpl_math_hex_to_color((const uint8_t*)asHex.c_str(), asHex.size(), RsPtr(&aCol), true);
	}

	void cMath::HexWToRGBAHelper(const tWString& asHex, cColor& aCol)
	{
		std::vector<uint32_t> vChars = RsWideChars(asHex);
		hpl_math_hex_to_color_wide(RsData(vChars), vChars.size(), RsPtr(&aCol), true);
	}

	tString cMath::RGBToHex(const cColor& aRGB)
	{
		char vBuf[8];
		hpl_math_color_to_hex(ToRs(aRGB), false, (uint8_t*)vBuf);
		return tString(vBuf, 6);
	}

	tWString cMath::RGBToHexW(const cColor& aRGB)
	{
		tString sHex = RGBToHex(aRGB);
		return tWString(sHex.begin(), sHex.end());
	}

	tString cMath::RGBAToHex(const cColor& aRGB)
	{
		char vBuf[8];
		hpl_math_color_to_hex(ToRs(aRGB), true, (uint8_t*)vBuf);
		return tString(vBuf, 8);
	}

	tWString cMath::RGBAToHexW(const cColor& aRGB)
	{
		tString sHex = RGBAToHex(aRGB);
		return tWString(sHex.begin(), sHex.end());
	}

	unsigned char cMath::HexStringToUChar(const tString& asHexStr)
	{
		return hpl_math_hex_string_to_uchar((const uint8_t*)asHexStr.c_str(), asHexStr.size());
	}

	unsigned char cMath::HexStringToUChar(const tWString& asHexStr)
	{
		std::vector<uint32_t> vChars = RsWideChars(asHexStr);
		return hpl_math_hex_string_to_uchar_wide(RsData(vChars), vChars.size());
	}

	tString cMath::UCharToHexString(unsigned char alValue)
	{
		char vBuf[2];
		hpl_math_uchar_to_hex(alValue, (uint8_t*)vBuf);
		return tString(vBuf, 2);
	}

	tWString cMath::UCharToHexStringW(unsigned char alValue)
	{
		tString sHex = UCharToHexString(alValue);
		return tWString(sHex.begin(), sHex.end());
	}

	//////////////////////////////////////////////////////////////////////////
	// BV AND AABB OPERATIONS, INTERSECTIONS
	//////////////////////////////////////////////////////////////////////////

	void cMath::ExpandAABB(cVector3f& avBaseMin,cVector3f& avBaseMax, const cVector3f& avAddMin, const cVector3f& avAddMax)
	{
		hpl_math_expand_aabb(RsPtr(&avBaseMin), RsPtr(&avBaseMax), ToRs(avAddMin), ToRs(avAddMax));
	}

	bool cMath::CheckRectIntersection(cRect2l aRect1, cRect2l aRect2) { return hpl_math_check_rect_intersection_l(ToRs(aRect1), ToRs(aRect2)); }
	bool cMath::CheckRectIntersection(cRect2f aRect1, cRect2f aRect2) { return hpl_math_check_rect_intersection_f(ToRs(aRect1), ToRs(aRect2)); }

	bool cMath::CheckPointInRectIntersection(const cVector2f& avPoint, const cRect2f& aRect)
	{
		return hpl_math_check_point_in_rect_intersection(ToRs(avPoint), ToRs(aRect));
	}

	bool cMath::CheckRectFit(const cRect2l& aRectSrc, const cRect2l& aRectDest) { return hpl_math_check_rect_fit_l(ToRs(aRectSrc), ToRs(aRectDest)); }
	bool cMath::CheckRectFit(const cRect2f& aRectSrc, const cRect2f& aRectDest) { return hpl_math_check_rect_fit_f(ToRs(aRectSrc), ToRs(aRectDest)); }

	cRect2f cMath::GetClipRect(const cRect2f& aRectSrc,const cRect2f& aRectDest)
	{
		return FromRs(hpl_math_get_clip_rect(ToRs(aRectSrc), ToRs(aRectDest)));
	}

	bool cMath::CheckSphereLineIntersection(const cVector3f& avSpherePos, float afSphereRadius, const cVector3f &avLineStart, const cVector3f &avLineEnd,
											float *apT1, cVector3f* apIntersection1, float *apT2, cVector3f* apIntersection2)
	{
		return hpl_math_check_sphere_line_intersection(ToRs(avSpherePos), afSphereRadius, ToRs(avLineStart), ToRs(avLineEnd),
														apT1, RsPtr(apIntersection1), apT2, RsPtr(apIntersection2));
	}

	bool cMath::CheckPointInFlatPolygon( const cVector3f& avRayStart, const cVector3f& avRayEnd,
										 const cVector2f& avPoint, const cVector2f& avViewportSize,
										 int alNumIndices, unsigned int* avIndices, float* avVertexPos, int alStride,
										 const cMatrixf& amtxObjWorldMatrix,
										 const cMatrixf& amtxCamViewMatrix, const cMatrixf& amtxCamProjMatrix,
										 unsigned int* apIntersectedTriIndex, cVector3f* apIntersectionWorldSpace, tVector3fVec* apTriangle)
	{
		HplVector3f vTriangle[3];
		bool bInside = hpl_math_check_point_in_flat_polygon(ToRs(avRayStart), ToRs(avRayEnd), ToRs(avPoint), ToRs(avViewportSize),
															alNumIndices, avIndices, avVertexPos, alStride,
															RsPtr(amtxObjWorldMatrix), RsPtr(amtxCamViewMatrix), RsPtr(amtxCamProjMatrix),
															apIntersectedTriIndex, RsPtr(apIntersectionWorldSpace), vTriangle);
		if(bInside && apTriangle)
		{
			apTriangle->clear();
			for(int i=0; i<3; ++i) apTriangle->push_back(FromRs(vTriangle[i]));
		}
		return bInside;
	}

	bool cMath::CheckPointInsideTriangle2D(const cVector2f& avPoint, const tVector3fVec& avTriangle)
	{
		return hpl_math_check_point_inside_triangle_2d(ToRs(avPoint), RsPtr(&avTriangle[0]));
	}

	bool cMath::CheckPointSameSideOfEdge2D(const cVector3f& avPoint1, const cVector3f& avPoint2, const cVector3f& avPointA, const cVector3f& avPointB)
	{
		return hpl_math_check_point_same_side_of_edge_2d(ToRs(avPoint1), ToRs(avPoint2), ToRs(avPointA), ToRs(avPointB));
	}

	bool cMath::CheckBVIntersection(cBoundingVolume& aBV1,cBoundingVolume& aBV2)
	{
		aBV1.UpdateSize();
		aBV2.UpdateSize();
		return hpl_math_check_aabb_intersection(ToRs(aBV1.mvWorldMin), ToRs(aBV1.mvWorldMax), ToRs(aBV2.mvWorldMin), ToRs(aBV2.mvWorldMax));
	}

	bool cMath::CheckAABBIntersection(	const cVector3f& avMin1,const cVector3f& avMax1,
										const cVector3f& avMin2,const cVector3f& avMax2)
	{
		return hpl_math_check_aabb_intersection(ToRs(avMin1), ToRs(avMax1), ToRs(avMin2), ToRs(avMax2));
	}

	bool cMath::CheckAABBInside(const cVector3f& avInsideMin,const cVector3f& avInsideMax,
								const cVector3f& avOutsideMin,const cVector3f& avOutsideMax)
	{
		return hpl_math_check_aabb_inside(ToRs(avInsideMin), ToRs(avInsideMax), ToRs(avOutsideMin), ToRs(avOutsideMax));
	}

	bool cMath::CheckPointInBVIntersection(const cVector3f& avPoint, cBoundingVolume& aBV)
	{
		cVector3f vMax = aBV.GetMax();
		cVector3f vMin = aBV.GetMin();
		return hpl_math_check_point_in_aabb_intersection(ToRs(avPoint), ToRs(vMin), ToRs(vMax));
	}

	bool cMath::CheckPointInAABBIntersection(const cVector3f& avPoint, const cVector3f& avMin,const cVector3f& avMax)
	{
		return hpl_math_check_point_in_aabb_intersection(ToRs(avPoint), ToRs(avMin), ToRs(avMax));
	}

	bool cMath::GetNormalizedClipRectFromBV(cVector3f &avDestMin, cVector3f &avDestMax,
											cBoundingVolume &aBV, cFrustum *apFrustum,
											float afTanHalfFov)
	{
		cVector3f vMax = aBV.GetMax();
		cVector3f vMin = aBV.GetMin();
		HplFrustumProjection proj = RsProjection(apFrustum);
		return hpl_math_get_normalized_clip_rect_from_bv(RsPtr(&avDestMin), RsPtr(&avDestMax), ToRs(vMin), ToRs(vMax), &proj, afTanHalfFov);
	}

	cRect2l cMath::GetClipRectFromNormalizedMinMax(const cVector3f &avMin, const cVector3f& avMax, const cVector2l &avScreenSize)
	{
		return FromRs(hpl_math_get_clip_rect_from_normalized_min_max(ToRs(avMin), ToRs(avMax), ToRs(avScreenSize)));
	}

	void cMath::GetClipRectFromBV(	cRect2l &aDestRect, cBoundingVolume &aBV,
									cFrustum *apFrustum, const cVector2l &avScreenSize,
									float afTanHalfFov)
	{
		cVector3f vMax = aBV.GetMax();
		cVector3f vMin = aBV.GetMin();
		HplFrustumProjection proj = RsProjection(apFrustum);
		aDestRect = FromRs(hpl_math_get_clip_rect_from_bv(ToRs(vMin), ToRs(vMax), &proj, ToRs(avScreenSize), afTanHalfFov));
	}

	cRect2l cMath::GetClipRectFromSphere(	const cVector3f& avPosition, float afRadius,
											cFrustum *apFrustum,
											const cVector2l &avScreenSize,
											bool abPositionIsScreenSpace, float afTanHalfFov)
	{
		HplFrustumProjection proj = RsProjection(apFrustum);
		return FromRs(hpl_math_get_clip_rect_from_sphere(ToRs(avPosition), afRadius, &proj, ToRs(avScreenSize), abPositionIsScreenSpace, afTanHalfFov));
	}

	bool cMath::CheckSphereInPlanes(const cVector3f &avCenter, float afRadius, const cPlanef* apPlanes, int alPlaneCount)
	{
		return hpl_math_check_sphere_in_planes(ToRs(avCenter), afRadius, RsPtr(apPlanes), alPlaneCount);
	}

	bool cMath::CheckAABBLineIntersection(	const cVector3f& avMin,const cVector3f& avMax,
											const cVector3f &avLineStart, const cVector3f &avLineEnd,
											cVector3f *apIntersectionPos, float *apT)
	{
		return hpl_math_check_aabb_line_intersection(ToRs(avMin), ToRs(avMax), ToRs(avLineStart), ToRs(avLineEnd), RsPtr(apIntersectionPos), apT);
	}

	//////////////////////////////////////////////////////////////////////////
	// SPHERICAL COORDS
	//////////////////////////////////////////////////////////////////////////

	cVector2f cMath::GetSphericalCoordsFromPoint3D(const cVector3f& avSphCenter, const cVector3f& avPoint)
	{
		return FromRs(hpl_math_get_spherical_coords_from_point_3d(ToRs(avSphCenter), ToRs(avPoint)));
	}

	cVector3f cMath::GetPoint3DFromSphericalCoords(const cVector3f& avSphCenter, float afSphRadius, const cVector2f& avSphCoords)
	{
		return FromRs(hpl_math_get_point_3d_from_spherical_coords(ToRs(avSphCenter), afSphRadius, ToRs(avSphCoords)));
	}

	//////////////////////////////////////////////////////////////////////////
	// FLOAT OPERATIONS
	//////////////////////////////////////////////////////////////////////////

	float cMath::RoundFloatToDecimals(float afVal, int alPrecision) { return hpl_math_round_float_to_decimals(afVal, alPrecision); }
	float cMath::GetFraction(float afVal) { return hpl_math_get_fraction(afVal); }
	float cMath::Modulus(float afDividend, float afDivisor) { return hpl_math_modulus(afDividend, afDivisor); }
	float cMath::ToRad(float afAngle) { return hpl_math_to_rad(afAngle); }
	float cMath::ToDeg(float afAngle) { return hpl_math_to_deg(afAngle); }
	int cMath::Log2ToInt(int alX) { return hpl_math_log2_to_int(alX); }
	int cMath::Pow2(int alX) { return hpl_math_pow2(alX); }
	bool cMath::IsPow2(int alX) { return hpl_math_is_pow2(alX); }
	float cMath::Wrap(float afX, float afMin, float afMax) { return hpl_math_wrap(afX, afMin, afMax); }
	float cMath::Clamp(float afX, float afMin, float afMax) { return hpl_math_clamp(afX, afMin, afMax); }

	float cMath::GetAngleDistance(float afAngle1, float afAngle2, float afMaxAngle) { return hpl_math_get_angle_distance(afAngle1, afAngle2, afMaxAngle); }
	float cMath::GetAngleDistanceRad(float afAngle1, float afAngle2) { return hpl_math_get_angle_distance_rad(afAngle1, afAngle2); }
	float cMath::GetAngleDistanceDeg(float afAngle1, float afAngle2) { return hpl_math_get_angle_distance_deg(afAngle1, afAngle2); }

	float cMath::TurnAngle(float afAngle,float afFinalAngle,float afSpeed,float afMaxAngle) { return hpl_math_turn_angle(afAngle, afFinalAngle, afSpeed, afMaxAngle); }
	float cMath::TurnAngleRad(float afAngle,float afFinalAngle,float afSpeed) { return hpl_math_turn_angle_rad(afAngle, afFinalAngle, afSpeed); }
	float cMath::TurnAngleDeg(float afAngle,float afFinalAngle,float afSpeed) { return hpl_math_turn_angle_deg(afAngle, afFinalAngle, afSpeed); }

	float cMath::Interpolate(float afA,float afB,float afT) { return hpl_math_interpolate(afA, afB, afT); }
	float cMath::IncreaseTo(float afX, float afAdd, float afDest) { return hpl_math_increase_to(afX, afAdd, afDest); }
	float cMath::SmoothCurve(float afX) { return hpl_math_smooth_curve(afX); }

	//////////////////////////////////////////////////////////////////////////
	// VECTOR 2D
	//////////////////////////////////////////////////////////////////////////

	float cMath::Vector2Dist(const cVector2f &avPosA,const cVector2f &avPosB) { return hpl_math_vector2_dist(ToRs(avPosA), ToRs(avPosB)); }
	float cMath::Vector2DistXY(const cVector3f &avPosA,const cVector3f &avPosB) { return hpl_math_vector2_dist_xy(ToRs(avPosA), ToRs(avPosB)); }
	float cMath::Vector2DistXZ(const cVector3f &avPosA,const cVector3f &avPosB) { return hpl_math_vector2_dist_xz(ToRs(avPosA), ToRs(avPosB)); }
	float cMath::Vector2DistSqr(const cVector2f &avPosA,const cVector2f &avPosB) { return hpl_math_vector2_dist_sqr(ToRs(avPosA), ToRs(avPosB)); }
	float cMath::Vector2DistSqrXY(const cVector3f &avPosA,const cVector3f &avPosB) { return hpl_math_vector2_dist_sqr_xy(ToRs(avPosA), ToRs(avPosB)); }
	float cMath::Vector2DistSqrXZ(const cVector3f &avPosA,const cVector3f &avPosB) { return hpl_math_vector2_dist_sqr_xz(ToRs(avPosA), ToRs(avPosB)); }

	cVector2f cMath::Vector2IncreaseTo(const cVector2f& avX, const cVector2f& avAdd, const cVector2f& avDest)
	{
		return FromRs(hpl_math_vector2_increase_to(ToRs(avX), ToRs(avAdd), ToRs(avDest)));
	}

	float cMath::GetAngleFromPoints2D(const cVector2f &avStartPos, const cVector2f &avGoalPos)
	{
		return hpl_math_get_angle_from_points_2d(ToRs(avStartPos), ToRs(avGoalPos));
	}

	cVector2f cMath::GetVectorFromAngle2D(float afAngle, float afLength) { return FromRs(hpl_math_get_vector_from_angle_2d(afAngle, afLength)); }

	void cMath::GetAngleFromVector(const cVector2f &avVec, float *apAngle, float *apLength)
	{
		hpl_math_get_angle_from_vector(ToRs(avVec), apAngle, apLength);
	}

	cVector2f cMath::ProjectVector2D(const cVector2f &avSrcVec, const cVector2f &avDestVec)
	{
		return FromRs(hpl_math_project_vector_2d(ToRs(avSrcVec), ToRs(avDestVec)));
	}

	//////////////////////////////////////////////////////////////////////////
	// VECTOR 3D
	//////////////////////////////////////////////////////////////////////////

	cVector3f cMath::Vector3MaxLength(const cVector3f &avVec, float afMaxLength) { return FromRs(hpl_math_vector3_max_length(ToRs(avVec), afMaxLength)); }
	cVector3f cMath::Vector3MinLength(const cVector3f &avVec, float afMinLength) { return FromRs(hpl_math_vector3_min_length(ToRs(avVec), afMinLength)); }

	cVector3f cMath::Vector3ClampLength(const cVector3f &avVec,float afMinLength, float afMaxLength)
	{
		return FromRs(hpl_math_vector3_clamp_length(ToRs(avVec), afMinLength, afMaxLength));
	}

	cVector3f cMath::Vector3AngleDistance(const cVector3f &avAngles1, const cVector3f &avAngles2, float afMaxAngle)
	{
		return FromRs(hpl_math_vector3_angle_distance(ToRs(avAngles1), ToRs(avAngles2), afMaxAngle));
	}

	cVector3f cMath::Vector3AngleDistanceRad(const cVector3f &avAngles1, const cVector3f &avAngles2)
	{
		return FromRs(hpl_math_vector3_angle_distance_rad(ToRs(avAngles1), ToRs(avAngles2)));
	}

	cVector3f cMath::Vector3AngleDistanceDeg(const cVector3f &avAngles1, const cVector3f &avAngles2)
	{
		return FromRs(hpl_math_vector3_angle_distance_deg(ToRs(avAngles1), ToRs(avAngles2)));
	}

	cVector3f cMath::GetAngleFromPoints3D(const cVector3f &avStartPos, const cVector3f &avGoalPos)
	{
		return FromRs(hpl_math_get_angle_from_points_3d(ToRs(avStartPos), ToRs(avGoalPos)));
	}

	cVector3f cMath::Vector3Cross(const cVector3f& avVecA,const cVector3f& avVecB) { return FromRs(hpl_math_vector3_cross(ToRs(avVecA), ToRs(avVecB))); }
	float cMath::Vector3Dot(const cVector3f& avVecA,const cVector3f& avVecB) { return hpl_math_vector3_dot(ToRs(avVecA), ToRs(avVecB)); }

	cVector3f cMath::Vector3Project(const cVector3f &avSrcVec, const cVector3f &avDestVec)
	{
		return FromRs(hpl_math_vector3_project(ToRs(avSrcVec), ToRs(avDestVec)));
	}

	cVector3f cMath::Vector3Reflect(const cVector3f &avVec, const cVector3f &avNormal)
	{
		return FromRs(hpl_math_vector3_reflect(ToRs(avVec), ToRs(avNormal)));
	}

	float cMath::Vector3Angle(const cVector3f& avVecA,const cVector3f& avVecB) { return hpl_math_vector3_angle(ToRs(avVecA), ToRs(avVecB)); }

	float cMath::Vector3SignedAngle(const cVector3f& avVecA,const cVector3f& avVecB, const cVector3f& avVecN)
	{
		return hpl_math_vector3_signed_angle(ToRs(avVecA), ToRs(avVecB), ToRs(avVecN));
	}

	cVector3f cMath::Vector3UnProject(const cVector3f& avVec,const cRect2f &aScreenRect, cMatrixf a_mtxViewProj)
	{
		return FromRs(hpl_math_vector3_unproject(ToRs(avVec), ToRs(aScreenRect), RsPtr(a_mtxViewProj)));
	}

	cVector3f cMath::Vector3IncreaseTo(const cVector3f& avX, const cVector3f& avAdd, const cVector3f& avDest)
	{
		return FromRs(hpl_math_vector3_increase_to(ToRs(avX), ToRs(avAdd), ToRs(avDest)));
	}

	void cMath::Vector3ClampToLength(cVector3f& avVec, float afMaxLength)
	{
		hpl_math_vector3_clamp_to_length(RsPtr(&avVec), afMaxLength);
	}

	void cMath::Vector3OrthonormalizeBasis(const cVector3f& av1, const cVector3f& av2, const cVector3f& av3,
											cVector3f& avDst1, cVector3f& avDst2, cVector3f& avDst3)
	{
		hpl_math_vector3_orthonormalize_basis(ToRs(av1), ToRs(av2), ToRs(av3), RsPtr(&avDst1), RsPtr(&avDst2), RsPtr(&avDst3));
	}

	cVector3f cMath::Vector3SphereSurfacePoint(const cVector2f& avSeed, float afRadius)
	{
		return FromRs(hpl_math_vector3_sphere_surface_point(ToRs(avSeed), afRadius));
	}

	//////////////////////////////////////////////////////////////////////////
	// PLANES
	//////////////////////////////////////////////////////////////////////////

	float cMath::PlaneDot(const cPlanef& aPlaneA, const cPlanef& aPlaneB) { return hpl_math_plane_dot(ToRs(aPlaneA), ToRs(aPlaneB)); }
	bool cMath::PlaneParallel(const cPlanef& aPlaneA, const cPlanef& aPlaneB) { return hpl_math_plane_parallel(ToRs(aPlaneA), ToRs(aPlaneB)); }

	cPlanef cMath::TransformPlane(const cMatrixf &a_mtxTransform, const cPlanef& aPlane)
	{
		return FromRs(hpl_math_transform_plane(RsPtr(a_mtxTransform), ToRs(aPlane)));
	}

	float cMath::PlaneToPointDist(const cPlanef& aPlane, const cVector3f& avVec) { return hpl_math_plane_to_point_dist(ToRs(aPlane), ToRs(avVec)); }

	void cMath::PlaneIntersectionLine(const cPlanef& aPA, const cPlanef& aPB, cVector3f &avDir, cVector3f &avPoint)
	{
		hpl_math_plane_intersection_line(ToRs(aPA), ToRs(aPB), RsPtr(&avDir), RsPtr(&avPoint));
	}

	bool cMath::PlaneIntersectionPoint(const cPlanef& aP1, const cPlanef& aP2, const cPlanef& aP3, cVector3f& avPoint)
	{
		return hpl_math_plane_intersection_point(ToRs(aP1), ToRs(aP2), ToRs(aP3), RsPtr(&avPoint));
	}

	eCollision cMath::CheckPlaneSphereCollision(const cPlanef& aPlane, const cVector3f &avCenter, float afRadius)
	{
		return (eCollision)hpl_math_check_plane_sphere_collision(ToRs(aPlane), ToRs(avCenter), afRadius);
	}

	eCollision cMath::CheckPlaneBVCollision(const cPlanef& aPlane, cBoundingVolume &aBV)
	{
		return CheckPlaneAABBCollision(aPlane, aBV.GetMin(), aBV.GetMax());
	}

	eCollision cMath::CheckPlaneAABBCollision(const cPlanef& aPlane, const cVector3f &avMin, const cVector3f &avMax)
	{
		return (eCollision)hpl_math_check_plane_aabb_collision(ToRs(aPlane), ToRs(avMin), ToRs(avMax));
	}

	eCollision cMath::CheckPlaneAABBCollision(const cPlanef& aPlane, const cVector3f &avMin, const cVector3f &avMax, const cVector3f &avSphereCenter,
											float afSphereRadius)
	{
		return (eCollision)hpl_math_check_plane_aabb_sphere_collision(ToRs(aPlane), ToRs(avMin), ToRs(avMax), ToRs(avSphereCenter), afSphereRadius);
	}

	bool cMath::CheckPlaneLineIntersection(	const cPlanef& aPlane, const cVector3f &avLineStart, const cVector3f &avLineEnd,
											cVector3f* apIntersectionPos, float *apT)
	{
		return hpl_math_check_plane_line_intersection(ToRs(aPlane), ToRs(avLineStart), ToRs(avLineEnd), RsPtr(apIntersectionPos), apT);
	}

	eCollision cMath::CheckPointsPlanesCollision(cVector3f *apVertices, int alNumOfVertices, cPlanef *apPlanes, int alNumOfPlanes)
	{
		return (eCollision)hpl_math_check_points_planes_collision(RsPtr(apVertices), alNumOfVertices, RsPtr(apPlanes), alNumOfPlanes);
	}

	eCollision cMath::CheckPointsAABBPlanesCollision(cVector3f *apVertices, int alNumOfVertices, const cVector3f& avMin, const cVector3f& avMax)
	{
		return (eCollision)hpl_math_check_points_aabb_planes_collision(RsPtr(apVertices), alNumOfVertices, ToRs(avMin), ToRs(avMax));
	}

	bool cMath::CheckSeparatingAxisIntersection(cVector3f *apVerticesA, int alNumOfVerticesA, cPlanef *apPlanesA, int alNumOfPlanesA,
												cVector3f *apVerticesB, int alNumOfVerticesB, cPlanef *apPlanesB, int alNumOfPlanesB)
	{
		return hpl_math_check_separating_axis_intersection(RsPtr(apVerticesA), alNumOfVerticesA, RsPtr(apPlanesA), alNumOfPlanesA,
															RsPtr(apVerticesB), alNumOfVerticesB, RsPtr(apPlanesB), alNumOfPlanesB);
	}

	//////////////////////////////////////////////////////////////////////////
	// TRIANGLES
	//////////////////////////////////////////////////////////////////////////

	bool cMath::CheckLineTriangleIntersection(	const cVector3f& avLineStart, const cVector3f& avLineEnd,
												const cVector3f& avP0, const cVector3f& avP1, const cVector3f& avP2,
												float *apT, bool abSkipBackfacing)
	{
		return hpl_math_check_line_triangle_intersection(ToRs(avLineStart), ToRs(avLineEnd), ToRs(avP0), ToRs(avP1), ToRs(avP2), apT, abSkipBackfacing);
	}

	bool cMath::CheckLineTriMeshIntersection(	const cVector3f& avLineStart, const cVector3f& avLineEnd,
												const cMatrixf& a_mtxInvMeshMtx,
												const unsigned int* apIndexArray,int alIndexNum,
												const float* apVertexArray, int alVtxStride,
												cVector3f *apIntersectionPos, float *apT, int *apTriIndex, bool abSkipBackfacing)
	{
		return hpl_math_check_line_tri_mesh_intersection(ToRs(avLineStart), ToRs(avLineEnd), RsPtr(a_mtxInvMeshMtx),
														apIndexArray, alIndexNum, apVertexArray, alVtxStride,
														RsPtr(apIntersectionPos), apT, apTriIndex, abSkipBackfacing);
	}

	bool cMath::CheckLineTriVertexBufferIntersection( const cVector3f& avLineStart, const cVector3f& avLineEnd,
														const cMatrixf& a_mtxInvMeshMtx, iVertexBuffer *apVtxBuffer,
														cVector3f *apIntersectionPos, float *apT, int *apTriIndex, bool abSkipBackfacing)
	{
		return CheckLineTriMeshIntersection(avLineStart, avLineEnd,
											a_mtxInvMeshMtx,
											apVtxBuffer->GetIndices(), apVtxBuffer->GetIndexNum(),
											apVtxBuffer->GetFloatArray(eVertexBufferElement_Position),
											apVtxBuffer->GetElementNum(eVertexBufferElement_Position),
											apIntersectionPos, apT, apTriIndex, abSkipBackfacing);
	}

	//////////////////////////////////////////////////////////////////////////
	// QUATERNIONS
	//////////////////////////////////////////////////////////////////////////

	float cMath::QuaternionDot(const cQuaternion& aqA,const cQuaternion& aqB) { return hpl_math_quaternion_dot(ToRs(aqA), ToRs(aqB)); }
	cQuaternion cMath::QuaternionMul(const cQuaternion& aqA,const cQuaternion& aqB) { return FromRs(hpl_math_quaternion_mul(ToRs(aqA), ToRs(aqB))); }

	cQuaternion cMath::QuaternionSlerp(float afT,const cQuaternion& aqA, const cQuaternion& aqB, bool abShortestPath)
	{
		return FromRs(hpl_math_quaternion_slerp(afT, ToRs(aqA), ToRs(aqB), abShortestPath));
	}

	//////////////////////////////////////////////////////////////////////////
	// MATRICES
	//////////////////////////////////////////////////////////////////////////

	cMatrixf cMath::MatrixSlerp(float afT,const cMatrixf& a_mtxA, const cMatrixf& a_mtxB, bool abShortestPath)
	{
		return FromRs(hpl_math_matrix_slerp(afT, RsPtr(a_mtxA), RsPtr(a_mtxB), abShortestPath));
	}

	cMatrixf cMath::MatrixMul(const cMatrixf &a_mtxA,const cMatrixf &a_mtxB) { return FromRs(hpl_math_matrix_mul(RsPtr(a_mtxA), RsPtr(a_mtxB))); }
	cVector3f cMath::MatrixMul(const cMatrixf &a_mtxA,const cVector3f &avB) { return FromRs(hpl_math_matrix_mul_vec3(RsPtr(a_mtxA), ToRs(avB))); }
	cVector3f cMath::MatrixMul3x3(const cMatrixf &a_mtxA,const cVector3f &avB) { return FromRs(hpl_math_matrix_mul_3x3(RsPtr(a_mtxA), ToRs(avB))); }
	cVector3f cMath::MatrixMulDivideW(const cMatrixf &a_mtxA,const cVector3f &avB) { return FromRs(hpl_math_matrix_mul_divide_w(RsPtr(a_mtxA), ToRs(avB))); }
	cMatrixf cMath::MatrixMulScalar(const cMatrixf &a_mtxA, float afB) { return FromRs(hpl_math_matrix_mul_scalar(RsPtr(a_mtxA), afB)); }

	cMatrixf cMath::MatrixRotate(cVector3f avRot, eEulerRotationOrder aOrder) { return FromRs(hpl_math_matrix_rotate(ToRs(avRot), (int32_t)aOrder)); }
	cMatrixf cMath::MatrixRotateX(float afAngle) { return FromRs(hpl_math_matrix_rotate_x(afAngle)); }
	cMatrixf cMath::MatrixRotateY(float afAngle) { return FromRs(hpl_math_matrix_rotate_y(afAngle)); }
	cMatrixf cMath::MatrixRotateZ(float afAngle) { return FromRs(hpl_math_matrix_rotate_z(afAngle)); }
	cMatrixf cMath::MatrixScale(cVector3f avScale) { return FromRs(hpl_math_matrix_scale(ToRs(avScale))); }
	cMatrixf cMath::MatrixTranslate(cVector3f avTrans) { return FromRs(hpl_math_matrix_translate(ToRs(avTrans))); }
	cMatrixf cMath::MatrixQuaternion(const cQuaternion &aqRot) { return FromRs(hpl_math_matrix_quaternion(ToRs(aqRot))); }
	cQuaternion cMath::MatrixToQuaternion(const cMatrixf &a_mtxA) { return FromRs(hpl_math_matrix_to_quaternion(RsPtr(a_mtxA))); }

	float cMath::MatrixMinor(const cMatrixf &a_mtxA,
		const size_t r0, const size_t r1, const size_t r2,
		const size_t c0, const size_t c1, const size_t c2)
	{
		return hpl_math_matrix_minor(RsPtr(a_mtxA), r0, r1, r2, c0, c1, c2);
	}

	cMatrixf cMath::MatrixAdjoint(const cMatrixf &a_mtxA) { return FromRs(hpl_math_matrix_adjoint(RsPtr(a_mtxA))); }
	float cMath::MatrixDeterminant(const cMatrixf &a_mtxA) { return hpl_math_matrix_determinant(RsPtr(a_mtxA)); }
	cMatrixf cMath::MatrixInverse(const cMatrixf &a_mtxA) { return FromRs(hpl_math_matrix_inverse(RsPtr(a_mtxA))); }
	cMatrixf cMath::MatrixPlaneMirror(const cPlanef &aPlane) { return FromRs(hpl_math_matrix_plane_mirror(ToRs(aPlane))); }

	cVector3f cMath::MatrixToEulerAngles(const cMatrixf &a_mtxA, eEulerRotationOrder aOrder)
	{
		return FromRs(hpl_math_matrix_to_euler_angles(RsPtr(a_mtxA), (int32_t)aOrder));
	}

	cVector3f cMath::MatrixEulerAngleDistance(const cMatrixf &a_mtxA, const cMatrixf &a_mtxB)
	{
		return FromRs(hpl_math_matrix_euler_angle_distance(RsPtr(a_mtxA), RsPtr(a_mtxB)));
	}

	cMatrixf cMath::MatrixUnitVectors(const cVector3f& avRight, const cVector3f& avUp, const cVector3f& avForward, const cVector3f& avTranslate)
	{
		return FromRs(hpl_math_matrix_unit_vectors(ToRs(avRight), ToRs(avUp), ToRs(avForward), ToRs(avTranslate)));
	}

	cMatrixf cMath::ProjectionMatrixObliqueNearClipPlane(const cMatrixf &a_mtxProjMatrix, const cPlanef& aClipPlane)
	{
		return FromRs(hpl_math_projection_matrix_oblique_near_clip_plane(RsPtr(a_mtxProjMatrix), ToRs(aClipPlane)));
	}

	cMatrixf cMath::MatrixPerspectiveProjection(float afNearClipPlane, float afFarClipPlane, float afFOV, float afAspect, bool abInfFarPlane)
	{
		return FromRs(hpl_math_matrix_perspective_projection(afNearClipPlane, afFarClipPlane, afFOV, afAspect, abInfFarPlane));
	}

	cMatrixf cMath::MatrixOrthographicProjection(float afNearClipPlane, float afFarClipPlane, const cVector2f& avViewSize)
	{
		return FromRs(hpl_math_matrix_orthographic_projection(afNearClipPlane, afFarClipPlane, ToRs(avViewSize)));
	}

	// Debug text with printf formatting; kept in C++ so the output is identical.
	const char* cMath::MatrixToChar(const cMatrixf &a_mtxA)
	{
		sprintf(mpTempChar,"[%.3f, %.3f, %.3f, %.3f] [%.3f, %.3f, %.3f, %.3f] [%.3f, %.3f, %.3f, %.3f] [%.3f, %.3f, %.3f, %.3f]",
							a_mtxA.m[0][0],a_mtxA.m[0][1],a_mtxA.m[0][2],a_mtxA.m[0][3],
							a_mtxA.m[1][0],a_mtxA.m[1][1],a_mtxA.m[1][2],a_mtxA.m[1][3],
							a_mtxA.m[2][0],a_mtxA.m[2][1],a_mtxA.m[2][2],a_mtxA.m[2][3],
							a_mtxA.m[3][0],a_mtxA.m[3][1],a_mtxA.m[3][2],a_mtxA.m[3][3]);
		return mpTempChar;
	}

	//////////////////////////////////////////////////////////////////////////
	// POLYGONS
	//////////////////////////////////////////////////////////////////////////

	bool cMath::CreateTriTangentVectors(float* apDestArray,
										const unsigned int* apIndexArray,int alIndexNum,
										const float* apVertexArray, int alVtxStride,
										const float *apTexArray,
										const float *apNormalArray,
										int alVertexNum)
	{
		return hpl_math_create_tri_tangent_vectors(apDestArray, apIndexArray, alIndexNum, apVertexArray, alVtxStride,
													apTexArray, apNormalArray, alVertexNum);
	}

	bool cMath::CreateTriangleData(tTriangleDataVec &avTriangles,
									const unsigned int* apIndexArray,int alIndexNum,
									const float* apVertexArray, int alVtxStride, int alVertexNum)
	{
		int lNumOfTri = alIndexNum / 3;
		if((int)avTriangles.size() < lNumOfTri) avTriangles.resize(lNumOfTri);

		HplTriangleData *pTriangles = avTriangles.empty() ? NULL : reinterpret_cast<HplTriangleData*>(&avTriangles[0]);
		return hpl_math_create_triangle_data(pTriangles, apIndexArray, alIndexNum, apVertexArray, alVtxStride, alVertexNum);
	}

	static void AppendEdges(void *apContext, const HplTriEdge *apEdges, size_t alCount)
	{
		tTriEdgeVec *pEdges = static_cast<tTriEdgeVec*>(apContext);
		pEdges->reserve(pEdges->size() + alCount);
		for(size_t i=0; i<alCount; ++i)
		{
			cTriEdge edge(apEdges[i].point1, apEdges[i].point2, apEdges[i].tri1, apEdges[i].tri2);
			edge.invert_tri2 = apEdges[i].invert_tri2;
			pEdges->push_back(edge);
		}
	}

	bool cMath::CreateEdges(tTriEdgeVec &avEdges,
							const unsigned int* apIndexArray,int alIndexNum,
							const float* apVertexArray, int alVtxStride,int alVertexNum,
							bool *apIsDoubleSided)
	{
		return hpl_math_create_edges(apIndexArray, alIndexNum, apVertexArray, alVtxStride, apIsDoubleSided, &avEdges, &AppendEdges);
	}

	//-----------------------------------------------------------------------
}

#endif // HPL2_RUST_MATH
