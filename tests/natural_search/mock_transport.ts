export const calls: { command: string; args: any }[] = [];
export const requests: { args: any; resolve: (filter: any) => void; reject: (error: any) => void }[] = [];
export class Channel { onmessage = () => {}; }
export async function invoke(command: string, args: any = {}) {
  calls.push({ command, args });
  if (command === "parse_search_filter") return new Promise((resolve, reject) => requests.push({ args, resolve, reject }));
  if (command === "load_feeds") return [{ id: 1, title: "技术", feed_url: "https://a.test/rss" }, { id: 2, title: "News", feed_url: "https://b.test/rss" }];
  if (command === "list_tags") return [{ id: 3, name: "Rust" }];
  if (command === "cancel_agent_task" || command === "delete_smart_view") return;
  if (command === "save_smart_view") return 42;
  throw new Error(`Unexpected command ${command}`);
}
