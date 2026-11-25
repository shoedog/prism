export function View({ label }: { label: string }) {
  return ["fixed"].map(label => <span>{label}</span>);
}
