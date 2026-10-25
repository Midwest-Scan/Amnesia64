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

// HPL2_RUST_AI selects native or Rust transition/update dispatch decisions.
// StateMachine.cpp is real production code. Concrete iAIState subclasses supply
// virtual callbacks and access only their ordinary protected ID/step members.
// Null-current ChangeState is original UB and is excluded. Callback deletion of
// live states, double-owned registrations and unbounded timer loops are excluded.
// Nonpositive/nonfinite timer steps are used only where the first callback fixes
// them or the original comparison terminates; the native scheduler is unchanged.
#include "ai/StateMachine.h"
#include "system/LowLevelSystem.h"
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <limits>
#if HARNESS_RUST
#include "system/RustBridge.h"
#endif

using namespace hpl;
extern FILE* gLogOut;
extern bool gThrowWarning;
static FILE* gOut;
static uint64_t gRandom=0x51E729D03BA48C6Full;
static unsigned gChanges, gUpdates, gCallbacks, gExceptions, gMachines;
static uint32_t Next() { gRandom^=gRandom<<13; gRandom^=gRandom>>7; gRandom^=gRandom<<17; return (uint32_t)(gRandom>>16); }
static int Int(int lo,int hi) { return lo+(int)(Next()%(uint32_t)(hi-lo+1)); }
static uint32_t Bits(float value) { uint32_t bits; memcpy(&bits,&value,4); return bits; }
static float Float(uint32_t bits) { float value; memcpy(&value,&bits,4); return value; }

class cFixtureState;
static int CurrentLabel(cStateMachine& machine);
static void InvokeChange(cStateMachine& machine,int id);
static void InvokeUpdate(cStateMachine& machine,float time);

struct cHook {
    int remaining, action, argument;
    float value;
    cFixtureState* target;
    cHook() : remaining(0), action(0), argument(0), value(0), target(NULL) {}
};
class cFixtureState : public iAIState
{
public:
    int label;
    int calls[3];
    cHook hooks[3];
    explicit cFixtureState(int index) : label(index) { calls[0]=calls[1]=calls[2]=0; }
    ~cFixtureState() override { fprintf(gOut,"destroy label=%d id=%d name=%s\n",label,GetId(),GetName().c_str()); }
    void SetId(int id) { mlId=id; }
    void SetStep(float step) { mfUpdateStep=step; }
    void Arm(int phase,int action,int argument=0,float value=0,cFixtureState* target=NULL,int occurrence=1)
    {
        hooks[phase].remaining=occurrence; hooks[phase].action=action; hooks[phase].argument=argument;
        hooks[phase].value=value; hooks[phase].target=target;
        fprintf(gOut,"arm label=%d phase=%d occurrence=%d action=%d argument=%d value=%08x target=%d\n",
            label,phase,occurrence,action,argument,Bits(value),target ? target->label : -1);
    }
    void OnUpdate(float time) override { Callback(0,0,time); }
    void OnEnterState(int previous) override { Callback(1,previous,0); }
    void OnLeaveState(int next) override { Callback(2,next,0); }
    void Callback(int phase,int argument,float time)
    {
        ++gCallbacks; ++calls[phase];
        fprintf(gOut,"callback label=%d id=%d phase=%d argument=%d time=%08x step=%08x current=%d active=%d call=%d\n",
            label,GetId(),phase,argument,Bits(time),Bits(GetUpdateStep()),CurrentLabel(*mpStateMachine),mpStateMachine->IsActive(),calls[phase]);
        cHook& slot=hooks[phase];
        if(slot.remaining<=0 || --slot.remaining!=0) return;
        cHook hook=slot; // Consume before reentry; no callback can execute this action twice.
        fprintf(gOut,"action %d label=%d phase=%d\n",hook.action,label,phase);
        switch(hook.action) {
        case 1: SetStep(hook.value); break;
        case 2: SetId(hook.argument); break;
        case 3: hook.target->SetId(hook.argument); break;
        case 4: InvokeChange(*mpStateMachine,hook.argument); break;
        case 5: mpStateMachine->SetActive(hook.argument!=0); break;
        case 6: InvokeUpdate(*mpStateMachine,hook.value); break;
        case 7: throw hook.argument;
        case 8: InvokeChange(*mpStateMachine,123456789); break;
        case 9: Sleep(hook.value); break;
        case 10: InvokeChange(*mpStateMachine,hook.argument); throw 707;
        case 11:
            // A caught nested warning failure must not remain in the outer
            // invocation's exception slot and prevent its eventual assignment.
            gThrowWarning=true;
            try { InvokeChange(*mpStateMachine,123456789); }
            catch(int value) { ++gExceptions; fprintf(gOut,"caught-inner %d current=%d\n",value,CurrentLabel(*mpStateMachine)); }
            break;
        case 12: hook.target->SetStep(hook.value); break;
        default: std::abort();
        }
    }
};

