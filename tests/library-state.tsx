import { useEffect } from "react";
import { createRoot } from "react-dom/client";
import { AppProvider, useStore } from "../src/store";
import type { GateState } from "../src/types";
const fixture: { state?: { id?: string; favs: number[]; history: number[] }; select?: (id: string) => void; favorite?: (id: number) => void; capture?: () => void; late?: () => void } = {};
Object.assign(window,{libraryStateFixture:fixture,__TAURI_INTERNALS__:{invoke:async(cmd:string)=>cmd==="gate_state"?{base:"",adapter:"",activated:false}:undefined}});
function Probe(){
  const store=useStore();
  useEffect(()=>{
    fixture.state={id:store.gate.id,favs:store.favs,history:store.history.map(i=>i.id)};
    fixture.select=(id:string)=>store.selectSource({id,name:id,base:"",adapter:"playlist_v1",activated:true} as GateState);
    fixture.favorite=store.toggleFavLocal;
    fixture.capture=()=>{const old=store.addHistory;fixture.late=()=>old(999,1,4,8);};
  });
  return <div>Library-state fixture</div>;
}
createRoot(document.getElementById("root")!).render(<AppProvider><Probe/></AppProvider>);
