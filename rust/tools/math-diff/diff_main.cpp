// Differential test for the HPL2 math module.
//
// Calls the public C++ API (cMath, cQuaternion, cFrustum, cBoundingVolume,
// cCRC) on many generated inputs and prints every result with exact float
// bits. Built once against the original C++ and once against the Rust port;
// the two outputs must be byte-identical.

#include "math/Math.h"
#include "math/Frustum.h"
#include "math/BoundingVolume.h"
#include "math/CRC.h"

#include <cstdio>
#include <cstring>
#include <cstdint>
#include <cmath>
#include <string>
#include <vector>

using namespace hpl;

//------------------------------------------------------------------------
// Deterministic input generation (independent of rand()).

static uint64_t gState = 0x9E3779B97F4A7C15ull;
static uint32_t Next() { gState ^= gState << 13; gState ^= gState >> 7; gState ^= gState << 17; return (uint32_t)(gState >> 16); }
static float Unit() { return (Next() & 0xFFFFFF) / float(0xFFFFFF); }
static int Int(int lo, int hi) { return lo + (int)(Next() % (uint32_t)(hi - lo + 1)); }

static float F(float range = 10.0f)
{
	static const float special[] = { 0.0f, -0.0f, 1.0f, -1.0f, 0.5f, 1e-9f, -1e-9f, 1e6f, 360.0f, 180.0f, 6.283185307f, 3.141592654f, 0.0001f, 2.0f, 90.0f };
	if(Next() % 8 == 0) return special[Next() % (sizeof(special)/sizeof(special[0]))];
	return (Unit() * 2.0f - 1.0f) * range;
}
static cVector2f V2(float r = 10.0f) { return cVector2f(F(r), F(r)); }
static cVector3f V3(float r = 10.0f) { return cVector3f(F(r), F(r), F(r)); }
static cVector3f Dir() { cVector3f v(F(1), F(1), F(1)); if(v.SqrLength() < 1e-6f) v = cVector3f(0,1,0); v.Normalize(); return v; }
static cColor Col() { return cColor(F(1.2f), F(1.2f), F(1.2f), F(1.2f)); }
static cColor Col01() { return cColor(Unit(), Unit(), Unit(), Unit()); }
static cPlanef Plane() { cVector3f n = Dir(); return cPlanef(n.x, n.y, n.z, F(10)); }
static cRect2f RectF() { return cRect2f(F(), F(), F(), F()); }
static cRect2l RectL() { return cRect2l(Int(-20,20), Int(-20,20), Int(-20,20), Int(-20,20)); }
static cQuaternion Quat() { cQuaternion q(F(1), F(1), F(1), F(1)); if(q.w == 0 && q.v.SqrLength() == 0) q.w = 1; q.Normalize(); return q; }
static cMatrixf RotTrans()
{
	cMatrixf m = cMath::MatrixRotate(V3(4), eEulerRotationOrder_XYZ);
	m.SetTranslation(V3(20));
	return m;
}
static cMatrixf AnyMatrix()
{
	switch(Next() % 4)
	{
	case 0: return RotTrans();
	case 1: return cMath::MatrixMul(RotTrans(), cMath::MatrixScale(cVector3f(0.5f + Unit()*2, 0.5f + Unit()*2, 0.5f + Unit()*2)));
	case 2: return cMath::MatrixPerspectiveProjection(0.05f + Unit(), 10 + Unit()*100, 0.5f + Unit()*1.5f, 0.5f + Unit()*2, Next()%2 == 0);
	default: { cMatrixf m; for(int i=0;i<16;++i) m.v[i] = F(3); return m; }
	}
}

//------------------------------------------------------------------------
// Output with exact bits. NaNs print as "nan" (payloads are not compared).

