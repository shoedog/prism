export function Badge(text: string) {
  type Label = typeof text;
  const kind = typeof text;
  return <span>{kind}</span>;
}
