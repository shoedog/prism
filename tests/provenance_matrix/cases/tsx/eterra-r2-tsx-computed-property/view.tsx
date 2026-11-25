type Props = { field: string; data: Record<string, string> };
export function View({ field, data }: Props) {
  return <span title={data[field]}>ok</span>;
}