static FILE* gOut;
static void P(const char* s) { fputs(s, gOut); }
static void P(float f)
{
	if(f != f) { fputs(" nan", gOut); return; }
	uint32_t u; memcpy(&u, &f, 4); fprintf(gOut, " %08x", u);
}
static void P(int i) { fprintf(gOut, " %d", i); }
static void P(unsigned int i) { fprintf(gOut, " %u", i); }
static void P(bool b) { fputs(b ? " T" : " F", gOut); }
static void P(const cVector2f& v) { P(v.x); P(v.y); }
static void P(const cVector2l& v) { P(v.x); P(v.y); }
static void P(const cVector3f& v) { P(v.x); P(v.y); P(v.z); }
static void P(const cColor& c) { P(c.r); P(c.g); P(c.b); P(c.a); }
static void P(const cPlanef& p) { P(p.a); P(p.b); P(p.c); P(p.d); }
static void P(const cRect2f& r) { P(r.x); P(r.y); P(r.w); P(r.h); }
static void P(const cRect2l& r) { P(r.x); P(r.y); P(r.w); P(r.h); }
static void P(const cQuaternion& q) { P(q.w); P(q.v); }
static void P(const cMatrixf& m) { for(int i=0;i<16;++i) P(m.v[i]); }
static void P(const cSpheref& s) { P(s.r); P(s.center); }
static void P(eCollision c) { P((int)c); }
static void P(const tString& s) { fprintf(gOut, " '%s'", s.c_str()); }
static void P(const tWString& s) { fputs(" W'", gOut); for(size_t i=0;i<s.size();++i) fputc((char)s[i], gOut); fputc('\'', gOut); }
static void EndLine() { fputc('\n', gOut); }

#define LINE(name, i) do { fprintf(gOut, "%s %d:", name, (int)(i)); } while(0)

static const int N = 3000;

//------------------------------------------------------------------------

static void TestScalars()
{
	for(int i=0;i<N;++i)
	{
		float a = F(), b = F(), c = F(400);
		int n = Int(-40, 9000);
		LINE("scalar", i);
		P(cMath::RoundFloatToDecimals(a, Int(0,5))); P(cMath::GetFraction(a)); P(cMath::Modulus(a, b));
		P(cMath::ToRad(c)); P(cMath::ToDeg(a)); P(cMath::Log2ToInt(Int(1, 5000))); P(cMath::Pow2(Int(0, 30))); P(cMath::IsPow2(n));
		P(cMath::Wrap(c, 0, 360)); P(cMath::Wrap(a, -3.14f, 3.14f)); P(cMath::Clamp(a, -1, 1));
		P(cMath::GetAngleDistance(c, F(400), 360)); P(cMath::GetAngleDistanceRad(a, b)); P(cMath::GetAngleDistanceDeg(c, F(400)));
		P(cMath::TurnAngle(c, F(400), Unit()*10, 180)); P(cMath::TurnAngleRad(a, b, Unit())); P(cMath::TurnAngleDeg(c, F(400), Unit()*20));
		P(cMath::Interpolate(a, b, Unit())); P(cMath::IncreaseTo(a, Unit(), b)); P(cMath::SmoothCurve(Unit()));
		P(cMath::GetFlagBit(Int(0, 31)));
		EndLine();
	}
}

// Values right at float/double rounding boundaries, where a port that
// evaluates a C++ double expression in float would differ.
static void TestBoundaries()
{
	for(int i=0;i<N;++i)
	{
		float fSpeed = 0.1f + Unit()*10;
		double dEdge = 2.5 * (double)fSpeed;
		float vTargets[5];
		vTargets[2] = (float)dEdge;
		vTargets[1] = nextafterf(vTargets[2], -1e30f); vTargets[0] = nextafterf(vTargets[1], -1e30f);
		vTargets[3] = nextafterf(vTargets[2], 1e30f); vTargets[4] = nextafterf(vTargets[3], 1e30f);
		LINE("boundary", i);
		for(int k=0;k<5;++k)
		{
			P(cMath::TurnAngle(0, vTargets[k], fSpeed, 1000));
			P(cMath::TurnAngle(0, -vTargets[k], fSpeed, 1000));
		}
		float fVal = (float)Int(-1000, 1000) + 0.5f + (float)Int(-2,2) * 1e-4f;
		P(cMath::RoundToInt(fVal)); P(cMath::RoundToInt(nextafterf(fVal, 1e30f))); P(cMath::RoundToInt(nextafterf(fVal, -1e30f)));
		P(cMath::RoundFloatToDecimals(fVal / 1000.0f, Int(0, 7)));
		EndLine();
	}
}