static int CurrentLabel(cStateMachine& machine)
{
    iAIState* state=machine.CurrentState();
    return state ? static_cast<cFixtureState*>(state)->label : -1;
}
static void Dump(cStateMachine& machine,cFixtureState** states,int count)
{
    fprintf(gOut,"machine current=%d id=%d active=%d\n",CurrentLabel(machine),
        machine.CurrentState() ? machine.CurrentState()->GetId() : -999,machine.IsActive());
    for(int i=0;i<count;++i) {
        cFixtureState* s=states[i];
        fprintf(gOut,"state label=%d id=%d step=%08x calls=%d,%d,%d\n",s->label,s->GetId(),Bits(s->GetUpdateStep()),s->calls[0],s->calls[1],s->calls[2]);
    }
}
static cFixtureState* Add(cStateMachine& machine,int label,int key,float step)
{
    cFixtureState* state=new cFixtureState(label);
    char name[40]; snprintf(name,sizeof(name),"state_%d",label);
    machine.AddState(state,name,key,step);
    fprintf(gOut,"add label=%d key=%d step=%08x current=%d\n",label,key,Bits(step),CurrentLabel(machine));
    return state;
}
static void InvokeChange(cStateMachine& machine,int id)
{
    ++gChanges; fprintf(gOut,"change %d from=%d active=%d\n",id,CurrentLabel(machine),machine.IsActive());
    machine.ChangeState(id);
    fprintf(gOut,"change-return current=%d id=%d\n",CurrentLabel(machine),machine.CurrentState()->GetId());
}
static void InvokeUpdate(cStateMachine& machine,float time)
{
    ++gUpdates; fprintf(gOut,"update %08x current=%d active=%d\n",Bits(time),CurrentLabel(machine),machine.IsActive());
    machine.Update(time);
    fprintf(gOut,"update-return current=%d\n",CurrentLabel(machine));
}
static void Change(cStateMachine& machine,int id)
{
    try { InvokeChange(machine,id); }
    catch(int value) { ++gExceptions; fprintf(gOut,"exception-change %d current=%d\n",value,CurrentLabel(machine)); }
}
static void Update(cStateMachine& machine,float time)
{
    try { InvokeUpdate(machine,time); }
    catch(int value) { ++gExceptions; fprintf(gOut,"exception-update %d current=%d\n",value,CurrentLabel(machine)); }
}
static void Begin(const char* section,int sequence) { ++gMachines; fprintf(gOut,"begin %s %d\n",section,sequence); }

