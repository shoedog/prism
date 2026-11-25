type Props = { title?: string };
export function View({ title = 'untitled' }: Props) {
  return <span>{title}</span>;
}