static void TestVectors()
{
	for(int i=0;i<N;++i)
	{
		cVector2f a2 = V2(), b2 = V2();
		cVector3f a = V3(), b = V3(), c = V3();
		LINE("vec2", i);
		P(cMath::Vector2Dist(a2, b2)); P(cMath::Vector2DistXY(a, b)); P(cMath::Vector2DistXZ(a, b));
		P(cMath::Vector2DistSqr(a2, b2)); P(cMath::Vector2DistSqrXY(a, b)); P(cMath::Vector2DistSqrXZ(a, b));
		P(cMath::Vector2IncreaseTo(a2, V2(1), b2)); P(cMath::GetAngleFromPoints2D(a2, b2)); P(cMath::GetVectorFromAngle2D(F(), F()));
		float fAng = 0, fLen = 0; cMath::GetAngleFromVector(a2, &fAng, &fLen); P(fAng); P(fLen);
		P(cMath::ProjectVector2D(a2, b2));
		EndLine();

		LINE("vec3", i);
		P(cMath::Vector3MaxLength(a, Unit()*10)); P(cMath::Vector3MinLength(a, Unit()*10)); P(cMath::Vector3ClampLength(a, Unit()*3, 3 + Unit()*10));
		P(cMath::Vector3AngleDistance(a, b, 360)); P(cMath::Vector3AngleDistanceRad(a, b)); P(cMath::Vector3AngleDistanceDeg(V3(400), V3(400)));
		P(cMath::GetAngleFromPoints3D(a, b)); P(cMath::Vector3Cross(a, b)); P(cMath::Vector3Dot(a, b));
		P(cMath::Vector3Project(a, b)); P(cMath::Vector3Reflect(a, Dir()));
		cVector3f da = Dir(), db = (Next()%5==0) ? da : Dir();
		P(cMath::Vector3Angle(da, db)); P(cMath::Vector3SignedAngle(da, db, Dir()));
		P(cMath::Vector3UnProject(cVector3f(Unit()*800, Unit()*600, Unit()), cRect2f(0,0,800,600), AnyMatrix()));
		cVector3f clamp = a; cMath::Vector3ClampToLength(clamp, F(5)); P(clamp);
		cMath m; P(m.Vector3IncreaseTo(a, V3(1), b));
		cVector3f r, u, f; cMath::Vector3OrthonormalizeBasis(a, b, c, r, u, f); P(r); P(u); P(f);
		P(cMath::Vector3SphereSurfacePoint(cVector2f(Unit(), Unit()), F()));
		P(cMath::GetSphericalCoordsFromPoint3D(a, b)); P(cMath::GetPoint3DFromSphericalCoords(a, F(), V2()));
		EndLine();
	}
}

static void TestColors()
{
	for(int i=0;i<N;++i)
	{
		cColor c = Col();
		LINE("color", i);
		P(cMath::RGBToHSB(c)); P(cMath::RGBToHSB(Col01()));
		P(cMath::HSBToRGB(cVector3f(F(400), Unit(), Unit()))); P(cMath::HSBToRGB(cVector3f((float)(Int(0,6)*60), Unit(), Unit())));
		P(cMath::RGBToHex(c)); P(cMath::RGBAToHex(c)); P(cMath::RGBToHexW(c)); P(cMath::RGBAToHexW(c));
		P(cMath::UCharToHexString((unsigned char)Int(0,255))); P(cMath::UCharToHexStringW((unsigned char)Int(0,255)));

		static const char chars[] = "0123456789abcdefABCDEFxX+- \tzZ\x7f";
		tString s; int len = Int(0, 12);
		for(int k=0;k<len;++k) s += chars[Next() % (sizeof(chars)-1)];
		tWString ws(s.begin(), s.end());
		P(s); P(cMath::HexToRGB(s)); P(cMath::HexToRGBA(s)); P(cMath::HexWToRGB(ws)); P(cMath::HexWToRGBA(ws));
		P((int)cMath::HexStringToUChar(s)); P((int)cMath::HexStringToUChar(ws));
		EndLine();
	}
}