static void Basic()
{
    Begin("empty",0);
    { cStateMachine empty; Update(empty,1); empty.SetActive(false); Update(empty,Float(0x7fc12345)); }
    Begin("basic",0);
    {
        cStateMachine m;
        cFixtureState* s[]={Add(m,0,-4,0.25f),Add(m,1,0,0.5f),Add(m,2,9,0.125f)};
        Dump(m,s,3); Change(m,-4); Change(m,88); Change(m,0); Update(m,0.75f);
        m.SetActive(false); Update(m,1); Change(m,9); Update(m,1); m.SetActive(true); Update(m,0.25f);
        Change(m,INT32_MIN); Change(m,INT32_MAX); Change(m,-4); Update(m,0.5f); Dump(m,s,3);
        gThrowWarning=true; Change(m,22); Dump(m,s,3); Change(m,0); Dump(m,s,3);
    }
    Begin("map-keys-vs-live-ids",0);
    {
        cStateMachine m;
        cFixtureState* s[]={Add(m,0,0,0.25f),Add(m,1,1,0.25f),Add(m,2,2,0.25f)};
        // Registration keys do not track protected member ID edits. Equal live
        // IDs can suppress a transition even when the requested map entry differs.
        s[0]->SetId(1); Change(m,1); Change(m,0); Change(m,2);
        s[1]->SetId(-10); Change(m,1); Change(m,-10); Change(m,1); Dump(m,s,3);
    }
    Begin("duplicate-registration",0);
    {
        cStateMachine m;
        cFixtureState* owned=Add(m,0,7,0.25f);
        cFixtureState* orphan=Add(m,1,7,0.5f);
        // Failed duplicate insertion leaves this object caller-owned. Do not
        // register one pointer under two map keys (that would double-delete).
        fprintf(gOut,"lookup duplicate=%d\n",m.GetState(7)==owned);
        Change(m,7); Update(m,0.5f); delete orphan;
    }
}

static void Reentry()
{
    for(int scenario=0;scenario<19;++scenario) {
        Begin("callbacks",scenario);
        cStateMachine m;
        cFixtureState* s[]={Add(m,0,0,0.25f),Add(m,1,1,0.5f),Add(m,2,2,0.125f)};
        switch(scenario) {
        case 0: s[0]->Arm(2,4,2); break; // Leave commits another current state.
        case 1: s[1]->Arm(1,4,2); break; // Enter sees the still-current old state.
        case 2: s[0]->Arm(2,2,77); break; // Enter rereads previous state's live ID.
        case 3: s[0]->Arm(2,3,-77,0,s[1]); break; // Destination ID changes after leave's argument.
        case 4: s[0]->Arm(2,7,400); break;
        case 5: s[1]->Arm(1,7,500); break;
        case 6: s[0]->Arm(2,10,2); break;
        case 7: s[1]->Arm(1,10,2); break;
        case 8: s[0]->Arm(2,11); break;
        case 9: s[1]->Arm(1,11); break;
        case 10: s[0]->Arm(2,5,0); break;
        case 11: s[1]->Arm(1,5,0); break;
        case 12: s[0]->Arm(2,6,0,0.5f); break;
        case 13: s[1]->Arm(1,6,0,0.5f); break;
        case 14: s[0]->Arm(2,4,2); s[2]->Arm(1,2,-1); break;
        case 15: s[0]->Arm(2,4,2); s[2]->Arm(1,3,0,0,s[1]); break;
        case 16: s[0]->Arm(2,8); break;
        case 17: s[1]->Arm(1,8); break;
        case 18: s[0]->Arm(2,9,0,Float(0x7fc12345)); break;
        }
        Change(m,1); Dump(m,s,3); Change(m,2); Update(m,0.5f); Dump(m,s,3);
    }
    // Reentrant Update executes the selected native state's scheduler. State
    // changes/activity changes inside it do not move or cancel that invocation.
    for(int scenario=0;scenario<10;++scenario) {
        Begin("update-callbacks",scenario);
        cStateMachine m;
        cFixtureState* s[]={Add(m,0,0,0.25f),Add(m,1,1,0.5f),Add(m,2,2,0.125f)};
        switch(scenario) {
        case 0: s[0]->Arm(0,4,1); break;
        case 1: s[0]->Arm(0,5,0); break;
        case 2: s[0]->Arm(0,1,0,0.5f); break;
        case 3: s[0]->Arm(0,6,0,0.125f); break;
        case 4: s[0]->Arm(0,7,700); break;
        case 5: s[0]->Arm(0,10,1); break;
        case 6: s[0]->Arm(0,11); break;
        case 7: s[0]->Arm(0,12,0,0.25f,s[1]); s[0]->Arm(2,6,0,0.25f); break;
        case 8: s[0]->Arm(0,9,0,100); break;
        case 9: s[0]->Arm(0,2,99); break;
        }
        Update(m,0.75f); Dump(m,s,3); m.SetActive(true); Update(m,0); Change(m,1); Update(m,0.75f); Dump(m,s,3);
    }
}

