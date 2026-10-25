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

// HPL2_RUST_AI selects the production scheduler. Only public machine APIs and
// ordinary protected state fields are used: no private counter hooks exist.
// Remainders are observed through later updates, state switches and callbacks.
// Nonpositive/nonfinite/stagnant loops have explicit bounded callback repairs
// or throws. Dangling states, double ownership and unregistered fields are not
// inputs. The emergency callback budget is a harness failure, never a detection.
#include "ai/StateMachine.h"
#include "system/LowLevelSystem.h"
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cmath>
#if HARNESS_RUST
#include "system/RustBridge.h"
#endif
using namespace hpl;
extern FILE* gLogOut;
extern bool gThrowWarning;
static FILE* gOut;
static uint64_t gRandom=0x7532EDE1B8A6FC09ull;
static unsigned gMachines=0,gUpdates=0,gCallbacks=0,gTicks=0,gExceptions=0,gNested=0,gBudgetFailures=0;
static int gDepth=0,gBudget=0;
static uint32_t Next() { gRandom^=gRandom<<13; gRandom^=gRandom>>7; gRandom^=gRandom<<17; return (uint32_t)(gRandom>>16); }
static int Int(int lo,int hi) { return lo+(int)(Next()%(uint32_t)(hi-lo+1)); }
static uint32_t Bits(float value) { uint32_t n; memcpy(&n,&value,4); return n; }
static float Float(uint32_t n) { float value; memcpy(&value,&n,4); return value; }
class cFixtureState;
static int Current(cStateMachine& machine);
static void Invoke(cStateMachine& machine,float time);
struct cAction {
    int occurrence,kind,argument; float value; cFixtureState* target;
    cAction() : occurrence(0),kind(0),argument(0),value(0),target(NULL) {}
};
class cFixtureState : public iAIState {
public:
    int label,ticks,enters,leaves;
    cAction actions[3];
    explicit cFixtureState(int index) : label(index),ticks(0),enters(0),leaves(0) {}
    ~cFixtureState() override { fprintf(gOut,"destroy %d id=%d ticks=%d\n",label,GetId(),ticks); }
    void Step(float step) { mfUpdateStep=step; fprintf(gOut,"step %d %08x\n",label,Bits(step)); }
    void Arm(int kind,float value=0,int argument=0,cFixtureState* target=NULL,int occurrence=1,int phase=0) {
        cAction& a=actions[phase]; a.kind=kind; a.value=value; a.argument=argument; a.target=target; a.occurrence=occurrence;
        fprintf(gOut,"arm %d phase=%d kind=%d at=%d value=%08x arg=%d target=%d\n",label,phase,kind,occurrence,Bits(value),argument,target ? target->label:-1);
    }
    void OnUpdate(float time) override { ++ticks; ++gTicks; Callback(0,time,0); }
    void OnEnterState(int previous) override { ++enters; Callback(1,0,previous); }
    void OnLeaveState(int next) override { ++leaves; Callback(2,0,next); }
    void Callback(int phase,float time,int argument) {
        if(gDepth && --gBudget<0) { ++gBudgetFailures; throw 99999; }
        ++gCallbacks;
        fprintf(gOut,"callback depth=%d state=%d phase=%d time=%08x arg=%d step=%08x current=%d active=%d ticks=%d\n",
            gDepth,label,phase,Bits(time),argument,Bits(mfUpdateStep),Current(*mpStateMachine),mpStateMachine->IsActive(),ticks);
        cAction& slot=actions[phase];
        if(slot.occurrence<=0 || --slot.occurrence!=0) return;
        cAction a=slot; // Consume before reentry.
        fprintf(gOut,"action %d state=%d\n",a.kind,label);
        switch(a.kind) {
        case 1: Step(a.value); break;
        case 2: throw a.argument;
        case 3: mpStateMachine->ChangeState(a.argument); break;
        case 4: mpStateMachine->SetActive(a.argument!=0); break;
        case 5: ++gNested; Invoke(*mpStateMachine,a.value); break;
        case 6: Step(a.value); ++gNested; Invoke(*mpStateMachine,0); break;
        case 7:
            // Nested invocation throws after modifying the same live counter.
            Arm(2,0,a.argument); ++gNested;
            try { Invoke(*mpStateMachine,a.value); }
            catch(int code) { ++gExceptions; fprintf(gOut,"caught-inner %d state=%d\n",code,label); }
            break;
        case 8: Arm(2,0,a.argument); ++gNested; Invoke(*mpStateMachine,a.value); break;
        case 9: Step(a.value); throw a.argument;
        case 10: mpStateMachine->ChangeState(a.argument); ++gNested; Invoke(*mpStateMachine,a.value); break;
        case 11: Sleep(a.value); break;
        case 12: mpStateMachine->AddState(this,GetName(),GetId(),a.value); break;
        case 13: a.target->Step(a.value); break;
        case 14:
            Step(a.value); mpStateMachine->SetActive(false); ++gNested; Invoke(*mpStateMachine,1.0f); break;
        case 15:
            gThrowWarning=true;
            try { mpStateMachine->ChangeState(123456); }
            catch(int code) { ++gExceptions; fprintf(gOut,"caught-warning %d\n",code); }
            break;
        case 16:
            // Execute two zero/nonprogressing ticks before restoring consumption.
            Arm(1,a.value,0,NULL,1); break;
        case 17: Step(-0.25f); Arm(1,a.value); break;
        default: std::abort();
        }
    }
};
static int Current(cStateMachine& machine) { return machine.CurrentState() ? static_cast<cFixtureState*>(machine.CurrentState())->label:-1; }
static cFixtureState* Add(cStateMachine& machine,int label,float step) {
    cFixtureState* state=new cFixtureState(label); char name[24]; snprintf(name,sizeof(name),"state_%d",label);
    machine.AddState(state,name,label,step);
    fprintf(gOut,"add %d step=%08x current=%d\n",label,Bits(step),Current(machine)); return state;
}
static void Invoke(cStateMachine& machine,float time) {
    if(!gDepth) gBudget=4096;
    ++gDepth; ++gUpdates;
    fprintf(gOut,"update depth=%d time=%08x current=%d active=%d\n",gDepth,Bits(time),Current(machine),machine.IsActive());
    try { machine.Update(time); }
    catch(...) { --gDepth; throw; }
    fprintf(gOut,"return depth=%d current=%d active=%d\n",gDepth,Current(machine),machine.IsActive()); --gDepth;
}
static void Update(cStateMachine& machine,float time) {
    try { Invoke(machine,time); }
    catch(int code) { ++gExceptions; fprintf(gOut,"exception %d current=%d active=%d\n",code,Current(machine),machine.IsActive()); }
}
static void Change(cStateMachine& machine,int id) {
    fprintf(gOut,"change %d current=%d active=%d\n",id,Current(machine),machine.IsActive());
    try { machine.ChangeState(id); fprintf(gOut,"change-return current=%d\n",Current(machine)); }
    catch(int code) { ++gExceptions; fprintf(gOut,"exception-change %d current=%d\n",code,Current(machine)); }
}
static void Dump(cStateMachine& machine,cFixtureState** states,int count) {
    fprintf(gOut,"machine current=%d active=%d\n",Current(machine),machine.IsActive());
    for(int i=0;i<count;++i) fprintf(gOut,"state %d step=%08x callbacks=%d,%d,%d\n",states[i]->label,Bits(states[i]->GetUpdateStep()),states[i]->ticks,states[i]->enters,states[i]->leaves);
}
static void Begin(const char* name,int number) { ++gMachines; fprintf(gOut,"begin %s %d\n",name,number); }
static void Probe(cStateMachine& m,cFixtureState** states,int count) {
    // Public follow-up pulses expose each private remainder. States are never
    // reset or recreated; changing step and reentering preserve that remainder.
    m.SetActive(true);
    for(int i=0;i<count;++i) {
        states[i]->Step(0.25f); Change(m,i);
        Update(m,0); Update(m,Float(0x3e7fffff)); Update(m,Float(0x33000000)); Update(m,0.125f);
    }
    Dump(m,states,count);
}
static void Boundaries() {
    const uint32_t steps[]={0x3e800000,0x3dcccccd,0x3c888889,0x3eaaaaab,0x3f000001,0x3d000001};
    for(unsigned i=0;i<sizeof(steps)/sizeof(*steps);++i) for(int side=-1;side<=1;++side) {
        Begin("boundary",(int)i*3+side+1); cStateMachine m; cFixtureState* s[]={Add(m,0,Float(steps[i]))};
        Update(m,Float(steps[i]+side)); Update(m,0); Update(m,Float(steps[i])); Update(m,Float(steps[i]-1));
        Update(m,Float(steps[i]+1)); Update(m,-Float(steps[i])); Update(m,Float(steps[i])); Probe(m,s,1);
    }
    for(int scenario=0;scenario<13;++scenario) {
        Begin("fixed",scenario); cStateMachine m; cFixtureState* s[]={Add(m,0,0.25f),Add(m,1,0.5f)};
        switch(scenario) {
        case 0: Update(m,0.75f); Update(m,0); break;
        case 1: Update(m,0.125f); Update(m,0.125f); Update(m,0.125f); break;
        case 2: Update(m,-0.5f); Update(m,0.25f); Update(m,0.5f); break;
        case 3: Update(m,-0.0f); Update(m,0.0f); break;
        case 4: Update(m,0.1f); Update(m,0.2f); Update(m,0.3f); break;
        case 5: s[0]->Step(1); Update(m,Float(0x3f7fffff)); Update(m,Float(0x33000000)); Update(m,0); break;
        case 6: s[0]->Step(1); Update(m,-16777216.0f); Update(m,0.25f); Update(m,16777216.0f); Update(m,1); break;
        case 7: s[0]->Step(1); Update(m,-16777216.0f); Update(m,1.0f); Update(m,16777216.0f); break;
        case 8: Update(m,0.125f); Change(m,1); Update(m,0.5f); Change(m,0); Update(m,0.125f); break;
        case 9: Update(m,0.125f); m.SetActive(false); Update(m,1); m.SetActive(true); Update(m,0.125f); break;
        case 10: Update(m,0.125f); m.AddState(s[0],s[0]->GetName(),0,0.5f); Update(m,0.375f); break;
        case 11: s[0]->Sleep(10); Update(m,0.5f); break;
        case 12: s[0]->Step(0.1f); Update(m,0.5f); Update(m,0); break;
        }
        Probe(m,s,2);
    }
}
static void Reentry() {
    for(int scenario=0;scenario<23;++scenario) {
        Begin("reentry",scenario); cStateMachine m; cFixtureState* s[]={Add(m,0,0.25f),Add(m,1,0.5f)};
        switch(scenario) {
        case 0: s[0]->Arm(1,0.5f); break;
        case 1: s[0]->Arm(1,0.125f); break;
        case 2: s[0]->Arm(17,0.5f); break;
        case 3: s[0]->Arm(3,0,1); break;
        case 4: s[0]->Arm(4,0,0); break;
        case 5: s[0]->Arm(5,0); break;
        case 6: s[0]->Arm(5,0.125f); break;
        case 7: s[0]->Arm(5,-0.5f); break;
        case 8: s[0]->Arm(6,0.5f); break;
        case 9: s[0]->Arm(7,0.25f,401); break;
        case 10: s[0]->Arm(8,0.125f,402); break;
        case 11: s[0]->Arm(2,0,403); break;
        case 12: s[0]->Arm(9,0.5f,404); break;
        case 13: s[0]->Arm(10,0.75f,1); break;
        case 14: s[0]->Arm(11,100); break;
        case 15: s[0]->Arm(12,0.5f); break;
        case 16: s[0]->Arm(14,0.5f); break;
        case 17: s[0]->Arm(15); break;
        case 18: s[0]->Arm(3,0,1); s[0]->Arm(5,0.125f,0,NULL,1,2); break;
        case 19: s[0]->Arm(3,0,1); s[1]->Arm(5,0.125f,0,NULL,1,1); break;
        case 20: s[0]->Arm(3,0,1); s[1]->Arm(13,0.75f,0,s[0],1,1); break;
        case 21: s[0]->Arm(3,0,1); s[0]->Arm(9,0.5f,405,NULL,1,2); break;
        case 22: s[0]->Arm(3,0,1); s[1]->Arm(2,0,406,NULL,1,1); break;
        }
        Update(m,0.75f); Dump(m,s,2); Update(m,0); Probe(m,s,2);
    }
}
static void Special() {
    // A NaN private counter cannot have its payload observed through this public
    // API; these cases verify its persistent suppression of all later callbacks.
    const uint32_t nans[]={0x7fc12345,0xffc67890,0x7f812345,0xff867890};
    for(int i=0;i<4;++i) for(int j=0;j<4;++j) {
        Begin("nan-counter",i*4+j); cStateMachine m; cFixtureState* s[]={Add(m,0,Float(nans[(i+1)%4]))};
        Update(m,Float(nans[i])); Update(m,Float(nans[j])); s[0]->Step(0.25f); Update(m,1); Probe(m,s,1);
    }
    const uint32_t steps[]={0,0x80000000,0xbe800000,0xff800000,1,0x80000001,0x7f800000,0x7fc12345};
    for(int i=0;i<8;++i) {
        Begin("special-step",i); cStateMachine m; cFixtureState* s[]={Add(m,0,Float(steps[i]))};
        s[0]->Arm(1,i==6 ? Float(0x7f800000):1.0f); Update(m,i==6 ? Float(0x7f800000):1.0f); Probe(m,s,1);
    }
    for(int scenario=0;scenario<8;++scenario) {
        Begin("special-callback",scenario); cStateMachine m; cFixtureState* s[]={Add(m,0,0.25f)};
        if(scenario==0) { s[0]->Step(0); s[0]->Arm(16,1); Update(m,1); }
        if(scenario==1) { s[0]->Step(-0.25f); s[0]->Arm(16,1); Update(m,0.25f); }
        if(scenario==2) { s[0]->Arm(1,Float(0x7fc12345)); Update(m,0.5f); }
        if(scenario==3) { s[0]->Arm(1,Float(0x7f800000)); Update(m,0.5f); }
        if(scenario==4) { s[0]->Arm(9,Float(0x7fc12345),407); Update(m,0.5f); s[0]->Step(0.25f); Update(m,0); }
        if(scenario==5) { s[0]->Step(Float(1)); s[0]->Arm(1,Float(0x7f800000)); Update(m,Float(0x7f7fffff)); }
        if(scenario==6) { Update(m,Float(0xff800000)); Update(m,Float(0x7f800000)); }
        if(scenario==7) { s[0]->Step(Float(0xff800000)); s[0]->Arm(1,1); Update(m,Float(0xff800000)); }
        Probe(m,s,1);
    }
    Begin("empty",0); { cStateMachine m; Update(m,1); m.SetActive(false); Update(m,Float(0x7fc12345)); }
}
static void Random() {
    for(int trial=0;trial<4096;++trial) {
        Begin("seeded",trial); cStateMachine m;
        cFixtureState* s[]={Add(m,0,0.125f),Add(m,1,0.25f),Add(m,2,0.5f)};
        for(int operation=0;operation<20;++operation) {
            int choice=Int(0,9); fprintf(gOut,"operation %d %d\n",operation,choice);
            if(choice<=4) {
                float time=Float(0x3c000000+(Next()%0x03000000)); if(Int(0,5)==0) time=-time;
                Update(m,time);
            } else if(choice==5) { Change(m,Int(0,2)); }
            else if(choice==6) { int index=Int(0,2); s[index]->Step(Float(0x3d000000+(Next()%0x02000000))); }
            else if(choice==7) { m.SetActive(Int(0,1)!=0); }
            else {
                int index=Current(m); int action=Int(0,6);
                if(action==0) s[index]->Arm(1,Float(0x3d000000+(Next()%0x02000000)));
                else if(action==1) s[index]->Arm(2,0,500);
                else if(action==2) s[index]->Arm(5,(float)Int(-2,3)*0.0625f);
                else if(action==3) s[index]->Arm(3,0,Int(0,2));
                else if(action==4) s[index]->Arm(4,0,0);
                else if(action==5) s[index]->Arm(7,0.125f,501);
                else s[index]->Arm(12,0.25f);
            }
            Dump(m,s,3);
        }
        Probe(m,s,3);
    }
}
int main(int argc,char** argv) {
    if(argc!=2) return 2;
    gOut=fopen(argv[1],"wb"); if(!gOut) return 3; gLogOut=gOut;
#if HARNESS_RUST
    RustBridge_Init();
#endif
    Boundaries(); Reentry(); Special(); Random();
    fprintf(gOut,"totals machines=%u updates=%u callbacks=%u ticks=%u exceptions=%u nested=%u budget_failures=%u seeded=4096\n",
        gMachines,gUpdates,gCallbacks,gTicks,gExceptions,gNested,gBudgetFailures);
    fclose(gOut); gLogOut=stdout; return gBudgetFailures ? 4:0;
}