static void TestRandom()
{
	cMath::Randomize(4242);
	for(int i=0;i<N;++i)
	{
		LINE("rand", i);
		P(cMath::RandRectl(-5, 17)); P(cMath::RandRectf(-2, 3)); P(cMath::RandRectVector2f(cVector2f(0), cVector2f(1, 5)));
		P(cMath::RandRectVector3f(cVector3f(-1), cVector3f(1, 2, 3))); P(cMath::RandRectColor(cColor(0,0,0,0), cColor(1,1,1,1)));
		P(cMath::RandomSphereSurfacePoint(2)); P(cMath::RandomSphereInsidePoint(3));
		EndLine();
	}
}

static void TestRectsAndBoxes()
{
	for(int i=0;i<N;++i)
	{
		cVector3f min1 = V3(), max1 = min1 + cVector3f(Unit()*8, Unit()*8, Unit()*8);
		cVector3f min2 = V3(), max2 = min2 + cVector3f(Unit()*8, Unit()*8, Unit()*8);
		LINE("rect", i);
		P(cMath::CheckRectIntersection(RectL(), RectL())); P(cMath::CheckRectIntersection(RectF(), RectF()));
		P(cMath::CheckPointInRectIntersection(V2(), RectF()));
		P(cMath::CheckRectFit(RectL(), RectL())); P(cMath::CheckRectFit(RectF(), RectF())); P(cMath::GetClipRect(RectF(), RectF()));
		cVector3f bmin = min1, bmax = max1; cMath::ExpandAABB(bmin, bmax, min2, max2); P(bmin); P(bmax);
		P(cMath::CheckAABBIntersection(min1, max1, min2, max2)); P(cMath::CheckAABBInside(min1, max1, min2, max2));
		P(cMath::CheckPointInAABBIntersection(V3(), min1, max1));
		P(cMath::CheckPointInSphereIntersection(V3(), V3(), Unit()*5)); P(cMath::CheckSphereIntersection(V3(), Unit()*5, V3(), Unit()*5));

		float t1 = -7, t2 = -7; cVector3f i1(-7), i2(-7);
		bool bHit = cMath::CheckSphereLineIntersection(V3(5), Unit()*6, V3(), V3(), &t1, &i1, &t2, &i2);
		P(bHit); P(t1); P(i1); P(t2); P(i2);

		cVector3f vPos(-7); float fT = -7;
		bHit = cMath::CheckAABBLineIntersection(min1, max1, V3(15), V3(15), &vPos, &fT);
		P(bHit); P(vPos); P(fT);

		cPlanef planes[6]; for(int k=0;k<6;++k) planes[k] = Plane();
		P(cMath::CheckSphereInPlanes(V3(), Unit()*5, planes, Int(0,6)));

		cBoundingVolume bv1, bv2;
		bv1.SetLocalMinMax(min1, max1); bv1.SetTransform(RotTrans());
		bv2.SetLocalMinMax(min2, max2); bv2.SetPosition(V3());
		P(cMath::CheckBVIntersection(bv1, bv2)); P(cMath::CheckPointInBVIntersection(V3(), bv1));
		P(cMath::CheckPlaneBVCollision(Plane(), bv1));
		EndLine();
	}
}

