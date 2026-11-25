type Entry = { label: string };
function Frame<T>({ children }: { children: string }) {
  return <span>{children}</span>;
}
export function View(Entry: Entry) {
  return <Frame<Entry>>{Entry.label}</Frame>;
}
