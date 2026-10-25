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

// Concrete implementation of the real graphics interface: unexpected calls
// abort, while DrawSphere records exactly what VoxelMap sends to the renderer.
#ifndef HPL_GENERATE_DIFF_CAPTURE_GRAPHICS_H
#define HPL_GENERATE_DIFF_CAPTURE_GRAPHICS_H
#include "graphics/LowLevelGraphics.h"
#include <cstdlib>

namespace hpl { class cVoxelMap; }
using namespace hpl;
class CaptureGraphics final : public iLowLevelGraphics {
public:
	int calls = 0;
	int mutation = 0;
	hpl::cVoxelMap* map = NULL;
	cColor* color = NULL;
	bool Init( int alWidth, int alHeight, int alDisplay, int alBpp, int abFullscreen, int alMultisampling, eGpuProgramFormat aGpuProgramFormat, const tString& asWindowCaption, const cVector2l &avWindowPos) override { std::abort(); }
	eGpuProgramFormat GetGpuProgramFormat() override { std::abort(); }
	int GetCaps(eGraphicCaps aType) override { std::abort(); }
	void ShowCursor(bool abX) override { std::abort(); }
	void SetWindowGrab(bool abX) override { std::abort(); }
	void SetRelativeMouse(bool abX) override { std::abort(); }
	void SetWindowCaption(const tString& asName) override { std::abort(); }
	bool GetWindowMouseFocus() override { std::abort(); }
	bool GetWindowInputFocus() override { std::abort(); }
	bool GetWindowIsVisible() override { std::abort(); }
	int GetMultisampling() override { std::abort(); }
	cVector2f GetScreenSizeFloat() override { std::abort(); }
	const cVector2l& GetScreenSizeInt() override { std::abort(); }
	bool GetFullscreenModeActive() override { std::abort(); }
	void SetVsyncActive(bool abX, bool abAdaptive = false) override { std::abort(); }
	void SetMultisamplingActive(bool abX) override { std::abort(); }
	void SetGammaCorrection(float afX) override { std::abort(); }
	float GetGammaCorrection() override { std::abort(); }
	iFontData* CreateFontData(const tString &asName) override { std::abort(); }
	iGpuProgram* CreateGpuProgram(const tString& asName) override { std::abort(); }
	iGpuShader* CreateGpuShader(const tString& asName, eGpuShaderType aType) override { std::abort(); }
	iTexture* CreateTexture(const tString &asName,eTextureType aType, eTextureUsage aUsage) override { std::abort(); }
	iVertexBuffer* CreateVertexBuffer( eVertexBufferType aType, eVertexBufferDrawType aDrawType, eVertexBufferUsageType aUsageType, int alReserveVtxSize=0,int alReserveIdxSize=0) override { std::abort(); }
	iFrameBuffer* CreateFrameBuffer(const tString& asName) override { std::abort(); }
	iDepthStencilBuffer* CreateDepthStencilBuffer(const cVector2l& avSize, int alDepthBits, int alStencilBits) override { std::abort(); }
	iOcclusionQuery* CreateOcclusionQuery() override { std::abort(); }
	void ClearFrameBuffer(tClearFrameBufferFlag aFlags) override { std::abort(); }
	void SetClearColor(const cColor& aCol) override { std::abort(); }
	void SetClearDepth(float afDepth) override { std::abort(); }
	void SetClearStencil(int alVal) override { std::abort(); }
	void FlushRendering() override { std::abort(); }
	void WaitAndFinishRendering() override { std::abort(); }
	void SwapBuffers() override { std::abort(); }
	void CopyFrameBufferToTexure( iTexture* apTex, const cVector2l &avPos, const cVector2l &avSize, const cVector2l &avTexOffset=0) override { std::abort(); }
	cBitmap* CopyFrameBufferToBitmap(const cVector2l &avScreenPos=0, const cVector2l &avScreenSize=-1) override { std::abort(); }
	void SetCurrentFrameBuffer(iFrameBuffer* apFrameBuffer, const cVector2l &avPos = 0, const cVector2l& avSize = -1) override { std::abort(); }
	iFrameBuffer* GetCurrentFrameBuffer() override { std::abort(); }
	void SetFrameBufferDrawTargets(int *apTargets, int alNumOfTargets) override { std::abort(); }
	void SetColorWriteActive(bool abR,bool abG,bool abB,bool abA) override { std::abort(); }
	void SetDepthWriteActive(bool abX) override { std::abort(); }
	void SetCullActive(bool abX) override { std::abort(); }
	void SetCullMode(eCullMode aMode) override { std::abort(); }
	void SetDepthTestActive(bool abX) override { std::abort(); }
	void SetDepthTestFunc(eDepthTestFunc aFunc) override { std::abort(); }
	void SetAlphaTestActive(bool abX) override { std::abort(); }
	void SetAlphaTestFunc(eAlphaTestFunc aFunc,float afRef) override { std::abort(); }
	void SetStencilActive(bool abX) override { std::abort(); }
	void SetStencilWriteMask(unsigned int alMask) override { std::abort(); }
	void SetStencil(eStencilFunc aFunc,int alRef, unsigned int aMask, eStencilOp aFailOp,eStencilOp aZFailOp,eStencilOp aZPassOp) override { std::abort(); }
	void SetStencilTwoSide(eStencilFunc aFrontFunc,eStencilFunc aBackFunc, int alRef, unsigned int aMask, eStencilOp aFrontFailOp,eStencilOp aFrontZFailOp,eStencilOp aFrontZPassOp, eStencilOp aBackFailOp,eStencilOp aBackZFailOp,eStencilOp aBackZPassOp) override { std::abort(); }
	void SetScissorActive(bool abX) override { std::abort(); }
	void SetScissorRect(const cVector2l& avPos, const cVector2l& avSize) override { std::abort(); }
	void SetClipPlane(int alIdx, const cPlanef& aPlane) override { std::abort(); }
	cPlanef GetClipPlane(int alIdx) override { std::abort(); }
	void SetClipPlaneActive(int alIdx, bool abX) override { std::abort(); }
	void SetColor(const cColor &aColor) override { std::abort(); }
	void SetBlendActive(bool abX) override { std::abort(); }
	void SetBlendFunc(eBlendFunc aSrcFactor, eBlendFunc aDestFactor) override { std::abort(); }
	void SetBlendFuncSeparate( eBlendFunc aSrcFactorColor, eBlendFunc aDestFactorColor, eBlendFunc aSrcFactorAlpha, eBlendFunc aDestFactorAlpha) override { std::abort(); }
	void SetPolygonOffsetActive(bool abX) override { std::abort(); }
	void SetPolygonOffset(float afBias, float afSlopeScaleBias) override { std::abort(); }
	void PushMatrix(eMatrix aMtxType) override { std::abort(); }
	void PopMatrix(eMatrix aMtxType) override { std::abort(); }
	void SetIdentityMatrix(eMatrix aMtxType) override { std::abort(); }
	void SetMatrix(eMatrix aMtxType, const cMatrixf& a_mtxA) override { std::abort(); }
	void SetOrthoProjection(const cVector2f& avSize, float afMin, float afMax) override { std::abort(); }
	void SetOrthoProjection(const cVector3f& avMin, const cVector3f& avMax) override { std::abort(); }
	void SetTexture(unsigned int alUnit,iTexture* apTex) override { std::abort(); }
	void SetActiveTextureUnit(unsigned int alUnit) override { std::abort(); }
	void SetTextureEnv(eTextureParam aParam, int alVal) override { std::abort(); }
	void SetTextureConstantColor(const cColor &aColor) override { std::abort(); }
	void DrawTriangle(tVertexVec& avVtx) override { std::abort(); }
	void DrawQuad( const cVector3f &avPos,const cVector2f &avSize, const cColor& aColor=cColor(1,1)) override { std::abort(); }
	void DrawQuad( const cVector3f &avPos,const cVector2f &avSize, const cVector2f &avMinTexCoord,const cVector2f &avMaxTexCoord, const cColor& aColor=cColor(1,1)) override { std::abort(); }
	void DrawQuad( const cVector3f &avPos,const cVector2f &avSize, const cVector2f &avMinTexCoord0,const cVector2f &avMaxTexCoord0, const cVector2f &avMinTexCoord1,const cVector2f &avMaxTexCoord1, const cColor& aColor=cColor(1,1)) override { std::abort(); }
	void DrawQuad(const tVertexVec &avVtx) override { std::abort(); }
	void DrawQuad(const tVertexVec &avVtx, const cColor aCol) override { std::abort(); }
	void DrawQuad(const tVertexVec &avVtx,const float afZ) override { std::abort(); }
	void DrawQuad(const tVertexVec &avVtx,const float afZ,const cColor &aCol) override { std::abort(); }
	void DrawQuadMultiTex(const tVertexVec &avVtx,const tVector3fVec &avExtraUvs) override { std::abort(); }
	void DrawLine(const cVector3f& avBegin, const cVector3f& avEnd, cColor aCol) override { std::abort(); }
	void DrawLine(const cVector3f& avBegin, const cColor& aBeginCol, const cVector3f& avEnd, const cColor& aEndCol) override { std::abort(); }
	void DrawBoxMinMax(const cVector3f& avMin, const cVector3f& avMax, cColor aCol) override { std::abort(); }
	void DrawSphere(const cVector3f& avPos, float afRadius, cColor aCol) override;
	void DrawSphere(const cVector3f& avPos, float afRadius, cColor aColX, cColor aColY, cColor aColZ) override { std::abort(); }
	void DrawLineQuad(const cRect2f& aRect, float afZ, cColor aCol) override { std::abort(); }
	void DrawLineQuad(const cVector3f &avPos,const cVector2f &avSize, cColor aCol) override { std::abort(); }
	void AddVertexToBatch(const cVertex *apVtx) override { std::abort(); }
	void AddVertexToBatch(const cVertex *apVtx, const cVector3f* avTransform) override { std::abort(); }
	void AddVertexToBatch(const cVertex *apVtx, const cMatrixf* aMtx) override { std::abort(); }
	void AddVertexToBatch_Size2D(const cVertex *apVtx, const cVector3f* avTransform, const cColor* apCol,const float& mfW, const float& mfH) override { std::abort(); }
	void AddVertexToBatch_Raw( const cVector3f& avPos, const cColor &aColor, const cVector3f& avTex) override { std::abort(); }
	void AddIndexToBatch(int alIndex) override { std::abort(); }
	void AddTexCoordToBatch(unsigned int alUnit,const cVector3f *apCoord) override { std::abort(); }
	void SetBatchTextureUnitActive(unsigned int alUnit,bool abActive) override { std::abort(); }
	void FlushTriBatch(tVtxBatchFlag aTypeFlags, bool abAutoClear=true) override { std::abort(); }
	void FlushQuadBatch(tVtxBatchFlag aTypeFlags, bool abAutoClear=true) override { std::abort(); }
	void ClearBatch() override { std::abort(); }
};
#endif
