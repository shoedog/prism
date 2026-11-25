export function Panel(article: string, title: string) {
  if (!article) return null;
  return <article title={title} />;
}