static void TestPlanes()
{
	for(int i=0;i<N;++i)
	{
		cPlanef a = Plane(), b = (Next()%6==0) ? a : Plane(), c = Plane();
		cVector3f min = V3(), max = min + cVector3f(Unit()*6, Unit()*6, Unit()*6);
		LINE("plane", i);
		P(cMath::PlaneDot(a, b)); P(cMath::PlaneParallel(a, b)); P(cMath::TransformPlane(RotTrans(), a));
		P(cMath::PlaneToPointDist(a, V3()));
		cVector3f vDir(-7), vPoint(-7); cMath::PlaneIntersectionLine(a, b, vDir, vPoint); P(vDir); P(vPoint);
		cVector3f vIntPoint(-7); P(cMath::PlaneIntersectionPoint(a, b, c, vIntPoint)); P(vIntPoint);
		P(cMath::CheckPlaneSphereCollision(a, V3(), Unit()*5)); P(cMath::CheckPlaneAABBCollision(a, min, max));
		P(cMath::CheckPlaneAABBCollision(a, min, max, (min+max)*0.5f, Unit()*6));
		cVector3f vLine(-7); float fT = -7; P(cMath::CheckPlaneLineIntersection(a, V3(), V3(), &vLine, &fT)); P(vLine); P(fT);

		cVector3f verts[8]; for(int k=0;k<8;++k) verts[k] = V3();
		cPlanef planes[6]; for(int k=0;k<6;++k) planes[k] = Plane();
		int lVerts = Int(0, 8), lPlanes = Int(0, 6);
		P(cMath::CheckPointsPlanesCollision(verts, lVerts, planes, lPlanes));
		P(cMath::CheckPointsAABBPlanesCollision(verts, lVerts, min, max));
		cVector3f verts2[8]; for(int k=0;k<8;++k) verts2[k] = V3();
		cPlanef planes2[6]; for(int k=0;k<6;++k) planes2[k] = Plane();
		P(cMath::CheckSeparatingAxisIntersection(verts, lVerts, planes, lPlanes, verts2, Int(0,8), planes2, Int(0,6)));
		P(cMath::CheckSeparatingAxisIntersection(verts, 8, NULL, 0, verts2, 8, planes2, 6));
		EndLine();
	}
}

static void TestMatrices()
{
	for(int i=0;i<N;++i)
	{
		cMatrixf a = AnyMatrix(), b = AnyMatrix();
		cMatrixf ra = RotTrans(), rb = RotTrans();
		cVector3f v = V3();
		LINE("matrix", i);
		P(cMath::MatrixMul(a, b)); P(cMath::MatrixMul(a, v)); P(cMath::MatrixMul3x3(a, v)); P(cMath::MatrixMulDivideW(a, v));
		P(cMath::MatrixMulScalar(a, F()));
		P(cMath::MatrixRotate(V3(4), (eEulerRotationOrder)Int(0, 5)));
		P(cMath::MatrixRotateX(F())); P(cMath::MatrixRotateY(F())); P(cMath::MatrixRotateZ(F()));
		P(cMath::MatrixScale(V3())); P(cMath::MatrixTranslate(V3()));
		cQuaternion q = Quat();
		P(cMath::MatrixQuaternion(q)); P(cMath::MatrixToQuaternion(ra)); P(cMath::MatrixToQuaternion(a));
		P(cMath::MatrixInverse(a)); P(cMath::MatrixPlaneMirror(Plane()));
		P(cMath::MatrixToEulerAngles(ra, eEulerRotationOrder_XYZ)); P(cMath::MatrixEulerAngleDistance(ra.GetRotation(), rb.GetRotation()));
		P(cMath::MatrixUnitVectors(V3(), V3(), V3(), V3()));
		P(cMath::MatrixSlerp(F(1.2f), ra, rb, Next()%2 == 0));
		cMatrixf proj = cMath::MatrixPerspectiveProjection(0.1f + Unit(), 50 + Unit()*100, 0.5f + Unit(), 0.7f + Unit(), Next()%2 == 0);
		P(proj); P(cMath::MatrixOrthographicProjection(0.1f + Unit(), 50 + Unit()*100, cVector2f(1 + Unit()*20, 1 + Unit()*20)));
		cPlanef clip = Plane(); if(clip.c > 0) clip = clip * -1.0f;
		P(cMath::ProjectionMatrixObliqueNearClipPlane(proj, clip));
		EndLine();

		LINE("quat", i);
		cQuaternion qa = Quat(), qb = (Next()%6==0) ? qa : Quat();
		P(cMath::QuaternionDot(qa, qb)); P(cMath::QuaternionMul(qa, qb)); P(cMath::QuaternionSlerp(F(1.2f), qa, qb, Next()%2 == 0));
		P(qa + qb); P(qa - qb); P(qa * qb); P(qa / qb); P(qa * F());
		cQuaternion qn(F(), F(), F(), F()); qn.Normalize(); P(qn);
		cQuaternion qaa(F(), Dir()); P(qaa);
		cQuaternion qm(ra); P(qm);
		cMatrixf mq = cMatrixf::Identity; qa.ToRotationMatrix(mq); P(mq);
		EndLine();
	}
}

//------------------------------------------------------------------------

