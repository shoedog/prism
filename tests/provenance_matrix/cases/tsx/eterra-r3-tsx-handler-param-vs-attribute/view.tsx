type Props = { title: string };
export function View({ title }: Props) {
  return <div title={title}>{title}</div>;
}