static void Timing()
{
    const uint32_t timeBits[]={0x00000000,0x80000000,0x3e7fffff,0x3e800000,0x3e800001,0x3f000000,0xbe800000,
        0x3eaaaaab,0x00000001,0x7fc12345,0xffc67890,0xff800000};
    for(int i=0;i<12;++i) {
        Begin("timing",i);
        cStateMachine m; cFixtureState* s[]={Add(m,0,0,0.25f),Add(m,1,1,0.5f)};
        Update(m,Float(timeBits[i])); Update(m,0.25f); Change(m,1); Update(m,0.5f); Change(m,0); Update(m,0.125f); Dump(m,s,2);
    }
    const uint32_t stepBits[]={0x00000000,0x80000000,0xbe800000,0x7f800000,0x7fc12345,0x33800000};
    for(int i=0;i<6;++i) {
        Begin("terminating-special-step",i);
        cStateMachine m; cFixtureState* s[]={Add(m,0,0,Float(stepBits[i]))};
        // Native zero/negative/nonprogress loops are not run unbounded: the
        // first callback changes the step to a finite consuming value, or Inf
        // for the Inf>=Inf case (one subtraction yields NaN and stops).
        s[0]->Arm(0,1,0,i==3 ? Float(0x7f800000) : 1.0f);
        Update(m,i==3 ? Float(0x7f800000) : 1.0f); Update(m,0.25f); Dump(m,s,1);
    }
}

static void Random()
{
    for(int trial=0;trial<4096;++trial) {
        Begin("seeded",trial);
        cStateMachine m; cFixtureState* s[]={Add(m,0,0,0.125f),Add(m,1,1,0.25f),Add(m,2,2,0.5f)};
        for(int operation=0;operation<24;++operation) {
            int choice=Int(0,10);
            fprintf(gOut,"operation %d %d\n",operation,choice);
            if(choice<5) Change(m,Int(-1,3));
            else if(choice<8) Update(m,(float)Int(-2,8)*0.125f);
            else if(choice==8) { bool active=Int(0,1)!=0; m.SetActive(active); fprintf(gOut,"active %d\n",active); }
            else if(choice==9) {
                int index=Int(0,2); float step=(float)Int(1,4)*0.125f;
                s[index]->SetStep(step); fprintf(gOut,"step label=%d value=%08x\n",index,Bits(step));
            } else {
                int index=Int(0,2), phase=Int(0,2), action=Int(0,4);
                if(action==0) s[index]->Arm(phase,4,Int(0,2));
                else if(action==1) s[index]->Arm(phase,7,800+phase);
                else if(action==2) s[index]->Arm(phase,5,Int(0,1));
                else if(action==3) s[index]->Arm(phase,1,0,(float)Int(1,4)*0.125f);
                else s[index]->Arm(phase,11);
            }
            Dump(m,s,3);
        }
    }
}

int main(int argc,char** argv)
{
    if(argc!=2) return 2;
    gOut=fopen(argv[1],"wb"); if(!gOut) return 3; gLogOut=gOut;
#if HARNESS_RUST
    RustBridge_Init();
#endif
    Basic(); Reentry(); Timing(); Random();
    fprintf(gOut,"totals machines=%u changes=%u updates=%u callbacks=%u exceptions=%u\n",gMachines,gChanges,gUpdates,gCallbacks,gExceptions);
    fclose(gOut); return 0;
}