static void MakeMesh(std::vector<float>& avVtx, std::vector<float>& avTex, std::vector<float>& avNrm, std::vector<unsigned int>& avIdx, int alStride)
{
	// A grid with duplicated (welded) vertices, random winding and a few
	// degenerate triangles.
	int w = Int(2, 5), h = Int(2, 5);
	avVtx.clear(); avTex.clear(); avNrm.clear(); avIdx.clear();
	for(int y=0; y<=h; ++y) for(int x=0; x<=w; ++x)
	{
		int copies = (Next()%3==0) ? 2 : 1;
		for(int c=0; c<copies; ++c)
		{
			avVtx.push_back((float)x); avVtx.push_back((float)y); avVtx.push_back(Next()%4==0 ? F(1) : 0.0f);
			for(int s=3; s<alStride; ++s) avVtx.push_back(F());
			avTex.push_back(x / (float)w + (c ? 0.5f : 0)); avTex.push_back(y / (float)h); avTex.push_back(0);
			avNrm.push_back(0); avNrm.push_back(0); avNrm.push_back(c ? -1.0f : 1.0f);
		}
	}
	int lVtxNum = (int)avNrm.size() / 3;
	int lTris = Int(1, 2*w*h);
	for(int t=0; t<lTris; ++t)
		for(int k=0;k<3;++k) avIdx.push_back((unsigned int)Int(0, lVtxNum-1));
}

static void TestMeshes()
{
	for(int i=0;i<N/10;++i)
	{
		int lStride = Int(3, 5);
		std::vector<float> vVtx, vTex, vNrm; std::vector<unsigned int> vIdx;
		MakeMesh(vVtx, vTex, vNrm, vIdx, lStride);
		int lVtxNum = (int)vNrm.size() / 3;

		LINE("mesh", i);
		std::vector<float> vTangents(lVtxNum * 4, -7.0f);
		P(cMath::CreateTriTangentVectors(&vTangents[0], &vIdx[0], (int)vIdx.size(), &vVtx[0], lStride, &vTex[0], &vNrm[0], lVtxNum));
		for(size_t k=0;k<vTangents.size();++k) P(vTangents[k]);

		tTriangleDataVec vTris;
		P(cMath::CreateTriangleData(vTris, &vIdx[0], (int)vIdx.size(), &vVtx[0], lStride, lVtxNum));
		for(size_t k=0;k<vTris.size();++k) P(vTris[k].normal);

		tTriEdgeVec vEdges; bool bDouble = false;
		P(cMath::CreateEdges(vEdges, &vIdx[0], (int)vIdx.size(), &vVtx[0], lStride, lVtxNum, &bDouble)); P(bDouble);
		P((int)vEdges.size());
		for(size_t k=0;k<vEdges.size();++k) { P(vEdges[k].point1); P(vEdges[k].point2); P(vEdges[k].tri1); P(vEdges[k].tri2); P(vEdges[k].invert_tri2); }

		for(int r=0; r<20; ++r)
		{
			cVector3f vStart(F(6), F(6), -5 + F(1)), vEnd(F(6), F(6), 5 + F(1));
			if(Next()%2) std::swap(vStart, vEnd);
			cMatrixf mtx = (Next()%2) ? cMatrixf::Identity : cMath::MatrixInverse(RotTrans());
			cVector3f vPos(-7); float fT = -7; int lTri = -7;
			P(cMath::CheckLineTriMeshIntersection(vStart, vEnd, mtx, &vIdx[0], (int)vIdx.size(), &vVtx[0], lStride, &vPos, &fT, &lTri, Next()%2 == 0));
			P(vPos); P(fT); P(lTri);

			float fTri = -7;
			P(cMath::CheckLineTriangleIntersection(vStart, vEnd, V3(3), V3(3), V3(3), &fTri, Next()%2 == 0)); P(fTri);

			cMatrixf mtxView = cMath::MatrixInverse(cMath::MatrixTranslate(cVector3f(2.0f, 2.0f, 8 + Unit()*4)));
			cMatrixf mtxProj = cMath::MatrixPerspectiveProjection(0.1f, 100, 1.2f, 1.33f, false);
			unsigned int lHitTri = 777; cVector3f vHit(-7); tVector3fVec vTri;
			bool bIn = cMath::CheckPointInFlatPolygon(vStart, vEnd, cVector2f(Unit()*800, Unit()*600), cVector2f(800, 600),
													 (int)vIdx.size(), &vIdx[0], &vVtx[0], lStride,
													 cMatrixf::Identity, mtxView, mtxProj, &lHitTri, &vHit, &vTri);
			P(bIn); P(lHitTri); P(vHit); P((int)vTri.size()); for(size_t k=0;k<vTri.size();++k) P(vTri[k]);
		}
		EndLine();
	}

	for(int i=0;i<N;++i)
	{
		LINE("tri2d", i);
		tVector3fVec tri; tri.push_back(V3(2)); tri.push_back(V3(2)); tri.push_back(V3(2));
		P(cMath::CheckPointInsideTriangle2D(V2(2), tri));
		P(cMath::CheckPointSameSideOfEdge2D(V3(), V3(), V3(), V3()));
		EndLine();
	}
}

