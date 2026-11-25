export {};
declare const settings: { get(name: string): string };
function Badge() {
  const caption = settings.get('caption');
  return <b>{caption}</b>;
}
