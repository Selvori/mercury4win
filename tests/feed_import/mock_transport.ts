export class Channel<T> { onmessage: (message: T) => void = () => {}; }
export let progress: Channel<any> | null = null;
export let finish: (() => void) | null = null;
export async function open() { return "C:/fixture/import.opml"; }
export async function invoke(command: string, args: any) {
  if (command !== "import_opml_stream") throw new Error(`Unexpected IPC ${command}`);
  progress = args.onProgress;
  return new Promise<void>((resolve) => { finish = resolve; });
}