//------------------------------------------------------------------------

static void SetupFrustum(cFrustum& f, int alKind)
{
	cMatrixf mtxView = cMath::MatrixInverse(RotTrans());
	cVector3f vOrigin = cMath::MatrixInverse(mtxView).GetTranslation();
	float fNear = 0.05f + Unit(), fFar = 20 + Unit()*100, fFov = 0.6f + Unit()*1.2f, fAspect = 0.75f + Unit();
	switch(alKind)
	{
	case 0:
		f.SetupPerspectiveProj(cMath::MatrixPerspectiveProjection(fNear, fFar, fFov, fAspect, false), mtxView, fFar, fNear, fFov, fAspect, vOrigin, false);
		break;
	case 1:
		f.SetupPerspectiveProj(cMath::MatrixPerspectiveProjection(fNear, fFar, fFov, fAspect, true), mtxView, fFar, fNear, fFov, fAspect, vOrigin, true);
		break;
	case 2: {
		cMatrixf mtxProj = cMath::MatrixPerspectiveProjection(fNear, fFar, fFov, fAspect, false);
		cPlanef clip(F(0.3f), F(0.3f), -1, -(fNear + Unit()*5)); clip.Normalize();
		cMatrixf mtxOblique = cMath::ProjectionMatrixObliqueNearClipPlane(mtxProj, clip);
		f.SetupPerspectiveProj(mtxOblique, mtxView, fFar, fNear, fFov, fAspect, vOrigin, false, &mtxProj, true);
		break; }
	default: {
		cVector2f vSize(2 + Unit()*20, 2 + Unit()*20);
		f.SetupOrthoProj(cMath::MatrixOrthographicProjection(fNear, fFar, vSize), mtxView, fFar, fNear, vSize, vOrigin, false);
		break; }
	}
}

static void PrintFrustum(cFrustum& f)
{
	for(int p=0;p<6;++p) P(f.GetPlane((eFrustumPlane)p));
	for(int v=0;v<8;++v) P(f.GetVertex(v));
	P(f.GetBoundingSphere());
	cBoundingVolume bv = f.GetBoundingVolume(); P(bv.GetMin()); P(bv.GetMax());
}

