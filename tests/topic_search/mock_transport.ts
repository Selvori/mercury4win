export const calls: { command: string; args: any }[] = [];
export const requests: { args: any; resolve: (filter: any) => void; reject: (error: any) => void }[] = [];
export let savedResume: any = null;
export function setSavedResume(value: any) { savedResume = value; }
export class Channel { onmessage: (v: string) => void = () => {}; }
export async function invoke(command: string, args: any = {}) {
  calls.push({ command, args });
  if (command === "search_local_topic") return new Promise((resolve, reject) => requests.push({ args, resolve, reject }));
  if (command === "recent_topic_search") return savedResume;
  if (command === "stop_topic_search") return null;
  if (command === "load_feeds") return [{ id: 1, title: "技术来源", feed_url: "https://tech.test/rss" }];
  if (command === "list_tags") return [];
  if (command === "load_entries") {
    const id = args.filter.ai_search_id;
    const quick=id?.startsWith("scan:quick");
    if(id==="scan:quick-empty")return {entries:[],total:0,next_cursor:null,search_info:[]};
    const entry = (i: number) => ({ id: i, title: quick?`Keyword article ${i}`:id ? `AI article ${i}` : `Ordinary article ${i}`, feed_id: 1, created_at: "2026-10-01", is_read: false, is_starred: false });
    return { entries: id ? [entry(args.cursor ? 2 : 1)] : [entry(9)], total: id ? 2 : 1,
      next_cursor: id && !args.cursor ? { id: 1, score: 90, published_at: null } : null,
      search_info: id ? [{ entry_id: args.cursor ? 2 : 1, hits: [], body_available: true, feed_title: "技术来源", ai: quick?{score:4,reason:"release / 语言模型",quote:"Local keyword snippet",limited:true,basis:"keyword_body",parts:0}:{ score: 90, reason: "Direct model release", quote: "Release evidence", limited: false, basis: "full_body", parts: 3 } }] : [] };
  }
  if (["cancel_agent_task", "mark_all_read_in_scope", "mark_read", "mark_starred"].includes(command)) return;
  if (command === "save_smart_view") return 42;
  throw new Error(`Unexpected command ${command}`);
}
