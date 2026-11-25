type Builder = new (text: string) => {label: string};
export function Card(Build: Builder, label: string) {
  return <span>{new Build(label).label}</span>;
}