static void TestFrustums()
{
	for(int i=0;i<N/3;++i)
	{
		cFrustum frustum, other;
		SetupFrustum(frustum, i % 4);
		SetupFrustum(other, Int(0, 3));
		LINE("frustum", i);
		PrintFrustum(frustum);

		for(int k=0;k<10;++k)
		{
			cBoundingVolume bv;
			bv.SetLocalMinMax(V3(2), V3(2) + cVector3f(Unit()*4, Unit()*4, Unit()*4));
			cMatrixf mtx = RotTrans(); mtx.SetTranslation(frustum.GetOrigin() + V3(30)); bv.SetTransform(mtx);

			P(frustum.CollidePoint(frustum.GetOrigin() + V3(30)));
			P(frustum.CollideBoundingVolume(&bv));
			P(frustum.CheckSphereNearPlaneIntersection(frustum.GetOrigin() + V3(2), Unit()*3));
			P(frustum.CheckAABBNearPlaneIntersection(bv.GetMin(), bv.GetMax()));
			P(frustum.CheckBVNearPlaneIntersection(&bv));
			cVector3f vInt(-7);
			P(frustum.CheckLineIntersection(frustum.GetOrigin() + V3(40), frustum.GetOrigin() + V3(40), vInt)); P(vInt);

			cRect2l rect; cMath::GetClipRectFromBV(rect, bv, &frustum, cVector2l(1280, 720), (Next()%2) ? 0.0f : Unit()); P(rect);
			cVector3f vMin(-7), vMax(-7); P(cMath::GetNormalizedClipRectFromBV(vMin, vMax, bv, &frustum, 0)); P(vMin); P(vMax);
			P(cMath::GetClipRectFromNormalizedMinMax(V3(2), V3(2), cVector2l(1280, 720)));
			P(cMath::GetClipRectFromSphere(frustum.GetOrigin() + V3(20), Unit()*10, &frustum, cVector2l(1280, 720), false, 0));
			P(cMath::GetClipRectFromSphere(V3(20), Unit()*10, &frustum, cVector2l(1280, 720), true, Unit()));
		}
		P(frustum.CollideFrustum(&other)); P(other.CollideFrustum(&frustum));
		P(frustum.CheckFrustumNearPlaneIntersection(&other));
		EndLine();
	}
}

static void TestBoundingVolumes()
{
	for(int i=0;i<N;++i)
	{
		cBoundingVolume bv;
		switch(i % 3)
		{
		case 0: bv.SetSize(cVector3f(Unit()*5, Unit()*5, Unit()*5)); bv.SetPosition(V3()); break;
		case 1: bv.SetLocalMinMax(V3(2), V3(2) + cVector3f(Unit()*4, Unit()*4, Unit()*4)); bv.SetTransform(RotTrans()); break;
		default: {
			std::vector<float> pts; int n = Int(1, 20), stride = Int(3, 5);
			for(int k=0;k<n*stride;++k) pts.push_back(F());
			bv.AddArrayPoints(&pts[0], n);
			bv.CreateFromPoints(stride);
			bv.SetTransform(RotTrans());
			break; }
		}
		LINE("bv", i);
		P(bv.GetMin()); P(bv.GetMax()); P(bv.GetSize()); P(bv.GetRadius()); P(bv.GetWorldCenter()); P(bv.GetLocalCenter());

		cShadowVolumeBV* pShadow = bv.GetShadowVolume(V3(15), 5 + Unit()*20, Next()%2 == 0);
		P(pShadow != NULL);
		if(pShadow)
		{
			P(pShadow->mlPlaneCount); P(pShadow->mlCapPlanes);
			for(int p=0;p<pShadow->mlPlaneCount;++p) P(pShadow->mvPlanes[p]);
			P((int)pShadow->mvPoints.size()); for(size_t p=0;p<pShadow->mvPoints.size();++p) P(pShadow->mvPoints[p]);
			for(int k=0;k<5;++k)
			{
				cBoundingVolume other; other.SetSize(cVector3f(Unit()*3)); other.SetPosition(V3(15));
				P(pShadow->CollideBoundingVolume(&other));
			}
		}
		EndLine();
	}
}

static void TestCRC()
{
	for(int i=0;i<200;++i)
	{
		std::vector<char> data(Int(0, 300)); for(size_t k=0;k<data.size();++k) data[k] = (char)Next();
		cCRC crc(0x04C11DB7u ^ (Next() % 3 == 0 ? 0u : (unsigned)i));
		if(!data.empty()) crc.PutData(&data[0], data.size());
		crc.PutByte(Next() & 0xFF);
		LINE("crc", i); P(crc.Done()); P(crc.Done()); EndLine();
	}
}

//------------------------------------------------------------------------

int main(int argc, char** argv)
{
	gOut = argc > 1 ? fopen(argv[1], "w") : stdout;
	TestScalars();
	TestBoundaries();
	TestVectors();
	TestColors();
	TestRandom();
	TestRectsAndBoxes();
	TestPlanes();
	TestMatrices();
	TestMeshes();
	TestFrustums();
	TestBoundingVolumes();
	TestCRC();
	fclose(gOut);
	return 0;
}
