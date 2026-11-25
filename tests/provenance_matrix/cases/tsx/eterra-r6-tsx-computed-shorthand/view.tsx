type Props = { label: string; keyName: string };
export function View({label, keyName}: Props) {
  return <div data-values={JSON.stringify({[keyName]: label, label})} />;
}
