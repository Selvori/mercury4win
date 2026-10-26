export const calls: { command: string; args: any }[] = [];
export class Channel<T> { onmessage: (value:T) => void = () => {}; }
export let tags: any[] = [];
export let detail: any;
export let saved: any = null;
export let started: any = null;
export let load_hold: (() => void) | null = null;
let hold_load = false;
export let fail_load = false;
export let profile: any = { primary_model_id: "model" };
export let fail_apply = false;
export function reset() {
  calls.length=0; saved=null; started=null; load_hold=null; hold_load=false;fail_load=false;fail_apply=false;
  profile={ primary_model_id: "model" };
  tags=[{id:1,name:"Rust",normalized_name:"rust",usage_count:2,is_provisional:false},{id:2,name:"Intel",normalized_name:"intel",usage_count:1,is_provisional:true}];
  detail={entry:{id:1,title:"Intel Rust",summary:"SQLite"},feed_title:null,tags:[],has_note:false};
}
export function delay_load() { hold_load=true; }
export function snapshot(id="batch", state="ready") {
 return { id,status:state,scope:{from:null,to:"2026-10-13T00:00:00Z",unread_only:false,skip_tagged:true,skip_applied:true},total:3,processed:2,succeeded:1,failures:[{entry_id:2,error:"fixture failure"}],proposals:[{name:"Edge AI",normalized_name:"edge ai",hits:1},{name:"SQLite",normalized_name:"sqlite",hits:1}],result:null };
}
export function complete() { saved=snapshot(started.args.requestId); started.resolve(saved); }
let aliases=[{id:1,tag_id:1,alias_normalized:"rs",alias:"RS"}];
export async function invoke(command:string,args:any={}) {
 calls.push({command,args});
  if (command === "list_smart_views") return [];
 switch(command) {
 case "load_feeds":return [{id:1,title:"Fixture",feed_url:"https://fixture.test",entry_count:3,unread_count:3}];
 case "load_entries":return {entries:[],total:0,next_cursor:null};
 case "list_providers":return [];
 case "list_tags": return tags.filter((t)=>!args.search || t.name.toLowerCase().includes(args.search.toLowerCase()));
 case "list_tag_aliases":return aliases.filter((a)=>a.tag_id===args.tagId);
 case "add_tag_alias": if(args.alias==="Intel")throw new Error("Alias conflicts");aliases.push({id:2,tag_id:args.tagId,alias_normalized:args.alias.toLowerCase(),alias:args.alias});return;
 case "delete_tag_alias":aliases=aliases.filter((a)=>a.id!==args.aliasId);return;
 case "promote_tag":tags.find((t)=>t.id===args.tagId).is_provisional=false;return;
 case "rename_tag":if(args.newName==="Intel")throw new Error("Name conflicts");tags.find((t)=>t.id===args.tagId).name=args.newName;return;
 case "merge_tags": tags=tags.filter((t)=>t.id!==args.sourceId);return;
 case "delete_tag":tags=tags.filter((t)=>t.id!==args.tagId);return;
 case "cleanup_unused_tags":return 0;
 case "get_agent_profile":return profile;
 case "get_agent_status":return {state:"idle",current_entry_id:null,current_request_id:null};
 case "load_entry_detail":return {...detail,entry:{...detail.entry,id:args.entryId}};
 case "local_tag_suggestions":return ["Intel","SQLite"];
 case "related_entries":return [{id:2,title:"Related Rust",feed_id:1}];
 case "add_tag":detail.tags=[...detail.tags,{tag_id:2,name:args.name,source:"manual"}];return {id:2,name:args.name};
 case "remove_tag":detail.tags=detail.tags.filter((t:any)=>t.tag_id!==args.tagId);return;
 case "count_tag_batch":return 3;
 case "load_tag_batch":
   if(fail_load)throw new Error("fixture recovery failed");
   if(hold_load){ hold_load=false;return new Promise((resolve)=>{load_hold=()=>resolve(saved);}); }
   return saved;
 case "batch_tag":return new Promise((resolve,reject)=>{started={args,resolve,reject};args.onEvent.onmessage(JSON.stringify({type:"task_state",request_id:args.requestId,state:"running"}));args.onEvent.onmessage(JSON.stringify({type:"batch_progress",request_id:args.requestId,processed:1,total:3,failed:0}));});
 case "cancel_agent_task":saved=snapshot(started.args.requestId);started.reject(new Error("Agent task cancelled: user_cancelled"));return;
 case "apply_tag_batch":if(fail_apply){saved={...saved,proposals:[{name:"Changed",normalized_name:"changed",hits:1}]};fail_apply=false;throw new Error("Review every current new proposal");}saved={...saved,status:"done",result:{entries_processed:1,tags_assigned:1,skipped_deleted:1,skipped_changed:1}};return saved.result;
 case "discard_tag_batch":saved=null;return;
 case "abandon_agent_waiting":return;
 default:throw new Error(`Unexpected command ${command}`);
 }
}
reset();

export function no_model(){profile=null;}
export function restore_stale(){saved=snapshot();fail_apply=true;}
export function recovery_failure(value:boolean){fail_load=value;}
